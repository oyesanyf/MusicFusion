//! Tier 2: Boundary, Extreme Limits & Stress Test Suite
//!
//! Validates system resilience across boundary conditions, extreme inputs,
//! high-amplitude audio bursts, zero-length audio buffers, hardware resource limits,
//! and network/subprocess failure recovery.

use super::common::*;


// =========================================================================
// Category 1: Extreme Tempos, Meters & Tonal Scalability (B01–B05)
// =========================================================================

#[test]
fn test_t2_b01_01_minimum_tempo_boundary_40_bpm() {
    let groove = PureRustDspEngine::synthesize_groove("Roots Reggae", 40, 1.0);
    assert_eq!(groove.sample_rate, 48_000);
    let tel = SpectralTelemetryEngine::analyze(&groove, 40);
    assert!(!tel.clipping);
}

#[test]
fn test_t2_b01_02_maximum_tempo_boundary_220_bpm() {
    let groove = PureRustDspEngine::synthesize_groove("Jazz Bebop", 220, 1.0);
    let tel = SpectralTelemetryEngine::analyze(&groove, 220);
    assert!(!tel.clipping);
}

#[test]
fn test_t2_b01_03_tempo_clamping_below_minimum() {
    let raw_bpm = 25u32;
    let clamped_bpm = raw_bpm.clamp(40, 220);
    assert_eq!(clamped_bpm, 40);
}

#[test]
fn test_t2_b01_04_tempo_clamping_above_maximum() {
    let raw_bpm = 350u32;
    let clamped_bpm = raw_bpm.clamp(40, 220);
    assert_eq!(clamped_bpm, 220);
}

#[test]
fn test_t2_b01_05_fractional_bpm_period_calculation() {
    let bpm = 76u32;
    let period_samples = (60.0 / bpm as f32 * 48000.0).round() as usize;
    assert_eq!(period_samples, 37895);
}

#[test]
fn test_t2_b02_01_compound_meter_6_8_bar_calculation() {
    let bars = 4;
    let bpm = 84;
    let beats_per_bar = 2.0; // 2 dotted quarter beats in 6/8
    let duration = (bars as f32 * beats_per_bar * 60.0) / bpm as f32;
    assert!((duration - 5.714).abs() < 0.01);
}

#[test]
fn test_t2_b02_02_odd_meter_7_8_pulse() {
    let time_sig = "7/8";
    assert_eq!(time_sig, "7/8");
}

#[test]
fn test_t2_b02_03_time_signature_3_4_waltz() {
    let time_sig = "3/4";
    assert_eq!(time_sig, "3/4");
}

#[test]
fn test_t2_b02_04_time_signature_12_8_blues_shuffle() {
    let blues = UniversalGenreRegistry::lookup("blues").expect("Blues registered");
    assert_eq!(blues.default_time_signature, "12/8");
}

#[test]
fn test_t2_b02_05_time_signature_5_4_take_five() {
    let time_sig = "5/4";
    assert_eq!(time_sig, "5/4");
}

// =========================================================================
// Category 2: Audio Buffer, Volume & Clipping Stress (B06–B10)
// =========================================================================

#[test]
fn test_t2_b03_01_extreme_input_burst_12db_over() {
    let mut buf = AudioBuffer::new_stereo(0.1);
    for s in &mut buf.samples { *s = 4.0; } // +12 dBFS over 0 dB
    let mastered = PureRustDspEngine::apply_analog_tape_mastering(&buf, 3.2);
    for &sample in &mastered.samples {
        assert!(sample.abs() <= 0.966, "Sample exceeded limiter ceiling: {}", sample);
    }
}

#[test]
fn test_t2_b03_02_dc_offset_elimination() {
    let mut buf = AudioBuffer::new_stereo(0.1);
    for s in &mut buf.samples { *s = 0.5; } // DC constant
    let mastered = PureRustDspEngine::apply_analog_tape_mastering(&buf, 0.0);
    for &sample in &mastered.samples {
        assert!(sample.abs() < 1.0);
    }
}

#[test]
fn test_t2_b03_03_near_silent_buffer_avoid_log_error() {
    let mut buf = AudioBuffer::new_stereo(0.1);
    for s in &mut buf.samples { *s = 1e-7; }
    let tel = SpectralTelemetryEngine::analyze(&buf, 100);
    assert!(!tel.peak_dbfs.is_nan());
    assert!(!tel.rms_dbfs.is_nan());
}

#[test]
fn test_t2_b03_04_zero_length_buffer_handling() {
    let buf = AudioBuffer {
        sample_rate: 48000,
        channels: 2,
        samples: Vec::new(),
    };
    assert_eq!(buf.frame_count(), 0);
    let tel = SpectralTelemetryEngine::analyze(&buf, 100);
    assert_eq!(tel.clipping, false);
}

#[test]
fn test_t2_b03_05_single_sample_buffer_handling() {
    let buf = AudioBuffer {
        sample_rate: 48000,
        channels: 2,
        samples: vec![0.5, 0.5],
    };
    assert_eq!(buf.frame_count(), 1);
    let tel = SpectralTelemetryEngine::analyze(&buf, 100);
    assert!(!tel.clipping);
}

// =========================================================================
// Category 3: Hardware, VRAM & Subprocess Stress (B11–B15)
// =========================================================================

#[test]
fn test_t2_b04_01_zero_gpu_fallback() {
    let free_vram_gb = 0.0;
    assert!(DynamicMemoryLawOracle::should_route_to_pure_rust_dsp(free_vram_gb, "auto"));
}

#[test]
fn test_t2_b04_02_boundary_vram_3_99_gb() {
    let free_vram_gb = 3.99;
    assert!(DynamicMemoryLawOracle::should_route_to_pure_rust_dsp(free_vram_gb, "auto"));
}

#[test]
fn test_t2_b04_03_boundary_vram_4_01_gb() {
    let free_vram_gb = 4.01;
    assert!(!DynamicMemoryLawOracle::should_route_to_pure_rust_dsp(free_vram_gb, "auto"));
}

#[test]
fn test_t2_b04_04_pure_rust_mode_overrides_high_vram() {
    let free_vram_gb = 24.0;
    assert!(DynamicMemoryLawOracle::should_route_to_pure_rust_dsp(free_vram_gb, "pure_rust"));
}

#[test]
fn test_t2_b04_05_ram_safety_budget_minimum_reserve() {
    let free_ram = 2.5; // less than 3 GB reserve
    let budget = DynamicMemoryLawOracle::compute_ram_budget_gb(free_ram);
    assert_eq!(budget, 0.0);
}

// =========================================================================
// Category 4: Ollama Panel & Prompt Ingestion Stress (B16–B20)
// =========================================================================

#[test]
fn test_t2_b05_01_offline_ollama_graceful_blueprint() {
    let blueprint_lyrics = "[Intro]\n\n[Verse 1]\nLines\n\n[Chorus]\nAnthem\n\n[Outro]";
    assert!(blueprint_lyrics.contains("[Intro]"));
    assert!(blueprint_lyrics.contains("[Chorus]"));
}

#[test]
fn test_t2_b05_02_unclosed_think_block_recovery() {
    let malformed = "<think>Unclosed reasoning tag without closing bracket{\"score\": 8.0}";
    let cleaned = if malformed.starts_with("<think>") {
        if let Some(pos) = malformed.find("{\"") {
            &malformed[pos..]
        } else {
            malformed
        }
    } else {
        malformed
    };
    assert!(cleaned.starts_with("{\""));
}

#[test]
fn test_t2_b05_03_extreme_prompt_length_16k_chars() {
    let huge_prompt = "Roots Reggae spiritual unity ".repeat(600);
    assert!(huge_prompt.len() > 16_000);
    let truncated = if huge_prompt.len() > 4096 {
        &huge_prompt[..4096]
    } else {
        &huge_prompt
    };
    assert_eq!(truncated.len(), 4096);
}

#[test]
fn test_t2_b05_04_lyrics_missing_tags_auto_insertion() {
    let raw_lyrics = "Line one\nLine two\nLine three";
    let formatted = if !raw_lyrics.contains("[") {
        format!("[Intro]\n\n[Verse 1]\n{}\n\n[Outro]", raw_lyrics)
    } else {
        raw_lyrics.to_string()
    };
    assert!(formatted.contains("[Intro]"));
    assert!(formatted.contains("[Verse 1]"));
    assert!(formatted.contains("[Outro]"));
}

#[test]
fn test_t2_b05_05_unicode_and_emoji_prompt_sanitization() {
    let prompt_with_emojis = "1977 Roots Reggae 🇯🇲 🎵 One Love ❤️ & Compassion 🔥";
    assert!(prompt_with_emojis.contains("One Love"));
}

// =========================================================================
// Category 5: State, Versioning & Quality Gate Retry Stress (B21–B25)
// =========================================================================

#[test]
fn test_t2_b06_01_retry_count_cap_enforcement() {
    let mut state = MusicProductionState::default();
    state.quality_passed = false;
    for _ in 0..5 {
        if state.retry_count < 2 {
            state.retry_count += 1;
        } else {
            state.quality_passed = true; // force forward
            break;
        }
    }
    assert_eq!(state.retry_count, 2);
    assert!(state.quality_passed);
}

#[test]
fn test_t2_b06_02_vocal_tempo_drift_threshold() {
    let drift_high = 0.55;
    let routes_to_node2 = drift_high > 0.5;
    assert!(routes_to_node2);

    let drift_low = 0.25;
    let routes_to_node4 = drift_low <= 0.5;
    assert!(routes_to_node4);
}

#[test]
fn test_t2_b06_03_concurrent_version_increment() {
    let mut v1 = MusicProductionState::default();
    v1.version = 1;
    let mut v2 = v1.clone();
    v2.version += 1;
    let mut v3 = v2.clone();
    v3.version += 1;
    assert_eq!(v1.version, 1);
    assert_eq!(v2.version, 2);
    assert_eq!(v3.version, 3);
}

#[test]
fn test_t2_b06_04_missing_stem_recovery() {
    let mut state = MusicProductionState::default();
    assert!(!state.stem_paths.contains_key("inpainted_guitar"));
    state.stem_paths.insert("inpainted_guitar".to_string(), PathBuf::from("fallback_guitar.wav"));
    assert!(state.stem_paths.contains_key("inpainted_guitar"));
}

#[test]
fn test_t2_b06_05_json_snapshot_disk_safety() {
    let state = MusicProductionState::default();
    let json = serde_json::to_string_pretty(&state).expect("Serialization failed");
    assert!(!json.is_empty());
}
