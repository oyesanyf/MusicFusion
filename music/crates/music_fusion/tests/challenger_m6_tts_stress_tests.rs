//! crates/music_fusion/tests/challenger_m6_tts_stress_tests.rs
//!
//! Milestone 6 Adversarial Challenge Suite 2:
//! - ModelFusion TTS Vocal Engine (`tts_engine.rs`) stress testing
//! - All model enum variants & lenient string parsing
//! - Pure-Rust Formant Singing Engine under adversarial inputs (empty lyrics, huge text, emojis)
//! - Zero panics, zero NaNs, zero infinities, bounded peaks
//! - Windows SAPI and pure-Rust zero-crash fallbacks
//! - WAV parser/writer error resilience

use std::fs;
use std::path::PathBuf;
use music_fusion::tts_engine::{
    FusionTtsEngine, FusionTtsModel, TtsVocalRequest, TtsVocalResponse,
};

fn get_temp_test_dir(test_name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("musicfusion_tts_stress_{}_{}", test_name, uuid::Uuid::new_v4()));
    fs::create_dir_all(&p).expect("Failed to create temp test dir");
    p
}

// ============================================================================
// Group 1: Model Enum Variants, Lenient Parsing & Display Names
// ============================================================================

#[test]
fn test_tts_model_enum_from_str_lenient_all_variants_and_fallbacks() {
    // 1. F5-TTS
    assert_eq!(FusionTtsModel::from_str_lenient("f5"), FusionTtsModel::F5Tts);
    assert_eq!(FusionTtsModel::from_str_lenient("F5-TTS"), FusionTtsModel::F5Tts);
    assert_eq!(FusionTtsModel::from_str_lenient("  f5_tts_v2  "), FusionTtsModel::F5Tts);

    // 2. Kokoro-82M
    assert_eq!(FusionTtsModel::from_str_lenient("kokoro"), FusionTtsModel::Kokoro82m);
    assert_eq!(FusionTtsModel::from_str_lenient("KOKORO-82M"), FusionTtsModel::Kokoro82m);

    // 3. Suno Bark
    assert_eq!(FusionTtsModel::from_str_lenient("bark"), FusionTtsModel::SunoBark);
    assert_eq!(FusionTtsModel::from_str_lenient("Suno Bark Singing"), FusionTtsModel::SunoBark);

    // 4. Coqui XTTS-v2
    assert_eq!(FusionTtsModel::from_str_lenient("xtts"), FusionTtsModel::CoquiXttsV2);
    assert_eq!(FusionTtsModel::from_str_lenient("coqui/xtts-v2"), FusionTtsModel::CoquiXttsV2);

    // 5. Fish-Speech-1.5
    assert_eq!(FusionTtsModel::from_str_lenient("fish"), FusionTtsModel::FishSpeech15);
    assert_eq!(FusionTtsModel::from_str_lenient("fish-speech-1.5"), FusionTtsModel::FishSpeech15);

    // 6. Parler-TTS Large
    assert_eq!(FusionTtsModel::from_str_lenient("parler"), FusionTtsModel::ParlerTtsLarge);
    assert_eq!(FusionTtsModel::from_str_lenient("parler-tts-large"), FusionTtsModel::ParlerTtsLarge);

    // 7. Plachtaa Seed-VC
    assert_eq!(FusionTtsModel::from_str_lenient("seed"), FusionTtsModel::SeedVc);
    assert_eq!(FusionTtsModel::from_str_lenient("plachtaa/seed-vc"), FusionTtsModel::SeedVc);

    // 8. Fallback / Windows SAPI
    assert_eq!(FusionTtsModel::from_str_lenient("sapi"), FusionTtsModel::WindowsSapi);
    assert_eq!(FusionTtsModel::from_str_lenient("windows"), FusionTtsModel::WindowsSapi);
    assert_eq!(FusionTtsModel::from_str_lenient(""), FusionTtsModel::WindowsSapi);
    assert_eq!(FusionTtsModel::from_str_lenient("unknown_garbage_model_123"), FusionTtsModel::WindowsSapi);
}

#[test]
fn test_tts_model_display_names_are_distinct_and_descriptive() {
    let models = [
        FusionTtsModel::F5Tts,
        FusionTtsModel::Kokoro82m,
        FusionTtsModel::SunoBark,
        FusionTtsModel::CoquiXttsV2,
        FusionTtsModel::FishSpeech15,
        FusionTtsModel::ParlerTtsLarge,
        FusionTtsModel::SeedVc,
        FusionTtsModel::WindowsSapi,
    ];

    let mut names = std::collections::HashSet::new();
    for m in models {
        let name = m.display_name();
        assert!(!name.is_empty(), "Display name for {:?} must not be empty", m);
        assert!(
            names.insert(name),
            "Duplicate display name found for {:?}: {}",
            m,
            name
        );
    }
}

// ============================================================================
// Group 2: Pure-Rust Formant Synthesis Fallback Under Adversarial Inputs
// ============================================================================

#[test]
fn test_pure_rust_formants_across_all_model_variants_produces_valid_audio() {
    let test_dir = get_temp_test_dir("all_models");

    let models = [
        FusionTtsModel::F5Tts,
        FusionTtsModel::Kokoro82m,
        FusionTtsModel::SunoBark,
        FusionTtsModel::CoquiXttsV2,
        FusionTtsModel::FishSpeech15,
        FusionTtsModel::ParlerTtsLarge,
        FusionTtsModel::SeedVc,
        FusionTtsModel::WindowsSapi,
    ];

    let lyrics = "One love, one heart, let's get together and feel alright";

    for model in models {
        let model_tag = format!("{:?}", model);
        let lead_path = test_dir.join(format!("{}_lead.wav", model_tag));
        let yue_path = test_dir.join(format!("{}_yue.wav", model_tag));

        let res = FusionTtsEngine::synthesize_pure_rust_formants(
            lyrics,
            &model,
            "Lead Male Tenor",
            &lead_path,
            &yue_path,
        );
        assert!(res.is_ok(), "synthesize_pure_rust_formants failed for {:?}", model);

        // Verify Lead Vocal Audio
        let (lead_l, lead_r, sr) = FusionTtsEngine::read_wav_pcm16_stereo(&lead_path)
            .expect("Failed to read generated lead wav");
        assert_eq!(sr, 48_000, "Sample rate must be 48 kHz");
        assert_eq!(lead_l.len(), lead_r.len());
        assert!(!lead_l.is_empty(), "Lead audio must not be empty");

        // Verify Yue Duet Audio
        let (yue_l, yue_r, yue_sr) = FusionTtsEngine::read_wav_pcm16_stereo(&yue_path)
            .expect("Failed to read generated yue wav");
        assert_eq!(yue_sr, 48_000);
        assert_eq!(yue_l.len(), yue_r.len());
        assert!(!yue_l.is_empty());

        // Numeric checks: no NaNs, no Infs, finite peak bounded
        for &s in lead_l.iter().chain(lead_r.iter()).chain(yue_l.iter()).chain(yue_r.iter()) {
            assert!(s.is_finite(), "Sample is not finite in model {:?}", model);
            assert!(s.abs() <= 1.0, "Sample exceeded full scale: {} in model {:?}", s, model);
        }

        // Audibility check: not complete silence
        let max_lead = lead_l.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        assert!(
            max_lead > 0.05,
            "Synthesized lead vocal was silent (peak: {}) in model {:?}",
            max_lead,
            model
        );
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_pure_rust_formants_adversarial_text_inputs() {
    let test_dir = get_temp_test_dir("adversarial_text");

    let adversarial_cases = [
        ("empty", ""),
        ("whitespace_only", "   \t\n\r   "),
        ("single_word", "Love"),
        ("symbols_and_numbers", "!@#$%^&*()_+ 1234567890 -= ~`"),
        ("emojis", "🎤 🎸 🎹 🥁 🎶 ❤️"),
        (
            "structured_brackets",
            "[Intro]\nYeah yeah\n[Verse 1]\nEarly in the morning\n[Chorus]\nOne love!\n[Outro]\nFading out",
        ),
        (
            "long_rap_lyrics",
            "This is a long continuous passage of lyrical content designed to stress test buffer allocation and formant calculation across multiple phrases without causing arithmetic overflow or memory exhaustion in the synthesis engine.",
        ),
    ];

    for (name, text) in adversarial_cases {
        let lead_path = test_dir.join(format!("{}_lead.wav", name));
        let yue_path = test_dir.join(format!("{}_yue.wav", name));

        let res = FusionTtsEngine::synthesize_pure_rust_formants(
            text,
            &FusionTtsModel::Kokoro82m,
            "Soulful Female Soprano",
            &lead_path,
            &yue_path,
        );
        assert!(
            res.is_ok(),
            "Formant synthesis panicked or failed on input '{}': {:?}",
            name,
            res.err()
        );

        assert!(lead_path.exists(), "Lead file missing for '{}'", name);
        assert!(yue_path.exists(), "Yue file missing for '{}'", name);

        let (lead_l, _, sr) = FusionTtsEngine::read_wav_pcm16_stereo(&lead_path).unwrap();
        assert_eq!(sr, 48_000);
        assert!(!lead_l.is_empty());
        for &s in &lead_l {
            assert!(s.is_finite());
            assert!(s.abs() <= 1.0);
        }
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_pure_rust_formants_various_voice_styles() {
    let test_dir = get_temp_test_dir("voice_styles");

    let styles = [
        "Lead Male Tenor",
        "Soulful Female Soprano",
        "Lead + Gospel Backing Choir Duet",
        "Gritty Rock Baritone",
        "",
        "Extremely Unusual Voice Archetype 42",
    ];

    let lyrics = "Singing through the night, guided by the morning light";

    for (idx, style) in styles.iter().enumerate() {
        let lead_path = test_dir.join(format!("style_{}_lead.wav", idx));
        let yue_path = test_dir.join(format!("style_{}_yue.wav", idx));

        let res = FusionTtsEngine::synthesize_pure_rust_formants(
            lyrics,
            &FusionTtsModel::F5Tts,
            style,
            &lead_path,
            &yue_path,
        );
        assert!(res.is_ok(), "Failed for voice style '{}'", style);

        let (l, r, sr) = FusionTtsEngine::read_wav_pcm16_stereo(&lead_path).unwrap();
        assert_eq!(sr, 48_000);
        assert_eq!(l.len(), r.len());
        assert!(!l.is_empty());
    }

    let _ = fs::remove_dir_all(&test_dir);
}

// ============================================================================
// Group 3: End-to-End Async Vocal Stem Synthesis
// ============================================================================

#[tokio::test]
async fn test_end_to_end_synthesize_vocal_stems_generates_valid_vocal_files() {
    let test_dir = get_temp_test_dir("e2e_stems");

    let lyrics = "Rise up this morning, smile with the rising sun";
    let (lead_path, yue_path) = FusionTtsEngine::synthesize_vocal_stems(
        lyrics,
        "kokoro-82m",
        Some("Lead Male Tenor"),
        &test_dir,
    )
    .await
    .expect("synthesize_vocal_stems should succeed");

    assert!(lead_path.exists(), "Lead path does not exist: {}", lead_path.display());
    assert!(yue_path.exists(), "Yue path does not exist: {}", yue_path.display());

    // Check audio content
    let (lead_l, lead_r, sr) = FusionTtsEngine::read_wav_pcm16_stereo(&lead_path).unwrap();
    assert_eq!(sr, 48_000);
    assert_eq!(lead_l.len(), lead_r.len());
    assert!(!lead_l.is_empty());

    let max_peak = lead_l.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    assert!(max_peak > 0.01, "Lead vocal must have audible energy, got peak {}", max_peak);
    assert!(max_peak <= 1.0, "Lead vocal exceeded full scale: {}", max_peak);

    let _ = fs::remove_dir_all(&test_dir);
}

/// Empirical Bug Proof: `read_wav_pcm16_stereo` in `tts_engine.rs` hardcodes `data_offset = 36`,
/// failing to parse standard WAVEFORMATEX files emitted by Windows System.Speech (18-byte fmt chunk).
#[test]
fn test_reproduce_sapi_18_byte_fmt_chunk_data_offset_bug() {
    // Construct standard WAVEFORMATEX WAV header with 18-byte fmt chunk (size 18)
    // SAPI emits this format (16 bytes audio format info + 2 bytes cbSize = 0)
    let mut wav_bytes = Vec::new();
    wav_bytes.extend_from_slice(b"RIFF");
    wav_bytes.extend_from_slice(&(46u32).to_le_bytes()); // file size
    wav_bytes.extend_from_slice(b"WAVE");
    wav_bytes.extend_from_slice(b"fmt ");
    wav_bytes.extend_from_slice(&(18u32).to_le_bytes()); // 18-byte fmt chunk!
    wav_bytes.extend_from_slice(&(1u16).to_le_bytes());  // PCM
    wav_bytes.extend_from_slice(&(2u16).to_le_bytes());  // stereo
    wav_bytes.extend_from_slice(&(48000u32).to_le_bytes()); // sample rate
    wav_bytes.extend_from_slice(&(192000u32).to_le_bytes()); // byte rate
    wav_bytes.extend_from_slice(&(4u16).to_le_bytes());  // block align
    wav_bytes.extend_from_slice(&(16u16).to_le_bytes()); // bits per sample
    wav_bytes.extend_from_slice(&(0u16).to_le_bytes());  // cbSize = 0 (2 bytes)
    wav_bytes.extend_from_slice(b"data");
    wav_bytes.extend_from_slice(&(4u32).to_le_bytes());  // 4 bytes data
    wav_bytes.extend_from_slice(&[0u8; 4]);              // 1 sample stereo

    let temp_file = std::env::temp_dir().join(format!("test_sapi_bug_{}.wav", uuid::Uuid::new_v4()));
    fs::write(&temp_file, &wav_bytes).unwrap();

    let res = FusionTtsEngine::read_wav_pcm16_stereo(&temp_file);
    let _ = fs::remove_file(&temp_file);

    // This verifies that read_wav_pcm16_stereo correctly parses 18-byte fmt chunks emitted by Windows SAPI
    assert!(
        res.is_ok(),
        "Remediated: read_wav_pcm16_stereo now dynamically walks chunks and handles 18-byte fmt chunks"
    );
    let (left, right, sr) = res.unwrap();
    assert_eq!(sr, 48_000);
    assert_eq!(left.len(), 1);
    assert_eq!(right.len(), 1);
}

// ============================================================================
// Group 4: WAV Parser & Writer Error Resilience
// ============================================================================

#[test]
fn test_read_wav_pcm16_stereo_invalid_and_corrupt_files() {
    let test_dir = get_temp_test_dir("corrupt_wavs");

    // 1. Non-existent file
    let non_existent = test_dir.join("does_not_exist.wav");
    assert!(FusionTtsEngine::read_wav_pcm16_stereo(&non_existent).is_err());

    // 2. Empty file (0 bytes)
    let empty_file = test_dir.join("empty.wav");
    fs::write(&empty_file, b"").unwrap();
    assert!(FusionTtsEngine::read_wav_pcm16_stereo(&empty_file).is_err());

    // 3. Truncated header (20 bytes)
    let truncated = test_dir.join("truncated.wav");
    fs::write(&truncated, b"RIFF\x14\x00\x00\x00WAVEfmt ").unwrap();
    assert!(FusionTtsEngine::read_wav_pcm16_stereo(&truncated).is_err());

    // 4. Invalid magic (not RIFF)
    let bad_magic = test_dir.join("bad_magic.wav");
    let mut bad_bytes = vec![0u8; 44];
    bad_bytes[0..4].copy_from_slice(b"FORM");
    fs::write(&bad_magic, &bad_bytes).unwrap();
    assert!(FusionTtsEngine::read_wav_pcm16_stereo(&bad_magic).is_err());

    // 5. Missing data chunk
    let no_data = test_dir.join("no_data.wav");
    let mut header_only = vec![0u8; 36];
    header_only[0..4].copy_from_slice(b"RIFF");
    header_only[8..12].copy_from_slice(b"WAVE");
    fs::write(&no_data, &header_only).unwrap();
    assert!(FusionTtsEngine::read_wav_pcm16_stereo(&no_data).is_err());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_write_wav_pcm16_stereo_channel_length_mismatch_and_extremes() {
    let test_dir = get_temp_test_dir("write_wav");
    let out_file = test_dir.join("mismatch.wav");

    // Channel length mismatch (Left has 1000 samples, Right has 500 samples)
    let left = vec![0.5f32; 1000];
    let right = vec![-0.5f32; 500];

    // Must safely truncate to min(left.len(), right.len()) without panic
    let write_res = FusionTtsEngine::write_wav_pcm16_stereo(&out_file, &left, &right, 48_000);
    assert!(write_res.is_ok(), "write_wav_pcm16_stereo failed on length mismatch");

    let (read_l, read_r, sr) = FusionTtsEngine::read_wav_pcm16_stereo(&out_file).unwrap();
    assert_eq!(sr, 48_000);
    assert_eq!(read_l.len(), 500);
    assert_eq!(read_r.len(), 500);

    // Overdriven sample values clamped safely to 16-bit PCM
    let overdriven_file = test_dir.join("overdrive.wav");
    let over_l = vec![10.0f32, -10.0f32, 100.0f32];
    let over_r = vec![-5.0f32, 5.0f32, -100.0f32];
    assert!(FusionTtsEngine::write_wav_pcm16_stereo(&overdriven_file, &over_l, &over_r, 48_000).is_ok());

    let (read_over_l, _, _) = FusionTtsEngine::read_wav_pcm16_stereo(&overdriven_file).unwrap();
    for &s in &read_over_l {
        assert!(s.abs() <= 1.0, "Overdriven sample was not clamped to valid PCM range: {}", s);
    }

    let _ = fs::remove_dir_all(&test_dir);
}

// ============================================================================
// Group 5: REST API Data Contract Serialization (TtsVocalRequest & Response)
// ============================================================================

#[test]
fn test_tts_vocal_request_and_response_serde_contract() {
    let req = TtsVocalRequest {
        song_id: "song_reggae_anthem_001".to_string(),
        version: 2,
        text: "One love, one heart".to_string(),
        model: "kokoro-82m".to_string(),
        voice_style: Some("Lead Male Tenor".to_string()),
    };

    let json_req = serde_json::to_string(&req).expect("Failed to serialize TtsVocalRequest");
    let parsed_req: TtsVocalRequest = serde_json::from_str(&json_req).expect("Failed to deserialize TtsVocalRequest");
    assert_eq!(parsed_req.song_id, req.song_id);
    assert_eq!(parsed_req.version, 2);
    assert_eq!(parsed_req.model, "kokoro-82m");
    assert_eq!(parsed_req.voice_style.as_deref(), Some("Lead Male Tenor"));

    let mut audio_urls = std::collections::HashMap::new();
    audio_urls.insert("master".to_string(), "/api/songs/1/versions/2/audio/final_master".to_string());
    audio_urls.insert("vocals".to_string(), "/api/songs/1/versions/2/audio/raspy_tenor_vocals".to_string());

    let resp = TtsVocalResponse {
        status: "success".to_string(),
        song_id: "song_reggae_anthem_001".to_string(),
        version: 2,
        model: "Kokoro-82M Clear Lead".to_string(),
        vocal_url: "/api/songs/1/versions/2/audio/raspy_tenor_vocals".to_string(),
        master_url: "/api/songs/1/versions/2/audio/final_master".to_string(),
        audio_urls,
    };

    let json_resp = serde_json::to_string(&resp).expect("Failed to serialize TtsVocalResponse");
    let parsed_resp: TtsVocalResponse = serde_json::from_str(&json_resp).expect("Failed to deserialize TtsVocalResponse");
    assert_eq!(parsed_resp.status, "success");
    assert_eq!(parsed_resp.audio_urls.len(), 2);
}
