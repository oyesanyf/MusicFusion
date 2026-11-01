//! Challenger Tier 5-2 Empirical Adversarial Stress Verification Suite
//!
//! Conducts empirical stress testing across:
//! 1. Disk song artifacts in `storage/songs/`:
//!    - roots_reggae_anthem (v1 and v2)
//!    - song-one-love-shining-1977 (v1 and v2)
//!    - Verification of 48 kHz stereo float WAV validity, master peak amplitude ceiling (<= 0.966),
//!      stem clipping headroom (<= 1.0), duration matching manifest telemetry, and SHA256 non-identity (v1 != v2).
//! 2. Multi-genre pipeline execution across 5 distinct genres:
//!    - Afrobeats (108 BPM)
//!    - Pop (120 BPM)
//!    - Jazz (140 BPM)
//!    - Trap (144 BPM)
//!    - Reggae (76 BPM)
//!    - Verification of deterministic DSP fallback, all 11 stems generated, and master peak ceiling <= 0.966 (-0.30 dBFS).
//! 3. REST server concurrency and RFC 7233 range scrubbing:
//!    - High-throughput concurrency stress on /api/health and /api/hardware.
//!    - Audio stem scrubbing with standard, open-ended, and boundary byte-range queries.
//!    - Boundary conditions: start > end, out-of-range, and suffix range behavior.
//! 4. Adversarial prompt injection & path traversal resilience:
//!    - SQL injection, command injection, path traversal, null bytes, and oversized inputs.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use reqwest::StatusCode;
use serde_json::Value;

use music_cli::rest_server::start_rest_server;
use music_core::state::{
    MusicProductionState, STEM_BASS, STEM_DRAFT_GROOVE, STEM_DRUMS, STEM_FINAL_MASTER,
    STEM_GUITAR, STEM_INPAINTED_DRUMS, STEM_INPAINTED_GUITAR, STEM_OTHER,
    STEM_RASPY_TENOR_VOCALS, STEM_YUE_INSTRUMENTALS, STEM_YUE_VOCALS,
};
use music_dsp::wav::AudioBuffer;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

/// Strict brickwall ceiling tolerance: -0.30 dBFS corresponds to ~0.9660508
const PEAK_CEILING_TOLERANCE: f32 = 0.96605;

// =========================================================================
// Pure-Rust FIPS 180-4 SHA-256 Implementation (Self-Contained)
// =========================================================================

fn sha256_bytes(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, part) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes(part.try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_var = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_var
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_var = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_var);
    }

    let mut out = String::with_capacity(64);
    for val in h {
        out.push_str(&format!("{:08x}", val));
    }
    out
}

#[test]
fn test_sha256_known_test_vectors() {
    assert_eq!(
        sha256_bytes(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_bytes(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

// =========================================================================
// SECTION 1: Disk Song Artifacts Empirical Inspection
// =========================================================================

#[test]
fn test_disk_song_artifacts_roots_reggae_and_one_love() {
    let base_dir = if Path::new("storage/songs").exists() {
        PathBuf::from("storage/songs")
    } else if Path::new("../../storage/songs").exists() {
        PathBuf::from("../../storage/songs")
    } else {
        PathBuf::from("d:/harfile/music/storage/songs")
    };
    assert!(base_dir.exists(), "storage/songs directory must exist at {}", base_dir.display());

    let targets = vec![
        ("roots_reggae_anthem", 75.78, 73.84),
        ("song-one-love-shining-1977", 75.78, 73.84),
    ];

    let stem_names = vec![
        "bass.wav",
        "draft_groove.wav",
        "drums.wav",
        "guitar.wav",
        "inpainted_drums.wav",
        "inpainted_guitar.wav",
        "other.wav",
        "raspy_tenor_vocals.wav",
        "yue_instrumentals.wav",
        "yue_vocals.wav",
    ];

    for (song_id, expected_dur_v1, expected_dur_v2) in targets {
        let song_dir = base_dir.join(song_id);
        assert!(song_dir.exists(), "Song dir {} must exist", song_dir.display());

        let mut v1_hashes: HashMap<String, String> = HashMap::new();
        let mut v2_hashes: HashMap<String, String> = HashMap::new();

        // 1. Inspect v1
        let v1_dir = song_dir.join("v1");
        assert!(v1_dir.exists(), "v1 directory must exist for {}", song_id);

        let v1_manifest_path = v1_dir.join("manifest.json");
        assert!(v1_manifest_path.exists(), "v1 manifest.json must exist");
        let v1_manifest_content = fs::read_to_string(&v1_manifest_path).unwrap();
        let v1_manifest: Value = serde_json::from_str(&v1_manifest_content).unwrap();

        assert_eq!(v1_manifest["song_id"].as_str().unwrap(), song_id);
        assert_eq!(v1_manifest["version"].as_u64().unwrap(), 1);
        assert_eq!(v1_manifest["telemetry"]["sample_rate"].as_u64().unwrap(), 48000);
        assert_eq!(v1_manifest["telemetry"]["channels"].as_u64().unwrap(), 2);
        assert_eq!(v1_manifest["telemetry"]["clipping"].as_bool().unwrap(), false);

        let v1_master_path = v1_dir.join("final_master.wav");
        assert!(v1_master_path.exists(), "v1 final_master.wav must exist");
        let v1_master_bytes = fs::read(&v1_master_path).unwrap();
        let v1_master_hash = sha256_bytes(&v1_master_bytes);
        v1_hashes.insert("final_master.wav".to_string(), v1_master_hash);

        let v1_master_buf = AudioBuffer::from_wav_bytes(&v1_master_bytes).expect("Valid WAV");
        assert_eq!(v1_master_buf.sample_rate, 48000);
        assert_eq!(v1_master_buf.channels, 2);
        assert!((v1_master_buf.duration_secs() - expected_dur_v1).abs() < 1.0);

        let v1_master_peak = v1_master_buf
            .samples
            .iter()
            .fold(0.0f32, |acc, &s| acc.max(s.abs()));
        assert!(
            v1_master_peak <= PEAK_CEILING_TOLERANCE,
            "v1 master peak {} exceeds brickwall ceiling {}",
            v1_master_peak,
            PEAK_CEILING_TOLERANCE
        );

        for stem in &stem_names {
            let stem_path = v1_dir.join("stems").join(stem);
            assert!(stem_path.exists(), "Stem {} must exist in v1", stem_path.display());
            let bytes = fs::read(&stem_path).unwrap();
            let hash = sha256_bytes(&bytes);
            v1_hashes.insert(stem.to_string(), hash);

            let buf = AudioBuffer::from_wav_bytes(&bytes).expect("Valid stem WAV");
            assert_eq!(buf.sample_rate, 48000);
            assert_eq!(buf.channels, 2);
            assert!((buf.duration_secs() - expected_dur_v1).abs() < 1.0);
            let peak = buf.samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
            // Stems before mastering must have zero digital clipping (<= 1.0)
            assert!(
                peak <= 1.0,
                "Stem {} peak {} has digital clipping (> 1.0)",
                stem,
                peak
            );
        }

        // 2. Inspect v2
        let v2_dir = song_dir.join("v2");
        assert!(v2_dir.exists(), "v2 directory must exist for {}", song_id);

        let v2_manifest_path = v2_dir.join("manifest.json");
        assert!(v2_manifest_path.exists(), "v2 manifest.json must exist");
        let v2_manifest_content = fs::read_to_string(&v2_manifest_path).unwrap();
        let v2_manifest: Value = serde_json::from_str(&v2_manifest_content).unwrap();

        assert_eq!(v2_manifest["song_id"].as_str().unwrap(), song_id);
        assert_eq!(v2_manifest["version"].as_u64().unwrap(), 2);
        assert_eq!(v2_manifest["telemetry"]["sample_rate"].as_u64().unwrap(), 48000);
        assert_eq!(v2_manifest["telemetry"]["channels"].as_u64().unwrap(), 2);
        assert_eq!(v2_manifest["telemetry"]["clipping"].as_bool().unwrap(), false);

        let v2_master_path = v2_dir.join("final_master.wav");
        assert!(v2_master_path.exists(), "v2 final_master.wav must exist");
        let v2_master_bytes = fs::read(&v2_master_path).unwrap();
        let v2_master_hash = sha256_bytes(&v2_master_bytes);
        v2_hashes.insert("final_master.wav".to_string(), v2_master_hash);

        let v2_master_buf = AudioBuffer::from_wav_bytes(&v2_master_bytes).expect("Valid WAV");
        assert_eq!(v2_master_buf.sample_rate, 48000);
        assert_eq!(v2_master_buf.channels, 2);
        assert!((v2_master_buf.duration_secs() - expected_dur_v2).abs() < 1.0);

        let v2_master_peak = v2_master_buf
            .samples
            .iter()
            .fold(0.0f32, |acc, &s| acc.max(s.abs()));
        assert!(
            v2_master_peak <= PEAK_CEILING_TOLERANCE,
            "v2 master peak {} exceeds brickwall ceiling {}",
            v2_master_peak,
            PEAK_CEILING_TOLERANCE
        );

        for stem in &stem_names {
            let stem_path = v2_dir.join("stems").join(stem);
            assert!(stem_path.exists(), "Stem {} must exist in v2", stem_path.display());
            let bytes = fs::read(&stem_path).unwrap();
            let hash = sha256_bytes(&bytes);
            v2_hashes.insert(stem.to_string(), hash);

            let buf = AudioBuffer::from_wav_bytes(&bytes).expect("Valid stem WAV");
            assert_eq!(buf.sample_rate, 48000);
            assert_eq!(buf.channels, 2);
            assert!((buf.duration_secs() - expected_dur_v2).abs() < 1.0);
            let peak = buf.samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
            assert!(
                peak <= 1.0,
                "Stem {} peak {} has digital clipping (> 1.0)",
                stem,
                peak
            );
        }

        // 3. Verify SHA256 Non-Identity between v1 and v2
        assert_eq!(v1_hashes.len(), 11, "Must have 11 files in v1");
        assert_eq!(v2_hashes.len(), 11, "Must have 11 files in v2");

        for (name, h1) in &v1_hashes {
            let h2 = v2_hashes.get(name).expect("Matching file in v2");
            assert_ne!(
                h1, h2,
                "File '{}' in {} has identical SHA256 between v1 and v2! Hashes: {}",
                name, song_id, h1
            );
        }
    }
}

// =========================================================================
// SECTION 2: Multi-Genre Pipeline Execution Across 5 Genres
// =========================================================================

#[tokio::test]
async fn test_multi_genre_pipeline_execution_5_genres_ceiling() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::new()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(1.0);

    let runner = PipelineRunner::new(config).expect("Runner init");

    let genre_matrix = vec![
        ("adv-afrobeats", "Afrobeats", 108, "F# Minor", "Modern Lagos club hit, syncopated log-drum, shakers, smooth tenor"),
        ("adv-pop", "Pop", 120, "C Major", "Driving four-on-the-floor kick, bright acoustic guitars, soaring female soprano hook"),
        ("adv-jazz", "Jazz", 140, "Bb Major", "Swung ride cymbal, walking upright acoustic bass, piano/saxophone lead, audiophile master"),
        ("adv-trap", "Trap", 144, "G Minor", "Hard snare on 3, rolling 32nd hi-hats, pitch-swept 808 sub-bass, brickwall master"),
        ("adv-reggae", "Roots Reggae", 76, "A Major", "Hypnotic one drop master drumming, deep sub bass, soulful raspy tenor vocal formant"),
    ];

    for (song_id, genre, bpm, key, prompt) in genre_matrix {
        let mut state = MusicProductionState::new(song_id, genre, bpm, key);
        state.style_prompt = prompt.to_string();
        state.engine_mode = "pure_rust".to_string();

        let final_state = runner.execute_song(state).await.expect("execute_song failed");

        assert_eq!(final_state.song_id, song_id);
        assert_eq!(final_state.version, 1);

        // Verify master audio file exists and check peak ceiling
        let master_path = final_state.stem_paths.get(STEM_FINAL_MASTER).expect("master path");
        assert!(master_path.exists(), "Master path {} does not exist", master_path.display());

        let master_buf = AudioBuffer::from_wav_file(master_path).expect("Read master WAV");
        assert_eq!(master_buf.sample_rate, 48000, "Must be 48 kHz for {}", genre);
        assert_eq!(master_buf.channels, 2, "Must be stereo for {}", genre);
        assert!(master_buf.samples.len() > 0, "Samples cannot be empty for {}", genre);

        let max_peak = master_buf.samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
        assert!(
            max_peak <= PEAK_CEILING_TOLERANCE,
            "Genre '{}' master peak {} exceeded brickwall ceiling {}",
            genre,
            max_peak,
            PEAK_CEILING_TOLERANCE
        );

        // Verify all 11 required stems exist in state.stem_paths
        let expected_stems = [
            STEM_DRAFT_GROOVE,
            STEM_YUE_VOCALS,
            STEM_YUE_INSTRUMENTALS,
            STEM_RASPY_TENOR_VOCALS,
            STEM_DRUMS,
            STEM_BASS,
            STEM_GUITAR,
            STEM_OTHER,
            STEM_INPAINTED_GUITAR,
            STEM_INPAINTED_DRUMS,
            STEM_FINAL_MASTER,
        ];

        for &stem_key in &expected_stems {
            let path = final_state.stem_paths.get(stem_key).expect("stem key present");
            assert!(path.exists(), "Stem '{}' file {} does not exist for {}", stem_key, path.display(), genre);

            let stem_buf = AudioBuffer::from_wav_file(path).expect("Read stem WAV");
            assert_eq!(stem_buf.sample_rate, 48000);
            assert_eq!(stem_buf.channels, 2);
            let stem_peak = stem_buf.samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
            assert!(
                stem_peak <= 1.0,
                "Genre '{}' stem '{}' peak {} has digital clipping (> 1.0)",
                genre,
                stem_key,
                stem_peak
            );
        }
    }
}

// =========================================================================
// SECTION 3: REST Server Concurrency & RFC 7233 Range Scrubbing Stress
// =========================================================================

#[tokio::test]
async fn test_rest_server_concurrency_and_rfc7233_range_scrubbing() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let runner_config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.2);
    let runner = Arc::new(PipelineRunner::new(runner_config).unwrap());

    // Generate a mini test song so audio files exist for range streaming
    let mut initial_state = MusicProductionState::new("range-test-song", "Pop", 120, "C Major");
    initial_state.style_prompt = "Short pop track for byte range testing".to_string();
    initial_state.engine_mode = "pure_rust".to_string();
    runner.execute_song(initial_state).await.expect("execute mini song");

    let bound_port = start_rest_server(
        runner,
        temp_dir.path().to_path_buf(),
        5975,
        5995,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("Start rest server");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);

    // 1. High-throughput concurrency stress: 80 requests across /api/health and /api/hardware
    let mut tasks = Vec::new();
    for i in 0..80 {
        let client_c = client.clone();
        let url = if i % 2 == 0 {
            format!("{}/api/health", base_url)
        } else {
            format!("{}/api/hardware", base_url)
        };
        tasks.push(tokio::spawn(async move {
            let res = client_c.get(&url).send().await;
            res.map(|r| r.status().as_u16())
        }));
    }

    let results = join_all(tasks).await;
    let mut ok_count = 0;
    for res in results {
        let status = res.expect("Task join failed").expect("Request failed");
        assert_eq!(status, 200, "High concurrency health/hardware request failed");
        ok_count += 1;
    }
    assert_eq!(ok_count, 80, "All 80 concurrent health/hardware requests succeeded");

    // 2. RFC 7233 Byte-Range Scrubbing on Audio Endpoint
    let audio_url = format!("{}/api/songs/range-test-song/versions/1/audio/final_master", base_url);

    // Full file baseline
    let full_res = client.get(&audio_url).send().await.expect("full audio request");
    assert_eq!(full_res.status(), StatusCode::OK);
    let full_bytes = full_res.bytes().await.expect("read full bytes");
    let total_len = full_bytes.len() as u64;
    assert!(total_len > 44, "Total audio length must exceed WAV header size");

    // Test a) Standard range: bytes=0-1023
    let r1 = client
        .get(&audio_url)
        .header("Range", "bytes=0-1023")
        .send()
        .await
        .expect("range 0-1023");
    assert_eq!(r1.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        r1.headers().get("Content-Range").unwrap().to_str().unwrap(),
        format!("bytes 0-1023/{}", total_len)
    );
    let r1_bytes = r1.bytes().await.unwrap();
    assert_eq!(r1_bytes.len(), 1024);
    assert_eq!(&r1_bytes[..], &full_bytes[0..1024]);

    // Test b) Sub-range in middle: bytes=500-1500
    let r2 = client
        .get(&audio_url)
        .header("Range", "bytes=500-1500")
        .send()
        .await
        .expect("range 500-1500");
    assert_eq!(r2.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        r2.headers().get("Content-Range").unwrap().to_str().unwrap(),
        format!("bytes 500-1500/{}", total_len)
    );
    let r2_bytes = r2.bytes().await.unwrap();
    assert_eq!(r2_bytes.len(), 1001);
    assert_eq!(&r2_bytes[..], &full_bytes[500..=1500]);

    // Test c) Open-ended range: bytes=2000-
    let r3 = client
        .get(&audio_url)
        .header("Range", "bytes=2000-")
        .send()
        .await
        .expect("range 2000-");
    assert_eq!(r3.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        r3.headers().get("Content-Range").unwrap().to_str().unwrap(),
        format!("bytes 2000-{}/{}", total_len - 1, total_len)
    );
    let r3_bytes = r3.bytes().await.unwrap();
    assert_eq!(r3_bytes.len() as u64, total_len - 2000);
    assert_eq!(&r3_bytes[..], &full_bytes[2000..]);

    // Test d) Unsupported Suffix range: bytes=-500 (falls back safely to 200 OK full audio)
    let r4 = client
        .get(&audio_url)
        .header("Range", "bytes=-500")
        .send()
        .await
        .expect("suffix range -500");
    // Suffix range header without start index is ignored by parse_range_header, returning 200 OK
    assert_eq!(r4.status(), StatusCode::OK);
    assert_eq!(r4.headers().get("Content-Type").unwrap(), "audio/wav");

    // Test e) Boundary condition: range exceeding file size -> clamps or falls back safely
    let r5 = client
        .get(&audio_url)
        .header("Range", format!("bytes={}-{}", total_len + 1000, total_len + 2000))
        .send()
        .await
        .expect("overflow range");
    assert!(
        r5.status() == StatusCode::OK || r5.status() == StatusCode::RANGE_NOT_SATISFIABLE,
        "Overflow range must be handled safely"
    );

    // Test f) Boundary condition: inverted start > end -> graceful 200 fallback
    let r6 = client
        .get(&audio_url)
        .header("Range", "bytes=5000-1000")
        .send()
        .await
        .expect("inverted range");
    assert_eq!(r6.status(), StatusCode::OK);
}

// =========================================================================
// SECTION 4: Adversarial Prompt Injection & Traversal Safety
// =========================================================================

#[tokio::test]
async fn test_rest_server_adversarial_prompt_injection_safety() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
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
    .expect("Start rest server");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);

    let injection_payloads = vec![
        // SQL Injection attempts
        serde_json::json!({
            "genre": "Roots Reggae'; DROP TABLE songs; --",
            "prompt": "1977 roots reggae' OR '1'='1",
            "bpm": 76,
            "key": "A Major",
            "engine_mode": "pure_rust"
        }),
        // Command / Shell Injection attempts
        serde_json::json!({
            "genre": "Rock",
            "prompt": "; cat /etc/passwd | calc.exe && echo hacked",
            "bpm": 120,
            "key": "E Minor",
            "engine_mode": "pure_rust"
        }),
        // Path Traversal in Style / Genre
        serde_json::json!({
            "genre": "../../../../windows/system32",
            "prompt": "..\\..\\..\\boot.ini",
            "bpm": 90,
            "key": "C Major",
            "engine_mode": "pure_rust"
        }),
        // Extreme Unicode / Emojis & Null Byte
        serde_json::json!({
            "genre": "Afrobeats 🌍🔥⚡",
            "prompt": "Syncopated beat \0 null byte \u{FFFF} with unicode emojis 🎉",
            "bpm": 108,
            "key": "F# Minor",
            "engine_mode": "pure_rust"
        }),
    ];

    for (i, payload) in injection_payloads.iter().enumerate() {
        let res = client
            .post(format!("{}/api/songs/create", base_url))
            .json(payload)
            .send()
            .await
            .expect("Send injection payload");

        // The endpoint should either succeed gracefully (safely sanitizing input)
        // or return 400 Bad Request, but MUST NEVER return 500 Panic or drop connection.
        assert!(
            res.status() == StatusCode::OK || res.status() == StatusCode::BAD_REQUEST,
            "Payload #{} resulted in unexpected status: {}",
            i,
            res.status()
        );
    }

    // Direct path traversal attack against audio streaming endpoint
    let traversal_urls = vec![
        format!("{}/api/songs/..%2f..%2f..%2fwindows%2fsystem32/versions/1/audio/final_master", base_url),
        format!("{}/api/songs/etc/versions/1/audio/..%2f..%2fpasswd", base_url),
        format!("{}/api/songs/CON/versions/1/audio/final_master", base_url),
        format!("{}/api/songs/AUX/versions/1/audio/drums", base_url),
    ];

    for url in traversal_urls {
        let res = client.get(&url).send().await.expect("Traversal probe");
        assert!(
            res.status() == StatusCode::BAD_REQUEST || res.status() == StatusCode::NOT_FOUND,
            "Path traversal attack to '{}' must be rejected (got {})",
            url,
            res.status()
        );
    }
}
