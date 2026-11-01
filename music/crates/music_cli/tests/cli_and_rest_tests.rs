use std::sync::Arc;
use std::time::Duration;
use clap::Parser;
use serde_json::json;

use music_cli::browser;
use music_cli::cli_args::CliArgs;
use music_cli::mcp_server::{self, JsonRpcRequest};
use music_cli::presets::{create_hit_state, flagship_reggae_anthem_state};
use music_cli::rest_server::{bind_server, start_rest_server};
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

#[test]
fn test_cli_args_parsing_all_flags() {
    let args = CliArgs::parse_from([
        "music_cli",
        "--create-hit",
        "--genre", "Afrobeats",
        "--prompt", "Lagos night party groove",
        "--lyrics", "[Intro]\nLagos!",
        "--bpm", "105",
        "--key", "C Major",
        "--time-signature", "4/4",
        "--vocal-archetype", "smooth_rnb",
        "--pure-rust",
        "--iterate", "Add more horns",
        "--assist-iteration",
        "--song-id", "test_song_1",
        "--version", "1",
        "--regenerate-stem", "guitar",
        "--master",
        "--profile", "modern_pop",
        "--serve",
        "--host", "127.0.0.1",
        "--port", "5055",
        "--no-browser",
        "--open-ui",
        "--storage-dir", "test_storage",
        "--ollama-host", "http://127.0.0.1:11434",
    ]);

    assert!(args.create_hit);
    assert_eq!(args.genre.as_deref(), Some("Afrobeats"));
    assert_eq!(args.prompt.as_deref(), Some("Lagos night party groove"));
    assert_eq!(args.bpm, Some(105));
    assert_eq!(args.key.as_deref(), Some("C Major"));
    assert!(args.pure_rust);
    assert!(args.no_browser);
    assert!(args.serve);
    assert_eq!(args.port, 5055);
    assert_eq!(args.profile, "modern_pop");
}

#[test]
fn test_browser_suppression_logic() {
    // When suppressed = true, spawn_browser_delayed exits immediately without error
    browser::spawn_browser_delayed("http://127.0.0.1:5050/".to_string(), 10, true);
}

#[test]
fn test_flagship_reggae_anthem_preset_builder() {
    let state = flagship_reggae_anthem_state(None, None, false);
    assert_eq!(state.bpm, 76);
    assert_eq!(state.key_signature, "A Major");
    assert!(state.style_prompt.contains("1977 roots reggae anthem"));
    assert!(state.lyrics.contains("One love shining through the pouring rain"));

    let state_override = flagship_reggae_anthem_state(Some(82), Some("G Minor"), true);
    assert_eq!(state_override.bpm, 82);
    assert_eq!(state_override.key_signature, "G Minor");
    assert_eq!(state_override.engine_mode, "pure_rust");
}

#[test]
fn test_create_hit_state_builder() {
    let args = CliArgs::parse_from([
        "music_cli",
        "--create-hit",
        "--genre", "Afrobeats",
        "--prompt", "Vibrant sunset celebration",
        "--pure-rust",
    ]);
    let state = create_hit_state(&args).expect("Failed to build state");
    assert_eq!(state.genre, "Modern Afrobeats Hit");
    assert_eq!(state.bpm, 105);
    assert_eq!(state.engine_mode, "pure_rust");
    assert!(state.style_prompt.contains("sunset"));
}

#[tokio::test]
async fn test_mcp_server_tools_list_and_call() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    // 1. Test tools/list
    let tools_val = mcp_server::list_tools();
    let tools = tools_val["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 5);
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"music_create_hit"));
    assert!(names.contains(&"music_iterate_song"));
    assert!(names.contains(&"music_regenerate_stem"));
    assert!(names.contains(&"music_master_track"));
    assert!(names.contains(&"music_assist_iteration"));

    // 2. Test initialize
    let init_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: json!({}),
    };
    let init_resp = mcp_server::handle_mcp_request(init_req, &runner).await.unwrap();
    assert_eq!(init_resp.id, json!(1));
    assert!(init_resp.result.is_some());

    // 3. Test ping
    let ping_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "ping".to_string(),
        params: json!({}),
    };
    let ping_resp = mcp_server::handle_mcp_request(ping_req, &runner).await.unwrap();
    assert_eq!(ping_resp.id, json!(2));

    // 4. Test tool call music_create_hit
    let create_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(3)),
        method: "tools/call".to_string(),
        params: json!({
            "name": "music_create_hit",
            "arguments": {
                "genre": "Roots Reggae",
                "prompt": "1977 roots reggae anthem",
                "bpm": 76,
                "key": "A Major",
                "engine_mode": "pure_rust"
            }
        }),
    };
    let create_resp = mcp_server::handle_mcp_request(create_req, &runner).await.unwrap();
    assert_eq!(create_resp.id, json!(3));
    let res = create_resp.result.expect("Tool call result");
    assert_eq!(res["status"], "completed");
    let song_id = res["song_id"].as_str().unwrap();

    // 5. Test tool call music_assist_iteration
    let assist_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(4)),
        method: "tools/call".to_string(),
        params: json!({
            "name": "music_assist_iteration",
            "arguments": {
                "song_id": song_id,
                "version": 1
            }
        }),
    };
    let assist_resp = mcp_server::handle_mcp_request(assist_req, &runner).await.unwrap();
    let assist_res = assist_resp.result.expect("Assist result");
    assert_eq!(assist_res["status"], "completed");
    assert!(assist_res["suggested_next_iterations"].as_array().unwrap().len() >= 3);
}

#[tokio::test]
async fn test_rest_server_binding_resilience() {
    let (listener, port) = bind_server(5050, 5060).await.expect("Failed to bind port in range 5050..5060");
    assert!((5050..=5060).contains(&port));
    drop(listener);
}

#[tokio::test]
async fn test_rest_api_full_lifecycle_and_streaming() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    // Bind on available port range
    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        5061,
        5080,
        "http://127.0.0.1:11434".to_string(),
    ).await.expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);

    // Wait a brief moment for background server to accept connections
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 1. GET /api/health
    let health_res = client.get(format!("{}/api/health", base_url))
        .send().await.expect("GET /api/health failed");
    assert_eq!(health_res.status(), 200);
    let health_json: serde_json::Value = health_res.json().await.unwrap();
    assert_eq!(health_json["status"], "healthy");
    assert_eq!(health_json["port"], bound_port);

    // 2. GET /api/hardware
    let hw_res = client.get(format!("{}/api/hardware", base_url))
        .send().await.expect("GET /api/hardware failed");
    assert_eq!(hw_res.status(), 200);
    let hw_json: serde_json::Value = hw_res.json().await.unwrap();
    assert!(hw_json["total_ram_gb"].as_f64().unwrap() > 0.0);

    // 3. POST /api/songs/create
    let create_payload = json!({
        "prompt": "1977 roots reggae anthem",
        "genre": "Roots Reggae",
        "bpm": 76,
        "key_signature": "A Major",
        "engine_mode": "pure_rust"
    });
    let create_res = client.post(format!("{}/api/songs/create", base_url))
        .json(&create_payload)
        .send().await.expect("POST /api/songs/create failed");
    assert_eq!(create_res.status(), 200);
    let create_json: serde_json::Value = create_res.json().await.unwrap();
    assert_eq!(create_json["status"], "success");
    let song_id = create_json["song_id"].as_str().unwrap().to_string();
    assert_eq!(create_json["version"], 1);

    // 4. POST /api/songs/assist-iteration
    let assist_payload = json!({
        "song_id": song_id,
        "version": 1
    });
    let assist_res = client.post(format!("{}/api/songs/assist-iteration", base_url))
        .json(&assist_payload)
        .send().await.expect("POST /api/songs/assist-iteration failed");
    assert_eq!(assist_res.status(), 200);
    let assist_json: serde_json::Value = assist_res.json().await.unwrap();
    let suggestions = assist_json["suggested_next_iterations"].as_array().unwrap();
    assert!(suggestions.len() >= 3);

    // 5. POST /api/songs/iterate
    let iterate_payload = json!({
        "song_id": song_id,
        "from_version": 1,
        "critique": "Boost guitar solo wah and enhance sub-bass warmth",
        "bpm": 78
    });
    let iterate_res = client.post(format!("{}/api/songs/iterate", base_url))
        .json(&iterate_payload)
        .send().await.expect("POST /api/songs/iterate failed");
    assert_eq!(iterate_res.status(), 200);
    let iterate_json: serde_json::Value = iterate_res.json().await.unwrap();
    assert_eq!(iterate_json["status"], "success");
    assert_eq!(iterate_json["from_version"], 1);
    assert_eq!(iterate_json["to_version"], 2);

    // 6. POST /api/songs/regenerate-stem
    let regen_payload = json!({
        "song_id": song_id,
        "version": 2,
        "stem": "guitar"
    });
    let regen_res = client.post(format!("{}/api/songs/regenerate-stem", base_url))
        .json(&regen_payload)
        .send().await.expect("POST /api/songs/regenerate-stem failed");
    assert_eq!(regen_res.status(), 200);

    // 7. POST /api/songs/master
    let master_payload = json!({
        "song_id": song_id,
        "version": 2,
        "profile": "WarmAnalogTape"
    });
    let master_res = client.post(format!("{}/api/songs/master", base_url))
        .json(&master_payload)
        .send().await.expect("POST /api/songs/master failed");
    assert_eq!(master_res.status(), 200);

    // 8. GET /api/songs/:id/versions
    let versions_res = client.get(format!("{}/api/songs/{}/versions", base_url, song_id))
        .send().await.expect("GET /api/songs/:id/versions failed");
    assert_eq!(versions_res.status(), 200);
    let versions_json: serde_json::Value = versions_res.json().await.unwrap();
    let version_list = versions_json["versions"].as_array().unwrap();
    assert!(version_list.len() >= 2);

    // 9. GET /api/songs/:id/versions/:v/audio/:stem with Range header (206 Partial Content)
    let audio_range_res = client.get(format!("{}/api/songs/{}/versions/1/audio/final_master", base_url, song_id))
        .header("Range", "bytes=0-100")
        .send().await.expect("GET audio with Range failed");
    assert_eq!(audio_range_res.status(), 206);
    assert_eq!(audio_range_res.headers().get("content-type").unwrap(), "audio/wav");
    assert!(audio_range_res.headers().get("content-range").unwrap().to_str().unwrap().starts_with("bytes 0-100/"));
    let bytes = audio_range_res.bytes().await.unwrap();
    assert_eq!(bytes.len(), 101);

    // 10. Web Studio UI Static Assets
    let index_res = client.get(format!("{}/", base_url)).send().await.unwrap();
    assert_eq!(index_res.status(), 200);
    assert!(index_res.headers().get("content-type").unwrap().to_str().unwrap().contains("text/html"));

    let css_res = client.get(format!("{}/styles.css", base_url)).send().await.unwrap();
    assert_eq!(css_res.status(), 200);
    assert!(css_res.headers().get("content-type").unwrap().to_str().unwrap().contains("text/css"));

    let js_res = client.get(format!("{}/app.js", base_url)).send().await.unwrap();
    assert_eq!(js_res.status(), 200);
    assert!(js_res.headers().get("content-type").unwrap().to_str().unwrap().contains("application/javascript"));

    // 11. Path Traversal Security Check
    let bad_path_res = client.get(format!("{}/api/songs/..%2fcon/versions", base_url)).send().await.unwrap();
    assert_eq!(bad_path_res.status(), 400);
}
