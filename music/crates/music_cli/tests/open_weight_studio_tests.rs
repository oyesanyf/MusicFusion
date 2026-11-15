//! End-to-End Integration Tests for Open-Weight Music Studio:
//! 1. Verification that removed sampling and proprietary cloud endpoints return HTTP 404.
//! 2. Dual-mode Lyrics Studio (`user_provided` 100% verbatim vs `ai_generated` structured songwriting).
//! 3. Nimble Decision Engine (`GET /api/decision/status` + `POST /api/decision` HITL gating).
//! 4. Open-weight catalog (`GET /api/models/open-weight` with 9 model families + `commercial_only`) and `/api/models/pull` guards.
//! 5. Full E2E creation -> Gate 4 AI Mix Copilot (`POST /api/mix/recommend`) -> 8-channel DAW non-destructive bounce (`POST /api/songs/mix` v1 -> v2) + 48 kHz stereo WAV validation.

use std::sync::Arc;
use std::time::Duration;
use serde_json::json;

use music_cli::rest_server::start_rest_server;
use music_dsp::wav::AudioBuffer;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

async fn spawn_isolated_server(port_start: u16, port_end: u16) -> (tempfile::TempDir, reqwest::Client, String) {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.2);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner,
        temp_dir.path().to_path_buf(),
        port_start,
        port_end,
        "http://127.0.0.1:1".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(80)).await;
    (temp_dir, client, base_url)
}

#[tokio::test]
async fn test_removed_sampling_and_legacy_cloud_endpoints_return_404() {
    let (_tmp, client, base_url) = spawn_isolated_server(6101, 6199).await;

    assert_eq!(client.get(format!("{}/api/reference-wavs", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(client.post(format!("{}/api/reference-wavs/upload", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(
        client
            .post(format!("{}/api/reference-wavs/generate-rl", base_url))
            .json(&json!({ "prompt": "groove" }))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(client.get(format!("{}/api/gemini-env", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(
        client
            .post(format!("{}/api/gemini-env", base_url))
            .json(&json!({ "api_key": "disabled" }))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
}

#[tokio::test]
async fn test_dual_mode_lyrics_verbatim_and_ai_generation() {
    let (_tmp, client, base_url) = spawn_isolated_server(6201, 6299).await;

    let exact_lyrics = "[Verse 1]\nLine 1 verbatim under silver stars\n[Chorus]\nLine 2 ringing through old guitars";
    let user_res = client
        .post(format!("{}/api/lyrics/generate", base_url))
        .json(&json!({
            "mode": "user_provided",
            "lyrics": exact_lyrics,
            "genre": "Acoustic Folk"
        }))
        .send()
        .await
        .expect("POST /api/lyrics/generate user_provided failed");
    assert_eq!(user_res.status(), 200);
    let user_json: serde_json::Value = user_res.json().await.unwrap();
    assert_eq!(user_json["status"], "ok");
    assert_eq!(user_json["preserved_verbatim"], true);
    let returned_lyrics = user_json["full_lyrics"].as_str().unwrap();
    assert!(returned_lyrics.contains("Line 1 verbatim under silver stars"));
    assert!(returned_lyrics.contains("Line 2 ringing through old guitars"));

    let ai_res = client
        .post(format!("{}/api/lyrics/generate", base_url))
        .json(&json!({
            "mode": "ai_generated",
            "topic": "Midnight river hope",
            "genre": "Acoustic Folk",
            "use_llm": false
        }))
        .send()
        .await
        .expect("POST /api/lyrics/generate ai_generated failed");
    assert_eq!(ai_res.status(), 200);
    let ai_json: serde_json::Value = ai_res.json().await.unwrap();
    assert_eq!(ai_json["status"], "ok");
    let ai_mode = ai_json["mode"].as_str().unwrap_or("");
    assert!(
        ai_mode == "ai_generated" || ai_mode == "ai_offline_composer",
        "unexpected AI lyrics mode: {}",
        ai_mode
    );
    assert_eq!(ai_json["preserved_verbatim"], false);
    assert!(!ai_json["full_lyrics"].as_str().unwrap_or("").trim().is_empty());
    assert!(!ai_json["sections"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_nimble_decision_status_and_calibrated_hitl_gate() {
    let (_tmp, client, base_url) = spawn_isolated_server(6301, 6399).await;

    let status_res = client
        .get(format!("{}/api/decision/status", base_url))
        .send()
        .await
        .expect("GET /api/decision/status failed");
    assert_eq!(status_res.status(), 200);
    let status_json: serde_json::Value = status_res.json().await.unwrap();
    assert!(!status_json["active_model"].as_str().unwrap_or("").is_empty());
    assert_eq!(status_json["system1_fallback_ready"], true);

    let confident_res = client
        .post(format!("{}/api/decision", base_url))
        .json(&json!({
            "gate_name": "model_gate",
            "options": ["ACE-Step/acestep-v15-turbo", "ASLP-lab/DiffRhythm2"],
            "question": "Select open-weight vocal model",
            "mode": "fast_deterministic",
            "feature_logits": [2.2, 0.1]
        }))
        .send()
        .await
        .expect("POST /api/decision confident failed");
    assert_eq!(confident_res.status(), 200);
    let confident_json: serde_json::Value = confident_res.json().await.unwrap();
    assert_eq!(confident_json["selected_choice"], "ACE-Step/acestep-v15-turbo");
    assert_eq!(confident_json["hitl_gate_triggered"], false);
    assert!(!confident_json["trace"]["backend_used"].as_str().unwrap_or("").is_empty());

    let borderline_res = client
        .post(format!("{}/api/decision", base_url))
        .json(&json!({
            "gate_name": "borderline_gate",
            "options": ["OptionA", "OptionB", "OptionC", "OptionD"],
            "question": "Tie-break close candidates",
            "mode": "fast_deterministic",
            "feature_logits": [0.10, 0.08, 0.05, 0.02]
        }))
        .send()
        .await
        .expect("POST /api/decision borderline failed");
    assert_eq!(borderline_res.status(), 200);
    let borderline_json: serde_json::Value = borderline_res.json().await.unwrap();
    assert_eq!(borderline_json["hitl_gate_triggered"], true);
}

#[tokio::test]
async fn test_open_weight_catalog_families_and_pull_license_guards() {
    let (_tmp, client, base_url) = spawn_isolated_server(6401, 6499).await;

    let cat_res = client
        .get(format!("{}/api/models/open-weight?commercial_only=false", base_url))
        .send()
        .await
        .expect("GET /api/models/open-weight failed");
    assert_eq!(cat_res.status(), 200);
    let cat_json: serde_json::Value = cat_res.json().await.unwrap();
    let models = cat_json["models"].as_array().unwrap();
    let ids: Vec<&str> = models
        .iter()
        .filter_map(|m| m["entry"]["id"].as_str())
        .collect();

    for expected in [
        "ACE-Step/acestep-v15-turbo",
        "tencent/SongGeneration",
        "m-a-p/YuE2-3B",
        "MiniMaxAI/MiniMax-Music3",
        "ASLP-lab/DiffRhythm2",
        "openmusic/Muse",
        "stabilityai/stable-audio-open-small",
        "facebook/musicgen-large",
        "google/magenta-realtime-2",
    ] {
        assert!(ids.contains(&expected), "Missing open-weight family '{}' in {:?}", expected, ids);
    }

    let comm_res = client
        .get(format!("{}/api/models/open-weight?commercial_only=true", base_url))
        .send()
        .await
        .expect("GET /api/models/open-weight commercial_only failed");
    assert_eq!(comm_res.status(), 200);
    let comm_json: serde_json::Value = comm_res.json().await.unwrap();
    let comm_models = comm_json["models"].as_array().unwrap();

    let levo = comm_models
        .iter()
        .find(|m| m["entry"]["id"] == "tencent/SongGeneration")
        .expect("LeVo 2 entry present");
    assert_eq!(levo["blocked_by_commercial_filter"], true);

    let acestep = comm_models
        .iter()
        .find(|m| m["entry"]["id"] == "ACE-Step/acestep-v15-turbo")
        .expect("ACE-Step entry present");
    assert_eq!(acestep["blocked_by_commercial_filter"], false);
    assert_eq!(acestep["entry"]["commercial_allowed"], true);

    let nc_pull = client
        .post(format!("{}/api/models/pull", base_url))
        .json(&json!({
            "model": "tencent/SongGeneration",
            "commercial_only": true
        }))
        .send()
        .await
        .expect("POST /api/models/pull commercial conflict failed");
    assert_eq!(nc_pull.status(), 403);

    let bad_prefix_pull = client
        .post(format!("{}/api/models/pull", base_url))
        .json(&json!({
            "model": "models/invalid-paid-endpoint",
            "commercial_only": false
        }))
        .send()
        .await
        .expect("POST /api/models/pull bad prefix failed");
    assert_eq!(bad_prefix_pull.status(), 400);
}

#[tokio::test]
async fn test_create_ai_mix_copilot_and_8ch_daw_bounce_to_48khz_wav() {
    let (_tmp, client, base_url) = spawn_isolated_server(6501, 6599).await;

    let user_lyrics = "[Verse 1]\nWarm sunlight over ocean breeze\n[Chorus]\nEvery heartbeat finding inner peace";
    let create_res = client
        .post(format!("{}/api/songs/create", base_url))
        .json(&json!({
            "genre": "Indie Soul",
            "prompt": "Warm live rhodes chords, analog bassline, soulful lead vocal",
            "lyrics_mode": "user_provided",
            "lyrics": user_lyrics,
            "hf_model": "ACE-Step/acestep-v15-turbo",
            "commercial_only": true,
            "decision_mode": "fast_deterministic",
            "pure_rust": true,
            "max_duration_secs": 0.2
        }))
        .send()
        .await
        .expect("POST /api/songs/create failed");
    assert_eq!(create_res.status(), 200);
    let created: serde_json::Value = create_res.json().await.unwrap();
    assert_eq!(created["status"], "success");
    assert_eq!(created["version"], 1);
    let song_id = created["song_id"].as_str().unwrap().to_string();
    assert!(created["lyrics"].as_str().unwrap().contains("Warm sunlight over ocean breeze"));
    assert!(!created["selected_model"].as_str().unwrap_or("").is_empty());
    assert!(!created["decision_traces"].as_array().unwrap().is_empty());

    let rec_res = client
        .post(format!("{}/api/mix/recommend", base_url))
        .json(&json!({
            "song_id": song_id,
            "version": 1,
            "focus_request": "Lift lead vocal presence and tighten bass punch",
            "commercial_only": true,
            "decision_mode": "fast_deterministic"
        }))
        .send()
        .await
        .expect("POST /api/mix/recommend failed");
    assert_eq!(rec_res.status(), 200);
    let rec_json: serde_json::Value = rec_res.json().await.unwrap();
    assert_eq!(rec_json["status"], "ok");
    let channels = rec_json["recommended_stem_mix"]["channels"].as_object().unwrap();
    for stem in [
        "vocals",
        "backing_harmonies",
        "drums",
        "bass",
        "guitar",
        "keys_synths",
        "percussion_fills",
        "ambience_fx",
    ] {
        assert!(channels.contains_key(stem), "Missing canonical stem '{}' in mix recommendation", stem);
    }

    let mix_res = client
        .post(format!("{}/api/songs/mix", base_url))
        .json(&json!({
            "song_id": song_id,
            "from_version": 1,
            "channels": {
                "vocals": {
                    "gain_db": 2.4,
                    "pan": 0.0,
                    "eq_low_db": -1.5,
                    "eq_mid_db": 1.8,
                    "eq_high_db": 2.1,
                    "reverb_send": 0.28,
                    "mute": false,
                    "solo": false
                },
                "drums": {
                    "gain_db": 1.2,
                    "pan": 0.0,
                    "eq_low_db": 1.6,
                    "eq_mid_db": -0.5,
                    "eq_high_db": 1.4,
                    "reverb_send": 0.14,
                    "mute": false,
                    "solo": false
                },
                "bass": {
                    "gain_db": 0.9,
                    "pan": 0.0,
                    "eq_low_db": 2.2,
                    "eq_mid_db": 0.0,
                    "eq_high_db": -1.0,
                    "reverb_send": 0.04,
                    "mute": false,
                    "solo": false
                }
            },
            "master_gain_db": 0.5,
            "analog_tape_warmth": 0.35,
            "stereo_width": 1.15,
            "target_lufs": -14.0
        }))
        .send()
        .await
        .expect("POST /api/songs/mix failed");
    assert_eq!(mix_res.status(), 200);
    let mix_json: serde_json::Value = mix_res.json().await.unwrap();
    assert_eq!(mix_json["status"], "success");
    assert_eq!(mix_json["from_version"], 1);
    assert_eq!(mix_json["to_version"], 2);
    assert_eq!(mix_json["version"], 2);

    let wav_res = client
        .get(format!("{}/api/songs/{}/versions/2/audio/final_master", base_url, song_id))
        .send()
        .await
        .expect("GET v2 final_master failed");
    assert_eq!(wav_res.status(), 200);
    let wav_bytes = wav_res.bytes().await.unwrap();
    let audio = AudioBuffer::from_wav_bytes(&wav_bytes).expect("Master bounce must be valid 48 kHz WAV");
    assert_eq!(audio.sample_rate, 48_000);
    assert_eq!(audio.channels, 2);
    assert!(!audio.samples.is_empty());
}

#[tokio::test]
async fn test_auto_mode_node1_preserves_user_lyrics_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let exact_user_lyrics = "[Verse 1]\nDo not overwrite my custom verse\n[Chorus]\nKeep every syllable verbatim in auto mode";
    let mut state = music_core::state::MusicProductionState::new("song_auto_lyr", "Indie Soul", 84, "E Major");
    state.engine_mode = "auto".to_string();
    state.lyrics_mode = music_core::state::LyricsMode::UserProvided;
    state.lyrics = exact_user_lyrics.to_string();
    state.style_prompt = "Warm rhodes chords, punchy kick".to_string();
    state.decision_mode = music_core::state::DecisionMode::FastDeterministic;

    let node1 = music_pipeline::nodes::OllamaProducerPanelNode::new().with_storage_dir(tmp.path());
    node1
        .execute_node(&mut state)
        .await
        .expect("Node 1 should preserve verbatim user lyrics even in auto mode");

    assert!(
        state.lyrics.contains("Do not overwrite my custom verse"),
        "Expected verbatim user verse in auto mode, got: {}",
        state.lyrics
    );
    assert!(
        state.lyrics.contains("Keep every syllable verbatim in auto mode"),
        "Expected verbatim user chorus in auto mode, got: {}",
        state.lyrics
    );
}
