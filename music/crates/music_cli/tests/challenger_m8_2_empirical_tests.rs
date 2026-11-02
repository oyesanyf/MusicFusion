//! Challenger M8-2 Empirical Adversarial Stress Test Suite
//!
//! Empirically tests:
//! 1. REST 20-File Cap & Corruption Handling:
//!    - Exactly 21 files uploaded in a single multipart request -> strictly HTTP 400 Bad Request.
//!    - Corrupt, truncated, and garbage WAV payloads -> strictly HTTP 400 Bad Request.
//!    - Fail-closed atomicity: Mixed batch of valid + corrupt files rejected before storage.
//!    - Exact boundary: 20 files accepted (HTTP 200), subsequent 21st file rejected (HTTP 400).
//! 2. Mathematical SSRF Security:
//!    - Comprehensive boundary checking for loopback, RFC1918, link-local, cloud metadata,
//!      CGNAT, IPv4-mapped IPv6, unique local IPv6, unspecified, and multicast.
//!    - Verified allowance of valid public IPv4 and IPv6 addresses.
//! 3. REST SSRF Validation Endpoint:
//!    - Tests URL conversion endpoint rejecting direct IP literals and reserved schemes.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;
use reqwest::multipart::{Form, Part};
use serde_json::json;

use music_cli::rest_server::{is_ssrf_forbidden_ip, start_rest_server};
use music_dsp::wav::AudioBuffer;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

/// Generates a valid test PCM WAV in bytes with synthetic tone at specified frequency.
fn make_test_wav_bytes(frequency_hz: f32, duration_secs: f32) -> Vec<u8> {
    let mut buffer = AudioBuffer::new_stereo(duration_secs);
    let sample_rate = buffer.sample_rate as f32;
    for f in 0..buffer.frame_count() {
        let t = f as f32 / sample_rate;
        let s = (2.0 * std::f32::consts::PI * frequency_hz * t).sin() * 0.5;
        buffer.samples[f * 2] = s;
        buffer.samples[f * 2 + 1] = s;
    }
    buffer.to_wav_bytes()
}

// =========================================================================
// 1. MATHEMATICAL SSRF SECURITY — EXHAUSTIVE RANGE & BOUNDARY TESTS
// =========================================================================

#[test]
fn test_ssrf_forbidden_ip_exhaustive_challenge() {
    // ---------------------------------------------------------------------
    // A. Loopback Addresses (127.0.0.0/8 and ::1)
    // ---------------------------------------------------------------------
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(127, 123, 45, 67))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(127, 255, 255, 254))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    assert!(is_ssrf_forbidden_ip("::1".parse().unwrap()));

    // ---------------------------------------------------------------------
    // B. RFC 1918 Private Networks & Exact Class Boundaries
    // ---------------------------------------------------------------------
    // 10.0.0.0/8 (10.0.0.0 - 10.255.255.255)
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(10, 100, 200, 50))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
    // Boundaries around 10.0.0.0/8
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(9, 255, 255, 255))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(11, 0, 0, 0))));

    // 172.16.0.0/12 (172.16.0.0 - 172.31.255.255)
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 0))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 20, 10, 5))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
    // Boundaries around 172.16.0.0/12 (Must be allowed public IPs!)
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 15, 255, 255))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 0))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));

    // 192.168.0.0/16 (192.168.0.0 - 192.168.255.255)
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 0))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 254, 254))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));
    // Boundaries around 192.168.0.0/16
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(192, 167, 255, 255))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(192, 169, 0, 1))));

    // ---------------------------------------------------------------------
    // C. Link-Local & AWS/GCP/Azure Cloud Metadata (169.254.0.0/16, fe80::/10)
    // ---------------------------------------------------------------------
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)))); // AWS/GCP/Azure IMDS
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 0, 0))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 255, 255))));
    // Boundaries around 169.254.0.0/16
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(169, 253, 255, 255))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(169, 255, 0, 1))));
    // IPv6 link-local (fe80::/10)
    assert!(is_ssrf_forbidden_ip("fe80::1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("fe80::dead:beef".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("febf:ffff:ffff:ffff:ffff:ffff:ffff:ffff".parse().unwrap()));
    assert!(!is_ssrf_forbidden_ip("fec0::1".parse().unwrap())); // Outside fe80::/10

    // ---------------------------------------------------------------------
    // D. Carrier-Grade NAT (CGNAT) 100.64.0.0/10 (100.64.0.0 - 100.127.255.255)
    // ---------------------------------------------------------------------
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 0))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 100, 50, 25))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255))));
    // Boundaries around CGNAT
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 63, 255, 255))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 128, 0, 0))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 128, 0, 1))));

    // ---------------------------------------------------------------------
    // E. IPv4-Mapped IPv6 (::ffff:0:0/96)
    // ---------------------------------------------------------------------
    assert!(is_ssrf_forbidden_ip("::ffff:127.0.0.1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("::ffff:10.0.0.1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("::ffff:169.254.169.254".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("::ffff:192.168.1.1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("::ffff:172.16.0.1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("::ffff:100.64.0.1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("::ffff:0.0.0.0".parse().unwrap()));
    // Valid public IPv4 mapped into IPv6 must be permitted
    assert!(!is_ssrf_forbidden_ip("::ffff:8.8.8.8".parse().unwrap()));
    assert!(!is_ssrf_forbidden_ip("::ffff:1.1.1.1".parse().unwrap()));

    // ---------------------------------------------------------------------
    // F. Unique Local IPv6 (fc00::/7)
    // ---------------------------------------------------------------------
    assert!(is_ssrf_forbidden_ip("fc00::1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("fc00::dead:beef".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("fd00::1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("fd12:3456:789a::1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("fdff:ffff:ffff:ffff:ffff:ffff:ffff:ffff".parse().unwrap()));
    // Boundary around fc00::/7
    assert!(!is_ssrf_forbidden_ip("fbff:ffff:ffff:ffff:ffff:ffff:ffff:ffff".parse().unwrap()));

    // ---------------------------------------------------------------------
    // G. Unspecified & Multicast / Reserved
    // ---------------------------------------------------------------------
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(0, 255, 255, 255))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
    assert!(is_ssrf_forbidden_ip("::".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(239, 255, 255, 255))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(240, 0, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255))));
    assert!(is_ssrf_forbidden_ip("ff00::1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("ff02::1".parse().unwrap()));

    // ---------------------------------------------------------------------
    // H. Valid Public Routable IPs MUST NOT BE FORBIDDEN
    // ---------------------------------------------------------------------
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)))); // Google DNS
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 4, 4))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)))); // Cloudflare DNS
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(1, 0, 0, 1))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)))); // example.com
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(140, 82, 121, 4)))); // GitHub
    assert!(!is_ssrf_forbidden_ip("2606:4700:4700::1111".parse().unwrap())); // Cloudflare IPv6
    assert!(!is_ssrf_forbidden_ip("2001:4860:4860::8888".parse().unwrap())); // Google IPv6
}

// =========================================================================
// 2. REST 20-FILE CAP & CORRUPTION EMPIRICAL HARNESS
// =========================================================================

#[tokio::test]
async fn test_rest_single_request_21_file_cap_rejection() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        5601,
        5700,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Build a multipart request with EXACTLY 21 valid WAV files
    let mut form_21 = Form::new();
    let sample_wav = make_test_wav_bytes(440.0, 0.05);
    for i in 1..=21 {
        form_21 = form_21.part(
            format!("file_{}", i),
            Part::bytes(sample_wav.clone())
                .file_name(format!("track_{:02}.wav", i))
                .mime_str("audio/wav")
                .unwrap(),
        );
    }

    let res_21 = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(form_21)
        .send().await.expect("POST 21 files failed");

    assert_eq!(res_21.status(), 400, "Uploading 21 files MUST return HTTP 400 Bad Request");
    let json_err: serde_json::Value = res_21.json().await.unwrap();
    let err_str = json_err["error"].as_str().expect("error message present");
    assert!(
        err_str.contains("exceeds maximum of 20 files"),
        "Error message must specify 20 files cap, got: {}",
        err_str
    );
    assert!(
        err_str.contains("incoming: 21"),
        "Error message must specify incoming count of 21, got: {}",
        err_str
    );

    // Verify storage remained completely empty (zero files accepted)
    let list_res = client.get(format!("{}/api/reference-wavs", base_url))
        .send().await.expect("GET /api/reference-wavs failed");
    assert_eq!(list_res.status(), 200);
    let list_json: serde_json::Value = list_res.json().await.unwrap();
    assert_eq!(list_json["count"], 0, "No files should be stored after rejection");
}

#[tokio::test]
async fn test_rest_boundary_20_files_accepted_then_overflow_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        5701,
        5800,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Upload exactly 20 files (the maximum allowed cap)
    let mut form_20 = Form::new();
    let sample_wav = make_test_wav_bytes(440.0, 0.05);
    for i in 1..=20 {
        form_20 = form_20.part(
            format!("file_{}", i),
            Part::bytes(sample_wav.clone())
                .file_name(format!("track_{:02}.wav", i))
                .mime_str("audio/wav")
                .unwrap(),
        );
    }

    let res_20 = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(form_20)
        .send().await.expect("POST 20 files failed");

    assert_eq!(res_20.status(), 200, "Uploading exactly 20 files MUST succeed with HTTP 200");
    let json_20: serde_json::Value = res_20.json().await.unwrap();
    assert_eq!(json_20["uploaded_count"], 20);
    assert_eq!(json_20["total_count"], 20);

    // Now try uploading 1 additional file (20 + 1 = 21 -> must be rejected)
    let overflow_form = Form::new().part(
        "extra_file",
        Part::bytes(sample_wav.clone())
            .file_name("track_21.wav")
            .mime_str("audio/wav")
            .unwrap(),
    );

    let res_overflow = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(overflow_form)
        .send().await.expect("POST 21st file failed");

    assert_eq!(res_overflow.status(), 400, "21st file must be rejected with HTTP 400");
    let json_over: serde_json::Value = res_overflow.json().await.unwrap();
    assert!(json_over["error"].as_str().unwrap().contains("exceeds maximum of 20 files"));
    assert!(json_over["error"].as_str().unwrap().contains("current: 20"));
}

#[tokio::test]
async fn test_rest_corrupt_and_truncated_wav_variations() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        5801,
        5900,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test Case 1: Truncated RIFF header (only 8 bytes)
    let truncated_riff = vec![b'R', b'I', b'F', b'F', 0x10, 0x00, 0x00, 0x00];
    let form1 = Form::new().part(
        "file",
        Part::bytes(truncated_riff).file_name("trunc_riff.wav").mime_str("audio/wav").unwrap(),
    );
    let res1 = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(form1)
        .send().await.unwrap();
    assert_eq!(res1.status(), 400, "Truncated RIFF must return HTTP 400");
    let j1: serde_json::Value = res1.json().await.unwrap();
    assert!(j1["error"].as_str().unwrap().contains("Corrupt or invalid WAV file"));

    // Test Case 2: Random garbage bytes (50 bytes of non-WAV data)
    let garbage = vec![0xCA, 0xFE, 0xBA, 0xBE, 0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67];
    let form2 = Form::new().part(
        "file",
        Part::bytes(garbage).file_name("garbage.wav").mime_str("audio/wav").unwrap(),
    );
    let res2 = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(form2)
        .send().await.unwrap();
    assert_eq!(res2.status(), 400, "Garbage data must return HTTP 400");

    // Test Case 3: Truncated valid WAV (< 50ms audio / insufficient samples for FFT analysis)
    let tiny_wav = make_test_wav_bytes(440.0, 0.01); // 10ms = 480 frames, below 1024 minimum FFT requirement
    let form3 = Form::new().part(
        "file",
        Part::bytes(tiny_wav).file_name("too_short.wav").mime_str("audio/wav").unwrap(),
    );
    let res3 = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(form3)
        .send().await.unwrap();
    assert_eq!(res3.status(), 400, "Truncated audio under 50ms must return HTTP 400");
    let j3: serde_json::Value = res3.json().await.unwrap();
    assert!(j3["error"].as_str().unwrap().contains("Acoustic analysis failed"));

    // Test Case 4: Zero-sample WAV (44-byte header with 0 data size)
    let zero_wav = AudioBuffer::new_stereo(0.0).to_wav_bytes();
    let form4 = Form::new().part(
        "file",
        Part::bytes(zero_wav).file_name("zero.wav").mime_str("audio/wav").unwrap(),
    );
    let res4 = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(form4)
        .send().await.unwrap();
    assert_eq!(res4.status(), 400, "Zero-sample WAV must return HTTP 400");

    // Test Case 5: Fail-closed transactionality — 2 valid WAVs mixed with 1 corrupt WAV
    let valid_a = make_test_wav_bytes(300.0, 0.1);
    let valid_b = make_test_wav_bytes(500.0, 0.1);
    let bad = vec![0xFF; 32];
    let mixed_form = Form::new()
        .part("f1", Part::bytes(valid_a).file_name("good_a.wav").mime_str("audio/wav").unwrap())
        .part("f2", Part::bytes(bad).file_name("bad.wav").mime_str("audio/wav").unwrap())
        .part("f3", Part::bytes(valid_b).file_name("good_b.wav").mime_str("audio/wav").unwrap());

    let mixed_res = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(mixed_form)
        .send().await.unwrap();
    assert_eq!(mixed_res.status(), 400, "Batch containing any corrupt file must be rejected with HTTP 400");

    // Verify 0 files stored
    let list_res = client.get(format!("{}/api/reference-wavs", base_url)).send().await.unwrap();
    let list_json: serde_json::Value = list_res.json().await.unwrap();
    assert_eq!(list_json["count"], 0, "No files should be committed when batch fails");
}

#[tokio::test]
async fn test_url_conversion_rejects_ssrf_and_invalid_schemes() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        5901,
        6000,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test loopback IP
    let r1 = client.post(format!("{}/api/reference-wavs/from-url", base_url))
        .json(&json!({ "url": "http://127.0.0.1:9090/evil.wav" }))
        .send().await.unwrap();
    assert_eq!(r1.status(), 400);
    assert!(r1.json::<serde_json::Value>().await.unwrap()["error"].as_str().unwrap().contains("SSRF Violation"));

    // Test AWS metadata IP
    let r2 = client.post(format!("{}/api/reference-wavs/from-url", base_url))
        .json(&json!({ "url": "http://169.254.169.254/latest/meta-data" }))
        .send().await.unwrap();
    assert_eq!(r2.status(), 400);

    // Test CGNAT IP
    let r3 = client.post(format!("{}/api/reference-wavs/from-url", base_url))
        .json(&json!({ "url": "http://100.64.1.1/stream.wav" }))
        .send().await.unwrap();
    assert_eq!(r3.status(), 400);

    // Test file:// scheme
    let r4 = client.post(format!("{}/api/reference-wavs/from-url", base_url))
        .json(&json!({ "url": "file:///etc/passwd" }))
        .send().await.unwrap();
    assert_eq!(r4.status(), 400);
    assert!(r4.json::<serde_json::Value>().await.unwrap()["error"].as_str().unwrap().contains("Forbidden scheme"));

    // Test gopher:// scheme
    let r5 = client.post(format!("{}/api/reference-wavs/from-url", base_url))
        .json(&json!({ "url": "gopher://127.0.0.1:70/" }))
        .send().await.unwrap();
    assert_eq!(r5.status(), 400);
}
