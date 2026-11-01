//! Comprehensive Unit Test Suite for `music_dsp` crate:
//! 1. WAV I/O, AudioBuffer & Serialization
//! 2. Spectral Telemetry Calculation & Radix-2 FFT
//! 3. Universal Drum & Rhythm Synthesizer (all 7 styles & BPM bounds)
//! 4. Extended Karplus-Strong Physical Guitar & Wah-Wah Modeler
//! 5. Vocal Formant Filter Bank & Asymmetric Vocal Rasp Saturation
//! 6. Sample-Accurate Equal-Power Stem Crossfader
//! 7. 48 kHz Analog Tape Saturation, Sub-Bass Warmth & Lookahead Brickwall Limiter

use std::f32::consts::PI;
use music_core::genre::RhythmGroove;
use music_dsp::{
    apply_analog_mastering, apply_analog_tape_mastering, apply_vocal_formant_and_rasp,
    apply_vocal_timbre, calculate_phase_alignment, calculate_snr, calculate_spectral_flatness,
    calculate_telemetry, calculate_tempo_drift, equal_power_crossfade, generate_groove,
    splice_stem, synthesize_lead_solo, AnalogTapeSaturator, AudioBuffer, BiquadFilter,
    BrickwallLimiter, CrossfadeCurve, DrumVoice, FormantFilterBank, GuitarAmpSimulator,
    KarplusStrongString, MasteringProfile, PitchLockVocoder, SoloGenerator, StemCrossfader,
    UniversalDrumSynthesizer, VocalArchetype, VocalRaspProcessor, WahWahFilter, WavBitDepth,
    BRICKWALL_CEILING, DEFAULT_SAMPLE_RATE,
};

// =========================================================================
// 1. WAV I/O & AudioBuffer Tests
// =========================================================================

#[test]
fn test_wav_16bit_pcm_roundtrip() {
    let mut buf = AudioBuffer::new_stereo(0.1);
    for i in 0..buf.frame_count() {
        let t = i as f32 / DEFAULT_SAMPLE_RATE as f32;
        let s = (2.0 * PI * 440.0 * t).sin() * 0.75;
        buf.set_channel_sample(i, 0, s);
        buf.set_channel_sample(i, 1, s * 0.5);
    }
    let bytes = buf.to_wav_bytes_with_format(WavBitDepth::Pcm16);
    let decoded = AudioBuffer::from_wav_bytes(&bytes).expect("16-bit WAV decode failed");
    assert_eq!(buf.sample_rate, decoded.sample_rate);
    assert_eq!(buf.channels, decoded.channels);
    assert_eq!(buf.frame_count(), decoded.frame_count());
    for i in 0..buf.samples.len() {
        assert!((buf.samples[i] - decoded.samples[i]).abs() < 1.0 / 32768.0 + 1e-4);
    }
}

#[test]
fn test_wav_24bit_pcm_roundtrip() {
    let mut buf = AudioBuffer::new_stereo(0.1);
    for i in 0..buf.frame_count() {
        let t = i as f32 / DEFAULT_SAMPLE_RATE as f32;
        let s = (2.0 * PI * 1000.0 * t).sin() * 0.85;
        buf.set_channel_sample(i, 0, s);
        buf.set_channel_sample(i, 1, -s);
    }
    let bytes = buf.to_wav_bytes_with_format(WavBitDepth::Pcm24);
    let decoded = AudioBuffer::from_wav_bytes(&bytes).expect("24-bit WAV decode failed");
    assert_eq!(buf.sample_rate, decoded.sample_rate);
    assert_eq!(buf.channels, decoded.channels);
    assert_eq!(buf.frame_count(), decoded.frame_count());
    for i in 0..buf.samples.len() {
        assert!((buf.samples[i] - decoded.samples[i]).abs() < 1.0 / 8388608.0 + 1e-6);
    }
}

#[test]
fn test_wav_32bit_float_exact_roundtrip() {
    let mut buf = AudioBuffer::new_stereo(0.05);
    for i in 0..buf.samples.len() {
        buf.samples[i] = (i as f32 * 0.01).sin() * 0.9;
    }
    let bytes = buf.to_wav_bytes_with_format(WavBitDepth::Float32);
    let decoded = AudioBuffer::from_wav_bytes(&bytes).expect("Float WAV decode failed");
    assert_eq!(buf.samples, decoded.samples);
}

#[test]
fn test_wav_file_roundtrip_disk() {
    let tmp_path = std::env::temp_dir().join("test_dsp_roundtrip_audio.wav");
    let mut buf = AudioBuffer::new_stereo(0.08);
    for i in 0..buf.samples.len() {
        buf.samples[i] = (i as f32 * 0.05).cos() * 0.6;
    }
    buf.to_wav_file(&tmp_path).expect("Failed to write WAV file");
    let read_buf = AudioBuffer::from_wav_file(&tmp_path).expect("Failed to read WAV file");
    assert_eq!(buf.sample_rate, read_buf.sample_rate);
    assert_eq!(buf.channels, read_buf.channels);
    assert_eq!(buf.samples, read_buf.samples);
    let _ = std::fs::remove_file(tmp_path);
}

#[test]
fn test_buffer_mix_and_scaling() {
    let mut base = AudioBuffer::new_stereo(0.1);
    let mut overlay = AudioBuffer::new_stereo(0.1);
    for s in &mut base.samples { *s = 0.2; }
    for s in &mut overlay.samples { *s = 0.3; }

    base.mix(&overlay, 2.0).expect("Mix failed");
    for s in &base.samples {
        assert!((s - 0.8).abs() < 1e-5);
    }

    base.scale_gain(0.5);
    for s in &base.samples {
        assert!((s - 0.4).abs() < 1e-5);
    }

    base.scale_db(-6.0206); // halve amplitude
    for s in &base.samples {
        assert!((s - 0.2).abs() < 1e-3);
    }
}

#[test]
fn test_buffer_normalize_and_levels() {
    let mut buf = AudioBuffer::new_stereo(0.1);
    buf.samples[10] = 0.5; // Peak 0.5
    assert_eq!(buf.peak_level(), 0.5);
    buf.normalize(-1.0); // Target -1.0 dBFS (0.89125)
    let tel = calculate_telemetry(&buf, 120);
    assert!((tel.peak_dbfs - (-1.0)).abs() < 0.05);
    assert!(buf.rms_level() > 0.0);
}

#[test]
fn test_buffer_slice_and_resample() {
    let mut buf = AudioBuffer::new_stereo(1.0);
    for i in 0..buf.frame_count() {
        buf.set_channel_sample(i, 0, i as f32);
    }
    let sliced = buf.slice(1000, 500);
    assert_eq!(sliced.frame_count(), 500);
    assert_eq!(sliced.channel_sample(0, 0), Some(1000.0));

    let resampled = sliced.resample_linear(24_000);
    assert_eq!(resampled.sample_rate, 24_000);
    assert_eq!(resampled.frame_count(), 250);
}

#[test]
fn test_buffer_mono_stereo_conversion() {
    let mut mono = AudioBuffer::new_mono(0.1);
    for s in &mut mono.samples { *s = 0.42; }
    let stereo = mono.to_stereo();
    assert_eq!(stereo.channels, 2);
    assert_eq!(stereo.samples.len(), mono.samples.len() * 2);
    assert_eq!(stereo.samples[0], 0.42);
    assert_eq!(stereo.samples[1], 0.42);

    let back_mono = stereo.to_mono();
    assert_eq!(back_mono.channels, 1);
    assert_eq!(back_mono.samples[0], 0.42);
}

#[test]
fn test_invalid_wav_header_rejection() {
    let bad_bytes = vec![0u8; 32];
    assert!(AudioBuffer::from_wav_bytes(&bad_bytes).is_err());
}

// =========================================================================
// 2. Spectral Telemetry Calculation Tests
// =========================================================================

#[test]
fn test_telemetry_sine_rms_and_peak() {
    let mut buf = AudioBuffer::new_mono(0.2);
    for i in 0..buf.frame_count() {
        let t = i as f32 / DEFAULT_SAMPLE_RATE as f32;
        buf.samples[i] = (2.0 * PI * 1000.0 * t).sin() * 0.95; // -0.45 dBFS peak
    }
    let tel = calculate_telemetry(&buf, 120);
    assert!((tel.peak_dbfs - (-0.45)).abs() < 0.05);
    // Theoretical sine RMS is -0.45 - 3.01 = -3.46 dBFS
    assert!((tel.rms_dbfs - (-3.46)).abs() < 0.15);
    assert!(!tel.clipping);
}

#[test]
fn test_telemetry_clipping_flag() {
    let mut buf = AudioBuffer::new_stereo(0.05);
    buf.samples[100] = 1.002;
    let tel = calculate_telemetry(&buf, 120);
    assert!(tel.clipping);
    assert!(tel.peak_dbfs > 0.0);
}

#[test]
fn test_spectral_flatness_tonal_vs_noise() {
    let mut tone_samples = vec![0.0f32; 2048];
    for i in 0..2048 {
        tone_samples[i] = (2.0 * PI * 440.0 * (i as f32 / 48000.0)).sin();
    }
    let sfm_tone = calculate_spectral_flatness(&tone_samples);
    assert!(sfm_tone < 0.05, "Tone flatness was unexpectedly high: {}", sfm_tone);

    let mut rng = music_dsp::DspRng::new(12345);
    let mut noise_samples = vec![0.0f32; 2048];
    for i in 0..2048 {
        noise_samples[i] = rng.next_f32();
    }
    let sfm_noise = calculate_spectral_flatness(&noise_samples);
    assert!(sfm_noise > 0.70, "Noise flatness was unexpectedly low: {}", sfm_noise);
}

#[test]
fn test_phase_alignment_in_phase_and_out_of_phase() {
    let mut in_phase = AudioBuffer::new_stereo(0.1);
    for i in 0..in_phase.frame_count() {
        in_phase.set_channel_sample(i, 0, 0.5);
        in_phase.set_channel_sample(i, 1, 0.5);
    }
    assert!((calculate_phase_alignment(&in_phase) - 1.0).abs() < 1e-4);

    let mut out_of_phase = AudioBuffer::new_stereo(0.1);
    for i in 0..out_of_phase.frame_count() {
        out_of_phase.set_channel_sample(i, 0, 0.5);
        out_of_phase.set_channel_sample(i, 1, -0.5);
    }
    assert!((calculate_phase_alignment(&out_of_phase) - 0.0).abs() < 1e-4);
}

#[test]
fn test_telemetry_near_silent_buffer() {
    let silent = AudioBuffer::new_stereo(0.1);
    let tel = calculate_telemetry(&silent, 76);
    assert_eq!(tel.peak_dbfs, -90.0);
    assert_eq!(tel.rms_dbfs, -90.0);
    assert!(!tel.clipping);
    assert!(!tel.peak_dbfs.is_nan());
    assert!(!tel.rms_dbfs.is_nan());
}

#[test]
fn test_telemetry_snr_and_tempo_drift() {
    let buf = generate_groove("Roots Reggae", 76, 4.0);
    let snr = calculate_snr(&buf);
    assert!(snr >= 20.0 && snr <= 96.0);
    let drift = calculate_tempo_drift(&buf, 76);
    assert!(drift <= 0.05, "On-grid groove drift: {}", drift);
}

#[test]
fn test_vocal_formant_and_rasp_alias() {
    let mut buf = AudioBuffer::new_stereo(0.05);
    for s in &mut buf.samples { *s = 0.5; }
    let out = apply_vocal_formant_and_rasp(&buf, 0.5);
    assert_eq!(out.frame_count(), buf.frame_count());
}

#[test]
fn test_splice_stem_helper() {
    let base = AudioBuffer::new_stereo(4.0);
    let splice = AudioBuffer::new_stereo(1.0);
    let out = splice_stem(&base, &splice, 1, 76, 4);
    assert_eq!(out.frame_count(), base.frame_count());
}

#[test]
fn test_drum_voice_and_solo_generator() {
    let dur = UniversalDrumSynthesizer::voice_duration_secs(DrumVoice::Kick808Sub);
    assert!(dur > 0.5);
    let solo = SoloGenerator::synthesize_lead_solo(4, 76, "Blues");
    assert_eq!(solo.sample_rate, 48_000);
}

// =========================================================================
// 3. Universal Drum & Rhythm Synthesizer Tests
// =========================================================================

#[test]
fn test_groove_one_drop_reggae_76_bpm() {
    let buf = generate_groove("Roots Reggae", 76, 4.0);
    assert_eq!(buf.sample_rate, 48_000);
    assert_eq!(buf.channels, 2);
    assert_eq!(buf.frame_count(), 48_000 * 4);

    // Verify zero digital clipping
    for s in &buf.samples {
        assert!(s.abs() < 1.0, "One-drop sample clipped: {}", s);
    }

    // Verify non-silent stereo channels
    let left_energy: f32 = buf.samples.iter().step_by(2).map(|s| s.abs()).sum();
    let right_energy: f32 = buf.samples.iter().skip(1).step_by(2).map(|s| s.abs()).sum();
    assert!(left_energy > 50.0);
    assert!(right_energy > 50.0);
}

#[test]
fn test_groove_four_on_the_floor_edm_128_bpm() {
    let buf = UniversalDrumSynthesizer::generate_groove_rhythm(RhythmGroove::FourOnTheFloor, 128, 2.0);
    assert_eq!(buf.frame_count(), 48_000 * 2);
    for s in &buf.samples {
        assert!(s.abs() <= 0.95);
    }
}

#[test]
fn test_groove_trap_808_sub_kick_144_bpm() {
    let buf = UniversalDrumSynthesizer::generate_groove_rhythm(RhythmGroove::Trap808, 144, 2.0);
    assert_eq!(buf.frame_count(), 48_000 * 2);
    let max_peak = buf.samples.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
    assert!(max_peak > 0.40);
    assert!(max_peak < 1.0);
}

#[test]
fn test_groove_afrobeats_amapiano_log_drum_108_bpm() {
    let buf = UniversalDrumSynthesizer::generate_groove_rhythm(RhythmGroove::AmapianoLogDrum, 108, 2.0);
    for s in &buf.samples {
        assert!(s.abs() < 1.0);
    }
}

#[test]
fn test_groove_metal_double_kick_180_bpm() {
    let buf = UniversalDrumSynthesizer::generate_groove_rhythm(RhythmGroove::MetalDoubleKick, 180, 1.0);
    assert_eq!(buf.frame_count(), 48_000);
    for s in &buf.samples {
        assert!(s.abs() < 1.0);
    }
}

#[test]
fn test_groove_jazz_swing_triplet_ride_140_bpm() {
    let buf = UniversalDrumSynthesizer::generate_groove_rhythm(RhythmGroove::JazzSwingRide, 140, 2.0);
    for s in &buf.samples {
        assert!(s.abs() < 1.0);
    }
}

#[test]
fn test_groove_orchestral_timpani_84_bpm() {
    let buf = UniversalDrumSynthesizer::generate_groove_rhythm(RhythmGroove::CinematicPercussion, 84, 3.0);
    for s in &buf.samples {
        assert!(s.abs() < 1.0);
    }
}

#[test]
fn test_groove_bpm_clamping_extreme_boundaries() {
    let buf_low = generate_groove("Roots Reggae", 25, 1.0);
    assert_eq!(buf_low.frame_count(), 48_000);

    let buf_high = generate_groove("Pop", 350, 1.0);
    assert_eq!(buf_high.frame_count(), 48_000);
}

// =========================================================================
// 4. Karplus-Strong Guitar & Wah-Wah Tests
// =========================================================================

#[test]
fn test_karplus_strong_pluck_and_decay() {
    let mut string = KarplusStrongString::new(48_000);
    string.pluck(220.0, 0.9); // A3

    let mut samples = Vec::with_capacity(48_000);
    for _ in 0..48_000 {
        samples.push(string.tick());
    }

    let early_energy: f32 = samples[0..1000].iter().map(|s| s.abs()).sum();
    let late_energy: f32 = samples[47_000..48_000].iter().map(|s| s.abs()).sum();
    assert!(early_energy > late_energy * 2.0, "String did not decay properly");

    for s in &samples {
        assert!(s.abs() < 1.0);
    }
}

#[test]
fn test_karplus_strong_pitch_tuning() {
    let mut string_a2 = KarplusStrongString::new(48_000);
    let mut string_a4 = KarplusStrongString::new(48_000);
    string_a2.pluck(110.0, 0.8);
    string_a4.pluck(440.0, 0.8);

    assert!(string_a2.buffer_len > string_a4.buffer_len);
}

#[test]
fn test_wah_wah_filter_stability_under_modulation() {
    let mut wah = WahWahFilter::new(48_000);
    let mut max_val = 0.0f32;

    for i in 0..48_000 {
        let t = i as f32 / 48_000.0;
        let test_signal = (2.0 * PI * 600.0 * t).sin() * 0.5;
        wah.auto_modulate(t, test_signal);
        let out = wah.process(test_signal);
        max_val = max_val.max(out.abs());
    }

    assert!(max_val < 3.0);
    assert!(max_val > 0.1);
}

#[test]
fn test_guitar_amp_overdrive_soft_clipping() {
    let mut amp = GuitarAmpSimulator::new(4.0);
    let loud_input = 2.5f32;
    let out = amp.process(loud_input);
    assert!(out.abs() <= 1.0, "Amp did not soft clip");
}

#[test]
fn test_soaring_16_bar_solo_synthesis() {
    let solo = synthesize_lead_solo(16, 76, "A Major");
    let expected_duration: f32 = (16.0 * 4.0 * 60.0) / 76.0;
    let expected_frames = (expected_duration * 48_000.0f32).round() as usize;

    assert_eq!(solo.sample_rate, 48_000);
    assert_eq!(solo.channels, 2);
    assert_eq!(solo.frame_count(), expected_frames);

    for s in &solo.samples {
        assert!(s.abs() <= 0.95, "Solo clipped: {}", s);
    }

    let left_sum: f32 = solo.samples.iter().step_by(2).map(|s| s.abs()).sum();
    let right_sum: f32 = solo.samples.iter().skip(1).step_by(2).map(|s| s.abs()).sum();
    assert!(left_sum > 100.0);
    assert!(right_sum > 100.0);
}

// =========================================================================
// 5. Vocal Formant & Rasp Tests
// =========================================================================

#[test]
fn test_vocal_formant_bank_instantiation_across_archetypes() {
    let archetypes = [
        VocalArchetype::JamaicanTenor,
        VocalArchetype::SoulfulRnbSoprano,
        VocalArchetype::GrittyRockBaritone,
        VocalArchetype::WarmAcousticCrooner,
        VocalArchetype::OperaticChoral,
        VocalArchetype::RapCadence,
    ];

    for arch in archetypes {
        let mut bank = FormantFilterBank::new(arch, 2);
        let out = bank.process_sample(0, 0.5);
        assert!(!out.is_nan());
        assert!(!out.is_infinite());
    }
}

#[test]
fn test_biquad_peaking_filter_resonance_and_stability() {
    let mut biquad = BiquadFilter::peaking_eq(500.0, 5.0, 6.0, 2);
    let mut max_abs = 0.0f32;
    for i in 0..1000 {
        let input = (2.0 * PI * 500.0 * (i as f32 / 48000.0)).sin();
        let output = biquad.process_sample(0, input);
        assert!(!output.is_nan());
        if output.abs() > max_abs {
            max_abs = output.abs();
        }
    }
    assert!(max_abs > 1.2, "Resonant filter must amplify center frequency");
}

#[test]
fn test_asymmetric_rasp_generates_harmonics() {
    let clean = 0.5f32;
    let raspy = VocalRaspProcessor::apply_rasp(clean, 0.82);
    assert!(raspy > clean, "Quadratic asymmetry should increase positive amplitude");
    let raspy_zero = VocalRaspProcessor::apply_rasp(clean, 0.0);
    assert_eq!(raspy_zero, clean, "Zero rasp must act as clean bypass");
}

#[test]
fn test_vocal_rasp_bounds_preservation() {
    let mut buf = AudioBuffer::new_stereo(0.05);
    for s in &mut buf.samples {
        *s = 0.85;
    }
    let processed = apply_vocal_timbre(&buf, 0.90, 0.0);
    for &s in &processed.samples {
        assert!(s.abs() <= 0.98, "Processed vocal sample exceeded 0.98 limit: {}", s);
    }
}

#[test]
fn test_pitch_lock_vocoder_phase_alignment() {
    let mut buf = AudioBuffer::new_stereo(0.2);
    buf.samples[500 * 2] = 0.9;
    buf.samples[500 * 2 + 1] = 0.9;
    let aligned = PitchLockVocoder::align_and_pitch_lock(&buf, 76, 220.0);
    assert_eq!(aligned.frame_count(), buf.frame_count());
}

// =========================================================================
// 6. Sample-Accurate Equal-Power Crossfader Tests
// =========================================================================

#[test]
fn test_equal_power_sine_curve_constant_energy() {
    for step in 0..=100 {
        let t = step as f32 / 100.0;
        let (w1, w2) = CrossfadeCurve::EqualPowerSine.compute_weights(t);
        let total_power = w1 * w1 + w2 * w2;
        assert!(
            (total_power - 1.0).abs() < 1e-4,
            "Sine/Cosine power sum must equal 1.0 at t={}, got {}",
            t,
            total_power
        );
    }
}

#[test]
fn test_equal_power_sqrt_curve_constant_energy() {
    for step in 0..=100 {
        let t = step as f32 / 100.0;
        let (w1, w2) = CrossfadeCurve::EqualPowerSqrt.compute_weights(t);
        let total_power = w1 * w1 + w2 * w2;
        assert!(
            (total_power - 1.0).abs() < 1e-4,
            "Sqrt power sum must equal 1.0 at t={}, got {}",
            t,
            total_power
        );
    }
}

#[test]
fn test_stem_inpainting_splice_duration_and_continuity() {
    let mut base = AudioBuffer::new_stereo(1.0);
    let mut solo = AudioBuffer::new_stereo(0.4);
    for s in &mut base.samples { *s = 0.3; }
    for s in &mut solo.samples { *s = 0.7; }

    let spliced = StemCrossfader::inpaint_splice(
        &base,
        &solo,
        4800,  // start at 100 ms
        480,   // 10 ms crossfade
        CrossfadeCurve::EqualPowerSine,
    );

    assert_eq!(spliced.frame_count(), base.frame_count());
    assert_eq!(spliced.samples[0], 0.3);
    assert_eq!(spliced.samples[(4800 + 1000) * 2], 0.7);
}

#[test]
fn test_inpaint_bars_boundary_calculation() {
    let base = AudioBuffer::new_stereo(5.0);
    let solo = AudioBuffer::new_stereo(1.0);
    let spliced = StemCrossfader::inpaint_bars(
        &base,
        &solo,
        2,
        76,
        4,
        15.0,
        CrossfadeCurve::EqualPowerSine,
    );
    assert_eq!(spliced.frame_count(), base.frame_count());
}

#[test]
fn test_equal_power_crossfade_out_of_bounds_protection() {
    let base = AudioBuffer::new_stereo(0.1);
    let splice = AudioBuffer::new_stereo(0.05);
    let res = equal_power_crossfade(&base, &splice, 999999, 100);
    assert_eq!(res.frame_count(), base.frame_count());
}

// =========================================================================
// 7. Analog Tape & Sub-Bass Mastering Tests
// =========================================================================

#[test]
fn test_tape_saturation_soft_compression() {
    let low = AnalogTapeSaturator::process_sample(0.1, 1.25, 0.0);
    let high = AnalogTapeSaturator::process_sample(3.0, 1.25, 0.0);
    assert!(low.abs() < 0.2);
    assert!(high.abs() <= 1.0, "Extreme input must be softly saturated");
}

#[test]
fn test_brickwall_limiter_strictly_enforces_ceiling() {
    let mut limiter = BrickwallLimiter::new(24, BRICKWALL_CEILING);
    for _ in 0..500 {
        let (out_l, out_r) = limiter.process_frame(3.5, 4.0); // +12 dBFS over
        assert!(out_l.abs() <= BRICKWALL_CEILING + 1e-5);
        assert!(out_r.abs() <= BRICKWALL_CEILING + 1e-5);
    }
}

#[test]
fn test_mastering_profiles_execute_cleanly() {
    let profiles = [
        MasteringProfile::StuderA800Tape1970s,
        MasteringProfile::ModernPunchyPop,
        MasteringProfile::ClubEdmSubHeavy,
        MasteringProfile::CinematicOrchestral,
        MasteringProfile::AudiophileJazz,
    ];

    let mut buf = AudioBuffer::new_stereo(0.2);
    for s in &mut buf.samples { *s = 0.5; }

    for prof in profiles {
        let master = apply_analog_mastering(&buf, prof);
        assert_eq!(master.frame_count(), buf.frame_count());
        for &s in &master.samples {
            assert!(s.abs() <= BRICKWALL_CEILING + 1e-4);
        }
    }
}

#[test]
fn test_tape_mastering_extreme_burst_headroom() {
    let mut buf = AudioBuffer::new_stereo(0.1);
    for s in &mut buf.samples { *s = 5.0; } // Extreme overs
    let master = apply_analog_tape_mastering(&buf, 3.2);
    for &s in &master.samples {
        assert!(s.abs() <= 0.966, "Mastered sample exceeded -0.3 dBFS ceiling: {}", s);
    }
}
