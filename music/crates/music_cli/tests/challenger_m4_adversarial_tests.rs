//! Adversarial Empirical Challenger Tests for Milestone 4
//! Tests Multi-LLM Iteration Co-Pilot, Stem Regeneration, and Mastering Endpoints.

use std::sync::Arc;
use std::time::Duration;
use serde_json::json;

use music_cli::rest_server::start_rest_server;
use music_core::state::{
    MusicProductionState, STEM_BASS, STEM_GUITAR,
};
use music_fusion::iteration_panel::{
    default_genre_suggestions, parse_bpm_from_critique, parse_key_from_critique,
    IterationFusionPanel, ParameterTweaks,
};
use music_fusion::ollama_client::OllamaClient;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

#[tokio::test]
async fn test_iteration_copilot_guaranteed_chips_and_zero_empty_states() {
    let mut state = MusicProductionState::new("song-test-copilot", "Roots Reggae", 76, "A Major");
    state.lyrics = "[Intro]\nOne Love\n[Verse 1]\nWalking down the street".to_string();

    // 1. Offline / missing models check: panel with non-existent offline client
    let offline_client = OllamaClient::new("http://127.0.0.1:9999");
    let panel = IterationFusionPanel::new(
        offline_client,
        "missing_lyricist",
        "missing_arranger",
        "missing_engineer",
        "missing_copilot",
        "missing_judge",
    );

    let assist_res = panel.assist_song_iteration(&state, None).await.expect("assist_song_iteration failed");
    assert_eq!(assist_res.song_id, "song-test-copilot");
    assert!(!assist_res.critique.is_empty(), "Critique must not be empty");
    assert!(
        assist_res.suggested_next_iterations.len() >= 3 && assist_res.suggested_next_iterations.len() <= 5,
        "Suggestions count must be between 3 and 5, got {}",
        assist_res.suggested_next_iterations.len()
    );
    for s in &assist_res.suggested_next_iterations {
        assert!(!s.trim().is_empty(), "Suggestion chip cannot be empty string");
    }

    // 2. Test multi-genre suggestion chips coverage
    for genre in &["Roots Reggae", "Afrobeats", "Rock", "Hip-Hop", "Jazz", "Unknown Genre"] {
        let suggestions = default_genre_suggestions(genre, 80);
        assert!(
            suggestions.len() >= 3 && suggestions.len() <= 5,
            "Genre '{}' must have 3-5 suggestions, got {}",
            genre,
            suggestions.len()
        );
    }

    // 3. Test diff architecting under offline / pure_rust fallback
    let tweaks = ParameterTweaks {
        bpm: Some(82),
        key_signature: Some("G Minor".to_string()),
        vocal_rasp: Some(0.8),
        guitar_wah: Some(0.9),
        drum_fill_complexity: Some(0.85),
        sub_bass_weight: Some(1.2),
        mastering_profile: Some("ClubEdmSubHeavy".to_string()),
        target_stems: vec!["guitar".to_string(), "drums".to_string()],
        ..Default::default()
    };
    let critique = "Make the guitar solo wilder with heavy wah, speed up to 82 bpm, and boost sub-bass";
    let iter_res = panel.offline_fallback_iteration(&state, critique, &tweaks);
    assert_eq!(iter_res.to_version, state.version + 1);
    assert_eq!(iter_res.blueprint.bpm as u32, 82);
    assert_eq!(iter_res.blueprint.key_signature, "G Minor");
    assert!(iter_res.stem_deltas.contains_key(STEM_GUITAR), "Guitar delta missing");
    assert!(iter_res.stem_deltas.contains_key(STEM_BASS), "Bass delta missing");
    assert!(!iter_res.iteration_summary.is_empty(), "Summary missing");
}

#[test]
fn test_critique_parsing_heuristics() {
    assert_eq!(parse_bpm_from_critique("Speed up to 95 bpm please"), Some(95));
    assert_eq!(parse_bpm_from_critique("Slow down tempo to 64"), Some(64));
    assert_eq!(parse_bpm_from_critique("Invalid bpm 9999"), None);
    assert_eq!(parse_key_from_critique("Modulate key to G Minor for chorus"), Some("G Minor".to_string()));
    assert_eq!(parse_key_from_critique("No key specified"), None);
}

#[tokio::test]
async fn test_rest_endpoints_regenerate_stem_and_mastering() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        5081,
        5100,
        "http://127.0.0.1:11434".to_string(),
    ).await.expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 1. Create song v1
    let create_payload = json!({
        "prompt": "1977 roots reggae anthem",
        "genre": "Roots Reggae",
        "bpm": 76,
        "key_signature": "A Major",
        "engine_mode": "pure_rust"
    });
    let create_res = client.post(format!("{}/api/songs/create", base_url))
        .json(&create_payload)
        .send().await.unwrap();
    assert_eq!(create_res.status(), 200);
    let create_json: serde_json::Value = create_res.json().await.unwrap();
    let song_id = create_json["song_id"].as_str().unwrap().to_string();
    assert_eq!(create_json["version"], 1);

    // Read v1 final_master and vocal stem bytes
    let master_v1_res = client.get(format!("{}/api/songs/{}/versions/1/audio/final_master", base_url, song_id))
        .send().await.unwrap();
    assert_eq!(master_v1_res.status(), 200);

    let vocal_v1_res = client.get(format!("{}/api/songs/{}/versions/1/audio/raspy_tenor_vocals", base_url, song_id))
        .send().await.unwrap();
    assert_eq!(vocal_v1_res.status(), 200);

    // 2. Test POST /api/songs/regenerate-stem
    let regen_payload = json!({
        "song_id": song_id,
        "version": 1,
        "stem": "guitar"
    });
    let regen_res = client.post(format!("{}/api/songs/regenerate-stem", base_url))
        .json(&regen_payload)
        .send().await.unwrap();
    assert_eq!(regen_res.status(), 200);
    let regen_json: serde_json::Value = regen_res.json().await.unwrap();
    assert_eq!(regen_json["status"], "success");
    assert_eq!(regen_json["version"], 2, "Regenerate stem must increment version to 2");
    assert_eq!(regen_json["regenerated_stem"], "guitar");

    // The regenerated stem in v2 is successfully accessible
    let guitar_v2_res = client.get(format!("{}/api/songs/{}/versions/2/audio/guitar", base_url, song_id))
        .send().await.unwrap();
    assert_eq!(guitar_v2_res.status(), 200, "Regenerated stem must be streamable at /versions/2/audio/guitar");

    // The un-regenerated vocal stem in v2 is also streamable via checkpoint state.stem_paths resolution
    let vocal_v2_res = client.get(format!("{}/api/songs/{}/versions/2/audio/raspy_tenor_vocals", base_url, song_id))
        .send().await.unwrap();
    assert_eq!(vocal_v2_res.status(), 200, "Un-regenerated vocal stem must be streamable at /versions/2/audio/raspy_tenor_vocals");

    // Also verify URL Pattern 2 format: /api/songs/:id/:version/audio/:file
    let vocal_v2_p2_res = client.get(format!("{}/api/songs/{}/2/audio/raspy_tenor_vocals.wav", base_url, song_id))
        .send().await.unwrap();
    assert_eq!(vocal_v2_p2_res.status(), 200, "Pattern 2 audio streaming must return 200");

    // The state correctly tracks that un-regenerated vocal stems point to v1
    let stem_paths = &regen_json["state"]["stem_paths"];
    assert!(
        stem_paths["raspy_tenor_vocals"].as_str().unwrap().contains("v1"),
        "Un-regenerated vocal stem in state must reference v1: {:?}",
        stem_paths["raspy_tenor_vocals"]
    );
    assert!(
        stem_paths["guitar"].as_str().unwrap().contains("v2"),
        "Regenerated guitar stem in state must reference v2: {:?}",
        stem_paths["guitar"]
    );

    // 3. Test POST /api/songs/master with custom mastering profile
    let master_payload = json!({
        "song_id": song_id,
        "version": 2,
        "profile": "ClubEdmSubHeavy"
    });
    let master_res = client.post(format!("{}/api/songs/master", base_url))
        .json(&master_payload)
        .send().await.unwrap();
    assert_eq!(master_res.status(), 200);
    let master_json: serde_json::Value = master_res.json().await.unwrap();
    assert_eq!(master_json["status"], "success");
    assert_eq!(master_json["version"], 2);
    let telemetry = &master_json["telemetry"];
    let peak_dbfs = telemetry["peak_dbfs"].as_f64().unwrap();
    assert!(peak_dbfs <= 0.0, "Master peak must not exceed 0.0 dBFS ceiling");
    assert_eq!(master_json["mastering_profile"], "ClubEdmSubHeavy");

    // Verify final master WAV exists in v2 and is streamable
    let master_v2_res = client.get(format!("{}/api/songs/{}/versions/2/audio/final_master", base_url, song_id))
        .send().await.unwrap();
    assert_eq!(master_v2_res.status(), 200);
}
