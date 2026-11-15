//! Integration Tests for Zero-Sampling & SSRF Safeguards:
//! 1. Mathematical SSRF IP blocking (`is_ssrf_forbidden_ip`) and URL validation (`validate_url_against_ssrf`).
//! 2. Verification that removed audio sampling (`/api/reference-wavs*`) and proprietary cloud (`/api/gemini-env`) routes return HTTP 404.
//! 3. Open-weight creation + online DPO Keep/Revert preference feedback without any sampling inputs.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;
use serde_json::json;

use music_cli::rest_server::{is_ssrf_forbidden_ip, start_rest_server, validate_url_against_ssrf};
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

#[test]
fn test_pure_mathematical_ssrf_blocking_exhaustive() {
    // 1. Loopback addresses (127.0.0.0/8 & ::1)
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(127, 255, 255, 254))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));

    // 2. RFC 1918 Private networks
    // 10.0.0.0/8
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(10, 254, 1, 99))));
    // 172.16.0.0/12
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1)))); // Outside RFC1918
    // 192.168.0.0/16
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 254, 254))));

    // 3. Link-local & Cloud Metadata (169.254.0.0/16 & fe80::/10)
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
    assert!(is_ssrf_forbidden_ip("fe80::1".parse().unwrap()));

    // 4. CGNAT (100.64.0.0/10)
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(100, 128, 0, 1))));

    // 5. Unspecified & Multicast / Reserved
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
    assert!(is_ssrf_forbidden_ip(IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1))));
    assert!(is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(240, 0, 0, 1))));

    // 6. IPv4-Mapped IPv6
    assert!(is_ssrf_forbidden_ip("::ffff:127.0.0.1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("::ffff:169.254.169.254".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("::ffff:10.0.0.1".parse().unwrap()));
    assert!(!is_ssrf_forbidden_ip("::ffff:8.8.8.8".parse().unwrap()));

    // 7. Unique Local IPv6 (fc00::/7)
    assert!(is_ssrf_forbidden_ip("fc00::1".parse().unwrap()));
    assert!(is_ssrf_forbidden_ip("fd12:3456:789a::1".parse().unwrap()));

    // 8. Public routable IPs must NOT be blocked
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    assert!(!is_ssrf_forbidden_ip(IpAddr::V4(Ipv4Addr::new(140, 82, 121, 4))));
    assert!(!is_ssrf_forbidden_ip("2606:4700:4700::1111".parse().unwrap()));
}

#[tokio::test]
async fn test_removed_sampling_and_legacy_cloud_routes_return_404() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner,
        temp_dir.path().to_path_buf(),
        5201,
        5300,
        "http://127.0.0.1:1".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(client.get(format!("{}/api/reference-wavs", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(client.post(format!("{}/api/reference-wavs/upload", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(client.get(format!("{}/api/reference-wavs/sample_1/audio", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(client.delete(format!("{}/api/reference-wavs/sample_1", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(client.delete(format!("{}/api/reference-wavs", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(client.post(format!("{}/api/reference-wavs/from-url", base_url)).json(&json!({ "url": "https://example.com/a.wav" })).send().await.unwrap().status(), 404);
    assert_eq!(client.post(format!("{}/api/reference-wavs/generate-rl", base_url)).json(&json!({ "prompt": "test" })).send().await.unwrap().status(), 404);
    assert_eq!(client.get(format!("{}/api/gemini-env", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(client.post(format!("{}/api/gemini-env", base_url)).json(&json!({ "api_key": "none" })).send().await.unwrap().status(), 404);
}

#[tokio::test]
async fn test_validate_url_against_ssrf_blocks_loopback_metadata_and_bad_schemes() {
    let err_loopback = validate_url_against_ssrf("http://127.0.0.1:8080/audio.wav").await.unwrap_err();
    assert!(err_loopback.contains("SSRF Violation"), "got: {}", err_loopback);

    let err_meta = validate_url_against_ssrf("http://169.254.169.254/latest/meta-data").await.unwrap_err();
    assert!(err_meta.contains("SSRF Violation"), "got: {}", err_meta);

    let err_rfc1918 = validate_url_against_ssrf("http://10.0.1.5/sample.wav").await.unwrap_err();
    assert!(err_rfc1918.contains("SSRF Violation"), "got: {}", err_rfc1918);

    let err_ftp = validate_url_against_ssrf("ftp://example.com/audio.wav").await.unwrap_err();
    assert!(err_ftp.contains("Forbidden scheme"), "got: {}", err_ftp);
}

#[tokio::test]
async fn test_open_weight_creation_records_dpo_preference_without_sampling() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.2);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner,
        temp_dir.path().to_path_buf(),
        5401,
        5500,
        "http://127.0.0.1:1".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let create_res = client.post(format!("{}/api/songs/create", base_url))
        .json(&json!({
            "prompt": "Roots reggae anthem with warm horn chords",
            "genre": "Roots Reggae",
            "lyrics_mode": "user_provided",
            "lyrics": "[Verse 1]\nOne love shining through the night\n[Chorus]\nRise up in the golden light",
            "hf_model": "ACE-Step/acestep-v15-turbo",
            "pure_rust": true,
            "max_duration_secs": 0.2
        }))
        .send().await.expect("POST /api/songs/create failed");
    assert_eq!(create_res.status(), 200);

    let keep_res = client.post(format!("{}/api/feedback/preference", base_url))
        .json(&json!({ "preference": "keep" }))
        .send().await.expect("POST /api/feedback/preference keep failed");
    assert_eq!(keep_res.status(), 200);
    let keep_json: serde_json::Value = keep_res.json().await.unwrap();
    assert_eq!(keep_json["status"], "ok");
    assert_eq!(keep_json["preference"], "keep");

    let revert_res = client.post(format!("{}/api/feedback/preference", base_url))
        .json(&json!({ "preference": "revert" }))
        .send().await.expect("POST /api/feedback/preference revert failed");
    assert_eq!(revert_res.status(), 200);
    let revert_json: serde_json::Value = revert_res.json().await.unwrap();
    assert_eq!(revert_json["status"], "ok");
    assert_eq!(revert_json["preference"], "revert");

    let bad_pref_res = client.post(format!("{}/api/feedback/preference", base_url))
        .json(&json!({ "preference": "invalid_choice" }))
        .send().await.expect("POST /api/feedback/preference invalid failed");
    assert_eq!(bad_pref_res.status(), 400);
}
