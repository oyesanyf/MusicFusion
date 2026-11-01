//! Adversarial Empirical Stress Test Suite for `crates/music_fusion`.
//! Validates memory detection, Dynamic Available Memory Law tiers, specialized model gating,
//! stage execution routing across all 5 stages, JudgeEvaluator resilience under corrupted/nested
//! `<think>` tags and malformed markdown fences, and zero-crash offline fallbacks.

use music_core::genre::{GenrePreset, GenreTaxonomy};
use music_core::state::MusicProductionState;
use music_fusion::judge::{parse_judge_json, EvaluatedSongBlueprint, JudgeEvaluator, SectionArrangement};
use music_fusion::memory::{ExecutionRoute, PipelineStageKind, SystemMemory};
use music_fusion::ollama_client::OllamaClient;
use music_fusion::panel::OllamaProducerPanel;
use std::time::{Duration, Instant};

// ============================================================================
// PART 1: SYSTEM MEMORY LIVE DETECTION & SIMULATED BOUNDARIES
// ============================================================================

#[test]
fn test_stress_live_memory_detection_invariants() {
    let mem = SystemMemory::detect_live();

    // RAM invariants
    assert!(mem.total_ram_gb > 0.0, "Total RAM must be positive, got {}", mem.total_ram_gb);
    assert!(mem.free_ram_gb > 0.0, "Free RAM must be positive, got {}", mem.free_ram_gb);
    assert!(
        mem.free_ram_gb <= mem.total_ram_gb,
        "Free RAM ({}) cannot exceed total RAM ({})",
        mem.free_ram_gb, mem.total_ram_gb
    );
    assert!(mem.cpu_cores >= 1, "CPU cores must be >= 1, got {}", mem.cpu_cores);

    // VRAM invariants
    assert!(
        mem.gpu_vram_free_gb <= mem.gpu_vram_total_gb,
        "Free VRAM ({}) cannot exceed total VRAM ({})",
        mem.gpu_vram_free_gb, mem.gpu_vram_total_gb
    );
    assert_eq!(
        mem.has_usable_gpu(),
        mem.gpu_name.is_some() && mem.gpu_vram_free_gb > 1.0,
        "has_usable_gpu must be true iff GPU name is Some and free VRAM > 1.0 GB"
    );

    // Budget calculations
    let gpu_budget = mem.gpu_budget_gb();
    assert!(gpu_budget >= 0.0, "GPU budget must be >= 0.0, got {}", gpu_budget);
    assert!(
        gpu_budget <= mem.gpu_vram_free_gb,
        "GPU budget ({}) cannot exceed free VRAM ({})",
        gpu_budget, mem.gpu_vram_free_gb
    );

    let ram_budget = mem.ram_budget_gb();
    assert!(ram_budget >= 0.0, "RAM budget must be >= 0.0, got {}", ram_budget);

    assert_eq!(mem.free_vram_mb(), mem.gpu_vram_free_gb * 1024.0);
}

#[test]
fn test_stress_simulated_gpu_and_fallback_configurations() {
    // 1. High-end RTX 4090 (24 GB VRAM total, 20 GB free, 64 GB RAM total, 48 GB free)
    let rtx4090 = SystemMemory::simulated(64.0, 48.0, 16, Some("NVIDIA GeForce RTX 4090".into()), 24.0, 20.0);
    assert!(rtx4090.has_usable_gpu());
    assert!((rtx4090.gpu_budget_gb() - 17.0).abs() < 1e-4);
    assert!((rtx4090.ram_budget_gb() - (45.0 * 0.70)).abs() < 1e-4);

    // 2. Budget GPU with low free VRAM (< 1.0 GB free) -> Usable GPU must be false
    let low_vram = SystemMemory::simulated(16.0, 10.0, 8, Some("GTX 1050".into()), 4.0, 0.8);
    assert!(!low_vram.has_usable_gpu(), "GPU with < 1.0 GB free VRAM must not be flagged usable");

    // 3. System with no discrete GPU (Integrated graphics / CPU only)
    let no_gpu = SystemMemory::simulated(32.0, 20.0, 8, None, 0.0, 0.0);
    assert!(!no_gpu.has_usable_gpu());
    assert_eq!(no_gpu.gpu_budget_gb(), 0.0);
    assert_eq!(no_gpu.free_vram_mb(), 0.0);

    // 4. Extreme zero memory boundary condition
    let zero_mem = SystemMemory::simulated(0.0, 0.0, 0, None, 0.0, 0.0);
    assert_eq!(zero_mem.gpu_budget_gb(), 0.0);
    assert_eq!(zero_mem.ram_budget_gb(), 0.0);
    assert_eq!(zero_mem.recommend_ollama_model(&[]), "qwen2.5:1.5b");
}

// ============================================================================
// PART 2: DYNAMIC AVAILABLE MEMORY LAW & SPECIALIZED GATING
// ============================================================================

#[test]
fn test_stress_dynamic_available_memory_law_all_five_tiers() {
    // TIER 1: Free RAM >= 48 GB OR Free VRAM >= 24 GB -> qwen2.5:32b
    // A: RAM threshold met (48 GB RAM, 0 VRAM)
    let m1_ram = SystemMemory::simulated(64.0, 48.0, 16, None, 0.0, 0.0);
    assert_eq!(m1_ram.recommend_ollama_model(&[]), "qwen2.5:32b");
    // B: VRAM threshold met (8 GB RAM, 24 GB VRAM)
    let m1_vram = SystemMemory::simulated(16.0, 8.0, 8, Some("RTX 4090".into()), 24.0, 24.0);
    assert_eq!(m1_vram.recommend_ollama_model(&[]), "qwen2.5:32b");

    // TIER 2: Free RAM >= 24 GB OR Free VRAM >= 14 GB -> qwen2.5:14b
    // A: RAM threshold met (24 GB RAM, 0 VRAM)
    let m2_ram = SystemMemory::simulated(32.0, 24.0, 8, None, 0.0, 0.0);
    assert_eq!(m2_ram.recommend_ollama_model(&[]), "qwen2.5:14b");
    // B: VRAM threshold met (8 GB RAM, 14 GB VRAM)
    let m2_vram = SystemMemory::simulated(16.0, 8.0, 8, Some("RTX 3090".into()), 24.0, 14.0);
    assert_eq!(m2_vram.recommend_ollama_model(&[]), "qwen2.5:14b");
    // C: Just below Tier 1 boundary (47.99 GB RAM, 23.99 GB VRAM)
    let m2_edge = SystemMemory::simulated(64.0, 47.99, 16, Some("GPU".into()), 24.0, 23.99);
    assert_eq!(m2_edge.recommend_ollama_model(&[]), "qwen2.5:14b");

    // TIER 3: Free RAM >= 12 GB OR Free VRAM >= 4.5 GB -> qwen2.5:7b
    // A: RAM threshold met (12 GB RAM, 0 VRAM)
    let m3_ram = SystemMemory::simulated(16.0, 12.0, 8, None, 0.0, 0.0);
    assert_eq!(m3_ram.recommend_ollama_model(&[]), "qwen2.5:7b");
    // B: VRAM threshold met (8 GB RAM, 4.5 GB VRAM)
    let m3_vram = SystemMemory::simulated(16.0, 8.0, 8, Some("RTX 3060".into()), 12.0, 4.5);
    assert_eq!(m3_vram.recommend_ollama_model(&[]), "qwen2.5:7b");
    // C: Just below Tier 2 boundary (23.99 GB RAM, 13.99 GB VRAM)
    let m3_edge = SystemMemory::simulated(32.0, 23.99, 8, Some("GPU".into()), 16.0, 13.99);
    assert_eq!(m3_edge.recommend_ollama_model(&[]), "qwen2.5:7b");

    // TIER 4: Free RAM >= 6 GB OR Free VRAM >= 2.0 GB -> qwen2.5:3b
    // A: RAM threshold met (6 GB RAM, 0 VRAM)
    let m4_ram = SystemMemory::simulated(8.0, 6.0, 4, None, 0.0, 0.0);
    assert_eq!(m4_ram.recommend_ollama_model(&[]), "qwen2.5:3b");
    // B: VRAM threshold met (4 GB RAM, 2.0 GB VRAM)
    let m4_vram = SystemMemory::simulated(8.0, 4.0, 4, Some("GTX 1650".into()), 4.0, 2.0);
    assert_eq!(m4_vram.recommend_ollama_model(&[]), "qwen2.5:3b");
    // C: Just below Tier 3 boundary (11.99 GB RAM, 4.49 GB VRAM)
    let m4_edge = SystemMemory::simulated(16.0, 11.99, 8, Some("GPU".into()), 8.0, 4.49);
    assert_eq!(m4_edge.recommend_ollama_model(&[]), "qwen2.5:3b");

    // TIER 5: Free RAM < 6 GB AND Free VRAM < 2.0 GB -> qwen2.5:1.5b
    // A: Low RAM and zero VRAM (5.99 GB RAM, 0 VRAM)
    let m5_edge = SystemMemory::simulated(8.0, 5.99, 4, None, 0.0, 0.0);
    assert_eq!(m5_edge.recommend_ollama_model(&[]), "qwen2.5:1.5b");
    // B: Very low RAM (1.5 GB RAM, 0 VRAM)
    let m5_starved = SystemMemory::simulated(4.0, 1.5, 2, None, 0.0, 0.0);
    assert_eq!(m5_starved.recommend_ollama_model(&[]), "qwen2.5:1.5b");
}

#[test]
fn test_stress_anti_oom_no_coder_suffix_ever_emitted() {
    let memory_levels = [
        SystemMemory::simulated(64.0, 50.0, 16, None, 0.0, 0.0),
        SystemMemory::simulated(32.0, 25.0, 8, None, 0.0, 0.0),
        SystemMemory::simulated(16.0, 13.0, 8, None, 0.0, 0.0),
        SystemMemory::simulated(8.0, 6.5, 4, None, 0.0, 0.0),
        SystemMemory::simulated(4.0, 3.0, 2, None, 0.0, 0.0),
    ];

    for mem in &memory_levels {
        let model = mem.recommend_ollama_model(&[]);
        assert!(!model.contains("-coder"), "Emitted forbidden -coder suffix: {}", model);
        assert!(!model.contains("coder"), "Emitted forbidden coder model: {}", model);
    }
}

#[test]
fn test_stress_specialized_model_gating_flint_and_glm4() {
    let installed_all = vec![
        "flint:latest".to_string(),
        "glm4:9b".to_string(),
        "qwen2.5:7b".to_string(),
        "qwen2.5:3b".to_string(),
        "qwen2.5:1.5b".to_string(),
    ];

    // Case 1: Flint requires >= 8.0 GB RAM or >= 4.0 GB VRAM.
    // If RAM is 14.0 GB, Tier 3 preferred is 7b, matching direct match step 1
    let mem_flint_ram = SystemMemory::simulated(16.0, 14.0, 8, None, 0.0, 0.0);
    assert_eq!(mem_flint_ram.recommend_ollama_model(&installed_all), "qwen2.5:7b");

    // When preferred tier (3b) is preferred, but Flint is available and memory is >= 8 GB RAM:
    let installed_flint_only = vec!["flint:latest".to_string(), "qwen2.5:1.5b".to_string()];
    let mem_flint_ok = SystemMemory::simulated(16.0, 8.0, 8, None, 0.0, 0.0);
    assert_eq!(mem_flint_ok.recommend_ollama_model(&installed_flint_only), "flint:latest");

    // Case 2: Flint via VRAM (4.0 GB VRAM meets Flint requirement)
    let mem_flint_vram = SystemMemory::simulated(8.0, 4.0, 4, Some("GPU".into()), 6.0, 4.0);
    assert_eq!(mem_flint_vram.recommend_ollama_model(&installed_flint_only), "flint:latest");

    // Case 3: Flint rejected when RAM < 8.0 and VRAM < 4.0
    let mem_flint_denied = SystemMemory::simulated(8.0, 7.9, 4, Some("GPU".into()), 4.0, 3.9);
    assert_eq!(mem_flint_denied.recommend_ollama_model(&installed_flint_only), "qwen2.5:1.5b");

    // Case 4: GLM-4 requires >= 8.0 GB RAM or >= 6.0 GB VRAM.
    let installed_glm_only = vec!["glm4:9b".to_string(), "qwen2.5:1.5b".to_string()];
    // RAM >= 8.0 -> GLM-4 allowed
    let mem_glm_ram = SystemMemory::simulated(16.0, 8.0, 8, None, 0.0, 0.0);
    assert_eq!(mem_glm_ram.recommend_ollama_model(&installed_glm_only), "glm4:9b");

    // VRAM >= 6.0 -> GLM-4 allowed
    let mem_glm_vram = SystemMemory::simulated(8.0, 4.0, 4, Some("GPU".into()), 8.0, 6.0);
    assert_eq!(mem_glm_vram.recommend_ollama_model(&installed_glm_only), "glm4:9b");

    // VRAM = 5.9, RAM = 7.9 -> GLM-4 denied, falls back to 1.5b
    let mem_glm_denied = SystemMemory::simulated(8.0, 7.9, 4, Some("GPU".into()), 8.0, 5.9);
    assert_eq!(mem_glm_denied.recommend_ollama_model(&installed_glm_only), "qwen2.5:1.5b");
}

// ============================================================================
// PART 3: EXECUTION ROUTE DECISIONS ACROSS ALL 5 STAGES
// ============================================================================

#[test]
fn test_stress_execution_routing_all_stages_under_auto_mode() {
    let stages = [
        (PipelineStageKind::Stage1Groove, 6.0, 12.0),
        (PipelineStageKind::Stage2YuE, 8.0, 24.0),
        (PipelineStageKind::Stage3VoiceConversion, 4.0, 8.0),
        (PipelineStageKind::Stage4Inpainting, 8.0, 16.0),
        (PipelineStageKind::Stage5Mastering, 4.0, 8.0),
    ];

    // Verify stage memory constants
    for (stage, vram, ram) in &stages {
        assert_eq!(stage.required_vram_gb(), *vram);
        assert_eq!(stage.required_ram_gb(), *ram);
    }

    // High VRAM system (16 GB free VRAM -> budget 13.6 GB): All 5 stages get HuggingFaceGpu
    let mem_gpu_high = SystemMemory::simulated(32.0, 20.0, 16, Some("RTX 4080".into()), 16.0, 16.0);
    for (stage, _, _) in &stages {
        assert_eq!(
            mem_gpu_high.route_stage(*stage, "auto"),
            ExecutionRoute::HuggingFaceGpu,
            "Stage {:?} should route to GPU under 16 GB free VRAM", stage
        );
        assert!(!mem_gpu_high.should_use_dsp(*stage, "auto"));
    }

    // Mid VRAM system (5.5 GB free VRAM -> budget 4.675 GB):
    // Stage 3 (needs 4.0) and Stage 5 (needs 4.0) -> HuggingFaceGpu
    // Stage 1 (needs 6.0), Stage 2 (needs 8.0), Stage 4 (needs 8.0) -> PureRustDsp
    let mem_gpu_mid = SystemMemory::simulated(16.0, 10.0, 8, Some("RTX 3060".into()), 12.0, 5.5);
    assert_eq!(mem_gpu_mid.route_stage(PipelineStageKind::Stage1Groove, "auto"), ExecutionRoute::PureRustDsp);
    assert_eq!(mem_gpu_mid.route_stage(PipelineStageKind::Stage2YuE, "auto"), ExecutionRoute::PureRustDsp);
    assert_eq!(mem_gpu_mid.route_stage(PipelineStageKind::Stage3VoiceConversion, "auto"), ExecutionRoute::HuggingFaceGpu);
    assert_eq!(mem_gpu_mid.route_stage(PipelineStageKind::Stage4Inpainting, "auto"), ExecutionRoute::PureRustDsp);
    assert_eq!(mem_gpu_mid.route_stage(PipelineStageKind::Stage5Mastering, "auto"), ExecutionRoute::HuggingFaceGpu);

    // Zero GPU / Low VRAM system (0.8 GB free VRAM): All stages default to PureRustDsp
    let mem_no_gpu = SystemMemory::simulated(16.0, 10.0, 8, None, 0.0, 0.0);
    for (stage, _, _) in &stages {
        assert_eq!(
            mem_no_gpu.route_stage(*stage, "auto"),
            ExecutionRoute::PureRustDsp,
            "Stage {:?} must route to PureRustDsp when GPU is absent", stage
        );
        assert!(mem_no_gpu.should_use_dsp(*stage, "auto"));
    }
}

#[test]
fn test_stress_pure_rust_and_env_force_overrides() {
    let mem_rtx4090 = SystemMemory::simulated(64.0, 48.0, 16, Some("RTX 4090".into()), 24.0, 24.0);
    let all_stages = [
        PipelineStageKind::Stage1Groove,
        PipelineStageKind::Stage2YuE,
        PipelineStageKind::Stage3VoiceConversion,
        PipelineStageKind::Stage4Inpainting,
        PipelineStageKind::Stage5Mastering,
    ];

    // Engine mode "pure_rust" forces PureRustDsp even with 24 GB free VRAM
    for stage in &all_stages {
        assert_eq!(
            mem_rtx4090.route_stage(*stage, "pure_rust"),
            ExecutionRoute::PureRustDsp
        );
        assert!(mem_rtx4090.should_use_dsp(*stage, "pure_rust"));
    }

    // with_force_dsp(true) flag override forces PureRustDsp regardless of engine_mode without mutating global process environment
    let mem_forced = mem_rtx4090.clone().with_force_dsp(true);
    for stage in &all_stages {
        assert_eq!(
            mem_forced.route_stage(*stage, "auto"),
            ExecutionRoute::PureRustDsp
        );
        assert_eq!(
            mem_forced.route_stage(*stage, "hf_hybrid"),
            ExecutionRoute::PureRustDsp
        );
        assert!(mem_forced.should_use_dsp(*stage, "auto"));
    }
}

#[test]
fn test_stress_hf_hybrid_cpu_offload_fallback() {
    // No GPU, but ample RAM (40.0 GB free -> RAM budget = (40.0 - 3.0) * 0.70 = 25.9 GB)
    let mem_cpu_heavy = SystemMemory::simulated(64.0, 40.0, 16, None, 0.0, 0.0);
    // Stage 2 YuE needs 24.0 GB RAM <= 25.9 GB budget -> HuggingFaceCpu
    assert_eq!(
        mem_cpu_heavy.route_stage(PipelineStageKind::Stage2YuE, "hf_hybrid"),
        ExecutionRoute::HuggingFaceCpu
    );

    // No GPU, limited RAM (10.0 GB free -> RAM budget = (10.0 - 3.0) * 0.70 = 4.9 GB)
    let mem_cpu_low = SystemMemory::simulated(16.0, 10.0, 8, None, 0.0, 0.0);
    // Stage 2 YuE needs 24.0 GB RAM > 4.9 GB budget -> fails closed to PureRustDsp
    assert_eq!(
        mem_cpu_low.route_stage(PipelineStageKind::Stage2YuE, "hf_hybrid"),
        ExecutionRoute::PureRustDsp
    );
}

// ============================================================================
// PART 4: JUDGE EVALUATOR & ADVERSARIAL JSON SANITIZATION
// ============================================================================

#[test]
fn test_stress_judge_json_deeply_nested_and_corrupted_think_tags() {
    let genre = GenrePreset::roots_reggae_1977();

    // 1. Unclosed <think> tag
    let unclosed = "<think>I am reasoning about roots reggae in A Major without closing the tag... \
    {\"title\": \"Unclosed Thought Anthem\", \"genre_tags\": \"Roots Reggae, 76 BPM\", \"bpm\": 76.0, \
    \"key_signature\": \"A Major\", \"time_signature\": \"4/4\", \"lyrics_text\": \"[intro]\\nOne love\", \
    \"arrangement_map\": [{\"section_id\": \"intro\", \"display_label\": \"Intro\", \"yue_clean_tag\": \"[intro]\", \
    \"bars\": 4, \"chords\": [\"A\"], \"production_cue\": \"Cue\", \"lyrics\": [], \"lead_instrument_intensity\": 0.5, \
    \"drum_fill_complexity\": 0.5, \"bass_weight\": 0.5}], \"judge_score\": 8.5, \"critique\": \"Good\"}";
    let bp_unclosed = parse_judge_json(unclosed, &genre, "human compassion");
    assert!(!bp_unclosed.title.is_empty());
    assert!(bp_unclosed.bpm > 0.0);
    assert!(!bp_unclosed.arrangement_map.is_empty());

    // 2. Deeply nested <think> tags (10 levels)
    let nested_thinks = "<think><think><think><think><think><think><think><think><think><think>\
    Deeply nested chain of thought reasoning \
    </think></think></think></think></think></think></think></think></think></think>\
    {\"title\": \"Nested Thought Anthem\", \"genre_tags\": \"Roots Reggae, 76 BPM\", \"bpm\": 76.0, \
    \"key_signature\": \"A Major\", \"time_signature\": \"4/4\", \"lyrics_text\": \"[intro]\\nBlessed\", \
    \"arrangement_map\": [{\"section_id\": \"intro\", \"display_label\": \"Intro\", \"yue_clean_tag\": \"[intro]\", \
    \"bars\": 4, \"chords\": [\"A\"], \"production_cue\": \"Cue\", \"lyrics\": [], \"lead_instrument_intensity\": 0.5, \
    \"drum_fill_complexity\": 0.5, \"bass_weight\": 0.5}], \"judge_score\": 9.0, \"critique\": \"Deep\"}";
    let bp_nested = parse_judge_json(nested_thinks, &genre, "human compassion");
    assert!(!bp_nested.title.is_empty());
    assert!(!bp_nested.arrangement_map.is_empty());

    // 3. Inverted think tags: closing tag before opening tag
    let inverted = "</think><think>inside thought</think>{\"title\": \"Inverted Anthem\", \
    \"genre_tags\": \"Reggae\", \"bpm\": 76.0, \"key_signature\": \"A\", \"time_signature\": \"4/4\", \
    \"lyrics_text\": \"[verse]\\nPeace\", \"arrangement_map\": [{\"section_id\": \"v\", \"display_label\": \"V\", \
    \"yue_clean_tag\": \"[verse]\", \"bars\": 8, \"chords\": [\"A\"], \"production_cue\": \"Cue\", \"lyrics\": [], \
    \"lead_instrument_intensity\": 0.5, \"drum_fill_complexity\": 0.5, \"bass_weight\": 0.5}], \
    \"judge_score\": 8.0, \"critique\": \"Inverted\"}";
    let bp_inverted = parse_judge_json(inverted, &genre, "peace");
    assert!(!bp_inverted.title.is_empty());

    // 4. Repeated sequential think tags
    let multiple_thinks = "<think>Block 1</think><think>Block 2</think><think>Block 3</think>\
    {\"title\": \"Multi Block\", \"genre_tags\": \"Reggae\", \"bpm\": 76.0, \"key_signature\": \"A\", \
    \"time_signature\": \"4/4\", \"lyrics_text\": \"[intro]\\nHello\", \"arrangement_map\": [\
    {\"section_id\": \"intro\", \"display_label\": \"Intro\", \"yue_clean_tag\": \"[intro]\", \"bars\": 4, \
    \"chords\": [\"A\"], \"production_cue\": \"Cue\", \"lyrics\": [], \"lead_instrument_intensity\": 0.5, \
    \"drum_fill_complexity\": 0.5, \"bass_weight\": 0.5}], \"judge_score\": 8.2, \"critique\": \"Multi\"}";
    let bp_multi = parse_judge_json(multiple_thinks, &genre, "unity");
    assert!(!bp_multi.title.is_empty());
}

#[test]
fn test_stress_judge_json_adversarial_markdown_and_outer_chatter() {
    let genre = GenrePreset::afrobeats();

    // 1. Markdown code fence with conversational preamble and postscript
    let chatter = "Sure, here is the full JSON production blueprint for your Afrobeats track:\n\n\
    ```json\n{\
        \"title\": \"Lagos Highlife Celebration\",\
        \"genre_tags\": \"Afrobeats, Amapiano, 105 BPM, Key of F# Minor, Smooth R&B\",\
        \"bpm\": 105.0,\
        \"key_signature\": \"F# Minor\",\
        \"time_signature\": \"4/4\",\
        \"lyrics_text\": \"[intro]\\n\\n[verse 1]\\nCelebrate tonight.\\n\\n[chorus]\\nJoy in our hearts.\\n\\n[outro]\",\
        \"arrangement_map\": [\
            {\
                \"section_id\": \"intro\",\
                \"display_label\": \"Intro\",\
                \"yue_clean_tag\": \"[intro]\",\
                \"bars\": 4,\
                \"chords\": [\"F#m\", \"D\"],\
                \"production_cue\": \"Log drum syncopation\",\
                \"lyrics\": [],\
                \"lead_instrument_intensity\": 0.4,\
                \"drum_fill_complexity\": 0.5,\
                \"bass_weight\": 0.9\
            }\
        ],\
        \"judge_score\": 9.1,\
        \"critique\": \"Authentic Afro-fusion rhythm\"\
    }\n```\n\nI hope this meets all your requirements! Let me know if you need any adjustments.";

    let bp = parse_judge_json(chatter, &genre, "joyful celebration in Lagos");
    assert_eq!(bp.title, "Lagos Highlife Celebration");
    assert_eq!(bp.bpm, 105.0);
    assert_eq!(bp.key_signature, "F# Minor");
    assert_eq!(bp.judge_score, 9.1);
    assert_eq!(bp.to_yue_genre_txt(), "Afrobeats, Amapiano, 105 BPM, Key of F# Minor, Smooth R&B.\n");
    assert!(bp.to_yue_lyrics_txt().contains("[intro]"));

    // 2. Unclosed markdown code fence
    let unclosed_fence = "```json\n{\
        \"title\": \"Unclosed Fence Song\",\
        \"genre_tags\": \"Afrobeats\",\
        \"bpm\": 105.0,\
        \"key_signature\": \"F# Minor\",\
        \"time_signature\": \"4/4\",\
        \"lyrics_text\": \"[verse]\\nUnclosed fence\",\
        \"arrangement_map\": [{\"section_id\": \"v\", \"display_label\": \"V\", \"yue_clean_tag\": \"[verse]\", \
        \"bars\": 4, \"chords\": [\"F#m\"], \"production_cue\": \"Cue\", \"lyrics\": [], \"lead_instrument_intensity\": 0.5, \
        \"drum_fill_complexity\": 0.5, \"bass_weight\": 0.5}],\
        \"judge_score\": 8.0,\
        \"critique\": \"Fence unclosed\"\
    }"; // no closing ```
    let bp_unclosed = parse_judge_json(unclosed_fence, &genre, "dance");
    assert_eq!(bp_unclosed.title, "Unclosed Fence Song");
}

#[test]
fn test_stress_judge_json_malformed_inputs_trigger_zero_crash_fallback() {
    let genre = GenreTaxonomy::find_or_create("Stadium Rock");

    let adversarial_inputs = [
        "",                                          // empty string
        "   \n\t  \r\n   ",                          // whitespace only
        "<think></think>",                           // empty think only
        "I cannot fulfill this request as an AI.",   // refusal without JSON
        "{\"title\": incomplete json without close", // truncated JSON
        "{\"title\": \"Bad Trailing Comma\", }",     // invalid trailing comma
        "{ \"lyrics_text\": \"\", \"arrangement_map\": [] }", // empty required fields
        "{\"title\": 12345, \"bpm\": \"not-a-number\"}",     // schema type mismatch
        "}{",                                       // reversed braces
        "{{{{{{{{{{",                               // unclosed repeated open braces
        "<think>thought</think>```json```",          // empty fence
    ];

    for input in &adversarial_inputs {
        let bp = parse_judge_json(input, &genre, "rock guitar anthem");
        assert!(
            !bp.title.is_empty(),
            "Fallback title must not be empty for input: {:?}", input
        );
        assert!(
            bp.bpm > 0.0,
            "Fallback BPM must be positive, got {} for input: {:?}", bp.bpm, input
        );
        assert!(
            !bp.lyrics_text.is_empty(),
            "Fallback lyrics must not be empty for input: {:?}", input
        );
        assert!(
            !bp.arrangement_map.is_empty(),
            "Fallback arrangement map must not be empty for input: {:?}", input
        );
        assert!(
            bp.judge_score >= 7.0,
            "Fallback judge score must be >= 7.0, got {} for input: {:?}", bp.judge_score, input
        );
    }
}

#[test]
fn test_stress_judge_json_huge_payload_and_unicode_resilience() {
    let genre = GenrePreset::roots_reggae_1977();

    // 100 KB of think noise before the valid JSON
    let noise = "All work and no play makes Jack a dull boy. ".repeat(2500);
    let valid_json = "{\
        \"title\": \"Massive Stream Survivor\",\
        \"genre_tags\": \"Roots Reggae, 1977, 76 BPM\",\
        \"bpm\": 76.0,\
        \"key_signature\": \"A Major\",\
        \"time_signature\": \"4/4\",\
        \"lyrics_text\": \"[intro]\\nSurvived 100KB noise.\",\
        \"arrangement_map\": [{\"section_id\": \"intro\", \"display_label\": \"Intro\", \"yue_clean_tag\": \"[intro]\", \
        \"bars\": 4, \"chords\": [\"A\"], \"production_cue\": \"Cue\", \"lyrics\": [], \"lead_instrument_intensity\": 0.5, \
        \"drum_fill_complexity\": 0.5, \"bass_weight\": 0.5}],\
        \"judge_score\": 9.5,\
        \"critique\": \"Resilient parse\"\
    }";
    let payload = format!("<think>{}</think>{}", noise, valid_json);
    let bp = parse_judge_json(&payload, &genre, "resilience");
    assert_eq!(bp.title, "Massive Stream Survivor");
    assert_eq!(bp.judge_score, 9.5);

    // Emoji, RTL, and non-ASCII Unicode
    let unicode_json = "<think>Reasoning with emoji 🇯🇲 🎵 🔥 and Arabic نغمات</think>\
    ```json\n{\
        \"title\": \"One Love ❤️ 🇯🇲\",\
        \"genre_tags\": \"Roots Reggae, 76 BPM, Soulful Jamaican Tenor 🇯🇲\",\
        \"bpm\": 76.0,\
        \"key_signature\": \"A Major\",\
        \"time_signature\": \"4/4\",\
        \"lyrics_text\": \"[intro]\\n❤️ Peace and love for all humanity 🌍\\n\\n[verse 1]\\nSun is shining ☀️\\n\\n[outro]\",\
        \"arrangement_map\": [{\"section_id\": \"intro\", \"display_label\": \"Intro\", \"yue_clean_tag\": \"[intro]\", \
        \"bars\": 4, \"chords\": [\"A\"], \"production_cue\": \"One-drop groove\", \"lyrics\": [], \"lead_instrument_intensity\": 0.5, \
        \"drum_fill_complexity\": 0.5, \"bass_weight\": 0.8}],\
        \"judge_score\": 9.8,\
        \"critique\": \"Beautiful global harmony\"\
    }\n```";
    let bp_u = parse_judge_json(unicode_json, &genre, "One love for all humanity");
    assert_eq!(bp_u.title, "One Love ❤️ 🇯🇲");
    assert!(bp_u.lyrics_text.contains("🌍"));
    assert!(bp_u.genre_tags.contains("🇯🇲"));
}

#[test]
fn test_stress_judge_apply_to_state_invariants() {
    let client = OllamaClient::new("http://127.0.0.1:11434");
    let judge = JudgeEvaluator::new(client, "qwen2.5:7b");

    let bp_pass = EvaluatedSongBlueprint {
        title: "Passed Track".into(),
        genre_tags: "Afrobeats, 105 BPM".into(),
        bpm: 105.0,
        key_signature: "F# Minor".into(),
        time_signature: "4/4".into(),
        lyrics_text: "[intro]\nTest".into(),
        arrangement_map: vec![SectionArrangement {
            section_id: "intro".into(),
            display_label: "Intro".into(),
            yue_clean_tag: "[intro]".into(),
            bars: 4,
            chords: vec!["F#m".into()],
            production_cue: "Cue".into(),
            lyrics: vec![],
            lead_instrument_intensity: 0.5,
            drum_fill_complexity: 0.5,
            bass_weight: 0.5,
        }],
        judge_score: 8.5,
        critique: "Great".into(),
    };

    let mut state = MusicProductionState::default_reggae_anthem();
    judge.apply_to_state(&bp_pass, &mut state);

    assert_eq!(state.style_prompt, "Afrobeats, 105 BPM");
    assert_eq!(state.bpm, 105);
    assert_eq!(state.key_signature, "F# Minor");
    assert_eq!(state.judge_score, 8.5);
    assert!(state.quality_passed, "Score >= 7.0 must set quality_passed = true");
    assert!(!state.history.is_empty());

    // Low score (< 7.0) sets quality_passed = false
    let mut bp_fail = bp_pass.clone();
    bp_fail.judge_score = 6.2;
    judge.apply_to_state(&bp_fail, &mut state);
    assert!(!state.quality_passed, "Score < 7.0 must set quality_passed = false");
    assert_eq!(state.judge_score, 6.2);
}

// ============================================================================
// PART 5: OLLAMA CLIENT & PRODUCER PANEL OFFLINE RESILIENCE
// ============================================================================

#[tokio::test]
async fn test_stress_ollama_client_probe_timeout_bounded() {
    // Unreachable endpoint
    let client = OllamaClient::new("http://127.0.0.1:54321");

    let start = Instant::now();
    let is_online = client.probe_health().await;
    let elapsed = start.elapsed();

    assert!(!is_online, "Offline endpoint must return false");
    // Probe timeout is configured for 400ms. Allow generous 2.5s buffer for OS connection refused.
    assert!(
        elapsed < Duration::from_millis(2500),
        "probe_health took too long: {:?}", elapsed
    );
}

#[tokio::test]
async fn test_stress_producer_panel_offline_fallback_multi_genre() {
    let genres_to_test = [
        "Roots Reggae",
        "Afrobeats",
        "Pop Electronic Synthwave",
        "Stadium Rock",
        "Fast Bebop Jazz",
    ];

    let offline_client = OllamaClient::new("http://127.0.0.1:54321");
    let panel = OllamaProducerPanel::new(
        offline_client,
        "qwen2.5:7b",
        "qwen2.5:7b",
        "qwen2.5:7b",
        "qwen2.5:7b",
    );

    for genre_name in &genres_to_test {
        let expected_genre = GenreTaxonomy::find_or_create(genre_name);
        let expected_bpm = expected_genre.default_bpm;
        let expected_key = &expected_genre.default_key;

        let song_id = format!("offline-{}", genre_name.to_lowercase().replace(' ', "-"));
        let mut state = MusicProductionState::new(
            &song_id,
            *genre_name,
            expected_bpm,
            expected_key,
        );

        let output = panel.run_panel(&mut state).await
            .unwrap_or_else(|e| panic!("Offline panel failed for {}: {}", genre_name, e));

        assert!(
            !output.yue_genre_txt.is_empty(),
            "YuE genre tags must not be empty for {}", genre_name
        );
        assert!(
            output.yue_genre_txt.ends_with(".\n"),
            "YuE genre text must terminate with '.\\n', got: {:?}", output.yue_genre_txt
        );
        assert!(
            !output.yue_lyrics_txt.is_empty(),
            "YuE lyrics must not be empty for {}", genre_name
        );
        assert!(
            output.yue_lyrics_txt.contains("[intro]") && output.yue_lyrics_txt.contains("[outro]"),
            "YuE lyrics must include structural tags for {}", genre_name
        );
        assert!(state.quality_passed, "Offline fallback blueprint must satisfy quality gate");
        assert_eq!(state.bpm, expected_bpm);
    }
}

// ============================================================================
// PART 6: EMPIRICAL CHALLENGER VERIFICATION (BUILDER ERGONOMICS & STATE ISOLATION)
// ============================================================================

#[test]
fn test_empirical_challenger_force_dsp_across_all_stages_and_gpu_tiers() {
    let stages = [
        PipelineStageKind::OllamaPanel,
        PipelineStageKind::Stage1Groove,
        PipelineStageKind::Stage2YuE,
        PipelineStageKind::Stage3VoiceConversion,
        PipelineStageKind::Stage4Inpainting,
        PipelineStageKind::Stage5Mastering,
        PipelineStageKind::GrooveFoundation,
        PipelineStageKind::YuESynthesis,
        PipelineStageKind::VoiceConversion,
        PipelineStageKind::StemSeparationAndInpainting,
        PipelineStageKind::Mastering,
    ];

    let gpu_vram_tiers = [
        (0.0, 0.0, None),                                 // 0 GB (No GPU)
        (8.0, 8.0, Some("RTX 3060 8GB".to_string())),     // 8 GB
        (24.0, 24.0, Some("RTX 4090 24GB".to_string())),  // 24 GB
        (48.0, 48.0, Some("RTX 6000 Ada".to_string())),   // 48 GB
        (80.0, 80.0, Some("NVIDIA A100 80GB".to_string())), // 80 GB Data center
    ];

    let engine_modes = ["auto", "pure_rust", "hf_hybrid", "custom_experimental"];

    for (total_vram, free_vram, gpu_name) in &gpu_vram_tiers {
        let base_mem = SystemMemory::simulated(
            128.0,
            96.0,
            32,
            gpu_name.clone(),
            *total_vram,
            *free_vram,
        );

        // Builder call ergonomics: with_force_dsp(true)
        let forced_mem = base_mem.clone().with_force_dsp(true);

        // 1. Verify state isolation: base instance unchanged
        assert!(!base_mem.force_dsp, "Base memory force_dsp must remain false");
        assert!(forced_mem.force_dsp, "Forced memory force_dsp must be true");

        // 2. Verify all stages and engine modes route to PureRustDsp
        for stage in &stages {
            for mode in &engine_modes {
                let route = forced_mem.route_stage(*stage, mode);
                assert_eq!(
                    route,
                    ExecutionRoute::PureRustDsp,
                    "Stage {:?} with mode {} under {} GB VRAM did not route to PureRustDsp with force_dsp(true)",
                    stage, mode, free_vram
                );
                assert!(
                    forced_mem.should_use_dsp(*stage, mode),
                    "should_use_dsp must return true for {:?} under mode {}",
                    stage, mode
                );
            }
        }
    }
}

#[test]
fn test_empirical_challenger_force_hf_routes_to_gpu_when_vram_sufficient() {
    let stages = [
        (PipelineStageKind::OllamaPanel, 2.0),
        (PipelineStageKind::Stage1Groove, 6.0),
        (PipelineStageKind::Stage2YuE, 8.0),
        (PipelineStageKind::Stage3VoiceConversion, 4.0),
        (PipelineStageKind::Stage4Inpainting, 8.0),
        (PipelineStageKind::Stage5Mastering, 4.0),
    ];

    // High VRAM: 24 GB and 48 GB free VRAM (budgets 20.4 GB and 40.8 GB >= 8.0 GB required)
    let high_vram_tiers = [
        (24.0, 24.0, "RTX 4090"),
        (48.0, 48.0, "RTX 6000 Ada"),
    ];

    for (total, free, name) in &high_vram_tiers {
        let base_mem = SystemMemory::simulated(
            64.0,
            48.0,
            16,
            Some(name.to_string()),
            *total,
            *free,
        );

        let forced_hf_mem = base_mem.clone().with_force_hf(true);

        // State isolation check
        assert!(!base_mem.force_hf);
        assert!(forced_hf_mem.force_hf);

        // When free VRAM is sufficient, route_stage must return HuggingFaceGpu
        for (stage, req_vram) in &stages {
            assert!(
                forced_hf_mem.gpu_budget_gb() >= *req_vram,
                "GPU budget should exceed requirement"
            );
            assert_eq!(
                forced_hf_mem.route_stage(*stage, "auto"),
                ExecutionRoute::HuggingFaceGpu,
                "Stage {:?} should route to HuggingFaceGpu under {} GB VRAM",
                stage, free
            );
            assert_eq!(
                forced_hf_mem.route_stage(*stage, "hf_hybrid"),
                ExecutionRoute::HuggingFaceGpu,
                "Stage {:?} should route to HuggingFaceGpu under hf_hybrid with {} GB VRAM",
                stage, free
            );
        }
    }

    // Mid VRAM: 8.0 GB free VRAM (budget 8.0 * 0.85 = 6.8 GB)
    // Stage1 (6.0), Stage3 (4.0), Stage5 (4.0) have sufficient VRAM -> HuggingFaceGpu
    let mid_mem = SystemMemory::simulated(
        64.0,
        48.0,
        16,
        Some("RTX 3070 8GB".into()),
        8.0,
        8.0,
    ).with_force_hf(true);

    assert_eq!(mid_mem.route_stage(PipelineStageKind::Stage1Groove, "auto"), ExecutionRoute::HuggingFaceGpu);
    assert_eq!(mid_mem.route_stage(PipelineStageKind::Stage3VoiceConversion, "auto"), ExecutionRoute::HuggingFaceGpu);
    assert_eq!(mid_mem.route_stage(PipelineStageKind::Stage5Mastering, "auto"), ExecutionRoute::HuggingFaceGpu);

    // Conflict resolution test: when both force_dsp AND force_hf are set, fail-closed pure-Rust DSP takes precedence
    let conflicting_mem = mid_mem.clone().with_force_dsp(true);
    assert_eq!(
        conflicting_mem.route_stage(PipelineStageKind::Stage1Groove, "auto"),
        ExecutionRoute::PureRustDsp,
        "Conflicting flags must fail closed to PureRustDsp"
    );
}

