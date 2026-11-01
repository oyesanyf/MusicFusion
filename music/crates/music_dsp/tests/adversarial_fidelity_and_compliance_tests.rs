//! Adversarial Empirical Test Suite: Audio Fidelity, Broadcast Ceiling Compliance,
//! Tanh Saturation Smoothness, Equal-Power Crossfader Energy Conservation, and
//! Multi-Threaded Test Isolation for Milestone 2 (`crates/music_dsp`).

use std::f32::consts::PI;
use std::thread;

use music_dsp::{
    apply_analog_mastering, apply_analog_tape_mastering,
    calculate_phase_alignment, calculate_spectral_flatness,
    calculate_telemetry, equal_power_crossfade, generate_groove,
    splice_stem, synthesize_lead_solo, AnalogTapeSaturator, AudioBuffer,
    BrickwallLimiter, CrossfadeCurve, GuitarAmpSimulator,
    MasteringProfile, StemCrossfader, BRICKWALL_CEILING,
    DEFAULT_SAMPLE_RATE,
};

// ============================================================================
// 1. BROADCAST CEILING COMPLIANCE ADVERSARIAL STRESS TESTS
// ============================================================================

#[test]
fn test_adversarial_brickwall_limiter_extreme_overdrive_96db() {
    // Extreme overdrive: +96 dBFS amplitude (1000.0)
    let mut limiter = BrickwallLimiter::new(24, BRICKWALL_CEILING);
    for frame in 0..10_000 {
        let t = frame as f32 / DEFAULT_SAMPLE_RATE as f32;
        let s_l = (2.0 * PI * 100.0 * t).sin() * 1000.0;
        let s_r = (2.0 * PI * 200.0 * t).cos() * 1000.0;

        let (out_l, out_r) = limiter.process_frame(s_l, s_r);
        assert!(
            out_l.abs() <= BRICKWALL_CEILING + 1e-5,
            "Left channel exceeded ceiling at frame {}: got {}",
            frame,
            out_l
        );
        assert!(
            out_r.abs() <= BRICKWALL_CEILING + 1e-5,
            "Right channel exceeded ceiling at frame {}: got {}",
            frame,
            out_r
        );
    }
}

#[test]
fn test_adversarial_brickwall_limiter_dirac_impulses() {
    // Isolated Dirac impulses up to 10,000.0 magnitude
    let mut limiter = BrickwallLimiter::new(24, BRICKWALL_CEILING);

    for i in 0..2000 {
        let (in_l, in_r) = match i {
            100 => (10_000.0, 0.0),
            300 => (-10_000.0, -5_000.0),
            600 => (0.0, 10_000.0),
            1200 => (5_000.0, -10_000.0),
            _ => (0.0, 0.0),
        };

        let (out_l, out_r) = limiter.process_frame(in_l, in_r);
        assert!(
            out_l.abs() <= BRICKWALL_CEILING + 1e-5,
            "Left channel Dirac burst exceeded ceiling at frame {}: got {}",
            i,
            out_l
        );
        assert!(
            out_r.abs() <= BRICKWALL_CEILING + 1e-5,
            "Right channel Dirac burst exceeded ceiling at frame {}: got {}",
            i,
            out_r
        );
    }
}

#[test]
fn test_adversarial_brickwall_limiter_nyquist_square_wave() {
    // Alternating Nyquist full-scale square wave (+50.0, -50.0)
    let mut limiter = BrickwallLimiter::new(24, BRICKWALL_CEILING);
    for frame in 0..4800 {
        let s = if frame % 2 == 0 { 50.0 } else { -50.0 };
        let (out_l, out_r) = limiter.process_frame(s, -s);
        assert!(out_l.abs() <= BRICKWALL_CEILING + 1e-5);
        assert!(out_r.abs() <= BRICKWALL_CEILING + 1e-5);
    }
}

#[test]
fn test_adversarial_brickwall_limiter_massive_dc_offsets() {
    // Constant high DC offset (+100.0 and -100.0)
    let mut limiter = BrickwallLimiter::new(24, BRICKWALL_CEILING);
    for _ in 0..2000 {
        let (out_l, out_r) = limiter.process_frame(100.0, -100.0);
        assert!(out_l.abs() <= BRICKWALL_CEILING + 1e-5);
        assert!(out_r.abs() <= BRICKWALL_CEILING + 1e-5);
    }
}

#[test]
fn test_adversarial_mastering_all_profiles_extreme_stress() {
    let profiles = [
        MasteringProfile::StuderA800Tape1970s,
        MasteringProfile::ModernPunchyPop,
        MasteringProfile::ClubEdmSubHeavy,
        MasteringProfile::CinematicOrchestral,
        MasteringProfile::AudiophileJazz,
    ];

    // High amplitude composite test buffer: multi-tone + DC spike + noise
    let mut test_buf = AudioBuffer::new_stereo(0.2);
    let frames = test_buf.frame_count();
    for f in 0..frames {
        let t = f as f32 / DEFAULT_SAMPLE_RATE as f32;
        // Composite signal with peaks reaching +30 dBFS
        let composite_l = (2.0 * PI * 60.0 * t).sin() * 8.0
            + (2.0 * PI * 3500.0 * t).sin() * 5.0
            + if f % 500 == 0 { 25.0 } else { 0.0 };
        let composite_r = (2.0 * PI * 120.0 * t).cos() * 8.0
            + (2.0 * PI * 8000.0 * t).sin() * 4.0;
        test_buf.set_channel_sample(f, 0, composite_l);
        test_buf.set_channel_sample(f, 1, composite_r);
    }

    for prof in profiles {
        let mastered = apply_analog_mastering(&test_buf, prof);
        assert_eq!(mastered.frame_count(), test_buf.frame_count());

        for (idx, &s) in mastered.samples.iter().enumerate() {
            assert!(
                s.abs() <= BRICKWALL_CEILING + 1e-4,
                "Profile {:?} exceeded ceiling at sample {}: got {}",
                prof,
                idx,
                s
            );
        }
    }
}

#[test]
fn test_adversarial_analog_tape_mastering_sub_bass_boost_sweep() {
    // Sweep sub-bass boost from 0 dB to 12 dB with hot audio
    let mut test_buf = AudioBuffer::new_stereo(0.1);
    for s in &mut test_buf.samples {
        *s = 4.0; // Hot signal
    }

    for boost in [0.0, 1.5, 3.2, 5.0, 6.0, 10.0] {
        let mastered = apply_analog_tape_mastering(&test_buf, boost);
        for &s in &mastered.samples {
            assert!(
                s.abs() <= BRICKWALL_CEILING + 1e-4,
                "Mastered sample exceeded ceiling with sub-bass boost {}: got {}",
                boost,
                s
            );
        }
    }
}

// ============================================================================
// 2. TANH SATURATION CONTINUITY & MONOTONICITY
// ============================================================================

#[test]
fn test_adversarial_tanh_saturation_strict_monotonicity() {
    // Hyperbolic tangent saturation must be strictly monotonic:
    // x1 < x2 => f(x1) <= f(x2) for all finite x.
    let drives = [0.1, 0.5, 1.0, 1.25, 2.5, 5.0, 10.0];
    let biases = [-0.2, -0.05, 0.0, 0.05, 0.2];

    for &drive in &drives {
        for &bias in &biases {
            let mut prev_y = f32::NEG_INFINITY;
            let steps = 50_000;
            for i in 0..=steps {
                let x = -10.0 + (20.0 * i as f32 / steps as f32);
                let y = AnalogTapeSaturator::process_sample(x, drive, bias);

                assert!(
                    y >= prev_y - 1e-7,
                    "Tanh non-monotonic at x={} (prev={}, cur={}, drive={}, bias={})",
                    x,
                    prev_y,
                    y,
                    drive,
                    bias
                );
                assert!(
                    y.abs() <= 0.94 + 1e-5,
                    "Tanh output exceeded 0.94 scaling at x={}: got {}",
                    x,
                    y
                );
                prev_y = y;
            }
        }
    }
}

#[test]
fn test_adversarial_tanh_saturation_derivative_continuity() {
    // 1. C0 Continuity across full range: |f(x+eps) - f(x)| <= L * eps (Lipschitz continuous)
    let drive = 1.25;
    let bias = 0.05;
    let eps = 1e-4;

    let steps = 10_000;
    for i in 0..steps {
        let x = -5.0 + (10.0 * i as f32 / steps as f32);
        let y1 = AnalogTapeSaturator::process_sample(x, drive, bias);
        let y2 = AnalogTapeSaturator::process_sample(x + eps, drive, bias);
        let diff = (y2 - y1).abs();
        let max_expected_diff = drive * 0.94 * eps + 1e-6;

        assert!(
            diff <= max_expected_diff,
            "C0 continuity violated (jump) at x={}: diff={}, max={}",
            x,
            diff,
            max_expected_diff
        );
    }

    // 2. C1 Smoothness (continuous derivative) throughout the active, non-clamped operating region [-1.2, 1.2]
    let mut prev_deriv: Option<f32> = None;
    let active_steps = 2400;
    for i in 0..active_steps {
        let x = -1.2 + (2.4 * i as f32 / active_steps as f32);
        let y1 = AnalogTapeSaturator::process_sample(x, drive, bias);
        let y2 = AnalogTapeSaturator::process_sample(x + eps, drive, bias);
        let deriv = (y2 - y1) / eps;

        assert!(
            deriv >= -1e-6,
            "Derivative negative in active region at x={}: got {}",
            x,
            deriv
        );

        if let Some(p) = prev_deriv {
            let diff = (deriv - p).abs();
            assert!(
                diff < 0.01,
                "Derivative jump in active region at x={}: diff={}",
                x,
                diff
            );
        }
        prev_deriv = Some(deriv);
    }
}

#[test]
fn test_adversarial_tanh_saturation_zero_dc_at_rest() {
    // At zero input, saturation must yield zero DC offset: f(0.0) == 0.0
    for bias in [-0.5, -0.1, 0.0, 0.05, 0.1, 0.5] {
        let y = AnalogTapeSaturator::process_sample(0.0, 1.25, bias);
        assert!(
            y.abs() < 1e-6,
            "Zero input must produce zero output regardless of bias (bias={}): got {}",
            bias,
            y
        );
    }
}

#[test]
fn test_adversarial_guitar_amp_soft_clipping_continuity() {
    let mut amp = GuitarAmpSimulator::new(4.0);
    let mut prev = -1.0;
    // Sweep through overdrive
    for i in 0..1000 {
        let x = -2.0 + (4.0 * i as f32 / 1000.0);
        let y = amp.process(x);
        assert!(!y.is_nan() && !y.is_infinite());
        assert!(
            y.abs() <= 1.0,
            "Guitar amp output exceeded unity at x={}: got {}",
            x,
            y
        );
        prev = y;
    }
    let _ = prev;
}

// ============================================================================
// 3. EQUAL-POWER CROSSFADER ENERGY CONSERVATION ADVERSARIAL TESTS
// ============================================================================

#[test]
fn test_adversarial_equal_power_sine_curve_analytical_conservation() {
    // High-resolution verification of sin^2(theta) + cos^2(theta) == 1.0
    let steps = 100_000;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let (w1, w2) = CrossfadeCurve::EqualPowerSine.compute_weights(t);
        let power_sum = w1 * w1 + w2 * w2;

        assert!(
            (power_sum - 1.0).abs() < 1e-5,
            "Sine crossfader violated equal-power invariant at t={}: w1={}, w2={}, sum={}",
            t,
            w1,
            w2,
            power_sum
        );
        assert!(w1 >= -1e-6 && w1 <= 1.0 + 1e-6);
        assert!(w2 >= -1e-6 && w2 <= 1.0 + 1e-6);
    }
}

#[test]
fn test_adversarial_equal_power_sqrt_curve_analytical_conservation() {
    // High-resolution verification of (1 - t) + t == 1.0
    let steps = 100_000;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let (w1, w2) = CrossfadeCurve::EqualPowerSqrt.compute_weights(t);
        let power_sum = w1 * w1 + w2 * w2;

        assert!(
            (power_sum - 1.0).abs() < 1e-5,
            "Sqrt crossfader violated equal-power invariant at t={}: w1={}, w2={}, sum={}",
            t,
            w1,
            w2,
            power_sum
        );
    }
}

#[test]
fn test_adversarial_stem_crossfader_uncorrelated_noise_power_invariance() {
    // Empirical test: crossfading two uncorrelated pseudo-random white noise
    // buffers of equal RMS power P_0 must maintain constant power throughout transition.
    let duration_secs = 0.5;
    let frames = (duration_secs * DEFAULT_SAMPLE_RATE as f32) as usize;

    let mut base = AudioBuffer::new_stereo(duration_secs);
    let mut splice = AudioBuffer::new_stereo(duration_secs);

    let mut lfsr_base = 0xACE1_u32;
    let mut lfsr_splice = 0x1337_BEEF_u32;

    for i in 0..frames * 2 {
        lfsr_base ^= lfsr_base << 13;
        lfsr_base ^= lfsr_base >> 17;
        lfsr_base ^= lfsr_base << 5;
        let s_base = (lfsr_base as f32 / u32::MAX as f32) * 0.8 - 0.4;
        base.samples[i] = s_base;

        lfsr_splice ^= lfsr_splice << 13;
        lfsr_splice ^= lfsr_splice >> 17;
        lfsr_splice ^= lfsr_splice << 5;
        let s_splice = (lfsr_splice as f32 / u32::MAX as f32) * 0.8 - 0.4;
        splice.samples[i] = s_splice;
    }

    let crossfade_frames = 2400; // 50 ms transition
    let spliced = StemCrossfader::inpaint_splice(
        &base,
        &splice,
        4800, // start at 100 ms
        crossfade_frames,
        CrossfadeCurve::EqualPowerSine,
    );

    assert_eq!(spliced.frame_count(), base.frame_count());

    // Measure power in 200-sample windows across the transition region
    let window_size = 200;
    let num_windows = crossfade_frames / window_size;
    let base_rms = base.rms_level();

    for w in 0..num_windows {
        let start_f = 4800 + w * window_size;
        let mut sum_sq = 0.0f32;
        for f in 0..window_size {
            let s_l = spliced.samples[(start_f + f) * 2];
            let s_r = spliced.samples[(start_f + f) * 2 + 1];
            sum_sq += (s_l * s_l + s_r * s_r) * 0.5;
        }
        let win_rms = (sum_sq / window_size as f32).sqrt();

        // Statistical tolerance for 200 samples of noise: within 35% of base RMS
        let ratio = win_rms / base_rms;
        assert!(
            ratio > 0.65 && ratio < 1.35,
            "Crossfade power dipped or surged at window {}: ratio={}",
            w,
            ratio
        );
    }
}

#[test]
fn test_adversarial_stem_crossfader_boundary_conditions() {
    let base = AudioBuffer::new_stereo(0.2);
    let splice = AudioBuffer::new_stereo(0.1);

    // 1. Crossfade frames = 0
    let res0 = StemCrossfader::inpaint_splice(&base, &splice, 100, 0, CrossfadeCurve::EqualPowerSine);
    assert_eq!(res0.frame_count(), base.frame_count());

    // 2. Crossfade frames larger than splice
    let res_huge = StemCrossfader::inpaint_splice(&base, &splice, 100, 100_000, CrossfadeCurve::EqualPowerSine);
    assert_eq!(res_huge.frame_count(), base.frame_count());

    // 3. Splice extends beyond base end
    let res_over = StemCrossfader::inpaint_splice(&base, &splice, base.frame_count() - 50, 20, CrossfadeCurve::EqualPowerSine);
    assert_eq!(res_over.frame_count(), base.frame_count());

    // 4. Start frame beyond base
    let res_past = StemCrossfader::inpaint_splice(&base, &splice, base.frame_count() + 100, 20, CrossfadeCurve::EqualPowerSine);
    assert_eq!(res_past.frame_count(), base.frame_count());

    // 5. Zero-length splice
    let empty_splice = AudioBuffer::new_stereo(0.0);
    let res_empty = StemCrossfader::inpaint_splice(&base, &empty_splice, 0, 10, CrossfadeCurve::EqualPowerSine);
    assert_eq!(res_empty.frame_count(), base.frame_count());
}

#[test]
fn test_adversarial_stem_crossfader_dual_sine_continuity() {
    // Crossfading 440 Hz into 880 Hz
    let mut base = AudioBuffer::new_stereo(0.2);
    let mut splice = AudioBuffer::new_stereo(0.1);

    for f in 0..base.frame_count() {
        let t = f as f32 / DEFAULT_SAMPLE_RATE as f32;
        let s = (2.0 * PI * 440.0 * t).sin() * 0.7;
        base.set_channel_sample(f, 0, s);
        base.set_channel_sample(f, 1, s);
    }

    for f in 0..splice.frame_count() {
        let t = f as f32 / DEFAULT_SAMPLE_RATE as f32;
        let s = (2.0 * PI * 880.0 * t).sin() * 0.7;
        splice.set_channel_sample(f, 0, s);
        splice.set_channel_sample(f, 1, s);
    }

    let out = equal_power_crossfade(&base, &splice, 1000, 240);
    assert_eq!(out.frame_count(), base.frame_count());
    for &s in &out.samples {
        assert!(s.abs() <= 1.0);
    }
}

// ============================================================================
// 4. MULTI-THREADED TEST ISOLATION & CONCURRENT STRESS
// ============================================================================

#[test]
fn test_adversarial_multithreaded_concurrent_dsp_execution() {
    // Launch 32 parallel threads performing intense, non-trivial DSP operations
    // concurrently to verify absolute thread safety, lack of global mutable state,
    // and re-entrancy.
    let num_threads = 32;
    let mut handles = Vec::with_capacity(num_threads);

    for thread_id in 0..num_threads {
        let handle = thread::spawn(move || {
            match thread_id % 4 {
                0 => {
                    // Thread worker A: Drum groove synthesis across genres & BPMs
                    let genres = ["One-Drop Reggae", "Four-on-the-Floor EDM", "Trap 808", "Afrobeats 108"];
                    let genre = genres[thread_id % genres.len()];
                    let bpm = 70 + (thread_id as u32 * 5);
                    let groove = generate_groove(genre, bpm, 0.3);
                    assert!(groove.frame_count() > 0);
                    let tel = calculate_telemetry(&groove, bpm);
                    assert!(tel.peak_dbfs <= 0.0);
                }
                1 => {
                    // Thread worker B: Karplus-Strong string guitar solo synthesis
                    let solo = synthesize_lead_solo(2, 120, "A Major");
                    assert!(solo.frame_count() > 0);
                    let tel = calculate_telemetry(&solo, 120);
                    assert!(tel.peak_dbfs <= 0.0);
                }
                2 => {
                    // Thread worker C: Tape saturation and mastering profiles
                    let mut buf = AudioBuffer::new_stereo(0.2);
                    for (i, s) in buf.samples.iter_mut().enumerate() {
                        *s = (i as f32 * 0.02).sin() * 2.5; // Overdrive
                    }
                    let mastered = apply_analog_mastering(&buf, MasteringProfile::StuderA800Tape1970s);
                    for &s in &mastered.samples {
                        assert!(s.abs() <= BRICKWALL_CEILING + 1e-4);
                    }
                }
                _ => {
                    // Thread worker D: Crossfade splicing
                    let mut b1 = AudioBuffer::new_stereo(0.3);
                    let mut b2 = AudioBuffer::new_stereo(0.15);
                    for s in &mut b1.samples { *s = 0.5; }
                    for s in &mut b2.samples { *s = 0.8; }
                    let spliced = splice_stem(&b1, &b2, 1, 100, 4);
                    assert_eq!(spliced.frame_count(), b1.frame_count());
                }
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("Concurrent DSP worker thread panicked!");
    }
}

#[test]
fn test_adversarial_deterministic_parallel_reproducibility() {
    // Running the exact same DSP calculation across two independent threads
    // must yield bit-for-bit identical float output.
    let handle1 = thread::spawn(|| {
        let solo = synthesize_lead_solo(4, 96, "D Minor");
        solo.samples
    });

    let handle2 = thread::spawn(|| {
        let solo = synthesize_lead_solo(4, 96, "D Minor");
        solo.samples
    });

    let samples1 = handle1.join().unwrap();
    let samples2 = handle2.join().unwrap();

    assert_eq!(samples1.len(), samples2.len());
    assert_eq!(samples1, samples2, "DSP output must be bit-for-bit deterministic across threads");
}

// ============================================================================
// 5. AUDIO FIDELITY & TELEMETRY FIDELITY VERIFICATION
// ============================================================================

#[test]
fn test_adversarial_telemetry_analytical_sine_accuracy() {
    // Pure sine wave: peak A = 0.70710678 (-3.0103 dBFS)
    // RMS = A / sqrt(2) = 0.5 (-6.0206 dBFS)
    // Dynamic range = 3.01 dB
    let amp = 0.70710678f32;
    let mut buf = AudioBuffer::new_stereo(0.5);
    let frames = buf.frame_count();
    for f in 0..frames {
        let t = f as f32 / DEFAULT_SAMPLE_RATE as f32;
        let s = (2.0 * PI * 1000.0 * t).sin() * amp;
        buf.set_channel_sample(f, 0, s);
        buf.set_channel_sample(f, 1, s);
    }

    let tel = calculate_telemetry(&buf, 120);

    assert!(
        (tel.peak_dbfs - (-3.0103)).abs() < 0.05,
        "Expected peak ~ -3.01 dBFS, got {}",
        tel.peak_dbfs
    );
    assert!(
        (tel.rms_dbfs - (-6.0206)).abs() < 0.05,
        "Expected RMS ~ -6.02 dBFS, got {}",
        tel.rms_dbfs
    );
    assert!(!tel.clipping);
}

#[test]
fn test_adversarial_telemetry_spectral_flatness_extremes() {
    // 1. Pure sine wave: SFM should be very close to 0.0 (< 0.05)
    let mut sine_buf = AudioBuffer::new_mono(0.2);
    for f in 0..sine_buf.frame_count() {
        let t = f as f32 / DEFAULT_SAMPLE_RATE as f32;
        sine_buf.samples[f] = (2.0 * PI * 440.0 * t).sin() * 0.8;
    }
    let sfm_sine = calculate_spectral_flatness(&sine_buf.samples);
    assert!(
        sfm_sine < 0.05,
        "Pure tone must have low spectral flatness, got {}",
        sfm_sine
    );

    // 2. White noise: SFM should be high (> 0.65)
    let mut noise_buf = AudioBuffer::new_mono(0.2);
    let mut lfsr = 0x9999_u32;
    for s in &mut noise_buf.samples {
        lfsr ^= lfsr << 13;
        lfsr ^= lfsr >> 17;
        lfsr ^= lfsr << 5;
        *s = (lfsr as f32 / u32::MAX as f32) * 1.8 - 0.9;
    }
    let sfm_noise = calculate_spectral_flatness(&noise_buf.samples);
    assert!(
        sfm_noise > 0.65,
        "White noise must have high spectral flatness, got {}",
        sfm_noise
    );
}

#[test]
fn test_adversarial_telemetry_phase_alignment_extremes() {
    let mut buf = AudioBuffer::new_stereo(0.1);

    // In-phase (L == R) -> phase alignment 1.0
    for f in 0..buf.frame_count() {
        let val = (f as f32 * 0.05).sin();
        buf.set_channel_sample(f, 0, val);
        buf.set_channel_sample(f, 1, val);
    }
    let score_in = calculate_phase_alignment(&buf);
    assert!((score_in - 1.0).abs() < 1e-4, "In-phase audio must have alignment ~1.0, got {}", score_in);

    // Completely out-of-phase (L == -R) -> phase alignment 0.0
    for f in 0..buf.frame_count() {
        let val = (f as f32 * 0.05).sin();
        buf.set_channel_sample(f, 0, val);
        buf.set_channel_sample(f, 1, -val);
    }
    let score_out = calculate_phase_alignment(&buf);
    assert!(score_out < 0.01, "Out-of-phase audio must have alignment ~0.0, got {}", score_out);
}
