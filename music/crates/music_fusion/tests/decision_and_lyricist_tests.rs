use std::path::PathBuf;

use music_core::{DecisionMode, LyricsMode, MusicProductionState, StemMixConfig};
use music_fusion::{
    calibrate_logits, compose_offline_structured_lyrics, default_models_cache_dir,
    parse_lyrics_sections, structure_user_lyrics_verbatim, DecisionRequest, DecisionSchema,
    HfModelDiscoveryEngine, NimbleDecisionEngine, OllamaClient, SystemMemory,
    HITL_CONFIDENCE_THRESHOLD,
};

fn isolated_test_engine() -> NimbleDecisionEngine {
    NimbleDecisionEngine::new(
        OllamaClient::new("http://127.0.0.1:1"),
        PathBuf::from("nonexistent_models_dir_for_test"),
        PathBuf::from("nonexistent_nimble_worker.py"),
        SystemMemory {
            total_ram_gb: 32.0,
            free_ram_gb: 20.0,
            cpu_cores: 8,
            gpu_name: Some("NVIDIA Quadro RTX 4000".into()),
            gpu_vram_total_gb: 8.0,
            gpu_vram_free_gb: 5.5,
            force_dsp: false,
            force_hf: false,
        },
    )
}

#[test]
fn test_open_weight_catalog_zero_lyria_and_commercial_filtering() {
    let builtins = HfModelDiscoveryEngine::curated_builtins();
    assert!(!builtins.is_empty());
    assert!(
        builtins.iter().all(|m| !m.id.to_ascii_lowercase().contains("lyria")),
        "No paid Lyria endpoints may appear in the open-weight catalog"
    );

    let mem = isolated_test_engine().memory;
    let commercial_catalog =
        HfModelDiscoveryEngine::list_open_weight_catalog(&mem, true, &default_models_cache_dir());
    let levo = commercial_catalog
        .iter()
        .find(|item| item.entry.id == "tencent/SongGeneration")
        .expect("LeVo 2 should exist in catalog");
    assert!(levo.blocked_by_commercial_filter);

    let ace = commercial_catalog
        .iter()
        .find(|item| item.entry.id == "ACE-Step/acestep-v15-turbo")
        .expect("ACE-Step 1.5 Turbo MIT should exist in catalog");
    assert!(!ace.blocked_by_commercial_filter);
    assert!(ace.entry.commercial_allowed);
    assert!(ace.fits_hardware);
}

#[test]
fn test_calibrate_logits_normalization_and_entropy() {
    let choices = vec!["A".into(), "B".into(), "C".into(), "D".into()];
    let (probs, entropy) = calibrate_logits(&choices, &[2.4, 0.7, -0.3, 1.1], 1.0);
    let sum: f32 = probs.iter().map(|(_, p)| *p).sum();
    assert!((sum - 1.0).abs() < 1e-4, "probabilities sum={}", sum);
    assert!(entropy > 0.0 && entropy <= (4.0_f32).ln() + 1e-4);
    assert_eq!(probs[0].0, "A");
}

#[test]
fn test_commercial_only_blocks_noncommercial_nimble_v3() {
    let (resolved, block_note) = NimbleDecisionEngine::resolve_decision_checkpoint(
        Some("bespokelabs/Bespoke-Nimble-9B-v3"),
        true,
    );
    assert_eq!(resolved.id, "bespokelabs/Bespoke-Nimble-9B");
    assert!(resolved.commercial_allowed);
    assert!(block_note.is_some());
}

#[tokio::test]
async fn test_nimble_hitl_threshold_and_system1_latency() {
    let engine = isolated_test_engine();
    let mut req = DecisionRequest::new(
        "hitl_gate",
        DecisionSchema::Categorical {
            options: vec!["Opt1".into(), "Opt2".into(), "Opt3".into(), "Opt4".into()],
        },
        "Select borderline option",
    );
    req.mode = DecisionMode::FastDeterministic;
    req.feature_logits = Some(vec![0.10, 0.08, 0.05, 0.02]);

    let res = engine.decide(&req).await;
    assert!(res.confidence < HITL_CONFIDENCE_THRESHOLD);
    assert!(res.hitl_gate_triggered);
    assert!(res.trace.hitl_gate_triggered);
    assert!(res.trace.latency_ms < 25, "latency={}ms", res.trace.latency_ms);
}

#[tokio::test]
async fn test_nimble_gates_1_through_5() {
    let engine = isolated_test_engine();

    let g1 = engine
        .gate1_select_generation_model(
            "uplifting soul pop vocal anthem, 108 BPM",
            LyricsMode::Auto,
            true,
            Some("tencent/SongGeneration"),
            true,
            None,
            DecisionMode::FastDeterministic,
            Some("bespokelabs/Bespoke-Nimble-9B-v3"),
        )
        .await;
    assert_ne!(g1.selected_choice, "tencent/SongGeneration");
    assert!(g1.hitl_gate_triggered);
    assert!(!g1.blocked_candidates.is_empty());

    let g2 = engine
        .gate2_resolve_lyrics_and_structure(
            "synthpop neon drive",
            "[Verse 1]\nNeon rivers under midnight rain",
            "",
            LyricsMode::Auto,
            DecisionMode::FastDeterministic,
            None,
        )
        .await;
    assert_eq!(g2.selected_choice, "UseUserLyricsVerbatim");

    let state = MusicProductionState::new("song-t", "Synthpop", 118, "C Minor");
    let g3 = engine
        .gate3_adjudicate_telemetry_quality(
            &state,
            0.002,
            0.0005,
            10.0,
            1500.0,
            2,
            DecisionMode::FastDeterministic,
            None,
        )
        .await;
    assert_eq!(g3.selected_choice, "RetryStage2Synthesis");

    let mix = StemMixConfig::default();
    let g4 = engine
        .gate4_recommend_mix_adjustments(
            "neo-soul ballad",
            &mix,
            "make vocals punchier and clearer",
            true,
            DecisionMode::FastDeterministic,
            None,
        )
        .await;
    assert_eq!(g4.selected_choice, "VocalPresenceAndPunch");
    assert!(g4.recommended_action.get("recommended_mix").is_some());

    let g5 = engine
        .gate5_license_and_export_gate(
            "m-a-p/YuE2-3B",
            Some("bespokelabs/Bespoke-Nimble-9B"),
            true,
            DecisionMode::FastDeterministic,
        )
        .await;
    assert_eq!(g5.selected_choice, "BlockedLicenseConflict");
}

#[test]
fn test_verbatim_lyrics_and_genre_adaptive_ai_composer() {
    let user_input = "Neon rain falls over Kyoto stations\nMidnight trains carry quiet conversations\n\nHold the skyline close tonight\nTill the silver morning light";
    let res = structure_user_lyrics_verbatim(user_input);
    assert!(res.preserved_verbatim);
    assert!(res.formatted_lyrics.contains("Neon rain falls over Kyoto stations"));
    assert!(!res.formatted_lyrics.to_ascii_lowercase().contains("trenchtown"));
    assert!(!res.formatted_lyrics.to_ascii_lowercase().contains("one love"));
    assert_eq!(res.formatted_lyrics.matches("Neon rain falls over Kyoto stations").count(), 1);

    let ai = compose_offline_structured_lyrics(
        "driving through chrome skylines at 2 AM",
        "80s Synthpop, 118 BPM, C Minor, analog Juno pads",
    );
    assert!(!ai.preserved_verbatim);
    assert!(ai.formatted_lyrics.contains("[Verse 1]"));
    assert!(ai.formatted_lyrics.contains("[Chorus]"));
    assert!(ai.formatted_lyrics.contains("[Synth Solo]"));
    assert!(!ai.formatted_lyrics.to_ascii_lowercase().contains("reggae"));
    assert!(!parse_lyrics_sections(&ai.formatted_lyrics).is_empty());
}
