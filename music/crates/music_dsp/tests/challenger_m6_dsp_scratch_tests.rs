//! crates/music_dsp/tests/challenger_m6_dsp_scratch_tests.rs
//!
//! Milestone 6 Adversarial Challenge Suite 1:
//! - Scratch & Harsh Noise Elimination in Drum and Guitar Synthesis
//! - Spectral Energy Distribution (Attenuated high-frequency hash above 12 kHz)
//! - Karplus-Strong string synthesis under extreme pitches and wah modulation
//! - True-peak audio ceiling enforcement (strictly <= 0.966 / -0.3 dBFS)
//! - Numeric sanity: Zero NaNs, zero infinities, zero digital clipping

use std::f32::consts::PI;
use music_dsp::groove_synth::{
    generate_groove, DrumHit, DrumVoice, DspRng, UniversalDrumSynthesizer,
};
use music_dsp::guitar_wah::{
    synthesize_lead_solo, GuitarAmpSimulator,
    KarplusStrongString, WahWahFilter,
};
use music_dsp::tape_mastering::{
    apply_analog_mastering, apply_analog_tape_mastering,
    MasteringProfile, BRICKWALL_CEILING,
};
use music_dsp::telemetry_calc::{calculate_telemetry, fft_radix2};
use music_dsp::wav::AudioBuffer;

const SAMPLE_RATE: u32 = 48_000;

// ============================================================================
// Helper: Compute Spectral Energy Ratio Above 12 kHz
// ============================================================================

fn compute_high_frequency_ratio_above_12khz(samples: &[f32]) -> f32 {
    const FFT_SIZE: usize = 2048;
    if samples.len() < FFT_SIZE {
        return 0.0;
    }

    let mut real = vec![0.0f32; FFT_SIZE];
    let mut imag = vec![0.0f32; FFT_SIZE];

    // Apply Hann window
    for i in 0..FFT_SIZE {
        let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos());
        real[i] = samples[i] * w;
        imag[i] = 0.0;
    }

    fft_radix2(&mut real, &mut imag);

    // At 48 kHz sample rate, bin width = 48000 / 2048 = 23.4375 Hz
    // 12 kHz is at bin: 12000 / 23.4375 = 512
    let cutoff_bin = 512;
    let nyquist_bin = FFT_SIZE / 2; // 1024 (24 kHz)

    let mut total_power = 0.0f64;
    let mut high_power = 0.0f64;

    for k in 1..nyquist_bin {
        let p = (real[k] * real[k] + imag[k] * imag[k]) as f64;
        total_power += p;
        if k >= cutoff_bin {
            high_power += p;
        }
    }

    if total_power <= 1e-12 {
        0.0
    } else {
        (high_power / total_power) as f32
    }
}

// ============================================================================
// Group 1: Drum Voice Spectral Analysis & Harsh Scratch Elimination
// ============================================================================

#[test]
fn test_spectral_energy_snare_and_shaker_attenuate_scratch_above_12khz() {
    let mut rng = DspRng::new(0xFEED_FACE);

    // Control: Unfiltered white noise has uniform spectral density (~50% power above 12 kHz)
    let mut white_noise = vec![0.0f32; 4800];
    for s in &mut white_noise {
        *s = rng.next_f32();
    }
    let control_ratio = compute_high_frequency_ratio_above_12khz(&white_noise);
    assert!(
        control_ratio > 0.40,
        "Control white noise should have ~50% energy above 12 kHz, got {}",
        control_ratio
    );

    // 1. SnareAcoustic: Bandpass wire sizzle at 3.2 kHz with lowpass rolloff
    let snare_dur = UniversalDrumSynthesizer::voice_duration_secs(DrumVoice::SnareAcoustic);
    let snare_frames = (snare_dur * SAMPLE_RATE as f32) as usize;
    let mut snare_samples = Vec::with_capacity(snare_frames);
    for f in 0..snare_frames {
        let t = f as f32 / SAMPLE_RATE as f32;
        snare_samples.push(UniversalDrumSynthesizer::synthesize_sample(
            DrumVoice::SnareAcoustic,
            t,
            1.0,
            &mut rng,
        ));
    }
    let snare_ratio = compute_high_frequency_ratio_above_12khz(&snare_samples);
    assert!(
        snare_ratio < 0.25,
        "Snare acoustic wire sizzle must attenuate harsh frequencies >12 kHz, got ratio {}",
        snare_ratio
    );
    assert!(
        snare_ratio < control_ratio * 0.5,
        "Snare high-frequency ratio ({}) must be < 50% of unfiltered noise ({})",
        snare_ratio,
        control_ratio
    );

    // 2. Shaker: Gentle 4.5 kHz bandpass bead ring (eliminating sandpaper scratch noise)
    let shaker_dur = UniversalDrumSynthesizer::voice_duration_secs(DrumVoice::Shaker);
    let shaker_frames = (shaker_dur * SAMPLE_RATE as f32) as usize;
    let mut shaker_samples = Vec::with_capacity(shaker_frames);
    for f in 0..shaker_frames {
        let t = f as f32 / SAMPLE_RATE as f32;
        shaker_samples.push(UniversalDrumSynthesizer::synthesize_sample(
            DrumVoice::Shaker,
            t,
            1.0,
            &mut rng,
        ));
    }
    let shaker_ratio = compute_high_frequency_ratio_above_12khz(&shaker_samples);
    assert!(
        shaker_ratio < 0.30,
        "Shaker must attenuate harsh high-frequency scratch above 12 kHz, got ratio {}",
        shaker_ratio
    );
}

#[test]
fn test_all_18_drum_voices_numeric_sanity_and_no_nans_or_infs() {
    let mut rng = DspRng::new(0x1234_5678);

    let all_voices = [
        DrumVoice::KickAcoustic,
        DrumVoice::Kick808Sub,
        DrumVoice::KickEdmPunch,
        DrumVoice::SnareAcoustic,
        DrumVoice::SnareRimshot,
        DrumVoice::Clap,
        DrumVoice::HiHatClosed,
        DrumVoice::HiHatOpen,
        DrumVoice::HiHatTriplet,
        DrumVoice::LogDrumAmapiano,
        DrumVoice::NyabinghiFunde,
        DrumVoice::NyabinghiAkete,
        DrumVoice::NyabinghiThunder,
        DrumVoice::Clave,
        DrumVoice::Shaker,
        DrumVoice::JazzRideCymbal,
        DrumVoice::OrchestralTimpani,
        DrumVoice::CrashCymbal,
    ];

    for voice in all_voices {
        let dur = UniversalDrumSynthesizer::voice_duration_secs(voice);
        let frames = (dur * SAMPLE_RATE as f32) as usize;

        for f in 0..frames {
            let t = f as f32 / SAMPLE_RATE as f32;
            let sample = UniversalDrumSynthesizer::synthesize_sample(voice, t, 1.0, &mut rng);
            assert!(
                sample.is_finite(),
                "DrumVoice {:?} produced non-finite sample {} at frame {}/{}",
                voice,
                sample,
                f,
                frames
            );
            assert!(
                sample.abs() <= 1.0,
                "DrumVoice {:?} exceeded full-scale [-1.0, 1.0]: got {} at frame {}",
                voice,
                sample,
                f
            );
        }
    }
}

// ============================================================================
// Group 2: Drum Synthesizer Extreme Conditions & Rapid Sequences
// ============================================================================

#[test]
fn test_drum_synthesis_under_extreme_velocity_and_rapid_sequences() {
    let mut buffer = AudioBuffer::new_stereo(2.0);

    // Rapid 32nd-note sequence (64 hits across 2 seconds) with extreme velocity variations
    let mut hits = Vec::new();
    for i in 0..64 {
        let t = i as f32 * (2.0 / 64.0);
        let vel = match i % 4 {
            0 => 0.0,    // Zero velocity
            1 => 1.0,    // Max velocity
            2 => 5.0,    // Overdriven velocity
            _ => -0.5,   // Negative velocity edge case
        };
        let voice = match i % 5 {
            0 => DrumVoice::SnareAcoustic,
            1 => DrumVoice::HiHatTriplet,
            2 => DrumVoice::Shaker,
            3 => DrumVoice::Clap,
            _ => DrumVoice::Kick808Sub,
        };
        hits.push(DrumHit {
            time_secs: t,
            voice,
            velocity: vel,
            pan: if i % 2 == 0 { -0.8 } else { 0.8 },
        });
    }

    UniversalDrumSynthesizer::render_hits(&hits, &mut buffer);

    for (idx, &s) in buffer.samples.iter().enumerate() {
        assert!(s.is_finite(), "Sample at index {} is not finite: {}", idx, s);
        assert!(
            s.abs() <= 0.9501,
            "render_hits must clamp samples to [-0.95, 0.95], got {} at index {}",
            s,
            idx
        );
    }
}

#[test]
fn test_drum_synthesis_simultaneous_all_18_voices_impact() {
    let mut buffer = AudioBuffer::new_stereo(1.0);

    // All 18 voices struck at the exact same downbeat t = 0.0 with velocity 1.0
    let all_voices = [
        DrumVoice::KickAcoustic,
        DrumVoice::Kick808Sub,
        DrumVoice::KickEdmPunch,
        DrumVoice::SnareAcoustic,
        DrumVoice::SnareRimshot,
        DrumVoice::Clap,
        DrumVoice::HiHatClosed,
        DrumVoice::HiHatOpen,
        DrumVoice::HiHatTriplet,
        DrumVoice::LogDrumAmapiano,
        DrumVoice::NyabinghiFunde,
        DrumVoice::NyabinghiAkete,
        DrumVoice::NyabinghiThunder,
        DrumVoice::Clave,
        DrumVoice::Shaker,
        DrumVoice::JazzRideCymbal,
        DrumVoice::OrchestralTimpani,
        DrumVoice::CrashCymbal,
    ];

    let hits: Vec<DrumHit> = all_voices
        .iter()
        .map(|&v| DrumHit {
            time_secs: 0.01,
            voice: v,
            velocity: 1.0,
            pan: 0.0,
        })
        .collect();

    UniversalDrumSynthesizer::render_hits(&hits, &mut buffer);

    // Verify all samples are clamped safely to [-0.95, 0.95]
    for (i, &s) in buffer.samples.iter().enumerate() {
        assert!(s.is_finite());
        assert!(s.abs() <= 0.9501, "Frame {} exceeded clamp: {}", i, s);
    }
}

#[test]
fn test_groove_generation_across_extreme_bpms_40_to_240() {
    let extreme_bpms = [0, 40, 76, 120, 144, 220, 240, 600];

    for &bpm in &extreme_bpms {
        let buf = generate_groove("reggae", bpm, 1.0);
        assert_eq!(buf.sample_rate, 48_000);
        assert_eq!(buf.channels, 2);
        assert_eq!(buf.frame_count(), 48_000);

        for &sample in &buf.samples {
            assert!(sample.is_finite(), "Non-finite sample at BPM {}", bpm);
            assert!(sample.abs() <= 0.9501, "Sample exceeded 0.95 at BPM {}", bpm);
        }
    }
}

// ============================================================================
// Group 3: Karplus-Strong String Synthesis & Wah Sweeping Stress
// ============================================================================

#[test]
fn test_karplus_strong_under_extreme_pitches_no_clicks_or_nans() {
    let mut string = KarplusStrongString::new(SAMPLE_RATE);

    let test_pitches = [
        10.0,    // Sub-audio (clamped to 50 Hz in pluck)
        40.0,
        82.4,    // Low E on guitar
        220.0,   // A3
        440.0,   // A4
        1318.5,  // High E
        4000.0,  // Clamped limit
        10000.0, // Ultrasonic
        0.0,     // Boundary zero
        -440.0,  // Negative pitch
    ];

    for &pitch in &test_pitches {
        string.pluck(pitch, 1.0);

        let mut prev_sample = 0.0f32;
        let mut max_diff = 0.0f32;

        for frame in 0..4800 {
            let s = string.tick();
            assert!(
                s.is_finite(),
                "Karplus-Strong produced non-finite sample {} at pitch {} frame {}",
                s,
                pitch,
                frame
            );

            // Phase continuity: check first difference
            if frame > 1 {
                let diff = (s - prev_sample).abs();
                if diff > max_diff {
                    max_diff = diff;
                }
            }
            prev_sample = s;
        }

        // Phase-continuous harmonic excitation ensures max sample-to-sample jump is physically bounded
        assert!(
            max_diff < 1.5,
            "Excessive sample discontinuity (possible scratch/click): max diff {} at pitch {}",
            max_diff,
            pitch
        );
    }
}

#[test]
fn test_wah_wah_filter_extreme_sweeping_and_out_of_bounds_positions() {
    let mut wah = WahWahFilter::new(SAMPLE_RATE);

    // Rapid alternating positions (square wave jumps between 0.0 and 1.0)
    for frame in 0..4800 {
        let pedal = if frame % 20 < 10 { 0.0 } else { 1.0 };
        wah.set_pedal_position(pedal);

        let t = frame as f32 / SAMPLE_RATE as f32;
        let input = (2.0 * PI * 440.0 * t).sin();
        let out = wah.process(input);

        assert!(
            out.is_finite(),
            "Wah-wah filter produced non-finite output {} at frame {}",
            out,
            frame
        );
        assert!(
            out.abs() <= 1.2001,
            "Wah output exceeded clamp bounds at frame {}: {}",
            frame,
            out
        );
    }

    // Out-of-bounds pedal positions (negative and extreme positive beyond [0, 1])
    let out_of_bounds = [-100.0, -1.0, 0.0, 0.5, 1.0, 2.0, 100.0, f32::INFINITY, -f32::INFINITY];
    for &pos in &out_of_bounds {
        wah.set_pedal_position(pos);
        let out = wah.process(0.5);
        assert!(out.is_finite(), "Wah failed with pos {}: got {}", pos, out);
    }

    // Dynamic auto-modulation sweeping under varying input amplitudes
    for frame in 0..4800 {
        let t = frame as f32 / SAMPLE_RATE as f32;
        let input = (2.0 * PI * 440.0 * t).sin() * (0.1 + 0.9 * (frame as f32 / 4800.0));
        wah.auto_modulate(t, input);
        let out = wah.process(input);
        assert!(out.is_finite(), "Wah auto_modulate produced non-finite output at frame {}", frame);
    }
}

#[test]
fn test_guitar_cabinet_simulator_heavily_rolls_off_above_12khz() {
    let mut amp = GuitarAmpSimulator::new(6.0); // High tube drive

    // Excite amp with broad harmonic test tone
    let mut amp_output = Vec::with_capacity(4800);
    for frame in 0..4800 {
        let t = frame as f32 / SAMPLE_RATE as f32;
        let x = (2.0 * PI * 440.0 * t).sin() * 0.7 + (2.0 * PI * 2200.0 * t).sin() * 0.3;
        amp_output.push(amp.process(x));
    }

    let high_ratio = compute_high_frequency_ratio_above_12khz(&amp_output);
    assert!(
        high_ratio < 0.05,
        "Cabinet simulator must heavily attenuate frequencies above 12 kHz, got ratio {}",
        high_ratio
    );
}

// ============================================================================
// Group 4: True-Peak Audio Ceiling Strict Compliance (<= 0.966 / -0.3 dBFS)
// ============================================================================

#[test]
fn test_true_peak_audio_ceiling_across_all_mastering_profiles() {
    let profiles = [
        MasteringProfile::StuderA800Tape1970s,
        MasteringProfile::ModernPunchyPop,
        MasteringProfile::ClubEdmSubHeavy,
        MasteringProfile::CinematicOrchestral,
        MasteringProfile::AudiophileJazz,
    ];

    // Generate a hot, multi-layer mix: loud drums + loud guitar solo
    let drums = generate_groove("reggae", 76, 2.0);
    let solo = synthesize_lead_solo(2, 76, "A Major");

    let mut hot_mix = AudioBuffer::new_stereo(2.0);
    for i in 0..hot_mix.samples.len() {
        let d = if i < drums.samples.len() { drums.samples[i] } else { 0.0 };
        let s = if i < solo.samples.len() { solo.samples[i] } else { 0.0 };
        // Hot sum + additional overdrive boost
        hot_mix.samples[i] = (d + s) * 1.5;
    }

    for profile in profiles {
        let mastered = apply_analog_mastering(&hot_mix, profile);

        let mut max_abs_sample = 0.0f32;
        for (idx, &sample) in mastered.samples.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Mastered sample is not finite in profile {:?}: got {}",
                profile,
                sample
            );
            let abs_s = sample.abs();
            if abs_s > max_abs_sample {
                max_abs_sample = abs_s;
            }
            assert!(
                abs_s <= BRICKWALL_CEILING + 1e-5,
                "Sample exceeded true-peak ceiling 0.966 (-0.3 dBFS) in profile {:?}: got {} at idx {}",
                profile,
                sample,
                idx
            );
        }

        // Telemetry check
        let telemetry = calculate_telemetry(&mastered, 76);
        assert!(
            !telemetry.clipping,
            "Mastered audio must never report clipping: profile {:?}",
            profile
        );
        assert!(
            telemetry.peak_dbfs <= -0.29,
            "Peak dBFS must be <= -0.3 dBFS (got {} dBFS) in profile {:?}",
            telemetry.peak_dbfs,
            profile
        );
    }
}

#[test]
fn test_analog_tape_mastering_extreme_overdrive_bursts_strictly_enforces_ceiling() {
    let mut overdriven = AudioBuffer::new_stereo(1.0);

    // Full scale bursts up to +20.0 magnitude
    for i in 0..overdriven.samples.len() {
        overdriven.samples[i] = if i % 100 < 50 { 20.0 } else { -20.0 };
    }

    let mastered = apply_analog_tape_mastering(&overdriven, 6.0);

    for (i, &s) in mastered.samples.iter().enumerate() {
        assert!(s.is_finite());
        assert!(
            s.abs() <= BRICKWALL_CEILING + 1e-5,
            "Dirac burst sample exceeded 0.966: got {} at sample {}",
            s,
            i
        );
    }
}
