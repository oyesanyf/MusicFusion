//! Integration Tests for Milestone 8:
//! 1. 1..=20 Reference WAV upload and strict 20-file capacity cap.
//! 2. Mathematical SSRF validation and URL conversion rejection.
//! 3. Reference WAV audition streaming and deletion.
//! 4. RL generation with Bridgeport and ELVIS Act legal compliance verification.
//! 5. Interactive DPO preference learning on Keep / Revert pairs.

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
async fn test_reference_wav_upload_listing_cap_and_audition() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        5201,
        5300,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 1. Initial listing: 0 files
    let list_res = client.get(format!("{}/api/reference-wavs", base_url))
        .send().await.expect("GET /api/reference-wavs failed");
    assert_eq!(list_res.status(), 200);
    let list_json: serde_json::Value = list_res.json().await.unwrap();
    assert_eq!(list_json["count"], 0);
    assert_eq!(list_json["max_allowed"], 20);

    // 2. Upload 3 valid WAV files
    let wav1 = make_test_wav_bytes(440.0, 1.0);
    let wav2 = make_test_wav_bytes(554.37, 1.0);
    let wav3 = make_test_wav_bytes(659.25, 1.0);

    let form = Form::new()
        .part("file1", Part::bytes(wav1).file_name("track_a.wav").mime_str("audio/wav").unwrap())
        .part("file2", Part::bytes(wav2).file_name("track_b.wav").mime_str("audio/wav").unwrap())
        .part("file3", Part::bytes(wav3).file_name("track_c.wav").mime_str("audio/wav").unwrap());

    let upload_res = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(form)
        .send().await.expect("POST /api/reference-wavs/upload failed");
    assert_eq!(upload_res.status(), 200);
    let upload_json: serde_json::Value = upload_res.json().await.unwrap();
    assert_eq!(upload_json["status"], "ok");
    assert_eq!(upload_json["uploaded_count"], 3);
    assert_eq!(upload_json["total_count"], 3);

    let uploaded = upload_json["reference_wavs"].as_array().unwrap();
    assert_eq!(uploaded.len(), 3);
    let first_id = uploaded[0]["id"].as_str().unwrap().to_string();

    // 3. Audition the first reference WAV
    let stream_res = client.get(format!("{}/api/reference-wavs/{}/audio", base_url, first_id))
        .send().await.expect("GET /api/reference-wavs/:id/audio failed");
    assert_eq!(stream_res.status(), 200);
    assert_eq!(stream_res.headers().get("content-type").unwrap(), "audio/wav");
    let audio_bytes = stream_res.bytes().await.unwrap();
    assert!(!audio_bytes.is_empty());
    let parsed_buffer = AudioBuffer::from_wav_bytes(&audio_bytes).expect("Audition WAV must be valid");
    assert_eq!(parsed_buffer.channels, 2);
    assert_eq!(parsed_buffer.sample_rate, 48000);

    // 4. Test Rejecting Corrupt Upload
    let corrupt_form = Form::new()
        .part("bad_file", Part::bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]).file_name("bad.wav").mime_str("audio/wav").unwrap());
    let corrupt_res = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(corrupt_form)
        .send().await.expect("POST corrupt file failed");
    assert_eq!(corrupt_res.status(), 400);

    // 5. Test Enforcing Strict 20-file Cap
    // Currently at 3 files. Uploading 18 more would reach 21 (> 20), must be rejected with 400.
    let mut overflow_form = Form::new();
    let sample_wav = make_test_wav_bytes(440.0, 0.5);
    for i in 0..18 {
        overflow_form = overflow_form.part(
            format!("file_{}", i),
            Part::bytes(sample_wav.clone()).file_name(format!("over_{}.wav", i)).mime_str("audio/wav").unwrap(),
        );
    }
    let overflow_res = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(overflow_form)
        .send().await.expect("POST overflow files failed");
    assert_eq!(overflow_res.status(), 400);
    let overflow_json: serde_json::Value = overflow_res.json().await.unwrap();
    assert!(overflow_json["error"].as_str().unwrap().contains("exceeds maximum of 20 files"));

    // 6. Delete single track
    let del_res = client.delete(format!("{}/api/reference-wavs/{}", base_url, first_id))
        .send().await.expect("DELETE /api/reference-wavs/:id failed");
    assert_eq!(del_res.status(), 200);
    let del_json: serde_json::Value = del_res.json().await.unwrap();
    assert_eq!(del_json["remaining_count"], 2);

    // 7. Clear all reference WAVs
    let clear_res = client.delete(format!("{}/api/reference-wavs", base_url))
        .send().await.expect("DELETE /api/reference-wavs failed");
    assert_eq!(clear_res.status(), 200);
    let clear_json: serde_json::Value = clear_res.json().await.unwrap();
    assert_eq!(clear_json["remaining_count"], 0);
}

#[tokio::test]
async fn test_url_ssrf_rejection_at_rest_endpoint() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        5301,
        5400,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test loopback IP URL rejection
    let loopback_res = client.post(format!("{}/api/reference-wavs/from-url", base_url))
        .json(&json!({ "url": "http://127.0.0.1:8080/audio.wav" }))
        .send().await.expect("POST /from-url loopback failed");
    assert_eq!(loopback_res.status(), 400);
    let json_loopback: serde_json::Value = loopback_res.json().await.unwrap();
    assert!(json_loopback["error"].as_str().unwrap().contains("SSRF Violation"));

    // Test cloud metadata IP rejection
    let metadata_res = client.post(format!("{}/api/reference-wavs/from-url", base_url))
        .json(&json!({ "url": "http://169.254.169.254/latest/meta-data" }))
        .send().await.expect("POST /from-url metadata failed");
    assert_eq!(metadata_res.status(), 400);
    let json_meta: serde_json::Value = metadata_res.json().await.unwrap();
    assert!(json_meta["error"].as_str().unwrap().contains("SSRF Violation"));

    // Test private RFC1918 IP rejection
    let rfc1918_res = client.post(format!("{}/api/reference-wavs/from-url", base_url))
        .json(&json!({ "url": "http://10.0.1.5/sample.wav" }))
        .send().await.expect("POST /from-url rfc1918 failed");
    assert_eq!(rfc1918_res.status(), 400);

    // Test non-http scheme rejection
    let ftp_res = client.post(format!("{}/api/reference-wavs/from-url", base_url))
        .json(&json!({ "url": "ftp://example.com/audio.wav" }))
        .send().await.expect("POST /from-url ftp failed");
    assert_eq!(ftp_res.status(), 400);
    let json_ftp: serde_json::Value = ftp_res.json().await.unwrap();
    assert!(json_ftp["error"].as_str().unwrap().contains("Forbidden scheme"));
}

#[tokio::test]
async fn test_rl_generation_legal_safeguards_and_dpo_learning() {
    let temp_dir = tempfile::tempdir().unwrap();
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        5401,
        5500,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("start_rest_server failed");

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 1. Attempt RL generation without reference WAVs -> HTTP 400 Bad Request
    let no_refs_res = client.post(format!("{}/api/reference-wavs/generate-rl", base_url))
        .json(&json!({ "prompt": "Blended groove" }))
        .send().await.expect("POST /generate-rl without refs failed");
    assert_eq!(no_refs_res.status(), 400);

    // 2. Upload 3 reference WAVs with distinct chords
    let wav1 = make_test_wav_bytes(261.63, 1.0); // C4
    let wav2 = make_test_wav_bytes(329.63, 1.0); // E4
    let wav3 = make_test_wav_bytes(392.00, 1.0); // G4

    let form = Form::new()
        .part("f1", Part::bytes(wav1).file_name("ref_c.wav").mime_str("audio/wav").unwrap())
        .part("f2", Part::bytes(wav2).file_name("ref_e.wav").mime_str("audio/wav").unwrap())
        .part("f3", Part::bytes(wav3).file_name("ref_g.wav").mime_str("audio/wav").unwrap());

    let up_res = client.post(format!("{}/api/reference-wavs/upload", base_url))
        .multipart(form)
        .send().await.expect("POST /api/reference-wavs/upload failed");
    assert_eq!(up_res.status(), 200);

    // 3. Generate track with RL and verify Legal Safeguards
    let gen_res = client.post(format!("{}/api/reference-wavs/generate-rl", base_url))
        .json(&json!({
            "prompt": "Roots reggae anthem with modern brass",
            "genre": "Roots Reggae",
            "lyrics": "One love shining through the night"
        }))
        .send().await.expect("POST /api/reference-wavs/generate-rl failed");
    assert_eq!(gen_res.status(), 200);
    let gen_json: serde_json::Value = gen_res.json().await.unwrap();
    assert_eq!(gen_json["status"], "ok");

    // Verify Legal Compliance Object
    let legal = &gen_json["legal_compliance"];
    assert_eq!(legal["zero_waveform_sampling"], true, "Must verify zero waveform sampling (Bridgeport Music)");
    assert_eq!(legal["no_master_sampling_verified"], true, "Must verify no master sampling");
    assert_eq!(legal["synthetic_voice_only_elvis_compliant"], true, "Must verify synthetic voice only (ELVIS Act)");
    assert_eq!(legal["synthetic_vocal_timbre_only"], true, "Must verify synthetic vocal timbre only");
    assert_eq!(legal["legal_safeguard_verified"], true, "Affirmative legal safeguards must pass");
    let orig_score = legal["originality_score_pct"].as_f64().unwrap();
    assert!(orig_score >= 90.0, "Originality score must be >= 90.0%, got {}", orig_score);

    // Verify Composite Reward breakdown
    let reward = &gen_json["composite_reward"];
    assert!(reward["r_total"].as_f64().is_some());
    assert!(reward["r_style"].as_f64().is_some());
    assert!(reward["r_originality_guard"].as_f64().is_some());

    // 4. Test DPO Preference Learning: Keep feedback (+Reward)
    let keep_res = client.post(format!("{}/api/feedback/preference", base_url))
        .json(&json!({ "preference": "keep" }))
        .send().await.expect("POST /api/feedback/preference keep failed");
    assert_eq!(keep_res.status(), 200);
    let keep_json: serde_json::Value = keep_res.json().await.unwrap();
    assert_eq!(keep_json["status"], "ok");
    assert_eq!(keep_json["preference"], "keep");

    // 5. Test DPO Preference Learning: Revert feedback (-Penalty)
    let revert_res = client.post(format!("{}/api/feedback/preference", base_url))
        .json(&json!({ "preference": "revert" }))
        .send().await.expect("POST /api/feedback/preference revert failed");
    assert_eq!(revert_res.status(), 200);
    let revert_json: serde_json::Value = revert_res.json().await.unwrap();
    assert_eq!(revert_json["status"], "ok");
    assert_eq!(revert_json["preference"], "revert");

    // 6. Test DPO Preference Learning: Invalid choice rejection
    let bad_pref_res = client.post(format!("{}/api/feedback/preference", base_url))
        .json(&json!({ "preference": "invalid_choice" }))
        .send().await.expect("POST /api/feedback/preference invalid failed");
    assert_eq!(bad_pref_res.status(), 400);
}
