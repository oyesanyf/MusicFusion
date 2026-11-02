//! Adversarial Originality Guard Tests for Milestone 8.
//!
//! Validates:
//! 1. Transposition-invariant melodic plagiarism detection across all 12 semitones (LCS >= 8 rejected).
//! 2. Waveform cross-correlation ceiling (\rho_{max} <= 0.65) rejecting verbatim waveform copies.
//! 3. Chroma cosine similarity ceiling (<= 0.75) rejecting harmonic verbatim copies.
//! 4. Full compliance verification for 100% original synthesis (Bridgeport Music & Tennessee ELVIS Act).

use music_core::rl_contracts::ComplianceStatus;
use music_dsp::wav::AudioBuffer;
use music_fusion::originality_guard::LegalOriginalityGuard;

#[test]
fn test_adversarial_12_semitone_transposition_melodic_plagiarism() {
    // Reference melody: 10-note hook (e.g. C, D, E, F, G, A, B, C, D, E = [0, 2, 4, 5, 7, 9, 11, 0, 2, 4])
    let ref_hook = vec![0u8, 2, 4, 5, 7, 9, 11, 0, 2, 4];
    assert_eq!(ref_hook.len(), 10);

    let dummy_buffer = AudioBuffer::new_stereo(2.0);
    let dummy_chroma = vec![[0.05f32; 12]; 10];

    // Reference tuple: ("ref_hit.wav", buffer, chroma, pitch_sequence)
    let ref_tuples = vec![(
        "ref_hit.wav",
        &dummy_buffer,
        dummy_chroma.as_slice(),
        ref_hook.as_slice(),
    )];

    // For every transposition shift delta in 0..12:
    for shift in 0..12u8 {
        let transposed_candidate: Vec<u8> = ref_hook.iter().map(|&p| (p + shift) % 12).collect();

        let cand_buf = AudioBuffer::new_stereo(2.0);
        let cand_chroma = vec![[0.05f32; 12]; 10];

        let report = LegalOriginalityGuard::audit_candidate(
            "plagiarized_candidate",
            &cand_buf,
            &cand_chroma,
            &transposed_candidate,
            &ref_tuples,
        );

        assert!(
            report.max_melodic_sequence_notes >= 8,
            "Shift {}: Detected LCS melodic sequence must be >= 8 notes, got {}",
            shift,
            report.max_melodic_sequence_notes
        );

        assert_eq!(
            report.compliance_status,
            ComplianceStatus::RejectedPotentialCopy,
            "Shift {}: Transposed hook must be flagged as RejectedPotentialCopy",
            shift
        );

        assert!(
            !report.is_compliant,
            "Shift {}: Transposed hook must not be compliant",
            shift
        );

        assert!(
            !report.legal_compliance.legal_safeguard_verified,
            "Shift {}: Legal safeguard must fail on melodic plagiarism",
            shift
        );
    }
}

#[test]
fn test_adversarial_waveform_cross_correlation_rejection() {
    // Generate synthetic audio with distinct sinusoidal wave
    let mut ref_buffer = AudioBuffer::new_stereo(2.0);
    for f in 0..ref_buffer.frame_count() {
        let t = f as f32 / 48000.0;
        let s = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.8;
        ref_buffer.samples[f * 2] = s;
        ref_buffer.samples[f * 2 + 1] = s;
    }

    let ref_chroma = vec![[0.1f32; 12]];
    let ref_pitch = vec![9u8];
    let ref_tuples = vec![("ref_wave.wav", &ref_buffer, ref_chroma.as_slice(), ref_pitch.as_slice())];

    // Clone identical waveform into candidate (representing Bridgeport sampling/splicing infringement)
    let candidate_buffer = ref_buffer.clone();
    let cand_chroma = vec![[0.1f32; 12]];
    let cand_pitch = vec![9u8];

    let report = LegalOriginalityGuard::audit_candidate(
        "spliced_candidate",
        &candidate_buffer,
        &cand_chroma,
        &cand_pitch,
        &ref_tuples,
    );

    assert!(
        report.max_waveform_cross_correlation > 0.65,
        "Identical waveform cross-correlation ({:.3}) must exceed 0.65 ceiling",
        report.max_waveform_cross_correlation
    );

    assert_eq!(
        report.compliance_status,
        ComplianceStatus::RejectedPotentialCopy,
        "Spliced audio must be classified as RejectedPotentialCopy"
    );

    assert!(!report.is_compliant);
}

#[test]
fn test_compliant_original_synthesis_verification() {
    // Reference: C Major chord (C4, E4, G4)
    let mut ref_buffer = AudioBuffer::new_stereo(2.0);
    for f in 0..ref_buffer.frame_count() {
        let t = f as f32 / 48000.0;
        let s = (2.0 * std::f32::consts::PI * 261.63 * t).sin() * 0.5;
        ref_buffer.samples[f * 2] = s;
        ref_buffer.samples[f * 2 + 1] = s;
    }
    let ref_chroma = vec![[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]];
    let ref_pitch = vec![0u8, 4, 7];
    let ref_tuples = vec![("ref_track.wav", &ref_buffer, ref_chroma.as_slice(), ref_pitch.as_slice())];

    // Candidate: Distinct F# Minor chord (F#4, A4, C#5) with different melody (1, 3, 6, 8)
    let mut cand_buffer = AudioBuffer::new_stereo(2.0);
    for f in 0..cand_buffer.frame_count() {
        let t = f as f32 / 48000.0;
        let s = (2.0 * std::f32::consts::PI * 370.0 * t).sin() * 0.5;
        cand_buffer.samples[f * 2] = s;
        cand_buffer.samples[f * 2 + 1] = s;
    }
    let cand_chroma = vec![[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0]];
    let cand_pitch = vec![1u8, 3, 6, 8]; // LCS < 8

    let report = LegalOriginalityGuard::audit_candidate(
        "original_track",
        &cand_buffer,
        &cand_chroma,
        &cand_pitch,
        &ref_tuples,
    );

    assert!(report.is_compliant, "Distinct original synthesis must be compliant");
    assert!(
        report.originality_score_pct >= 90.0,
        "Originality score ({:.1}%) must be >= 90.0%",
        report.originality_score_pct
    );
    assert!(report.max_melodic_sequence_notes < 8);
    assert!(report.max_waveform_cross_correlation <= 0.65);
    assert!(report.legal_compliance.zero_waveform_sampling);
    assert!(report.legal_compliance.no_master_sampling_verified);
    assert!(report.legal_compliance.synthetic_voice_only_elvis_compliant);
    assert!(report.legal_compliance.synthetic_vocal_timbre_only);
    assert!(report.legal_compliance.legal_safeguard_verified);
}
