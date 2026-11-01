//! Tier 5 Comprehensive Adversarial Coverage & Hardening Suite
//!
//! Evaluates the 5 critical challenge dimensions specified in DISPATCH.md:
//! 1. Floating-point overflow / underflow / NaN in DSP routines.
//! 2. Concurrency race conditions in REST server and StateGraph execution.
//! 3. Extreme inputs (0 BPM, >240 BPM, empty prompts, unsupported genres, malformed JSON).
//! 4. Windows path traversal and DOS device name edge cases.
//! 5. Brickwall limiter ceiling enforcement (<= 0.966 / -0.30 dBFS) across all genre modes.

use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use reqwest::StatusCode;

use music_core::genre::GenreTaxonomy;
use music_core::state::{
    MusicProductionState, STEM_FINAL_MASTER, STEM_INPAINTED_GUITAR,
};
use music_dsp::groove_synth::UniversalDrumSynthesizer;
use music_dsp::guitar_wah::SoloGenerator;
use music_dsp::tape_mastering::{
    apply_analog_tape_mastering, BRICKWALL_CEILING,
};
use music_dsp::telemetry_calc::calculate_telemetry;
use music_dsp::vocal_formant::apply_vocal_timbre;
use music_dsp::wav::AudioBuffer;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};
use music_cli::rest_server::start_rest_server;

const CEILING_LIMIT: f32 = 0.96605;

// =========================================================================
// 1. DSP Floating-Point Overflow, Underflow, NaN, and Boundary Resilience
// =========================================================================

#[test]
fn test_tier5_dsp_telemetry_extreme_boundaries_and_zero_bpm() {
    // Test 1: Zero BPM must not cause division-by-zero or panic
    let buf_normal = AudioBuffer::new_stereo(0.5);
    let telem_zero_bpm = calculate_telemetry(&buf_normal, 0);
    assert!(!telem_zero_bpm.tempo_drift_score.is_nan());
    assert!(telem_zero_bpm.tempo_drift_score.is_finite());

    // Test 2: Extreme High BPM (> 1000)
    let telem_high_bpm = calculate_telemetry(&buf_normal, 1200);
    assert!(telem_high_bpm.tempo_drift_score.is_finite());

    // Test 3: AudioBuffer with extreme +48 dB overdrive signal
    let mut hot_buf = AudioBuffer::new_stereo(0.2);
    for s in &mut hot_buf.samples {
        *s = 251.18; // +48 dBFS
    }
    let telem_hot = calculate_telemetry(&hot_buf, 120);
    assert!(telem_hot.clipping, "Extreme overdrive must register as clipping");
    assert!(telem_hot.peak_dbfs > 40.0);
    assert!(telem_hot.rms_dbfs.is_finite());
    assert!(!telem_hot.rms_dbfs.is_nan());

    // Test 4: Subnormal / denormal audio floats
    let mut subnormal_buf = AudioBuffer::new_stereo(0.1);
    for s in &mut subnormal_buf.samples {
        *s = f32::MIN_POSITIVE / 4.0;
    }
    let telem_subnormal = calculate_telemetry(&subnormal_buf, 76);
    assert!(!telem_subnormal.peak_dbfs.is_nan());
    assert!(!telem_subnormal.rms_dbfs.is_nan());
    assert!(telem_subnormal.dynamic_range_db >= 3.0);
}

#[test]
fn test_tier5_dsp_tape_mastering_extreme_sub_bass_boost_resilience() {
    // Test extreme sub_bass_boost values (+100.0 dB, -50.0 dB, 0.0 dB)
    let mut buf = AudioBuffer::new_stereo(0.1);
    for i in 0..buf.samples.len() {
        buf.samples[i] = ((i as f32 * 0.05).sin()) * 2.0; // overdriven input
    }

    let boosts = [-50.0, 0.0, 3.0, 6.0, 100.0];
    for boost in boosts {
        let mastered = apply_analog_tape_mastering(&buf, boost);
        for s in &mastered.samples {
            assert!(!s.is_nan(), "Tape mastering produced NaN with boost {}", boost);
            assert!(!s.is_infinite(), "Tape mastering produced Inf with boost {}", boost);
            assert!(
                s.abs() <= CEILING_LIMIT,
                "Sample {:.6} exceeded ceiling with boost {}",
                s.abs(),
                boost
            );
        }
    }
}

#[test]
fn test_tier5_dsp_vocal_and_guitar_extreme_parameter_clamping() {
    // Vocal Timbre with extreme rasp (-10.0, 0.0, 1.0, 50.0)
    let mut vocal_buf = AudioBuffer::new_stereo(0.1);
    for i in 0..vocal_buf.samples.len() {
        vocal_buf.samples[i] = (i as f32 * 0.1).sin() * 1.5;
    }

    for rasp in [-10.0, 0.0, 0.5, 1.0, 50.0] {
        let processed = apply_vocal_timbre(&vocal_buf, rasp, 0.0);
        for s in &processed.samples {
            assert!(!s.is_nan(), "Vocal timbre produced NaN for rasp {}", rasp);
            assert!(!s.is_infinite(), "Vocal timbre produced Inf for rasp {}", rasp);
            assert!(s.abs() <= 1.0, "Vocal sample exceeded unity for rasp {}", rasp);
        }
    }

    // Guitar Solo with 0 bars, 0 BPM, 9999 BPM
    let solo_zero_bars = SoloGenerator::synthesize_lead_solo(0, 0, "A Major");
    assert_eq!(solo_zero_bars.frame_count(), 0);

    let solo_extreme_bpm = SoloGenerator::synthesize_lead_solo(1, 9999, "Blues");
    assert!(solo_extreme_bpm.frame_count() > 0);
    for s in &solo_extreme_bpm.samples {
        assert!(!s.is_nan());
        assert!(!s.is_infinite());
        assert!(s.abs() <= 1.0);
    }

    // Groove synth with 0 BPM, 0 duration, extreme BPM
    let groove_zero_dur = UniversalDrumSynthesizer::generate_groove("Roots Reggae", 0, 0.0);
    assert_eq!(groove_zero_dur.frame_count(), 0);

    let groove_high_bpm = UniversalDrumSynthesizer::generate_groove("Afrobeats", 9999, 0.5);
    assert!(groove_high_bpm.frame_count() > 0);
    for s in &groove_high_bpm.samples {
        assert!(!s.is_nan());
        assert!(!s.is_infinite());
        assert!(s.abs() <= 1.0);
    }
}

// =========================================================================
// 2. Extreme Inputs: Empty Prompts, Unsupported Genres, Malformed JSON
// =========================================================================

#[test]
fn test_tier5_extreme_inputs_unsupported_genres_and_empty_prompts() {
    // Completely arbitrary, hostile, or esoteric genre strings
    let esoteric_genres = [
        "",
        "   ",
        "12345!@#$%^&*()_+",
        "DROP TABLE songs;--",
        "<script>alert('xss')</script>",
        "Yoruba Bata Drum Trap",
        "Cyberpunk Tuvan Throat Metal",
        "Ancient Sumerian Lyre Ambient",
        "日本語のシティポップ",
        "موسيقى شرقية معاصرة",
    ];

    for genre in esoteric_genres {
        let preset = GenreTaxonomy::find_or_create(genre);
        assert!(!preset.name.is_empty());
        assert!(preset.default_bpm >= 40 && preset.default_bpm <= 240);
        assert!(!preset.default_key.is_empty());

        let groove = UniversalDrumSynthesizer::generate_groove(genre, 100, 0.2);
        assert!(groove.frame_count() > 0);
        assert_eq!(groove.sample_rate, 48_000);
    }
}

#[test]
fn test_tier5_state_serde_malformed_json_rejection() {
    let malformed_payloads = [
        "",
        "{",
        "{\"song_id\": 123}",
        "{\"bpm\": \"not_a_number\"}",
        "[1, 2, 3]",
        "null",
        "{\"song_id\": \"test\", \"version\": -1}",
    ];

    for payload in malformed_payloads {
        let res = MusicProductionState::from_json(payload);
        assert!(
            res.is_err(),
            "Malformed JSON payload '{}' should have failed deserialization",
            payload
        );
    }
}

// =========================================================================
// 3. Concurrency: Multi-Song Parallel StateGraph Execution
// =========================================================================

#[tokio::test]
async fn test_tier5_concurrency_multi_song_parallel_pipeline_execution() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::new()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.3); // Short duration for fast execution

    let runner = Arc::new(PipelineRunner::new(config).expect("Runner initialization"));

    // Spawn 8 parallel songs concurrently across distinct genres and tempos
    let test_genres = [
        ("Roots Reggae", 76, "A Major"),
        ("Afrobeats", 108, "F# Minor"),
        ("Pop", 120, "C Major"),
        ("Rock", 140, "E Minor"),
        ("Jazz", 160, "Bb Major"),
        ("Amapiano", 112, "C Minor"),
        ("Cinematic Orchestral", 80, "D Minor"),
        ("Trap", 140, "F Minor"),
    ];

    let mut handles = Vec::new();
    for (i, (genre, bpm, key)) in test_genres.iter().enumerate() {
        let runner_clone = runner.clone();
        let song_id = format!("parallel_song_{}", i);
        let g = genre.to_string();
        let k = key.to_string();
        let b = *bpm;

        handles.push(tokio::spawn(async move {
            let mut state = MusicProductionState::new(&song_id, &g, b, &k);
            state.engine_mode = "pure_rust".to_string();
            state.style_prompt = format!("{} test anthem at {} BPM", g, b);
            runner_clone.execute_song(state).await
        }));
    }

    let results = join_all(handles).await;
    assert_eq!(results.len(), 8);

    for (i, res) in results.into_iter().enumerate() {
        let song_state = res
            .expect("Tokio task join failed")
            .unwrap_or_else(|e| panic!("Song {} pipeline failed: {}", i, e));

        assert_eq!(song_state.version, 1);
        assert!(song_state.has_stem(STEM_FINAL_MASTER));

        let master_p = song_state.get_stem(STEM_FINAL_MASTER).unwrap();
        assert!(master_p.exists(), "Master WAV must exist on disk");

        let buf = AudioBuffer::from_wav_file(master_p).expect("Read master");
        assert_eq!(buf.sample_rate, 48_000);
        assert_eq!(buf.channels, 2);

        for s in &buf.samples {
            assert!(
                s.abs() <= CEILING_LIMIT,
                "Song {} master clipped above ceiling: {}",
                i,
                s.abs()
            );
        }
    }

    // Verify all 8 songs are recorded in SQLite
    for i in 0..8 {
        let song_id = format!("parallel_song_{}", i);
        let versions = runner.checkpointer().list_versions(&song_id).unwrap();
        assert_eq!(versions, vec![1]);
    }
}

// =========================================================================
// 4. REST Concurrency & Path Traversal Security Verification
// =========================================================================

#[tokio::test]
async fn test_tier5_rest_concurrency_and_path_security_stress() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::default()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.1);
    let runner = Arc::new(PipelineRunner::new(config).expect("Runner init"));

    let base_port = 6100;
    let end_port = 6120;
    let bound_port = start_rest_server(
        runner.clone(),
        temp_dir.path().to_path_buf(),
        base_port,
        end_port,
        "http://127.0.0.1:11434".to_string(),
    )
    .await
    .expect("Failed to start rest server");

    tokio::time::sleep(Duration::from_millis(150)).await;

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", bound_port);

    // 1. High-concurrency probe: 40 simultaneous requests
    let mut tasks = Vec::new();
    for i in 0..40 {
        let c = client.clone();
        let url = match i % 4 {
            0 => format!("{}/api/health", base_url),
            1 => format!("{}/api/hardware", base_url),
            2 => format!("{}/", base_url),
            _ => format!("{}/styles.css", base_url),
        };
        tasks.push(tokio::spawn(async move {
            let res = c.get(&url).send().await;
            res.map(|r| r.status().as_u16())
        }));
    }

    let results = join_all(tasks).await;
    for res in results {
        let status = res.expect("Task join failed").expect("HTTP request failed");
        assert_eq!(status, 200, "All concurrent requests must return HTTP 200");
    }

    // 2. Adversarial Path Traversal & DOS device names on REST audio streaming
    let hostile_stems = [
        "../../../../Windows/win.ini",
        "..%2f..%2f..%2fWindows",
        "CON",
        "con.wav",
        "PRN",
        "AUX",
        "aux.txt",
        "NUL",
        "nul.bin",
        "COM1",
        "LPT1",
        "bad_stem.",
        "trailing_space ",
        "C:",
    ];

    for stem in hostile_stems {
        let probe_url = format!("{}/api/songs/test_song/versions/1/audio/{}", base_url, stem);
        let res = client.get(&probe_url).send().await;
        if let Ok(resp) = res {
            let status = resp.status();
            assert!(
                status == StatusCode::BAD_REQUEST
                    || status == StatusCode::NOT_FOUND
                    || status == StatusCode::FORBIDDEN,
                "Hostile stem '{}' must be rejected with 400/403/404, got: {}",
                stem,
                status
            );
        }
    }
}

// =========================================================================
// 5. Full End-to-End Multi-Iteration (v1 -> v2 -> v3) Lineage Verification
// =========================================================================

#[tokio::test]
async fn test_tier5_full_e2e_multi_version_iteration_lifecycle() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = PipelineConfig::new()
        .with_storage_dir(temp_dir.path())
        .with_pure_rust_mode(true)
        .with_max_duration(0.5);

    let runner = PipelineRunner::new(config).expect("Runner initialization");
    let song_id = "tier5_iter_lineage";

    // 1. Initial song creation -> v1
    let mut state_v1 = MusicProductionState::new(song_id, "Roots Reggae", 76, "A Major");
    state_v1.style_prompt = "1977 roots reggae anthem v1".to_string();
    state_v1.engine_mode = "pure_rust".to_string();

    let v1 = runner.execute_song(state_v1).await.expect("execute v1");
    assert_eq!(v1.version, 1);
    assert!(v1.has_stem(STEM_FINAL_MASTER));

    // 2. Iteration to v2: adjust lyrics, sub-bass, wah
    let v2 = runner
        .iterate_song(song_id, 1, Some("Boost sub-bass by 3dB and increase guitar wah"))
        .await
        .expect("iterate v2");
    assert_eq!(v2.version, 2);
    assert!(v2.has_stem(STEM_FINAL_MASTER));
    assert!(v2.style_prompt.contains("Boost sub-bass"));

    // 3. Targeted stem regeneration to v3 (guitar only)
    let v3 = runner
        .regenerate_stem(song_id, 2, "guitar")
        .await
        .expect("regenerate guitar -> v3");
    assert_eq!(v3.version, 3);
    assert!(v3.has_stem(STEM_INPAINTED_GUITAR));
    assert!(v3.has_stem(STEM_FINAL_MASTER));

    // 4. Validate lineage across all 3 versions
    let version_history = runner.checkpointer().list_versions(song_id).expect("list_versions");
    assert_eq!(version_history, vec![1, 2, 3]);

    for v in [1, 2, 3] {
        let loaded = runner.checkpointer().load_checkpoint(song_id, v).await.expect("load checkpoint");
        assert_eq!(loaded.version, v);
        assert!(loaded.has_stem(STEM_FINAL_MASTER));

        let master_path = loaded.get_stem(STEM_FINAL_MASTER).unwrap();
        assert!(master_path.exists());

        let master_buf = AudioBuffer::from_wav_file(master_path).expect("read master");
        assert_eq!(master_buf.sample_rate, 48_000);
        assert_eq!(master_buf.channels, 2);

        for s in &master_buf.samples {
            assert!(
                s.abs() <= CEILING_LIMIT,
                "v{} master peak {:.6} exceeded ceiling {:.6}",
                v,
                s.abs(),
                BRICKWALL_CEILING
            );
        }
    }
}
