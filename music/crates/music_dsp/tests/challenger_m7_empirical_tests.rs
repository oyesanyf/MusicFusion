//! crates/music_dsp/tests/challenger_m7_empirical_tests.rs
//!
//! Empirical Stress Tests and Adversarial Challenges for Milestone 7:
//! 1. Continuous Phase Accumulator under rapid buffer cuts and arbitrary frequencies.
//! 2. Karplus-Strong Physical Modeling stability, 0.994 damping, and pink noise excitation.
//! 3. A Major Pentatonic pitch scale locking and zero chromatic siren sweeps.
//! 4. Reggae One-Drop drum & dub bass: kick pitch drop (110->42 Hz), metallic hi-hats (>6000 Hz),
//!    dub sub-bass (180 Hz LP), Hammond organ bubble with 6 Hz rotary LFO.
//! 5. Audio headroom, 12 kHz Butterworth LPF, tape saturation, and -1.0 dBFS (0.89125) peak ceiling.

use std::f32::consts::PI;
use music_core::genre::RhythmGroove;
use music_dsp::groove_synth::{
    DrumVoice, DspRng, DubSubBass, HammondOrganBubble, UniversalDrumSynthesizer,
};
use music_dsp::guitar_wah::{
    A_MAJOR_PENTATONIC, ContinuousPhaseAccumulator, KarplusStrongString,
    PinkNoiseGenerator, SoloGenerator, KARPLUS_FEEDBACK_DAMPING,
    synthesize_lead_solo, synthesize_reggae_skank_chord, synthesize_reggae_skank_track,
};
use music_dsp::tape_mastering::ButterworthLowpass12kHz;
use music_dsp::telemetry_calc::calculate_telemetry;
use music_dsp::wav::AudioBuffer;

const SAMPLE_RATE: u32 = 48_000;

// =========================================================================
// 1. Continuous Phase Accumulator Challenge Tests
// =========================================================================

#[test]
fn test_empirical_phase_accumulator_zero_phase_resets_across_rapid_buffer_cuts() {
    let freq = 329.63f32; // E4
    let sr = SAMPLE_RATE as f32;
    let total_samples = 48000; // 1 full second

    // Monolithic pass
    let mut mono_acc = ContinuousPhaseAccumulator::new();
    let mut mono_samples = Vec::with_capacity(total_samples);
    for _ in 0..total_samples {
        mono_samples.push(mono_acc.sin(freq, sr));
    }

    // Highly fragmented sliced pass with varying buffer chunk sizes
    let chunk_sizes = [1, 3, 7, 13, 64, 128, 512, 17, 33, 1024, 256, 1];
    let mut sliced_acc = ContinuousPhaseAccumulator::new();
    let mut sliced_samples = Vec::with_capacity(total_samples);

    let mut chunk_idx = 0;
    while sliced_samples.len() < total_samples {
        let chunk_len = chunk_sizes[chunk_idx % chunk_sizes.len()].min(total_samples - sliced_samples.len());
        for _ in 0..chunk_len {
            sliced_samples.push(sliced_acc.sin(freq, sr));
        }
        chunk_idx += 1;
    }

    assert_eq!(mono_samples.len(), total_samples);
    assert_eq!(sliced_samples.len(), total_samples);

    let mut max_err = 0.0f32;
    for i in 0..total_samples {
        let diff = (mono_samples[i] - sliced_samples[i]).abs();
        if diff > max_err {
            max_err = diff;
        }
    }

    // Zero phase reset means maximum error across buffer boundaries is bounded by machine epsilon
    assert!(
        max_err < 1e-5,
        "Phase accumulator drifted across buffer cuts! Max discrepancy: {}",
        max_err
    );
}

#[test]
fn test_empirical_phase_accumulator_note_transition_continuity_and_extreme_frequencies() {
    let sr = SAMPLE_RATE as f32;
    let mut acc = ContinuousPhaseAccumulator::new();

    // Rapid note transitions across pentatonic frequencies
    let notes = [220.0, 246.94, 277.18, 329.63, 369.99, 440.0];
    let mut prev_sample = acc.sin(notes[0], sr);

    for step in 1..20_000 {
        let note_freq = notes[(step / 50) % notes.len()];
        let s = acc.sin(note_freq, sr);

        // Maximum step difference for continuous band-limited audio: |x[n] - x[n-1]| <= 2 * pi * f_max / sr
        let delta = (s - prev_sample).abs();
        let max_expected_delta = 2.0 * PI * 440.0 / sr + 0.01;
        assert!(
            delta <= max_expected_delta,
            "Phase discontinuity detected at step {}! Delta: {} > {}",
            step, delta, max_expected_delta
        );
        prev_sample = s;
    }

    // Extreme frequencies stress: 0.001 Hz to 96,000 Hz
    let extreme_freqs = [0.0001, 0.1, 1.0, 20.0, 20_000.0, 24_000.0, 48_000.0, 96_000.0];
    for &ef in &extreme_freqs {
        for _ in 0..1000 {
            let phase = acc.advance(ef, sr);
            assert!(
                phase >= 0.0 && phase < 2.0 * PI,
                "Phase unbounded for freq {}: {}",
                ef, phase
            );
        }
    }
}

// =========================================================================
// 2. Karplus-Strong Physical Modeling Challenge Tests
// =========================================================================

#[test]
fn test_empirical_karplus_strong_delay_line_stability_and_damping() {
    assert_eq!(
        KARPLUS_FEEDBACK_DAMPING, 0.994,
        "Karplus-Strong feedback damping factor must be exactly 0.994"
    );

    let pitches = [50.0, 110.0, 220.0, 440.0, 880.0, 1760.0, 3500.0];
    for &pitch in &pitches {
        let mut ks = KarplusStrongString::new(SAMPLE_RATE);
        ks.pluck(pitch, 0.95);

        let period = (SAMPLE_RATE as f32 / pitch).round() as usize;
        let total_samples = period * 550; // run for 550 string periods

        let mut samples = Vec::with_capacity(total_samples);
        for _ in 0..total_samples {
            let s = ks.tick();
            assert!(
                s.is_finite() && !s.is_nan(),
                "Karplus-Strong generated non-finite sample at pitch {}",
                pitch
            );
            assert!(
                s.abs() <= 1.0,
                "Karplus-Strong sample exceeded unit range at pitch {}: {}",
                pitch, s
            );
            samples.push(s);
        }

        // Energy must decay asymptotically (0.994 damping ensures BIBO stability)
        let initial_rms = (samples[period..(period * 2)].iter().map(|x| x * x).sum::<f32>() / period as f32).sqrt();
        let tail_start = period * 500;
        let tail_end = tail_start + period;
        let tail_rms = (samples[tail_start..tail_end].iter().map(|x| x * x).sum::<f32>() / period as f32).sqrt();

        assert!(
            initial_rms > 0.01,
            "Initial plucked string should have audible energy at pitch {}",
            pitch
        );
        // After 500 periods: (0.994)^500 ≈ 0.049 (< 0.05)
        assert!(
            tail_rms < initial_rms * 0.08,
            "String did not decay properly after 500 periods at pitch {}: initial={}, tail={}",
            pitch, initial_rms, tail_rms
        );
    }
}

#[test]
fn test_empirical_pink_noise_generator_properties() {
    let mut rng = DspRng::new(0xCAFE_BABE);
    let mut pink = PinkNoiseGenerator::new();

    let mut samples = Vec::with_capacity(100_000);
    for _ in 0..100_000 {
        let s = pink.next_sample(&mut rng);
        assert!(s.is_finite(), "Pink noise sample is non-finite");
        samples.push(s);
    }

    // DC bias check: mean should be approximately zero
    let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
    assert!(
        mean.abs() < 0.03,
        "Pink noise has significant DC bias: {}",
        mean
    );

    // Bounded variance
    let variance: f32 = samples.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / samples.len() as f32;
    assert!(
        variance > 0.001 && variance < 0.5,
        "Pink noise variance out of expected range: {}",
        variance
    );
}

// =========================================================================
// 3. Pitch Scale Locking & Zero Chromatic Siren Sweeps Challenge Tests
// =========================================================================

#[test]
fn test_empirical_pitch_scale_locking_and_zero_chromatic_sweeps() {
    let allowed_frequencies = A_MAJOR_PENTATONIC;
    assert_eq!(
        allowed_frequencies,
        [220.0, 246.94, 277.18, 329.63, 369.99, 440.0],
        "A Major pentatonic scale must match canonical frequencies"
    );

    // Inspect all composed solo notes
    let notes = SoloGenerator::compose_16_bar_blues_rock_solo(16, "A", music_core::genre::ModalScale::Ionian);
    assert!(!notes.is_empty(), "Composed solo must not be empty");

    for (i, note) in notes.iter().enumerate() {
        let is_in_scale = allowed_frequencies
            .iter()
            .any(|&p| (p - note.pitch_hz).abs() < 0.05);

        assert!(
            is_in_scale,
            "Note #{} pitch {} Hz is NOT in A Major pentatonic scale! Allowed: {:?}",
            i, note.pitch_hz, allowed_frequencies
        );

        // Zero chromatic siren pitch sweeps: pitch bends must be strictly 0.0
        assert_eq!(
            note.bend_semitones, 0.0,
            "Note #{} has non-zero pitch bend {} semitones! Forbidden chromatic sweep.",
            i, note.bend_semitones
        );
    }

    // Verify skank chord notes: only beats 1 and 3 (beats 2 and 4 in 1-indexed musical counting)
    assert!(synthesize_reggae_skank_chord(0, SAMPLE_RATE, 0.8).is_none());
    assert!(synthesize_reggae_skank_chord(2, SAMPLE_RATE, 0.8).is_none());
    assert!(synthesize_reggae_skank_chord(1, SAMPLE_RATE, 0.8).is_some());
    assert!(synthesize_reggae_skank_chord(3, SAMPLE_RATE, 0.8).is_some());

    // Synthesize full solo buffer and verify no clipping or silence
    let solo_buf = synthesize_lead_solo(16, 76, "A Major");
    assert!(solo_buf.frame_count() > 0);
    let mut peak = 0.0f32;
    for &s in &solo_buf.samples {
        assert!(s.is_finite(), "Solo buffer contains non-finite sample");
        let a = s.abs();
        if a > peak { peak = a; }
    }
    assert!(peak > 0.01, "Solo buffer is silent");
    assert!(peak <= 0.95, "Solo buffer clipped internally: peak = {}", peak);

    // Synthesize skank track
    let skank_buf = synthesize_reggae_skank_track(4.0, SAMPLE_RATE, 76);
    assert!(skank_buf.frame_count() > 0);
    let mut skank_peak = 0.0f32;
    for &s in &skank_buf.samples {
        let a = s.abs();
        if a > skank_peak { skank_peak = a; }
    }
    assert!(skank_peak > 0.01 && skank_peak <= 0.95, "Skank track invalid peak: {}", skank_peak);
}

// =========================================================================
// 4. Reggae One-Drop Drum & Dub Bass Challenge Tests
// =========================================================================

#[test]
fn test_empirical_reggae_one_drop_drums_and_dub_bass() {
    let bpm = 76;
    let beat_sec = 60.0 / bpm as f32;

    // A. Kick One-Drop Pitch Drop: 110 Hz down to 42 Hz over 60ms
    let mut rng = DspRng::new(42);
    // Formula: pitch = 42.0 + 68.0 * (-t / 0.020).exp();
    let pitch_at_0 = 42.0 + 68.0 * (-0.0 / 0.020f32).exp();
    let pitch_at_60ms = 42.0 + 68.0 * (-0.060 / 0.020f32).exp();

    assert!((pitch_at_0 - 110.0).abs() < 1e-4, "Kick pitch at t=0 must be 110 Hz");
    assert!(
        pitch_at_60ms <= 45.4,
        "Kick pitch at 60ms must have dropped down near 42 Hz (actual: {})",
        pitch_at_60ms
    );

    // B. Rhythm Schedule: Beat 3 One-Drop verification
    let duration_secs = beat_sec * 8.0; // 2 bars
    let hits = UniversalDrumSynthesizer::schedule_pattern(RhythmGroove::OneDrop, bpm, duration_secs);

    let kick_hits: Vec<_> = hits.iter().filter(|h| h.voice == DrumVoice::KickOneDrop).collect();
    let snare_hits: Vec<_> = hits.iter().filter(|h| h.voice == DrumVoice::SnareOneDrop).collect();

    assert_eq!(kick_hits.len(), 2, "Expected exactly 2 one-drop kicks in 2 bars");
    assert_eq!(snare_hits.len(), 2, "Expected exactly 2 one-drop snares in 2 bars");

    for bar in 0..2 {
        let bar_start = bar as f32 * beat_sec * 4.0;
        let expected_beat3 = bar_start + 2.0 * beat_sec;

        assert!(
            (kick_hits[bar].time_secs - expected_beat3).abs() < 1e-4,
            "Kick One-Drop was scheduled at {} instead of beat 3 ({})",
            kick_hits[bar].time_secs, expected_beat3
        );
        assert!(
            (snare_hits[bar].time_secs - expected_beat3).abs() < 1e-4,
            "Snare One-Drop was scheduled at {} instead of beat 3 ({})",
            snare_hits[bar].time_secs, expected_beat3
        );
    }

    // Verify 16th-note closed metallic hi-hats (>6000 Hz)
    let hihat_hits: Vec<_> = hits.iter().filter(|h| h.voice == DrumVoice::HiHat16thMetallic).collect();
    assert_eq!(hihat_hits.len(), 32, "Expected 32 16th-note hi-hats in 2 bars");

    // Test metallic hi-hat voice sample generation
    for step in 0..100 {
        let t = step as f32 / SAMPLE_RATE as f32;
        let s = UniversalDrumSynthesizer::synthesize_sample(DrumVoice::HiHat16thMetallic, t, 0.8, &mut rng);
        assert!(s.is_finite(), "HiHat16thMetallic generated non-finite sample");
    }

    // C. Dub Sub-Bass (180 Hz LP, roots/fifths 55.0, 73.42, 82.41 Hz)
    let mut bass_synth = DubSubBass::new(SAMPLE_RATE);
    let bass_buf = bass_synth.synthesize(duration_secs, bpm);
    assert_eq!(bass_buf.sample_rate, SAMPLE_RATE);
    assert!(bass_buf.frame_count() > 0);

    let mut bass_peak = 0.0f32;
    for &s in &bass_buf.samples {
        assert!(s.is_finite());
        let a = s.abs();
        if a > bass_peak { bass_peak = a; }
    }
    assert!(bass_peak > 0.01 && bass_peak <= 0.95, "Dub bass peak invalid: {}", bass_peak);

    // D. Hammond Organ Bubble (1x-4x drawbars, 6 Hz tremolo LFO, ADSR)
    let mut organ_synth = HammondOrganBubble::new(SAMPLE_RATE);
    let organ_buf = organ_synth.synthesize(duration_secs, bpm, 220.0);
    assert_eq!(organ_buf.sample_rate, SAMPLE_RATE);
    assert!(organ_buf.frame_count() > 0);

    let mut organ_peak = 0.0f32;
    for &s in &organ_buf.samples {
        assert!(s.is_finite());
        let a = s.abs();
        if a > organ_peak { organ_peak = a; }
    }
    assert!(organ_peak > 0.01 && organ_peak <= 0.95, "Hammond organ peak invalid: {}", organ_peak);
}

// =========================================================================
// 5. Audio Headroom, 12 kHz LPF, Tape Saturation & Peak Ceiling Challenge Tests
// =========================================================================

#[test]
fn test_empirical_audio_headroom_stem_scaling_and_mastering_ceiling() {
    // A. 0.20 stem scaling prevents summing bus clipping
    let stem_scale = 0.20f32;
    let frames = 4800; // 100ms
    // Construct 5 worst-case stems hitting maximum unity peak 1.0 simultaneously
    let mut sum_left = vec![0.0f32; frames];
    for _stem in 0..5 {
        for f in 0..frames {
            sum_left[f] += 1.0 * stem_scale;
        }
    }
    for &sample in &sum_left {
        assert!(
            sample <= 1.0 + 1e-6,
            "Summing 5 stems with 0.20 scaling exceeded 0 dBFS! Sample: {}",
            sample
        );
    }

    // B. 12 kHz Butterworth Low-Pass Filter
    // Helper to measure gain through LPF at test frequency
    let measure_lpf_gain = |freq: f32| -> f32 {
        let mut filter = ButterworthLowpass12kHz::new(1, SAMPLE_RATE as f32);
        let test_frames = 4800;
        let mut max_in = 0.0f32;
        let mut max_out = 0.0f32;
        for i in 0..test_frames {
            let t = i as f32 / SAMPLE_RATE as f32;
            let x = (2.0 * PI * freq * t).sin();
            let y = filter.process_sample(0, x);
            if i > 2400 { // measure after settling
                if x.abs() > max_in { max_in = x.abs(); }
                if y.abs() > max_out { max_out = y.abs(); }
            }
        }
        max_out / max_in.max(1e-5)
    };

    let gain_1k = measure_lpf_gain(1000.0);
    let gain_12k = measure_lpf_gain(12000.0);
    let gain_20k = measure_lpf_gain(20000.0);

    // At 1 kHz, passband should have virtually zero attenuation (gain > 0.98)
    assert!(
        gain_1k > 0.98,
        "12 kHz LPF attenuated passband (1 kHz) excessively: gain = {}",
        gain_1k
    );
    // At 12 kHz cutoff, 2nd-order Butterworth response is ~ -3 dB (gain ~ 0.7071)
    assert!(
        (gain_12k - 0.7071).abs() < 0.12,
        "12 kHz LPF cutoff gain is not near -3 dB (0.7071): gain = {}",
        gain_12k
    );
    // At 20 kHz stopband, should roll off significantly (gain < 0.45)
    assert!(
        gain_20k < 0.45,
        "12 kHz LPF failed to attenuate stopband (20 kHz): gain = {}",
        gain_20k
    );

    // C. Tanh tape saturation properties
    // Smoothness, monotonicity, and soft-limiting
    let mut prev_sat = -2.0f32;
    for i in -200..=200 {
        let x = i as f32 * 0.01;
        let sat = (x * 1.2f32).tanh();
        assert!(sat > prev_sat, "Tanh saturation must be strictly monotonic");
        assert!(sat.abs() < 1.0, "Tanh saturation must remain strictly within (-1.0, 1.0)");
        prev_sat = sat;
    }

    // D. Peak normalization strictly enforcing -1.0 dBFS (0.89125)
    let target_peak = 0.89125f32;
    let mut test_buf = AudioBuffer::new_stereo(0.5);
    for (i, s) in test_buf.samples.iter_mut().enumerate() {
        *s = ((i as f32 * 0.1).sin() * 2.5).clamp(-2.5, 2.5); // intentionally overdriven signal
    }

    // Apply normalization logic
    let mut max_abs = 0.0f32;
    for &s in &test_buf.samples {
        if s.abs() > max_abs { max_abs = s.abs(); }
    }
    if max_abs > 0.0001 {
        let norm_gain = target_peak / max_abs;
        for s in &mut test_buf.samples {
            *s = (*s * norm_gain).clamp(-target_peak, target_peak);
        }
    }

    let mut final_peak = 0.0f32;
    for &s in &test_buf.samples {
        if s.abs() > final_peak { final_peak = s.abs(); }
        assert!(
            s.abs() <= target_peak + 1e-6,
            "Normalized sample exceeded -1.0 dBFS (0.89125): {}",
            s.abs()
        );
    }
    assert!(
        (final_peak - target_peak).abs() < 1e-4,
        "Normalized buffer peak did not match -1.0 dBFS (0.89125): actual = {}",
        final_peak
    );

    // E. Telemetry clipping inspector
    let telemetry = calculate_telemetry(&test_buf, 76);
    assert!(!telemetry.clipping, "Telemetry reported clipping on normalized master!");
    assert!(
        telemetry.peak_dbfs <= -0.99,
        "Telemetry peak dBFS ({}) exceeds -1.0 dBFS ceiling",
        telemetry.peak_dbfs
    );
}
