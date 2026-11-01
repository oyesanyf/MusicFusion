use music_core::genre::GenrePreset;
use music_core::state::MusicProductionState;
use music_fusion::{
    parse_judge_json, ExecutionRoute, OllamaClient, OllamaProducerPanel, PipelineStageKind,
    SystemMemory,
};

#[test]
fn test_live_memory_detection() {
    let mem = SystemMemory::detect_live();
    assert!(mem.total_ram_gb > 0.0);
    assert!(mem.free_ram_gb > 0.0);
    assert!(mem.cpu_cores >= 1);
    assert!(mem.free_ram_gb <= mem.total_ram_gb);
}

#[test]
fn test_dynamic_available_memory_law_tiers() {
    // Tier 1: >= 48 GB RAM -> qwen2.5:32b
    let mem32 = SystemMemory::simulated(64.0, 52.0, 16, None, 0.0, 0.0);
    assert_eq!(mem32.recommend_ollama_model(&[]), "qwen2.5:32b");

    // Tier 2: >= 24 GB RAM -> qwen2.5:14b
    let mem14 = SystemMemory::simulated(32.0, 26.0, 12, None, 0.0, 0.0);
    assert_eq!(mem14.recommend_ollama_model(&[]), "qwen2.5:14b");

    // Tier 3: >= 12 GB RAM -> qwen2.5:7b
    let mem7 = SystemMemory::simulated(16.0, 13.0, 8, None, 0.0, 0.0);
    assert_eq!(mem7.recommend_ollama_model(&[]), "qwen2.5:7b");

    // Tier 4: >= 6 GB RAM -> qwen2.5:3b
    let mem3 = SystemMemory::simulated(8.0, 6.5, 4, None, 0.0, 0.0);
    assert_eq!(mem3.recommend_ollama_model(&[]), "qwen2.5:3b");

    // Tier 5: < 6 GB RAM -> qwen2.5:1.5b
    let mem1 = SystemMemory::simulated(4.0, 3.2, 2, None, 0.0, 0.0);
    assert_eq!(mem1.recommend_ollama_model(&[]), "qwen2.5:1.5b");
}

#[test]
fn test_specialized_model_gating() {
    // Flint requires 8 GB RAM or 4 GB VRAM
    let mem_flint = SystemMemory::simulated(16.0, 10.0, 8, None, 0.0, 0.0);
    let tags = vec!["flint:latest".to_string(), "qwen2.5:7b".to_string()];
    assert_eq!(mem_flint.recommend_ollama_model(&tags), "flint:latest");

    // GLM4 requires 8 GB RAM or 6 GB VRAM
    let mem_low = SystemMemory::simulated(8.0, 4.0, 4, None, 0.0, 0.0);
    let glm_tags = vec!["glm4:9b".to_string(), "qwen2.5:1.5b".to_string()];
    // Low memory cannot run GLM-4, falls back to 1.5b
    assert_eq!(mem_low.recommend_ollama_model(&glm_tags), "qwen2.5:1.5b");
}

#[test]
fn test_stage_routing_gpu_vs_dsp() {
    // High VRAM: 16 GB free VRAM (budget 13.6 GB) -> GPU route for YuE
    let mem_gpu = SystemMemory::simulated(32.0, 20.0, 16, Some("RTX 4090".into()), 24.0, 16.0);
    assert_eq!(
        mem_gpu.route_stage(PipelineStageKind::Stage2YuE, "auto"),
        ExecutionRoute::HuggingFaceGpu
    );

    // Low VRAM: 2 GB free VRAM (budget 1.7 GB) -> PureRustDsp route for YuE (needs 8 GB)
    let mem_low_vram = SystemMemory::simulated(32.0, 20.0, 16, Some("GTX 1650".into()), 4.0, 2.0);
    assert_eq!(
        mem_low_vram.route_stage(PipelineStageKind::Stage2YuE, "auto"),
        ExecutionRoute::PureRustDsp
    );
    assert!(mem_low_vram.should_use_dsp(PipelineStageKind::Stage2YuE, "auto"));
}

#[test]
fn test_stage_routing_force_dsp_override() {
    let mem = SystemMemory::simulated(64.0, 48.0, 16, Some("RTX 4090".into()), 24.0, 20.0);

    // 1. Engine mode "pure_rust" forces PureRustDsp even with 24 GB high-end GPU
    assert_eq!(
        mem.route_stage(PipelineStageKind::Stage1Groove, "pure_rust"),
        ExecutionRoute::PureRustDsp
    );
    assert!(mem.should_use_dsp(PipelineStageKind::Stage1Groove, "pure_rust"));

    // 2. with_force_dsp(true) flag override forces PureRustDsp under "auto" mode without mutating global process env
    let mem_forced = mem.with_force_dsp(true);
    assert_eq!(
        mem_forced.route_stage(PipelineStageKind::Stage1Groove, "auto"),
        ExecutionRoute::PureRustDsp
    );
    assert!(mem_forced.should_use_dsp(PipelineStageKind::Stage1Groove, "auto"));

    // 3. with_force_hf(true) forces HuggingFaceCpu when GPU is absent and RAM budget is sufficient
    let mem_no_gpu = SystemMemory::simulated(64.0, 48.0, 16, None, 0.0, 0.0).with_force_hf(true);
    assert_eq!(
        mem_no_gpu.route_stage(PipelineStageKind::Stage1Groove, "auto"),
        ExecutionRoute::HuggingFaceCpu
    );
}

#[test]
fn test_judge_json_parsing_with_think_tags() {
    let raw = "<think>The user wants a roots reggae anthem in A Major. Let's arrange it.</think>\
    {\
        \"title\": \"One Love Calling\",\
        \"genre_tags\": \"Roots Reggae, 1977, 76 BPM, A Major, Raspy Tenor\",\
        \"bpm\": 76.0,\
        \"key_signature\": \"A Major\",\
        \"time_signature\": \"4/4\",\
        \"lyrics_text\": \"[intro]\\n\\n[verse]\\nSun is shining.\",\
        \"arrangement_map\": [\
            {\
                \"section_id\": \"intro\",\
                \"display_label\": \"Intro\",\
                \"yue_clean_tag\": \"[intro]\",\
                \"bars\": 4,\
                \"chords\": [\"A\", \"E\"],\
                \"production_cue\": \"One drop\",\
                \"lyrics\": [],\
                \"lead_instrument_intensity\": 0.6,\
                \"drum_fill_complexity\": 0.5,\
                \"bass_weight\": 0.8\
            }\
        ],\
        \"judge_score\": 9.2,\
        \"critique\": \"Excellent anthem\"\
    }";

    let genre = GenrePreset::roots_reggae_1977();
    let bp = parse_judge_json(raw, &genre, "human compassion");
    assert_eq!(bp.title, "One Love Calling");
    assert_eq!(bp.bpm, 76.0);
    assert_eq!(bp.judge_score, 9.2);
    assert_eq!(bp.arrangement_map.len(), 1);
}

#[test]
fn test_judge_json_parsing_with_markdown_fences() {
    let raw = "```json\n{\
        \"title\": \"Afrobeats Groove\",\
        \"genre_tags\": \"Afrobeats, 104 BPM, F# Minor\",\
        \"bpm\": 104.0,\
        \"key_signature\": \"F# Minor\",\
        \"time_signature\": \"4/4\",\
        \"lyrics_text\": \"[intro]\\n\\n[verse]\\nDance all night.\",\
        \"arrangement_map\": [\
            {\
                \"section_id\": \"verse_1\",\
                \"display_label\": \"Verse 1\",\
                \"yue_clean_tag\": \"[verse]\",\
                \"bars\": 8,\
                \"chords\": [\"F#m\", \"D\"],\
                \"production_cue\": \"Tresillo drum groove\",\
                \"lyrics\": [\"Dance all night.\"],\
                \"lead_instrument_intensity\": 0.4,\
                \"drum_fill_complexity\": 0.6,\
                \"bass_weight\": 0.9\
            }\
        ],\
        \"judge_score\": 8.8,\
        \"critique\": \"Punchy rhythms\"\
    }\n```";

    let genre = GenrePreset::afrobeats();
    let bp = parse_judge_json(raw, &genre, "celebration");
    assert_eq!(bp.title, "Afrobeats Groove");
    assert_eq!(bp.bpm, 104.0);
}

#[test]
fn test_judge_json_fail_safe_fallback() {
    let malformed = "I am an AI that forgot how to output JSON. Sorry!";
    let genre = GenrePreset::roots_reggae_1977();
    let bp = parse_judge_json(malformed, &genre, "One love and compassion");

    assert!(bp.title.contains("Roots Reggae"));
    assert_eq!(bp.bpm, 76.0);
    assert!(!bp.arrangement_map.is_empty());
    assert!(bp.lyrics_text.contains("[intro]"));
    assert!(bp.judge_score >= 8.0);
}

#[test]
fn test_missing_model_guidance() {
    let client = OllamaClient::new("http://127.0.0.1:11434");
    let installed = vec!["qwen2.5:7b".to_string(), "mistral:latest".to_string()];
    let guidance = client.get_missing_model_guidance("qwen2.5:32b", &installed);

    assert_eq!(guidance.missing_model, "qwen2.5:32b");
    assert_eq!(guidance.pull_command, "ollama pull qwen2.5:32b");
    assert_eq!(guidance.recommended_fallback, Some("qwen2.5:7b".to_string()));
}

#[tokio::test]
async fn test_probe_health() {
    let client = OllamaClient::new("http://127.0.0.1:11434");
    let is_online = client.probe_health().await;
    println!("IS_ONLINE: {}", is_online);
}

#[tokio::test]
async fn test_list_installed_models() {
    let client = OllamaClient::new("http://127.0.0.1:11434");
    let models = client.list_installed_tags().await.unwrap_or_default();
    println!("INSTALLED_MODELS: {:?}", models);
}

#[tokio::test]
async fn test_producer_panel_offline_fallback() {
    let client = OllamaClient::new("http://127.0.0.1:54321");
    let panel = OllamaProducerPanel::new(
        client,
        "qwen2.5:7b",
        "qwen2.5:7b",
        "qwen2.5:7b",
        "qwen2.5:7b",
    );
    let mut state = MusicProductionState::default_reggae_anthem();
    let output = panel.run_panel(&mut state).await.expect("Panel execution failed");

    assert!(!output.yue_genre_txt.is_empty());
    assert!(!output.yue_lyrics_txt.is_empty());
    assert!(state.quality_passed);
    assert_eq!(state.bpm, 76);
}
