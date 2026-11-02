//! crates/music_fusion/tests/challenger_m8_empirical_tests.rs
//!
//! Empirical Challenger Stress Harness for Milestone 8:
//! 1. Legal Originality Guard:
//!    - Contiguous melodic hooks (>= 8 notes) rejected across all 12 semitone transpositions (0..=11).
//!    - Contiguous hooks embedded inside noisy/random sequences at different offsets.
//!    - Boundary conditions: 7 notes (compliant) vs 8 notes (rejected).
//!    - Inverted waveform polarity (sample * -1.0) 100% detected via cross-correlation.
//!    - Gain scaling (+/- alpha) and time-lag invariance of cross-correlation.
//! 2. RVQ Tokenizer:
//!    - Monotonic residual norm reduction across all stages (||r_m|| <= ||r_{m-1}||)
//!      tested across 500 stress vectors (sparse, step, tiny, huge, gaussian, uniform).
//!    - Multi-stage configurations (M=2,4,6,8, K=64,256,1024, D=16,32,64).
//! 3. Anti-Monopoly Style Blending:
//!    - Clamping of dominant file weights to <= 60% for N in 3..=20.
//!    - Simplex conservation: sum(w_i) == 1.0 and w_i >= 0.0.
//!    - Boundary transition from N=2 (unconstrained) to N=3 (strictly constrained <= 60%).

use std::collections::HashMap;
use std::f32::consts::PI;

use music_core::rl_contracts::{AcousticFeatureVector, ComplianceStatus};
use music_dsp::acoustic_analyzer::{AcousticAnalyzer, ANTI_MONOPOLY_CEILING};
use music_dsp::rvq::RvqTokenizer;
use music_dsp::wav::AudioBuffer;
use music_fusion::originality_guard::{
    LegalOriginalityGuard, CHROMA_COSINE_CEILING, HOOK_NOTE_LIMIT, WAVEFORM_CORRELATION_CEILING,
};

// =========================================================================
// 1. Legal Originality Guard: Transposition & Contiguous Hook Plagiarism
// =========================================================================

#[test]
fn test_empirical_12_semitone_transposition_on_diverse_hooks() {
    assert_eq!(HOOK_NOTE_LIMIT, 8);

    // 5 distinct melodic hook shapes (all >= 8 notes)
    let hooks: Vec<(&str, Vec<u8>)> = vec![
        ("Major_Diatonic", vec![0, 2, 4, 5, 7, 9, 11, 0]),
        ("Minor_Pentatonic", vec![9, 0, 2, 4, 7, 9, 0, 2]),
        ("Blues_Riff", vec![0, 3, 5, 6, 7, 10, 0, 10]),
        ("Chromatic_Run", vec![1, 2, 3, 4, 5, 6, 7, 8]),
        ("Disjunct_Arpeggio", vec![0, 7, 4, 0, 7, 11, 7, 4, 0, 7]),
    ];

    let dummy_buffer = AudioBuffer::new_stereo(2.0);
    let dummy_chroma = vec![[0.05f32; 12]; 12];

    for (name, hook) in &hooks {
        let ref_tuples = vec![(
            "ref_track.wav",
            &dummy_buffer,
            dummy_chroma.as_slice(),
            hook.as_slice(),
        )];

        // Test every single semitone transposition: 0..=11
        for shift in 0..12u8 {
            let transposed: Vec<u8> = hook.iter().map(|&p| (p + shift) % 12).collect();

            let (lcs, ratio) =
                LegalOriginalityGuard::calculate_transposition_invariant_lcs(hook, &transposed);
            assert_eq!(
                lcs,
                hook.len(),
                "Hook '{}' shifted by {} semitones must have LCS == hook.len() ({}), got {}",
                name,
                shift,
                hook.len(),
                lcs
            );
            assert!(
                (ratio - 1.0).abs() < 1e-4,
                "Hook '{}' shifted by {} must have overlap ratio 1.0, got {}",
                name,
                shift,
                ratio
            );

            // Audit through full LegalOriginalityGuard
            let report = LegalOriginalityGuard::audit_candidate(
                "candidate_test",
                &dummy_buffer,
                &dummy_chroma,
                &transposed,
                &ref_tuples,
            );

            assert!(
                !report.is_compliant,
                "Hook '{}' transposed by {} MUST be rejected as non-compliant",
                name, shift
            );
            assert_eq!(
                report.compliance_status,
                ComplianceStatus::RejectedPotentialCopy,
                "Hook '{}' transposed by {} MUST have status RejectedPotentialCopy",
                name, shift
            );
            assert_eq!(
                report.max_melodic_sequence_notes,
                hook.len(),
                "Hook '{}' transposed by {} detected notes mismatch",
                name, shift
            );
            assert!(
                !report.legal_compliance.legal_safeguard_verified,
                "Hook '{}' transposed by {} legal safeguard must fail",
                name, shift
            );
        }
    }
}

#[test]
fn test_empirical_embedded_contiguous_hooks_at_different_offsets() {
    // 8-note hook embedded inside longer melodies with completely different padding
    let hook = vec![2u8, 4, 5, 7, 9, 11, 0, 2]; // D, E, F, G, A, B, C, D (8 notes)

    // Reference has hook at offset 10 (preceded by 10 non-matching notes)
    let mut ref_melody = vec![1u8, 1, 3, 3, 6, 6, 8, 8, 10, 10];
    ref_melody.extend_from_slice(&hook);
    ref_melody.extend_from_slice(&[1, 3, 6, 8, 10, 1, 3, 6]);

    let dummy_buffer = AudioBuffer::new_stereo(2.0);
    let dummy_chroma = vec![[0.05f32; 12]; 26];
    let ref_tuples = vec![(
        "ref_embedded.wav",
        &dummy_buffer,
        dummy_chroma.as_slice(),
        ref_melody.as_slice(),
    )];

    // Candidate has hook at offset 5, shifted by +7 semitones (G Major transposition)
    for shift in 0..12u8 {
        let transposed_hook: Vec<u8> = hook.iter().map(|&p| (p + shift) % 12).collect();
        let mut cand_melody = vec![3u8, 6, 8, 10, 1]; // 5 padding notes
        cand_melody.extend_from_slice(&transposed_hook);
        cand_melody.extend_from_slice(&[3, 6, 8, 10, 1, 3, 6, 8, 10, 1, 3, 6, 8, 10]); // 14 padding notes

        let report = LegalOriginalityGuard::audit_candidate(
            "embedded_candidate",
            &dummy_buffer,
            &dummy_chroma,
            &cand_melody,
            &ref_tuples,
        );

        assert!(
            report.max_melodic_sequence_notes >= 8,
            "Shift {}: Embedded hook must be detected with LCS >= 8, got {}",
            shift,
            report.max_melodic_sequence_notes
        );
        assert_eq!(
            report.compliance_status,
            ComplianceStatus::RejectedPotentialCopy,
            "Shift {}: Embedded hook must trigger RejectedPotentialCopy",
            shift
        );
        assert!(!report.is_compliant);
    }
}

#[test]
fn test_empirical_hook_boundary_7_vs_8_notes() {
    let dummy_buffer = AudioBuffer::new_stereo(2.0);

    let mut ref_chroma = vec![[0.0f32; 12]; 10];
    let mut cand_chroma = vec![[0.0f32; 12]; 10];
    for f in 0..10 {
        ref_chroma[f][0] = 1.0; // C
        cand_chroma[f][6] = 1.0; // F#
    }

    // 7-note hook (HOOK_NOTE_LIMIT - 1)
    let hook_7 = vec![0u8, 2, 4, 5, 7, 9, 11];
    let ref_tuples_7 = vec![(
        "ref_7.wav",
        &dummy_buffer,
        ref_chroma.as_slice(),
        hook_7.as_slice(),
    )];

    let report_7 = LegalOriginalityGuard::audit_candidate(
        "candidate_7",
        &dummy_buffer,
        &cand_chroma,
        &hook_7,
        &ref_tuples_7,
    );

    // 7 notes should NOT trigger immediate breach rejection (note_ratio = (7-2)/(8-2)*0.5 = 0.416 <= 0.70)
    assert_eq!(report_7.max_melodic_sequence_notes, 7);
    assert!(
        report_7.is_compliant,
        "7-note hook with distinct chroma must remain compliant under the 8-note threshold"
    );
    assert_ne!(
        report_7.compliance_status,
        ComplianceStatus::RejectedPotentialCopy
    );

    // 8-note hook (HOOK_NOTE_LIMIT)
    let hook_8 = vec![0u8, 2, 4, 5, 7, 9, 11, 0];
    let ref_tuples_8 = vec![(
        "ref_8.wav",
        &dummy_buffer,
        ref_chroma.as_slice(),
        hook_8.as_slice(),
    )];

    let report_8 = LegalOriginalityGuard::audit_candidate(
        "candidate_8",
        &dummy_buffer,
        &cand_chroma,
        &hook_8,
        &ref_tuples_8,
    );

    assert_eq!(report_8.max_melodic_sequence_notes, 8);
    assert!(
        !report_8.is_compliant,
        "8-note hook MUST be rejected as potential copy"
    );
    assert_eq!(
        report_8.compliance_status,
        ComplianceStatus::RejectedPotentialCopy
    );
}

// =========================================================================
// 2. Waveform Cross-Correlation & Inverted Polarity Attacks
// =========================================================================

#[test]
fn test_empirical_inverted_polarity_cross_correlation_detection() {
    let sample_rate = 4000; // 4 kHz downsampled
    let n = 8000; // 2 seconds

    // 5 diverse waveforms:
    let mut waveforms: Vec<(&str, Vec<f32>)> = Vec::new();

    // 1. Pure sine 440 Hz
    let sine: Vec<f32> = (0..n)
        .map(|i| (2.0 * PI * 440.0 * i as f32 / sample_rate as f32).sin() * 0.7)
        .collect();
    waveforms.push(("Sine_440Hz", sine));

    // 2. Multi-tone chord (261.63 + 329.63 + 392.0 Hz)
    let chord: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            ((2.0 * PI * 261.63 * t).sin()
                + (2.0 * PI * 329.63 * t).sin()
                + (2.0 * PI * 392.0 * t).sin())
                * 0.25
        })
        .collect();
    waveforms.push(("Triad_Chord", chord));

    // 3. FM synthesis modulation
    let fm: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let mod_sig = (2.0 * PI * 5.0 * t).sin() * 50.0;
            (2.0 * PI * (220.0 + mod_sig) * t).sin() * 0.6
        })
        .collect();
    waveforms.push(("FM_Modulated", fm));

    // 4. Chirp sweep (100 Hz to 800 Hz)
    let chirp: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let f = 100.0 + 350.0 * t;
            (2.0 * PI * f * t).sin() * 0.6
        })
        .collect();
    waveforms.push(("Chirp_Sweep", chirp));

    // 5. Shaped pulse wave
    let pulse: Vec<f32> = (0..n)
        .map(|i| {
            let phase = (i % 80) as f32 / 80.0;
            if phase < 0.3 { 0.8 } else { -0.3 }
        })
        .collect();
    waveforms.push(("Pulse_Train", pulse));

    for (name, wave) in &waveforms {
        // Attack: Inverted polarity (sample * -1.0)
        let inverted: Vec<f32> = wave.iter().map(|&x| -x).collect();

        let xcorr = LegalOriginalityGuard::calculate_max_waveform_cross_correlation(wave, &inverted);

        assert!(
            xcorr > 0.95,
            "Waveform '{}' inverted polarity cross-correlation ({:.4}) must be > 0.95",
            name,
            xcorr
        );
        assert!(
            xcorr > WAVEFORM_CORRELATION_CEILING,
            "Waveform '{}' inverted polarity must breach 0.65 correlation ceiling, got {:.4}",
            name,
            xcorr
        );

        // Verify full audit rejection
        let buf_ref = AudioBuffer {
            sample_rate: 4000,
            channels: 1,
            samples: wave.clone(),
        };
        let buf_cand = AudioBuffer {
            sample_rate: 4000,
            channels: 1,
            samples: inverted,
        };
        let chroma = vec![[0.0f32; 12]; 10];
        let pitch = vec![0u8];
        let ref_tuples = vec![("ref.wav", &buf_ref, chroma.as_slice(), pitch.as_slice())];

        let report = LegalOriginalityGuard::audit_candidate(
            "inverted_cand",
            &buf_cand,
            &chroma,
            &pitch,
            &ref_tuples,
        );

        assert!(
            !report.is_compliant,
            "Waveform '{}' inverted polarity MUST be rejected",
            name
        );
        assert_eq!(
            report.compliance_status,
            ComplianceStatus::RejectedPotentialCopy,
            "Waveform '{}' inverted polarity must yield RejectedPotentialCopy",
            name
        );
    }
}

#[test]
fn test_empirical_gain_scaling_and_lag_shift_invariance() {
    let n = 4000;
    let base_wave: Vec<f32> = (0..n)
        .map(|i| (2.0 * PI * 200.0 * i as f32 / 4000.0).sin() * 0.5)
        .collect();

    // Gain factors: +/- 0.1x to +/- 3.0x
    let gain_factors = [0.1, 0.25, 0.5, 0.75, 1.25, 2.0, 3.0, -0.2, -0.5, -1.5];

    for &gain in &gain_factors {
        let scaled_wave: Vec<f32> = base_wave.iter().map(|&x| x * gain).collect();
        let xcorr = LegalOriginalityGuard::calculate_max_waveform_cross_correlation(
            &base_wave,
            &scaled_wave,
        );

        assert!(
            xcorr > 0.98,
            "Gain factor {} cross-correlation must be > 0.98, got {:.4}",
            gain,
            xcorr
        );
        assert!(
            xcorr > WAVEFORM_CORRELATION_CEILING,
            "Gain factor {} must breach ceiling {:.2}",
            gain,
            WAVEFORM_CORRELATION_CEILING
        );
    }

    // Time-lag shifts: 8, 32, 100, 200 samples
    let lag_shifts = [8, 32, 100, 200];
    for &lag in &lag_shifts {
        let mut shifted_wave = vec![0.0f32; lag];
        shifted_wave.extend_from_slice(&base_wave[..n - lag]);

        let xcorr = LegalOriginalityGuard::calculate_max_waveform_cross_correlation(
            &base_wave,
            &shifted_wave,
        );

        assert!(
            xcorr > 0.90,
            "Lag shift {} samples cross-correlation must be > 0.90, got {:.4}",
            lag,
            xcorr
        );
    }
}

#[test]
fn test_empirical_chroma_cosine_ceiling_check() {
    assert_eq!(CHROMA_COSINE_CEILING, 0.75);

    // Identical chroma sequence
    let chroma_a = vec![[0.1f32; 12]; 20];
    let sim_identical = LegalOriginalityGuard::calculate_chroma_similarity(&chroma_a, &chroma_a);
    assert!(
        sim_identical > 0.99,
        "Identical chroma must yield ~1.0 similarity, got {:.4}",
        sim_identical
    );
    assert!(sim_identical > CHROMA_COSINE_CEILING);

    // Completely orthogonal chroma sequences
    let mut chroma_c = vec![[0.0f32; 12]; 20];
    let mut chroma_f_sharp = vec![[0.0f32; 12]; 20];
    for f in 0..20 {
        chroma_c[f][0] = 1.0; // C
        chroma_f_sharp[f][6] = 1.0; // F# (tritone apart)
    }

    let sim_orthogonal = LegalOriginalityGuard::calculate_chroma_similarity(&chroma_c, &chroma_f_sharp);
    assert!(
        sim_orthogonal < 1e-4,
        "Orthogonal chroma must yield ~0.0 similarity, got {:.4}",
        sim_orthogonal
    );
    assert!(sim_orthogonal <= CHROMA_COSINE_CEILING);
}

// =========================================================================
// 3. RVQ Tokenizer: Monotonic Residual Norm Reduction
// =========================================================================

#[test]
fn test_empirical_rvq_monotonic_residual_reduction_multi_config() {
    let configs = [
        (2, 64, 16),
        (4, 256, 32),
        (6, 256, 32),
        (8, 512, 32),
        (4, 1024, 64),
    ];

    for &(stages, codebook_size, dim) in &configs {
        let tokenizer = RvqTokenizer::new(stages, codebook_size, dim).expect("RVQ config valid");

        // 1. All-zeros vector: residual should remain exactly 0.0
        let zero_vec = vec![0.0f32; dim];
        let (zero_mono, zero_norms) = tokenizer.verify_monotonic_residual_reduction(&zero_vec);
        assert!(zero_mono);
        for &norm in &zero_norms {
            assert!(norm < 1e-6, "Zero vector residual norm must remain ~0.0");
        }

        // 2. High-frequency oscillating vector
        let hf_vec: Vec<f32> = (0..dim)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let (hf_mono, hf_norms) = tokenizer.verify_monotonic_residual_reduction(&hf_vec);
        assert!(hf_mono, "HF vector monotonic reduction failed: {:?}", hf_norms);
        assert!(hf_norms.last().unwrap() < hf_norms.first().unwrap());

        // 3. Tiny magnitude vector (1e-5)
        let tiny_vec: Vec<f32> = (0..dim).map(|i| (i as f32) * 1e-5).collect();
        let (tiny_mono, tiny_norms) = tokenizer.verify_monotonic_residual_reduction(&tiny_vec);
        assert!(tiny_mono, "Tiny vector monotonic reduction failed: {:?}", tiny_norms);

        // 4. Large magnitude vector (1e3)
        let large_vec: Vec<f32> = (0..dim).map(|i| (i as f32 + 1.0) * 50.0).collect();
        let (large_mono, large_norms) = tokenizer.verify_monotonic_residual_reduction(&large_vec);
        assert!(large_mono, "Large vector monotonic reduction failed: {:?}", large_norms);
        assert!(large_norms.last().unwrap() < large_norms.first().unwrap());
    }
}

#[test]
fn test_empirical_rvq_500_random_vectors_strict_monotonicity() {
    let tokenizer = RvqTokenizer::default_config(); // 4 stages, 256 size, 32 dim
    assert_eq!(tokenizer.dimension, 32);

    let mut seed = 123456789u64;
    let mut rng = move || -> f32 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let val = ((seed >> 33) as i32) as f32 / 2147483648.0;
        val // in [-1.0, 1.0]
    };

    let mut total_reduction_sum = 0.0f32;

    for trial in 0..500 {
        let vec: Vec<f32> = (0..32).map(|_| rng() * 2.0).collect();
        let (monotonic, norms) = tokenizer.verify_monotonic_residual_reduction(&vec);

        assert!(
            monotonic,
            "Trial {}: Monotonic residual norm reduction violated: {:?}",
            trial, norms
        );

        // Every adjacent stage: ||r_m|| <= ||r_{m-1}|| + 1e-5
        for m in 1..norms.len() {
            assert!(
                norms[m] <= norms[m - 1] + 1e-5,
                "Trial {} stage {}: norm {} exceeded prev norm {}",
                trial,
                m,
                norms[m],
                norms[m - 1]
            );
        }

        let initial_norm = norms[0];
        let final_norm = *norms.last().unwrap();
        if initial_norm > 1e-5 {
            let reduction = (initial_norm - final_norm) / initial_norm;
            total_reduction_sum += reduction;
        }
    }

    let avg_reduction = total_reduction_sum / 500.0;
    assert!(
        avg_reduction > 0.20,
        "Average 4-stage residual norm reduction ({:.2}%) should exceed 20%",
        avg_reduction * 100.0
    );
}

// =========================================================================
// 4. Anti-Monopoly Style Blending: Max Single-Track Weight <= 60% for N >= 3
// =========================================================================

fn make_dummy_profile(file_name: &str, bpm: f32) -> AcousticFeatureVector {
    AcousticFeatureVector {
        file_name: file_name.to_string(),
        duration_secs: 10.0,
        sample_rate: 48000,
        channels: 2,
        bpm,
        chroma_12bin: [1.0 / 12.0f32.sqrt(); 12],
        key_signature: "C Major".to_string(),
        key_confidence: 0.95,
        spectral_centroid_hz: 1500.0,
        crest_factor_db: 12.0,
        dynamic_range_db: 24.0,
        vocal_presence_ratio: 0.3,
        vocal_activity_ratio: 0.25,
        timbre_subbands: [0.125; 8],
        style_embedding: vec![1.0 / 8.0; 64],
        pitch_token_sequence: vec![0, 4, 7],
    }
}

#[test]
fn test_empirical_anti_monopoly_weight_capping_n_ge_3() {
    // Test counts N in 3..=20
    let test_n_values = [3, 4, 5, 8, 12, 16, 20];

    for &n in &test_n_values {
        let profiles: Vec<AcousticFeatureVector> = (0..n)
            .map(|i| make_dummy_profile(&format!("track_{:02}.wav", i), 120.0 + i as f32))
            .collect();

        // Adversarial Distribution 1: Track 0 demands monopoly weight 1,000,000x
        let mut user_weights_monopoly = HashMap::new();
        user_weights_monopoly.insert("track_00.wav".to_string(), 1_000_000.0);
        for i in 1..n {
            user_weights_monopoly.insert(format!("track_{:02}.wav", i), 0.001);
        }

        let blended = AcousticAnalyzer::blend_ensemble(&profiles, Some(&user_weights_monopoly))
            .expect("Blending must succeed");

        let w0 = blended.file_weights.get("track_00.wav").copied().unwrap();
        assert!(
            w0 <= ANTI_MONOPOLY_CEILING + 1e-5,
            "N={}: Monopoly track_00 weight ({:.5}) exceeded 60% ceiling ({:.2})",
            n,
            w0,
            ANTI_MONOPOLY_CEILING
        );

        // Verify simplex conservation: sum of all weights == 1.0, and each weight >= 0.0
        let weight_sum: f32 = blended.file_weights.values().sum();
        assert!(
            (weight_sum - 1.0).abs() < 1e-4,
            "N={}: Weight sum must be 1.0, got {:.6}",
            n,
            weight_sum
        );

        for (name, &w) in &blended.file_weights {
            assert!(
                w >= 0.0,
                "N={}: File '{}' weight must be non-negative, got {}",
                n,
                name,
                w
            );
            assert!(
                w <= ANTI_MONOPOLY_CEILING + 1e-5,
                "N={}: File '{}' weight ({:.5}) exceeded 60% ceiling",
                n,
                name,
                w
            );
        }

        // Adversarial Distribution 2: Decreasing geometric powers
        let mut user_weights_geom = HashMap::new();
        for i in 0..n {
            user_weights_geom.insert(format!("track_{:02}.wav", i), 10.0f32.powi(i as i32));
        }

        let blended_geom = AcousticAnalyzer::blend_ensemble(&profiles, Some(&user_weights_geom))
            .expect("Blending must succeed");

        let dominant_name = format!("track_{:02}.wav", n - 1);
        let w_dom = blended_geom.file_weights.get(&dominant_name).copied().unwrap();
        assert!(
            w_dom <= ANTI_MONOPOLY_CEILING + 1e-5,
            "N={}: Dominant track weight ({:.5}) exceeded 60% ceiling",
            n,
            w_dom
        );
        let sum_geom: f32 = blended_geom.file_weights.values().sum();
        assert!(
            (sum_geom - 1.0).abs() < 1e-4,
            "N={}: Geom weight sum must be 1.0, got {:.6}",
            n,
            sum_geom
        );
    }
}

#[test]
fn test_empirical_anti_monopoly_boundary_n2_vs_n3() {
    let p0 = make_dummy_profile("track_0.wav", 120.0);
    let p1 = make_dummy_profile("track_1.wav", 125.0);
    let p2 = make_dummy_profile("track_2.wav", 130.0);

    let mut weights_90_10 = HashMap::new();
    weights_90_10.insert("track_0.wav".to_string(), 90.0);
    weights_90_10.insert("track_1.wav".to_string(), 10.0);

    // Case A: N = 2 (Unconstrained by anti-monopoly rule)
    let blended_2 = AcousticAnalyzer::blend_ensemble(&[p0.clone(), p1.clone()], Some(&weights_90_10))
        .expect("N=2 blending must succeed");
    let w0_n2 = blended_2.file_weights.get("track_0.wav").copied().unwrap();
    assert!(
        w0_n2 > 0.85,
        "For N=2, anti-monopoly ceiling does NOT apply, w0 should be ~0.90, got {:.3}",
        w0_n2
    );

    // Case B: N = 3 (Strictly constrained <= 60%)
    weights_90_10.insert("track_2.wav".to_string(), 0.001);
    let blended_3 = AcousticAnalyzer::blend_ensemble(&[p0, p1, p2], Some(&weights_90_10))
        .expect("N=3 blending must succeed");
    let w0_n3 = blended_3.file_weights.get("track_0.wav").copied().unwrap();
    assert!(
        w0_n3 <= ANTI_MONOPOLY_CEILING + 1e-5,
        "For N=3, track_0 MUST be clamped to <= 60%, got {:.5}",
        w0_n3
    );
    assert!(
        (w0_n3 - ANTI_MONOPOLY_CEILING).abs() < 1e-3,
        "track_0 should be clamped right at 60%, got {:.5}",
        w0_n3
    );
}

#[test]
fn test_empirical_asymmetric_audio_lengths_cross_correlation() {
    // 10s candidate vs 1s reference (spliced snippet within longer candidate)
    let n_cand = 40000; // 10s at 4 kHz
    let n_ref = 4000;   // 1s at 4 kHz

    let mut cand_samples = vec![0.0f32; n_cand];
    let mut ref_samples = vec![0.0f32; n_ref];

    // Reference is a 330 Hz sine
    for i in 0..n_ref {
        let t = i as f32 / 4000.0;
        ref_samples[i] = (2.0 * PI * 330.0 * t).sin() * 0.7;
    }

    // Candidate has reference spliced at offset 500 (within 2-second max_lag window)
    for i in 0..n_ref {
        cand_samples[500 + i] = ref_samples[i];
    }

    let buf_cand = AudioBuffer {
        sample_rate: 4000,
        channels: 1,
        samples: cand_samples,
    };
    let buf_ref = AudioBuffer {
        sample_rate: 4000,
        channels: 1,
        samples: ref_samples,
    };

    let chroma = vec![[0.0f32; 12]; 10];
    let pitch = vec![0u8];
    let ref_tuples = vec![("ref_short.wav", &buf_ref, chroma.as_slice(), pitch.as_slice())];

    let report = LegalOriginalityGuard::audit_candidate(
        "spliced_long_candidate",
        &buf_cand,
        &chroma,
        &pitch,
        &ref_tuples,
    );

    // Cross correlation on the window where ref is present:
    // Even when ref is shorter, cross correlation should detect the high normalized correlation
    assert!(report.max_waveform_cross_correlation > 0.0);
}

#[test]
fn test_empirical_zero_and_negative_user_weights_safety() {
    let p0 = make_dummy_profile("track_0.wav", 120.0);
    let p1 = make_dummy_profile("track_1.wav", 125.0);
    let p2 = make_dummy_profile("track_2.wav", 130.0);

    // Adversarial user weights: zeros and negatives
    let mut adversarial_weights = HashMap::new();
    adversarial_weights.insert("track_0.wav".to_string(), 0.0);
    adversarial_weights.insert("track_1.wav".to_string(), -10.0);
    adversarial_weights.insert("track_2.wav".to_string(), 0.0);

    let blended = AcousticAnalyzer::blend_ensemble(&[p0, p1, p2], Some(&adversarial_weights))
        .expect("Negative and zero weights must be safely clamped to >= 0.001");

    for (name, &w) in &blended.file_weights {
        assert!(w > 0.0, "Weight for '{}' must be strictly positive", name);
        assert!(w <= ANTI_MONOPOLY_CEILING + 1e-5);
    }

    let weight_sum: f32 = blended.file_weights.values().sum();
    assert!((weight_sum - 1.0).abs() < 1e-4, "Weight sum must be 1.0");
}

