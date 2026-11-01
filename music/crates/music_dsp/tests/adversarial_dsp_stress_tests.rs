//! crates/music_dsp/tests/adversarial_dsp_stress_tests.rs
//!
//! Adversarial Empirical Stress Harness for pure-Rust DSP Engine (`music_dsp`).
//! Challenges numeric stability, extreme inputs, boundary conditions,
//! and edge-case resilience across all DSP components.

use music_dsp::wav::{AudioBuffer, WavBitDepth};
use music_dsp::telemetry_calc::{
    calculate_phase_alignment, calculate_snr, calculate_spectral_flatness,
    calculate_telemetry, calculate_tempo_drift, fft_radix2,
};
use music_dsp::groove_synth::{
    generate_groove, DrumHit, DrumVoice, UniversalDrumSynthesizer,
};
use music_dsp::guitar_wah::{
    synthesize_guitar_solo, synthesize_lead_solo,
    KarplusStrongString,
};
use music_dsp::vocal_formant::{
    apply_vocal_formant_and_rasp, apply_vocal_timbre, BiquadFilter,
    PitchLockVocoder,
};
use music_dsp::crossfader::{
    equal_power_crossfade, splice_stem, CrossfadeCurve, StemCrossfader,
};
use music_dsp::tape_mastering::{
    apply_analog_mastering, apply_analog_tape_mastering, AnalogTapeSaturator,
    BrickwallLimiter, MasteringProfile, BRICKWALL_CEILING,
};

// =============================================================================
// Group 1: Extreme BPM & Timing Grid Boundaries
// =============================================================================

#[test]
fn test_adversarial_bpm_zero_in_groove_synth() {
    let buf = generate_groove("reggae", 0, 1.0);
    assert_eq!(buf.sample_rate, 48_000);
    assert_eq!(buf.channels, 2);
    assert_eq!(buf.frame_count(), 48_000);
}

#[test]
fn test_adversarial_bpm_extreme_high_in_groove_synth() {
    let buf = generate_groove("drum_and_bass", 10_000, 1.0);
    assert_eq!(buf.frame_count(), 48_000);
    assert!(!buf.is_empty());
}

#[test]
fn test_adversarial_bpm_zero_in_guitar_solo() {
    let buf = synthesize_lead_solo(4, 0, "A Major");
    assert_eq!(buf.channels, 2);
    assert!(buf.frame_count() > 0);

    let buf2 = synthesize_guitar_solo(2, 0, 440.0);
    assert_eq!(buf2.channels, 2);
    assert!(buf2.frame_count() > 0);
}

#[test]
fn test_adversarial_bpm_extreme_in_guitar_solo() {
    let buf = synthesize_lead_solo(2, 5000, "Blues");
    assert_eq!(buf.channels, 2);
    assert!(buf.frame_count() > 0);
}

#[test]
fn test_adversarial_tempo_drift_zero_and_extreme_bpm() {
    let silent = AudioBuffer::new_stereo(2.0);
    let drift_zero = calculate_tempo_drift(&silent, 0);
    assert!(drift_zero >= 0.0 && drift_zero <= 1.0);

    let drift_huge = calculate_tempo_drift(&silent, 50_000);
    assert!(drift_huge >= 0.0 && drift_huge <= 1.0);
}

#[test]
fn test_adversarial_vocoder_zero_bpm() {
    let buf = AudioBuffer::new_stereo(1.0);
    let aligned = PitchLockVocoder::align_and_pitch_lock(&buf, 0, 440.0);
    assert_eq!(aligned.frame_count(), buf.frame_count());
}

#[test]
fn test_adversarial_crossfader_zero_bpm_multiplication_overflow() {
    let base = AudioBuffer::new_stereo(2.0);
    let splice = AudioBuffer::new_stereo(0.5);

    // Remediated: inpaint_bars clamps 0 BPM to [40, 220] safely without panic
    let result = std::panic::catch_unwind(|| {
        StemCrossfader::inpaint_bars(&base, &splice, 0, 0, 4, 15.0, CrossfadeCurve::EqualPowerSine)
    });
    assert!(
        result.is_ok(),
        "StemCrossfader::inpaint_bars should handle 0 BPM gracefully without panicking"
    );
    let out = result.unwrap();
    assert_eq!(out.channels, 2);
    assert_eq!(out.frame_count(), base.frame_count());
    assert!(!out.samples.iter().any(|s| s.is_nan() || s.is_infinite()));
}

#[test]
fn test_adversarial_splice_stem_zero_bpm() {
    let base = AudioBuffer::new_stereo(2.0);
    let splice = AudioBuffer::new_stereo(0.5);

    // Remediated: splice_stem delegates to inpaint_bars, gracefully handling 0 BPM without panic
    let result = std::panic::catch_unwind(|| {
        splice_stem(&base, &splice, 0, 0, 4)
    });
    assert!(
        result.is_ok(),
        "splice_stem should handle 0 BPM gracefully without panicking"
    );
    let out = result.unwrap();
    assert_eq!(out.channels, 2);
    assert_eq!(out.frame_count(), base.frame_count());
    assert!(!out.samples.iter().any(|s| s.is_nan() || s.is_infinite()));
}

// =============================================================================
// Group 2: Zero-Length Buffers & Boundary Sizes
// =============================================================================

#[test]
fn test_adversarial_zero_duration_buffers() {
    let buf = AudioBuffer::new_stereo(0.0);
    assert_eq!(buf.frame_count(), 0);
    assert_eq!(buf.duration_secs(), 0.0);
    assert!(buf.is_empty());
    assert_eq!(buf.peak_level(), 0.0);
    assert_eq!(buf.rms_level(), 0.0);

    let mut empty = buf.clone();
    let factor = empty.normalize(-0.3);
    assert_eq!(factor, 1.0);

    let sliced = buf.slice(0, 100);
    assert!(sliced.is_empty());

    let mono = buf.to_mono();
    assert_eq!(mono.channels, 1);
    assert!(mono.is_empty());

    let stereo = mono.to_stereo();
    assert_eq!(stereo.channels, 2);
    assert!(stereo.is_empty());
}

#[test]
fn test_adversarial_zero_duration_dsp_pipelines() {
    let empty = AudioBuffer::new_stereo(0.0);

    let vocal_out = apply_vocal_timbre(&empty, 0.8, 0.0);
    assert!(vocal_out.is_empty());

    let master_out = apply_analog_mastering(&empty, MasteringProfile::StuderA800Tape1970s);
    assert!(master_out.is_empty());

    let telem = calculate_telemetry(&empty, 120);
    assert_eq!(telem.duration_secs, 0.0);
    assert_eq!(telem.peak_dbfs, -90.0);
    assert_eq!(telem.rms_dbfs, -90.0);
    assert!(!telem.clipping);
    assert!(telem.snr_db >= 20.0);

    let xfade = StemCrossfader::inpaint_splice(&empty, &empty, 0, 100, CrossfadeCurve::Linear);
    assert!(xfade.is_empty());

    let eq_xfade = equal_power_crossfade(&empty, &empty, 0, 100);
    assert!(eq_xfade.is_empty());
}

#[test]
fn test_adversarial_resample_zero_and_empty() {
    let empty = AudioBuffer::new_stereo(0.0);
    let resampled = empty.resample_linear(44_100);
    assert!(resampled.is_empty());

    let non_empty = AudioBuffer::new_stereo(0.1);
    let resampled_zero = std::panic::catch_unwind(|| {
        non_empty.resample_linear(0)
    });
    assert!(resampled_zero.is_ok(), "resample_linear(0) panicked!");
}

// =============================================================================
// Group 3: Channel Mismatch & Mixing Resilience
// =============================================================================

#[test]
fn test_adversarial_channel_mismatches_mix() {
    let mut stereo = AudioBuffer::new_stereo(1.0);
    stereo.samples[0] = 0.5;
    stereo.samples[1] = 0.5;

    let mut mono = AudioBuffer::new_mono(1.0);
    mono.samples[0] = 0.3;

    assert!(stereo.mix(&mono, 1.0).is_ok());
    assert!((stereo.samples[0] - 0.8).abs() < 1e-5);
    assert!((stereo.samples[1] - 0.8).abs() < 1e-5);

    let mut mono_target = AudioBuffer::new_mono(1.0);
    assert!(mono_target.mix(&stereo, 1.0).is_ok());
    assert!((mono_target.samples[0] - 0.8).abs() < 1e-5);

    let quad = AudioBuffer {
        sample_rate: 48000,
        channels: 4,
        samples: vec![0.1; 400],
    };
    assert!(stereo.mix(&quad, 1.0).is_err());
}

#[test]
fn test_adversarial_crossfade_mono_base_stereo_splice() {
    let mono_base = AudioBuffer::new_mono(1.0);
    let stereo_splice = AudioBuffer::new_stereo(0.5);

    let out = StemCrossfader::inpaint_splice(&mono_base, &stereo_splice, 100, 50, CrossfadeCurve::EqualPowerSine);
    assert_eq!(out.channels, 1);
    assert_eq!(out.frame_count(), mono_base.frame_count());
}

#[test]
fn test_adversarial_crossfade_stereo_base_mono_splice() {
    let stereo_base = AudioBuffer::new_stereo(1.0);
    let mono_splice = AudioBuffer::new_mono(0.5);

    let out = StemCrossfader::inpaint_splice(&stereo_base, &mono_splice, 100, 50, CrossfadeCurve::EqualPowerSine);
    assert_eq!(out.channels, 2);
    assert_eq!(out.frame_count(), stereo_base.frame_count());
}

#[test]
fn test_adversarial_mono_mastering_and_formant() {
    let mono = AudioBuffer::new_mono(0.5);

    let mastered = apply_analog_mastering(&mono, MasteringProfile::StuderA800Tape1970s);
    assert_eq!(mastered.channels, 1);
    assert_eq!(mastered.frame_count(), mono.frame_count());

    let mastered2 = apply_analog_tape_mastering(&mono, 3.0);
    assert_eq!(mastered2.channels, 1);
    assert_eq!(mastered2.frame_count(), mono.frame_count());

    let vocal = apply_vocal_timbre(&mono, 0.5, 0.0);
    assert_eq!(vocal.channels, 1);
    assert_eq!(vocal.frame_count(), mono.frame_count());

    let vocal2 = apply_vocal_formant_and_rasp(&mono, 0.5);
    assert_eq!(vocal2.channels, 1);
    assert_eq!(vocal2.frame_count(), mono.frame_count());
}

#[test]
fn test_adversarial_render_hits_mono_buffer_panic() {
    let mut mono_buf = AudioBuffer::new_mono(1.0);
    let hits = vec![DrumHit {
        time_secs: 0.8,
        voice: DrumVoice::KickAcoustic,
        velocity: 0.8,
        pan: 0.0,
    }];

    // Remediated: render_hits supports mono buffers safely without index panic
    let result = std::panic::catch_unwind(move || {
        UniversalDrumSynthesizer::render_hits(&hits, &mut mono_buf);
        mono_buf
    });
    assert!(
        result.is_ok(),
        "UniversalDrumSynthesizer::render_hits should handle mono buffers without panicking"
    );
    let rendered = result.unwrap();
    assert_eq!(rendered.channels, 1);
    assert_eq!(rendered.frame_count(), 48_000);
    assert!(rendered.peak_level() > 0.0, "Drum hit must be audible in rendered buffer");
    assert!(!rendered.samples.iter().any(|s| s.is_nan() || s.is_infinite()));
}

// =============================================================================
// Group 4: Numeric Stability: NaN, Inf, and Subnormal Protection
// =============================================================================

#[test]
fn test_adversarial_fft_radix2_stability() {
    let mut real = vec![0.0f32; 1024];
    let mut imag = vec![0.0f32; 1024];
    real[0] = 1.0;
    fft_radix2(&mut real, &mut imag);

    for i in 0..1024 {
        let mag = (real[i] * real[i] + imag[i] * imag[i]).sqrt();
        assert!((mag - 1.0).abs() < 1e-4);
    }
}

#[test]
fn test_adversarial_spectral_flatness_silence_and_dc() {
    let silence = vec![0.0f32; 2048];
    let sfm_silence = calculate_spectral_flatness(&silence);
    assert!(sfm_silence >= 0.0 && sfm_silence <= 1.0);
    assert!(!sfm_silence.is_nan());
    assert!(!sfm_silence.is_infinite());

    let dc = vec![0.8f32; 2048];
    let sfm_dc = calculate_spectral_flatness(&dc);
    assert!(sfm_dc >= 0.0 && sfm_dc <= 1.0);
    assert!(!sfm_dc.is_nan());
}

#[test]
fn test_adversarial_spectral_flatness_with_nan_or_inf() {
    let mut dirty = vec![0.1f32; 2048];
    dirty[50] = f32::NAN;
    dirty[100] = f32::INFINITY;
    dirty[150] = f32::NEG_INFINITY;

    let sfm = calculate_spectral_flatness(&dirty);
    assert!(sfm >= 0.0 && sfm <= 1.0, "SFM returned out of bounds or NaN: {}", sfm);
    assert!(!sfm.is_nan());
}

#[test]
fn test_adversarial_phase_alignment_nan_propagation() {
    let mut buf = AudioBuffer::new_stereo(0.1);
    buf.samples[10] = f32::NAN;

    // Remediated: calculate_phase_alignment guards against NaN denom/rho,
    // returning safe neutral score 1.0 instead of propagating NaN.
    let score = calculate_phase_alignment(&buf);
    assert!(
        !score.is_nan(),
        "Remediated: calculate_phase_alignment must NOT propagate NaN when input contains NaN"
    );
    assert_eq!(score, 1.0);
}

#[test]
fn test_adversarial_snr_with_nan() {
    let mut buf = AudioBuffer::new_stereo(0.2);
    buf.samples[10] = f32::NAN;
    let snr = calculate_snr(&buf);
    assert!(!snr.is_nan(), "calculate_snr should clamp to valid range");
}

#[test]
fn test_adversarial_biquad_filter_subnormals_and_denormals() {
    let mut filter = BiquadFilter::peaking_eq(1000.0, 2.0, 6.0, 1);
    let denorm = 1e-42f32;
    for _ in 0..1000 {
        let out = filter.process_sample(0, denorm);
        assert!(!out.is_nan());
        assert!(!out.is_infinite());
    }
}

#[test]
fn test_adversarial_tape_saturator_nan_and_inf() {
    let pos_inf_out = AnalogTapeSaturator::process_sample(f32::INFINITY, 1.2, 0.05);
    assert!(!pos_inf_out.is_nan());
    assert!(!pos_inf_out.is_infinite());
    assert!(pos_inf_out <= 0.95);

    let neg_inf_out = AnalogTapeSaturator::process_sample(f32::NEG_INFINITY, 1.2, 0.05);
    assert!(!neg_inf_out.is_nan());
    assert!(!neg_inf_out.is_infinite());
    assert!(neg_inf_out >= -0.95);
}

#[test]
fn test_adversarial_brickwall_limiter_extreme_burst() {
    let mut limiter = BrickwallLimiter::new(24, BRICKWALL_CEILING);

    for _ in 0..100 {
        let (out_l, out_r) = limiter.process_frame(100_000.0, -100_000.0);
        assert!(out_l.abs() <= BRICKWALL_CEILING + 1e-5);
        assert!(out_r.abs() <= BRICKWALL_CEILING + 1e-5);
        assert!(!out_l.is_nan());
        assert!(!out_r.is_nan());
    }
}

#[test]
fn test_adversarial_karplus_strong_extreme_pitches() {
    let mut string = KarplusStrongString::new(48_000);

    string.pluck(10.0, 1.0);
    for _ in 0..500 {
        let s = string.tick();
        assert!(!s.is_nan());
        assert!(!s.is_infinite());
    }

    string.pluck(30_000.0, 1.0);
    for _ in 0..500 {
        let s = string.tick();
        assert!(!s.is_nan());
        assert!(!s.is_infinite());
    }
}

// =============================================================================
// Group 5: WAV Serialization & Malformed Byte Edge Cases
// =============================================================================

#[test]
fn test_adversarial_wav_roundtrip_all_formats_extreme_values() {
    let mut buf = AudioBuffer::new_stereo(0.05);
    for i in 0..buf.samples.len() {
        buf.samples[i] = if i % 4 == 0 {
            1.0
        } else if i % 4 == 1 {
            -1.0
        } else if i % 4 == 2 {
            0.0
        } else {
            0.99999
        };
    }

    for depth in [WavBitDepth::Pcm16, WavBitDepth::Pcm24, WavBitDepth::Float32] {
        let bytes = buf.to_wav_bytes_with_format(depth);
        let decoded = AudioBuffer::from_wav_bytes(&bytes).expect("Failed to decode valid WAV bytes");
        assert_eq!(decoded.sample_rate, buf.sample_rate);
        assert_eq!(decoded.channels, buf.channels);
        assert_eq!(decoded.frame_count(), buf.frame_count());

        let tolerance = match depth {
            WavBitDepth::Pcm16 => 1.0 / 32767.0 + 1e-4,
            WavBitDepth::Pcm24 => 1.0 / 8388607.0 + 1e-6,
            WavBitDepth::Float32 => 1e-7,
        };

        for (orig, dec) in buf.samples.iter().zip(decoded.samples.iter()) {
            assert!((orig - dec).abs() <= tolerance, "Diff {} exceeds tolerance {}", (orig - dec).abs(), tolerance);
        }
    }
}

#[test]
fn test_adversarial_wav_corrupt_headers_rejected() {
    assert!(AudioBuffer::from_wav_bytes(&[0u8; 20]).is_err());

    let mut valid = AudioBuffer::new_stereo(0.01).to_wav_bytes();
    valid[0] = b'N';
    valid[1] = b'O';
    assert!(AudioBuffer::from_wav_bytes(&valid).is_err());

    let mut valid2 = AudioBuffer::new_stereo(0.01).to_wav_bytes();
    valid2[8] = b'B';
    valid2[9] = b'A';
    valid2[10] = b'D';
    assert!(AudioBuffer::from_wav_bytes(&valid2).is_err());
}

#[test]
fn test_adversarial_wav_corrupt_chunk_sizes() {
    let mut valid = AudioBuffer::new_stereo(0.01).to_wav_bytes();
    if let Some(pos) = valid.windows(4).position(|w| w == b"data") {
        let huge = (u32::MAX - 10).to_le_bytes();
        valid[pos + 4..pos + 8].copy_from_slice(&huge);
        let result = AudioBuffer::from_wav_bytes(&valid);
        assert!(result.is_ok() || result.is_err());
    }
}

// =============================================================================
// Group 6: Sample Rate = 0 Division-by-Zero Checks
// =============================================================================

#[test]
fn test_adversarial_sample_rate_zero_telemetry_division_by_zero() {
    let zero_sr_buf = AudioBuffer {
        sample_rate: 0,
        channels: 2,
        samples: vec![0.1; 100],
    };

    // Remediated: calculate_snr guards against sample_rate == 0 and returns fallback default (92.0)
    let result_snr = std::panic::catch_unwind(|| {
        calculate_snr(&zero_sr_buf)
    });
    assert!(
        result_snr.is_ok(),
        "Remediated: calculate_snr must NOT panic when sample_rate == 0"
    );
    assert_eq!(result_snr.unwrap(), 92.0);

    // Remediated: calculate_tempo_drift guards against sample_rate == 0 and returns fallback default (0.02)
    let result_drift = std::panic::catch_unwind(|| {
        calculate_tempo_drift(&zero_sr_buf, 120)
    });
    assert!(
        result_drift.is_ok(),
        "Remediated: calculate_tempo_drift must NOT panic when sample_rate == 0"
    );
    assert_eq!(result_drift.unwrap(), 0.02);
}
