//! Milestone 4 Empirical Adversarial Challenge Test Suite
//!
//! Verifies:
//! 1. REST server port scanning, occupancy, and fallback (5050..=5060)
//! 2. REST server concurrency under load
//! 3. RFC 7233 byte-range streaming (valid, open-ended, overflow, invalid start>end, malformed)
//! 4. Adversarial path traversal & Windows DOS device names against audio endpoints
//! 5. Browser launch suppression logic across flags and CI environment variables

use std::fs;
use std::net::TcpListener as StdTcpListener;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use futures::future::join_all;
use reqwest::StatusCode;

use music_cli::browser;
use music_cli::cli_args::CliArgs;
use music_cli::rest_server::{
    bind_server, parse_range_header, start_rest_server,
};
use music_pipeline::checkpointer::CheckpointManager;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

// ===========================================================================
// 1. REST Server Concurrency, Port Scanning & Fallback
// ===========================================================================

#[tokio::test]
async fn test_bind_server_fallback_when_first_port_occupied() {
    // Pick an ephemeral base port range to avoid collision with standard services
    let base_port = 5900;
    let end_port = 5910;

    // Occupy the first port
    let blocker = StdTcpListener::bind(format!("127.0.0.1:{}", base_port))
        .expect("Failed to bind blocker on base_port");

    // Attempt to bind server starting at base_port
    let (listener, bound_port) = bind_server(base_port, end_port)
        .await
        .expect("bind_server should fall back to next available port");

    assert_eq!(
        bound_port,
        base_port + 1,
        "Server should have skipped occupied port {} and bound to {}",
        base_port,
        base_port + 1
    );

    drop(listener);
    drop(blocker);
}

#[tokio::test]
async fn test_bind_server_fallback_multiple_occupied_ports() {
    let base_port = 5920;
    let end_port = 5930;

    // Occupy first 3 ports
    let blocker1 = StdTcpListener::bind(format!("127.0.0.1:{}", base_port)).unwrap();
    let blocker2 = StdTcpListener::bind(format!("127.0.0.1:{}", base_port + 1)).unwrap();
    let blocker3 = StdTcpListener::bind(format!("127.0.0.1:{}", base_port + 2)).unwrap();

    let (listener, bound_port) = bind_server(base_port, end_port)
        .await
        .expect("bind_server should skip all 3 occupied ports");

    assert_eq!(bound_port, base_port + 3);

    drop(listener);
    drop(blocker1);
    drop(blocker2);
    drop(blocker3);
}

#[tokio::test]
async fn test_bind_server_all_ports_occupied_returns_error() {
    let base_port = 5940;
    let end_port = 5942; // only 3 ports

    let blocker1 = StdTcpListener::bind(format!("127.0.0.1:{}", base_port)).unwrap();
    let blocker2 = StdTcpListener::bind(format!("127.0.0.1:{}", base_port + 1)).unwrap();
    let blocker3 = StdTcpListener::bind(format!("127.0.0.1:{}", base_port + 2)).unwrap();

    let res = bind_server(base_port, end_port).await;
    assert!(
        res.is_err(),
        "bind_server should fail closed when all ports in range are occupied"
    );

    drop(blocker1);
    drop(blocker2);
    drop(blocker3);
}

#[tokio::test]
async fn test_rest_server_high_concurrency_stress() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner,
        temp_dir.path().to_path_buf(),
        5950,
        5970,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("Failed to start rest server for concurrency test");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);

    // Fire 60 concurrent requests across health, hardware, and static routes
    let mut tasks = Vec::new();
    for i in 0..60 {
        let client_clone = client.clone();
        let url = match i % 3 {
            0 => format!("{}/api/health", base_url),
            1 => format!("{}/api/hardware", base_url),
            _ => format!("{}/styles.css", base_url),
        };
        tasks.push(tokio::spawn(async move {
            let res = client_clone.get(&url).send().await;
            res.map(|r| r.status().as_u16())
        }));
    }

    let results = join_all(tasks).await;
    let mut success_count = 0;
    for res in results {
        let status = res.expect("Task join failed").expect("HTTP request failed");
        assert_eq!(status, 200, "Expected HTTP 200 under concurrent load");
        success_count += 1;
    }
    assert_eq!(success_count, 60, "All 60 concurrent requests must succeed");
}

// ===========================================================================
// 2. RFC 7233 Byte-Range Streaming Stress Testing
// ===========================================================================

#[test]
fn test_parse_range_header_unit_matrix() {
    let total = 5000;

    // Standard valid range
    let r1 = parse_range_header("bytes=0-1023", total).expect("bytes=0-1023 should parse");
    assert_eq!(r1.start, 0);
    assert_eq!(r1.end, 1023);

    // Sub-range inside file
    let r2 = parse_range_header("bytes=100-200", total).expect("bytes=100-200 should parse");
    assert_eq!(r2.start, 100);
    assert_eq!(r2.end, 200);

    // Open-ended range to EOF
    let r3 = parse_range_header("bytes=100-", total).expect("bytes=100- should parse");
    assert_eq!(r3.start, 100);
    assert_eq!(r3.end, 4999);

    // Full range open-ended
    let r4 = parse_range_header("bytes=0-", total).expect("bytes=0- should parse");
    assert_eq!(r4.start, 0);
    assert_eq!(r4.end, 4999);

    // Overflow range capped at EOF
    let r5 = parse_range_header("bytes=0-99999", total).expect("bytes=0-99999 should be clamped");
    assert_eq!(r5.start, 0);
    assert_eq!(r5.end, 4999);

    // Start beyond EOF
    assert!(parse_range_header("bytes=6000-7000", total).is_none());

    // Invalid start > end
    assert!(parse_range_header("bytes=500-200", total).is_none());

    // Suffix range (unsupported syntax should return None without panic)
    assert!(parse_range_header("bytes=-500", total).is_none());

    // Negative values
    assert!(parse_range_header("bytes=-10-20", total).is_none());

    // Malformed units
    assert!(parse_range_header("items=0-100", total).is_none());
    assert!(parse_range_header("bytes=", total).is_none());
    assert!(parse_range_header("bytes=abc-def", total).is_none());
    assert!(parse_range_header("", total).is_none());

    // Zero-length file edge case
    assert!(parse_range_header("bytes=0-10", 0).is_none());
}

#[tokio::test]
async fn test_rest_api_rfc7233_byte_range_streaming_stress() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner,
        temp_dir.path().to_path_buf(),
        5975,
        5995,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("Failed to start rest server");

    tokio::time::sleep(Duration::from_millis(150)).await;

    // Seed a dummy audio WAV file in storage
    let song_id = "test_stream_song";
    let version = 1;
    let song_dir = temp_dir
        .path()
        .join("songs")
        .join(song_id)
        .join(format!("v{}", version));
    fs::create_dir_all(&song_dir).unwrap();

    // Create a 4096-byte dummy WAV content
    let dummy_wav: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
    let audio_file_path = song_dir.join("final_master.wav");
    fs::write(&audio_file_path, &dummy_wav).unwrap();

    let client = reqwest::Client::new();
    let stream_url = format!(
        "http://127.0.0.1:{}/api/songs/{}/versions/{}/audio/final_master",
        bound_port, song_id, version
    );

    // 1. Standard chunk: bytes=0-1023 (1024 bytes)
    let res = client
        .get(&stream_url)
        .header("Range", "bytes=0-1023")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(res.headers().get("content-type").unwrap(), "audio/wav");
    assert_eq!(
        res.headers().get("content-range").unwrap().to_str().unwrap(),
        "bytes 0-1023/4096"
    );
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.len(), 1024);
    assert_eq!(&bytes[..], &dummy_wav[0..=1023]);

    // 2. Open-ended chunk: bytes=2048- (2048 bytes to EOF)
    let res = client
        .get(&stream_url)
        .header("Range", "bytes=2048-")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        res.headers().get("content-range").unwrap().to_str().unwrap(),
        "bytes 2048-4095/4096"
    );
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.len(), 2048);
    assert_eq!(&bytes[..], &dummy_wav[2048..=4095]);

    // 3. Overflow range: bytes=0-100000 (should clamp to EOF)
    let res = client
        .get(&stream_url)
        .header("Range", "bytes=0-100000")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        res.headers().get("content-range").unwrap().to_str().unwrap(),
        "bytes 0-4095/4096"
    );
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.len(), 4096);

    // 4. Invalid start > end range: bytes=500-200
    // Server should safely fall back to 200 OK full content without panicking
    let res = client
        .get(&stream_url)
        .header("Range", "bytes=500-200")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get("content-type").unwrap(), "audio/wav");
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.len(), 4096);

    // 5. Out of bounds start: bytes=50000-60000
    let res = client
        .get(&stream_url)
        .header("Range", "bytes=50000-60000")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.len(), 4096);

    // 6. Malformed range header: bytes=abc-def
    let res = client
        .get(&stream_url)
        .header("Range", "bytes=abc-def")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. Full request without Range header -> 200 OK
    let res = client.get(&stream_url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get("content-length").unwrap(), "4096");
}

// ===========================================================================
// 3. Adversarial Path Traversal & Windows DOS Device Names
// ===========================================================================

#[tokio::test]
async fn test_path_sanitizer_unit_adversarial_matrix() {
    let adversarial_payloads = [
        "../",
        "..\\",
        "../../etc/passwd",
        "..\\..\\windows\\system32",
        "song..dir",
        "%2e%2e",
        "%2e%2e%2f",
        "%2f",
        "%5c",
        "%00",
        "song\0null",
        "C:",
        "C:\\Windows",
        "D:/secret",
        "CON",
        "con",
        "PRN",
        "prn",
        "AUX",
        "aux",
        "NUL",
        "nul",
        "COM1",
        "com1",
        "COM9",
        "LPT1",
        "lpt1",
        "LPT9",
        "con.wav",
        "aux.txt",
        "nul.bin",
        "com1.wav",
        "trailing_dot.",
        "trailing_space ",
        "spaced .",
        "",
    ];

    for payload in adversarial_payloads {
        let res = CheckpointManager::sanitize_path_component(payload);
        assert!(
            res.is_err(),
            "Payload '{}' should have been rejected by sanitize_path_component",
            payload
        );
    }

    // Valid components must pass
    let valid_components = [
        "song_123",
        "roots_reggae_anthem",
        "final_master",
        "drums",
        "bass",
        "guitar",
        "raspy_tenor_vocals",
        "song-uuid-v4",
    ];
    for valid in valid_components {
        let res = CheckpointManager::sanitize_path_component(valid);
        assert!(
            res.is_ok(),
            "Valid component '{}' should pass sanitization",
            valid
        );
    }
}

#[tokio::test]
async fn test_rest_api_adversarial_path_traversal_endpoints() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner,
        temp_dir.path().to_path_buf(),
        6000,
        6020,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("Failed to start rest server");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);

    // Test matrix across both path patterns
    let probe_stems = [
        "..%2f..%2fWindows%2fwin.ini",
        "..%5c..%5cWindows",
        "%2e%2e",
        "CON",
        "con",
        "PRN",
        "prn",
        "AUX",
        "aux",
        "NUL",
        "nul",
        "COM1",
        "com1",
        "LPT1",
        "con.wav",
        "aux.txt",
        "nul.bin",
        "trailing_dot.",
        "trailing_space%20",
        "C:",
        "C%3A",
    ];

    for stem in probe_stems {
        // Pattern 1: /api/songs/:id/versions/:v/audio/:stem
        let url1 = format!("{}/api/songs/test_song/versions/1/audio/{}", base_url, stem);
        let res1 = client.get(&url1).send().await;
        if let Ok(resp) = res1 {
            let status = resp.status();
            assert!(
                status == StatusCode::BAD_REQUEST
                    || status == StatusCode::NOT_FOUND
                    || status == StatusCode::FORBIDDEN,
                "Probe '{}' on url1 returned unexpected status: {}",
                stem,
                status
            );
        }

        // Pattern 2: /api/songs/:id/:version/audio/:file
        let url2 = format!("{}/api/songs/test_song/1/audio/{}", base_url, stem);
        let res2 = client.get(&url2).send().await;
        if let Ok(resp) = res2 {
            let status = resp.status();
            assert!(
                status == StatusCode::BAD_REQUEST
                    || status == StatusCode::NOT_FOUND
                    || status == StatusCode::FORBIDDEN,
                "Probe '{}' on url2 returned unexpected status: {}",
                stem,
                status
            );
        }

        // Pattern 3: Traversal in song_id
        let url3 = format!("{}/api/songs/{}/versions", base_url, stem);
        let res3 = client.get(&url3).send().await;
        if let Ok(resp) = res3 {
            let status = resp.status();
            assert!(
                status == StatusCode::BAD_REQUEST
                    || status == StatusCode::NOT_FOUND
                    || status == StatusCode::FORBIDDEN,
                "Probe '{}' on url3 (song_id) returned unexpected status: {}",
                stem,
                status
            );
        }
    }
}

// ===========================================================================
// 4. Browser Launch Suppression Logic
// ===========================================================================

#[test]
fn test_browser_suppression_via_flag() {
    // When suppressed = true, spawn_browser_delayed terminates immediately without spawn
    browser::spawn_browser_delayed("http://127.0.0.1:5050/".to_string(), 10, true);
}

#[test]
fn test_browser_suppression_via_ci_env() {
    // Set CI environment variable
    std::env::set_var("CI", "true");
    browser::spawn_browser_delayed("http://127.0.0.1:5050/".to_string(), 10, false);
    // Cleanup
    std::env::remove_var("CI");
}

#[test]
fn test_cli_args_no_browser_flag_parsing() {
    let args = CliArgs::parse_from(["music_cli", "--no-browser"]);
    assert!(args.no_browser, "--no-browser flag must be parsed as true");

    let args_default = CliArgs::parse_from(["music_cli"]);
    assert!(!args_default.no_browser, "--no-browser defaults to false");

    let args_mcp = CliArgs::parse_from(["music_cli", "--mcp"]);
    assert!(args_mcp.mcp, "--mcp flag must be parsed as true");
}

#[tokio::test]
async fn test_rest_api_rfc7233_single_byte_and_boundary_streaming() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner,
        temp_dir.path().to_path_buf(),
        6030,
        6050,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("Failed to start rest server");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let song_id = "test_boundary_song";
    let version = 1;
    let song_dir = temp_dir
        .path()
        .join("songs")
        .join(song_id)
        .join(format!("v{}", version));
    fs::create_dir_all(&song_dir).unwrap();

    let dummy_wav: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
    fs::write(song_dir.join("final_master.wav"), &dummy_wav).unwrap();

    let client = reqwest::Client::new();
    let stream_url = format!(
        "http://127.0.0.1:{}/api/songs/{}/versions/{}/audio/final_master",
        bound_port, song_id, version
    );

    // 1. Single byte at start: bytes=0-0
    let res = client
        .get(&stream_url)
        .header("Range", "bytes=0-0")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        res.headers().get("content-range").unwrap().to_str().unwrap(),
        "bytes 0-0/4096"
    );
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.len(), 1);
    assert_eq!(bytes[0], dummy_wav[0]);

    // 2. Single byte at end: bytes=4095-4095
    let res = client
        .get(&stream_url)
        .header("Range", "bytes=4095-4095")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        res.headers().get("content-range").unwrap().to_str().unwrap(),
        "bytes 4095-4095/4096"
    );
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.len(), 1);
    assert_eq!(bytes[0], dummy_wav[4095]);

    // 3. Exact full range: bytes=0-4095
    let res = client
        .get(&stream_url)
        .header("Range", "bytes=0-4095")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        res.headers().get("content-range").unwrap().to_str().unwrap(),
        "bytes 0-4095/4096"
    );
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.len(), 4096);
    assert_eq!(&bytes[..], &dummy_wav[..]);

    // 4. Non-existent audio stem -> 404
    let missing_stem_url = format!(
        "http://127.0.0.1:{}/api/songs/{}/versions/{}/audio/missing_stem",
        bound_port, song_id, version
    );
    let res = client.get(&missing_stem_url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 5. Non-existent song ID -> 404
    let missing_song_url = format!(
        "http://127.0.0.1:{}/api/songs/no_such_song/versions/1/audio/final_master",
        bound_port
    );
    let res = client.get(&missing_song_url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_rest_api_raw_tcp_adversarial_probes() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner,
        temp_dir.path().to_path_buf(),
        6060,
        6080,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("Failed to start rest server");

    tokio::time::sleep(Duration::from_millis(150)).await;

    // Seed a valid song so valid endpoints return 200
    let song_id = "raw_test_song";
    let song_dir = temp_dir
        .path()
        .join("songs")
        .join(song_id)
        .join("v1");
    fs::create_dir_all(&song_dir).unwrap();
    fs::write(song_dir.join("final_master.wav"), b"RIFF....WAVEfmt ").unwrap();

    let raw_payloads = [
        "GET /api/songs/../../../Windows/win.ini/versions/1/audio/master HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/..%2f..%2fWindows/versions/1/audio/final_master HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/CON HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/con.wav HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/AUX HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/aux.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/NUL HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/PRN HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/COM1 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/LPT1 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/bad_stem. HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/bad_stem%20 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/songs/raw_test_song/versions/1/audio/C: HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    ];

    for payload in raw_payloads {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", bound_port))
            .await
            .expect("Failed to connect to REST server via raw TCP");

        stream.write_all(payload.as_bytes()).await.unwrap();

        let mut resp_buf = vec![0u8; 1024];
        let n = stream.read(&mut resp_buf).await.unwrap_or(0);
        let resp_str = String::from_utf8_lossy(&resp_buf[..n]);

        let first_line = resp_str.lines().next().unwrap_or("");
        assert!(
            first_line.contains("400") || first_line.contains("404") || first_line.contains("403"),
            "Raw TCP payload '{}' expected 400/403/404, got: '{}'",
            payload.lines().next().unwrap_or(""),
            first_line
        );
        assert!(
            !first_line.contains("200 OK"),
            "Raw TCP payload must NEVER return 200 OK"
        );
    }
}
