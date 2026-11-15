//! Challenger M8-2 Empirical Adversarial Security & Route Removal Suite
//!
//! Empirically tests:
//! 1. Mathematical SSRF Security:
//!    - Comprehensive boundary checking for loopback, RFC1918, link-local, cloud metadata,
//!      CGNAT, IPv4-mapped IPv6, unique local IPv6, unspecified, and multicast.
//!    - Verified allowance of valid public IPv4 and IPv6 addresses.
//! 2. SSRF URL Validation & Sampling Route Removal:
//!    - Tests `validate_url_against_ssrf` directly against IP literals and reserved schemes.
//!    - Verifies removed `/api/reference-wavs*` endpoints return HTTP 404.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;
use serde_json::json;

use music_cli::rest_server::{is_ssrf_forbidden_ip, start_rest_server, validate_url_against_ssrf};
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

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
// 2. SSRF URL VALIDATION & SAMPLING ROUTE REMOVAL BOUNDARY TESTS
// =========================================================================

#[tokio::test]
async fn test_validate_url_against_ssrf_rejects_reserved_ranges_and_schemes() {
    let r1 = validate_url_against_ssrf("http://127.0.0.1:9090/evil.wav").await.unwrap_err();
    assert!(r1.contains("SSRF Violation"), "got: {}", r1);

    let r2 = validate_url_against_ssrf("http://169.254.169.254/latest/meta-data").await.unwrap_err();
    assert!(r2.contains("SSRF Violation"), "got: {}", r2);

    let r3 = validate_url_against_ssrf("http://100.64.1.1/stream.wav").await.unwrap_err();
    assert!(r3.contains("SSRF Violation"), "got: {}", r3);

    let r4 = validate_url_against_ssrf("file:///etc/passwd").await.unwrap_err();
    assert!(r4.contains("Forbidden scheme"), "got: {}", r4);

    let r5 = validate_url_against_ssrf("gopher://127.0.0.1:70/").await.unwrap_err();
    assert!(r5.contains("Forbidden scheme"), "got: {}", r5);
}

#[tokio::test]
async fn test_removed_reference_wav_endpoints_strictly_unreachable() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner,
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

    assert_eq!(client.get(format!("{}/api/reference-wavs", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(client.post(format!("{}/api/reference-wavs/upload", base_url)).send().await.unwrap().status(), 404);
    assert_eq!(client.post(format!("{}/api/reference-wavs/from-url", base_url)).json(&json!({ "url": "http://example.com/a.wav" })).send().await.unwrap().status(), 404);
    assert_eq!(client.post(format!("{}/api/reference-wavs/generate-rl", base_url)).json(&json!({ "prompt": "test" })).send().await.unwrap().status(), 404);
}
