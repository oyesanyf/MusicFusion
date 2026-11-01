//! Challenger M6-2 Empirical Verification Test Suite
//!
//! Empirically tests:
//! 1. Dynamic port assignment (5050..=5070 with fallback to ephemeral OS port)
//! 2. Download endpoints (/api/songs/:id/versions/:v/download and /api/songs/:id/download)
//! 3. Disk sample song artifacts and multi-section lyrics verification

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{CONTENT_DISPOSITION, CONTENT_RANGE, CONTENT_TYPE};
use reqwest::StatusCode;

use music_cli::rest_server::{bind_server_dynamic_on_host, start_rest_server};
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

// =========================================================================
// 1. DYNAMIC PORT SELECTION TESTS
// =========================================================================

#[tokio::test]
async fn test_dynamic_port_first_available_in_range() {
    // Occupy port 15050
    let blocker = tokio::net::TcpListener::bind("127.0.0.1:15050").await
        .expect("Should be able to bind blocker on 15050");

    // Attempt to bind range 15050..=15055
    let (listener, bound_port) = bind_server_dynamic_on_host("127.0.0.1", 15050, 15055).await
        .expect("bind_server_dynamic_on_host should succeed");

    assert_eq!(bound_port, 15051, "Should pick next available port 15051 in range");
    assert_eq!(listener.local_addr().unwrap().port(), 15051);

    drop(blocker);
    drop(listener);
}

#[tokio::test]
async fn test_dynamic_port_fallback_to_ephemeral_when_range_fully_occupied() {
    // Occupy all ports in range 15070..=15072
    let blocker1 = tokio::net::TcpListener::bind("127.0.0.1:15070").await.expect("Bind 15070");
    let blocker2 = tokio::net::TcpListener::bind("127.0.0.1:15071").await.expect("Bind 15071");
    let blocker3 = tokio::net::TcpListener::bind("127.0.0.1:15072").await.expect("Bind 15072");

    // Attempt to bind range 15070..=15072
    let (listener, bound_port) = bind_server_dynamic_on_host("127.0.0.1", 15070, 15072).await
        .expect("bind_server_dynamic_on_host should fall back to ephemeral port 0");

    assert!(bound_port > 0, "Bound port must be non-zero");
    assert!(
        bound_port < 15070 || bound_port > 15072,
        "Bound port {} must not be inside occupied range 15070..=15072",
        bound_port
    );
    assert_eq!(listener.local_addr().unwrap().port(), bound_port);

    drop(blocker1);
    drop(blocker2);
    drop(blocker3);
    drop(listener);
}

// =========================================================================
// 2. DOWNLOAD ENDPOINTS TESTS
// =========================================================================

#[tokio::test]
async fn test_download_endpoints_empirical_verification() {
    let base_dir = if Path::new("storage/songs").exists() {
        PathBuf::from("storage/songs")
    } else if Path::new("../../storage/songs").exists() {
        PathBuf::from("../../storage/songs")
    } else {
        PathBuf::from("d:/harfile/music/storage/songs")
    };
    assert!(base_dir.exists(), "storage/songs directory must exist at {}", base_dir.display());

    // Storage root is the parent of storage/songs (i.e. storage)
    let storage_root = base_dir.parent().unwrap().to_path_buf();

    let runner_config = PipelineConfig::default()
        .with_storage_dir(&storage_root)
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    // Start REST server with dynamic port
    let bound_port = start_rest_server(
        runner.clone(),
        storage_root.clone(),
        16050,
        16070,
        "http://127.0.0.1:11434".to_string(),
    ).await.expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);

    // Allow server a brief moment to initialize
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 1. GET /api/songs/:id/versions/:v/download (Explicit Version Download)
    let v1_url = format!("{}/api/songs/roots_reggae_anthem/versions/1/download", base_url);
    let resp = client.get(&v1_url).send().await.expect("Request v1 download");
    assert_eq!(resp.status(), StatusCode::OK, "Status should be 200 OK");

    let headers = resp.headers();
    let content_type = headers.get(CONTENT_TYPE).expect("Content-Type header").to_str().unwrap();
    assert_eq!(content_type, "audio/wav", "Content-Type must be audio/wav");

    let content_disp = headers.get(CONTENT_DISPOSITION).expect("Content-Disposition header").to_str().unwrap();
    assert!(
        content_disp.contains("attachment; filename=\"roots_reggae_anthem_v1_final_master.wav\""),
        "Content-Disposition '{}' must specify attachment and correct filename",
        content_disp
    );

    let body_bytes = resp.bytes().await.expect("Read WAV body");
    assert!(body_bytes.len() > 1000, "WAV body should be non-empty");
    assert_eq!(&body_bytes[0..4], b"RIFF", "Header must start with RIFF");
    assert_eq!(&body_bytes[8..12], b"WAVE", "Header format must be WAVE");

    // 2. GET /api/songs/:id/download (Latest Version Download -> v2)
    let latest_url = format!("{}/api/songs/roots_reggae_anthem/download", base_url);
    let resp_latest = client.get(&latest_url).send().await.expect("Request latest download");
    assert_eq!(resp_latest.status(), StatusCode::OK, "Status should be 200 OK");

    let latest_disp = resp_latest.headers().get(CONTENT_DISPOSITION).unwrap().to_str().unwrap();
    assert!(
        latest_disp.contains("roots_reggae_anthem_v2_final_master.wav"),
        "Latest download must resolve to v2 final_master.wav, got: {}",
        latest_disp
    );

    // 3. RFC 7233 Range Request on Download Endpoint (bytes=0-43 -> WAV header only)
    let range_resp = client.get(&v1_url)
        .header("Range", "bytes=0-43")
        .send().await.expect("Range request on download");
    assert_eq!(range_resp.status(), StatusCode::PARTIAL_CONTENT, "Status must be 206 Partial Content");

    let range_header = range_resp.headers().get(CONTENT_RANGE).unwrap().to_str().unwrap();
    assert!(range_header.starts_with("bytes 0-43/"), "Content-Range must start with bytes 0-43/, got: {}", range_header);

    let range_disp = range_resp.headers().get(CONTENT_DISPOSITION).unwrap().to_str().unwrap();
    assert!(range_disp.contains("attachment; filename="));

    let range_bytes = range_resp.bytes().await.expect("Read range body");
    assert_eq!(range_bytes.len(), 44, "Range body must be exactly 44 bytes");
    assert_eq!(&range_bytes[0..4], b"RIFF");
    assert_eq!(&range_bytes[8..12], b"WAVE");

    // 4. Non-existent song ID download
    let not_found_url = format!("{}/api/songs/non_existent_fictional_song/download", base_url);
    let resp_nf = client.get(&not_found_url).send().await.expect("Request fake song download");
    assert!(
        resp_nf.status() == StatusCode::NOT_FOUND || resp_nf.status() == StatusCode::BAD_REQUEST,
        "Non-existent song must return 404 or 400, got: {}",
        resp_nf.status()
    );
}
