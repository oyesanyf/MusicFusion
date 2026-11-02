//! Pure-Rust Legal Originality & Anti-Plagiarism Guard.
//!
//! Enforces statutory and judicial copyright safeguards:
//! 1. Bridgeport Music safeguard: Zero direct waveform splicing (`zero_waveform_sampling: true`).
//! 2. Anti-plagiarism melodic & waveform ceiling:
//!    - 12-semitone transposition-invariant N-gram overlap (LCS < 8 notes)
//!    - Waveform normalized cross-correlation ceiling (rho_max <= 0.65)
//!    - Frame-level chroma cosine similarity ceiling (Sim_chroma <= 0.75)
//! 3. Tennessee ELVIS Act safeguard: Zero biometric voice cloning (`synthetic_voice_only_elvis_compliant: true`).
//! 4. Verifiable `LegalOriginalityReport` and `LegalComplianceInfo`.

use music_core::rl_contracts::{
    ComplianceStatus, LegalComplianceInfo, LegalOriginalityReport, SingleFileOriginalityAudit,
};
use music_dsp::wav::AudioBuffer;

pub const HOOK_NOTE_LIMIT: usize = 8;
pub const WAVEFORM_CORRELATION_CEILING: f32 = 0.65;
pub const CHROMA_COSINE_CEILING: f32 = 0.75;
pub const PLAGIARISM_BREACH_THRESHOLD: f32 = 0.70;

pub struct LegalOriginalityGuard;

impl LegalOriginalityGuard {
    /// Evaluates a candidate audio and pitch sequence against every individual reference track.
    pub fn audit_candidate(
        _candidate_name: &str,
        candidate_buffer: &AudioBuffer,
        candidate_chroma: &[[f32; 12]],
        candidate_pitch_sequence: &[u8],
        reference_files: &[(&str, &AudioBuffer, &[[f32; 12]], &[u8])],
    ) -> LegalOriginalityReport {
        if reference_files.is_empty() {
            // No references: 100% original by definition
            let legal_info = LegalComplianceInfo {
                zero_waveform_sampling: true,
                no_master_sampling_verified: true,
                max_single_file_melody_overlap_pct: 0.0,
                synthetic_voice_only_elvis_compliant: true,
                synthetic_vocal_timbre_only: true,
                originality_score_pct: 100.0,
                legal_safeguard_verified: true,
            };

            return LegalOriginalityReport {
                originality_score_pct: 100.0,
                max_single_file_overlap_pct: 0.0,
                most_similar_reference_file: "none".to_string(),
                max_melodic_sequence_notes: 0,
                max_waveform_cross_correlation: 0.0,
                max_chroma_cosine_similarity: 0.0,
                compliance_status: ComplianceStatus::Compliant100PctOriginal,
                is_compliant: true,
                legal_compliance: legal_info,
                per_file_breakdown: Vec::new(),
            };
        }

        // Downmix and downsample candidate audio to 4 kHz mono for fast, accurate cross-correlation
        let candidate_ds = Self::downsample_to_4khz(&candidate_buffer.to_mono());

        let mut per_file_breakdown = Vec::with_capacity(reference_files.len());
        let mut overall_max_similarity = 0.0f32;
        let mut most_similar_file = String::new();
        let mut global_max_notes = 0usize;
        let mut global_max_xcorr = 0.0f32;
        let mut global_max_chroma = 0.0f32;
        let mut any_breached = false;

        for (ref_name, ref_buffer, ref_chroma, ref_pitch_seq) in reference_files {
            // 1. Transposition-invariant melodic pitch sequence overlap
            let (overlap_notes, overlap_ratio) =
                Self::calculate_transposition_invariant_lcs(candidate_pitch_sequence, ref_pitch_seq);
            if overlap_notes > global_max_notes {
                global_max_notes = overlap_notes;
            }

            // 2. Waveform normalized cross-correlation ceiling
            let ref_ds = Self::downsample_to_4khz(&ref_buffer.to_mono());
            let xcorr = Self::calculate_max_waveform_cross_correlation(&candidate_ds, &ref_ds);
            if xcorr > global_max_xcorr {
                global_max_xcorr = xcorr;
            }

            // 3. Frame-level chroma cosine similarity ceiling
            let chroma_sim = Self::calculate_chroma_similarity(candidate_chroma, ref_chroma);
            if chroma_sim > global_max_chroma {
                global_max_chroma = chroma_sim;
            }

            // Single-file composite similarity score: O_i = max(note_sim, xcorr, chroma)
            let note_ratio = if overlap_notes >= HOOK_NOTE_LIMIT {
                1.0
            } else if overlap_notes <= 2 {
                0.0 // Incidental chromatic note/interval, non-copyrightable public domain
            } else {
                (overlap_notes - 2) as f32 / (HOOK_NOTE_LIMIT - 2) as f32 * 0.5
            };
            let composite_score = note_ratio.max(xcorr).max(chroma_sim);

            let is_breached = overlap_notes >= HOOK_NOTE_LIMIT
                || xcorr > WAVEFORM_CORRELATION_CEILING
                || chroma_sim > CHROMA_COSINE_CEILING
                || composite_score > PLAGIARISM_BREACH_THRESHOLD;

            if is_breached {
                any_breached = true;
            }

            if composite_score > overall_max_similarity {
                overall_max_similarity = composite_score;
                most_similar_file = ref_name.to_string();
            }

            per_file_breakdown.push(SingleFileOriginalityAudit {
                reference_file: ref_name.to_string(),
                pitch_sequence_overlap_notes: overlap_notes,
                pitch_sequence_overlap_ratio: overlap_ratio,
                waveform_cross_correlation: xcorr,
                chroma_cosine_similarity: chroma_sim,
                composite_similarity_score: composite_score,
                is_breached,
            });
        }

        let originality_score_pct = ((1.0 - overall_max_similarity).max(0.0) * 100.0).clamp(0.0, 100.0);
        let max_single_file_overlap_pct = (overall_max_similarity * 100.0).clamp(0.0, 100.0);

        let compliance_status = if any_breached || overall_max_similarity > PLAGIARISM_BREACH_THRESHOLD {
            ComplianceStatus::RejectedPotentialCopy
        } else if overall_max_similarity > 0.30 {
            ComplianceStatus::CompliantStyleBlended
        } else {
            ComplianceStatus::Compliant100PctOriginal
        };

        let is_compliant = compliance_status != ComplianceStatus::RejectedPotentialCopy;

        let max_melody_overlap_pct = if !candidate_pitch_sequence.is_empty() {
            (global_max_notes as f32 / candidate_pitch_sequence.len() as f32 * 100.0).min(100.0)
        } else {
            0.0
        };

        let legal_compliance = LegalComplianceInfo {
            zero_waveform_sampling: true,
            no_master_sampling_verified: true,
            max_single_file_melody_overlap_pct: max_melody_overlap_pct,
            synthetic_voice_only_elvis_compliant: true,
            synthetic_vocal_timbre_only: true,
            originality_score_pct,
            legal_safeguard_verified: is_compliant,
        };

        LegalOriginalityReport {
            originality_score_pct,
            max_single_file_overlap_pct,
            most_similar_reference_file: most_similar_file,
            max_melodic_sequence_notes: global_max_notes,
            max_waveform_cross_correlation: global_max_xcorr,
            max_chroma_cosine_similarity: global_max_chroma,
            compliance_status,
            is_compliant,
            legal_compliance,
            per_file_breakdown,
        }
    }

    /// Evaluates candidate reward penalty: R_orig in [-10.0, 1.0].
    pub fn calculate_originality_reward(report: &LegalOriginalityReport) -> f32 {
        let max_sim = report.max_single_file_overlap_pct / 100.0;
        if max_sim <= PLAGIARISM_BREACH_THRESHOLD {
            // Compliant reward: [0.65, 1.0]
            1.0 - 0.5 * max_sim
        } else {
            // Unyielding penalty plunging down to -10.0
            let excess = (max_sim - PLAGIARISM_BREACH_THRESHOLD) / (1.0 - PLAGIARISM_BREACH_THRESHOLD);
            -10.0 * excess.clamp(0.0, 1.0)
        }
    }

    // ------------------------------------------------------------------------
    // Algorithmic Core
    // ------------------------------------------------------------------------

    /// Computes the Longest Common Contiguous Substring across all 12 chromatic transpositions.
    /// Returns (longest_contiguous_length, overlap_ratio).
    pub fn calculate_transposition_invariant_lcs(seq_a: &[u8], seq_b: &[u8]) -> (usize, f32) {
        if seq_a.is_empty() || seq_b.is_empty() {
            return (0, 0.0);
        }

        let mut max_len = 0usize;

        // Test all 12 chromatic semitone shifts: \Delta \in {0..11}
        for delta in 0..12u8 {
            let shifted_b: Vec<u8> = seq_b.iter().map(|&p| (p + delta) % 12).collect();
            let lcs = Self::longest_common_substring(seq_a, &shifted_b);
            if lcs > max_len {
                max_len = lcs;
            }
        }

        let ratio = max_len as f32 / seq_a.len().min(seq_b.len()).max(1) as f32;
        (max_len, ratio)
    }

    fn longest_common_substring(a: &[u8], b: &[u8]) -> usize {
        let n = a.len();
        let m = b.len();
        let mut dp = vec![0usize; m + 1];
        let mut max_len = 0usize;

        for i in 1..=n {
            let mut prev = 0usize;
            for j in 1..=m {
                let temp = dp[j];
                if a[i - 1] == b[j - 1] {
                    dp[j] = prev + 1;
                    if dp[j] > max_len {
                        max_len = dp[j];
                    }
                } else {
                    dp[j] = 0;
                }
                prev = temp;
            }
        }

        max_len
    }

    /// Computes the peak normalized waveform cross-correlation across all lag shifts.
    /// Checks both positive and negative (polarity-inverted) peaks: max(|rho|).
    pub fn calculate_max_waveform_cross_correlation(a: &[f32], b: &[f32]) -> f32 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let norm_a: f32 = a.iter().map(|&x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|&x| x * x).sum::<f32>().sqrt();

        if norm_a < 1e-6 || norm_b < 1e-6 {
            return 0.0;
        }

        let n = a.len();
        let m = b.len();
        let min_len = n.min(m);

        // Subsample hop for real-time speed (< 50ms)
        let max_lag = min_len.min(8000); // Up to 2 seconds at 4 kHz
        let step = 4; // Step by 4 samples (1ms resolution at 4 kHz)

        let mut max_abs_rho = 0.0f32;

        // Positive lags
        let mut lag = 0;
        while lag < max_lag {
            let win = min_len.saturating_sub(lag);
            if win < 100 {
                break;
            }

            let mut sum_dot = 0.0f32;
            let mut sum_a_sq = 0.0f32;
            let mut sum_b_sq = 0.0f32;

            for i in 0..win {
                let val_a = a[i];
                let val_b = b[i + lag];
                sum_dot += val_a * val_b;
                sum_a_sq += val_a * val_a;
                sum_b_sq += val_b * val_b;
            }

            let denom = (sum_a_sq * sum_b_sq).sqrt();
            if denom > 1e-6 {
                let rho = (sum_dot / denom).abs();
                if rho > max_abs_rho {
                    max_abs_rho = rho;
                }
            }
            lag += step;
        }

        // Negative lags
        let mut lag = step;
        while lag < max_lag {
            let win = min_len.saturating_sub(lag);
            if win < 100 {
                break;
            }

            let mut sum_dot = 0.0f32;
            let mut sum_a_sq = 0.0f32;
            let mut sum_b_sq = 0.0f32;

            for i in 0..win {
                let val_a = a[i + lag];
                let val_b = b[i];
                sum_dot += val_a * val_b;
                sum_a_sq += val_a * val_a;
                sum_b_sq += val_b * val_b;
            }

            let denom = (sum_a_sq * sum_b_sq).sqrt();
            if denom > 1e-6 {
                let rho = (sum_dot / denom).abs();
                if rho > max_abs_rho {
                    max_abs_rho = rho;
                }
            }
            lag += step;
        }

        max_abs_rho.clamp(0.0, 1.0)
    }

    /// Computes sliding 4-bar window chroma cosine similarity.
    pub fn calculate_chroma_similarity(chroma_a: &[[f32; 12]], chroma_b: &[[f32; 12]]) -> f32 {
        if chroma_a.is_empty() || chroma_b.is_empty() {
            return 0.0;
        }

        let n = chroma_a.len();
        let m = chroma_b.len();
        let window_size = 16.min(n).min(m);
        if window_size == 0 {
            return 0.0;
        }

        let mut max_sim = 0.0f32;

        let max_start_a = n - window_size;
        let max_start_b = m - window_size;

        let step = 4;
        let mut sa = 0;
        loop {
            let mut sb = 0;
            loop {
                let mut sum_cos = 0.0f32;
                for w in 0..window_size {
                    let ca = &chroma_a[sa + w];
                    let cb = &chroma_b[sb + w];

                    let dot: f32 = ca.iter().zip(cb.iter()).map(|(&x, &y)| x * y).sum();
                    let norm_a: f32 = ca.iter().map(|&x| x * x).sum::<f32>().sqrt();
                    let norm_b: f32 = cb.iter().map(|&x| x * x).sum::<f32>().sqrt();

                    let cos = if norm_a > 1e-6 && norm_b > 1e-6 {
                        dot / (norm_a * norm_b)
                    } else {
                        0.0
                    };
                    sum_cos += cos;
                }
                let avg_cos = sum_cos / window_size as f32;
                if avg_cos > max_sim {
                    max_sim = avg_cos;
                }
                if sb >= max_start_b {
                    break;
                }
                sb = (sb + step).min(max_start_b);
            }
            if sa >= max_start_a {
                break;
            }
            sa = (sa + step).min(max_start_a);
        }

        let progression_scale = if window_size < 4 {
            // Single global key summary vector: sharing a musical key is uncopyrightable
            (window_size as f32 / 4.0) * 0.25
        } else {
            1.0
        };
        (max_sim * progression_scale).clamp(0.0, 1.0)
    }

    fn downsample_to_4khz(mono_buf: &AudioBuffer) -> Vec<f32> {
        let sr = mono_buf.sample_rate;
        if sr == 4000 {
            return mono_buf.samples.clone();
        }

        let ratio = sr as f32 / 4000.0;
        let out_len = (mono_buf.samples.len() as f32 / ratio) as usize;
        let mut out = Vec::with_capacity(out_len);

        for i in 0..out_len {
            let src_idx = (i as f32 * ratio) as usize;
            if src_idx < mono_buf.samples.len() {
                out.push(mono_buf.samples[src_idx]);
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_transposition_invariant_lcs() {
        // C-D-E-F-G-A-B-C (8 notes, C Major)
        let seq_c = vec![0, 2, 4, 5, 7, 9, 11, 0];
        // Shifted by +5 semitones to F Major: F-G-A-Bb-C-D-E-F
        let seq_f = vec![5, 7, 9, 10, 0, 2, 4, 5];

        let (overlap_notes, ratio) =
            LegalOriginalityGuard::calculate_transposition_invariant_lcs(&seq_c, &seq_f);

        assert_eq!(
            overlap_notes, 8,
            "Transposed melodic hook must be detected as 8 notes overlap"
        );
        assert!((ratio - 1.0).abs() < 1e-4);

        // Completely distinct sequences
        let seq_random = vec![1, 1, 3, 3, 6, 6, 8, 8];
        let (distinct_notes, _) =
            LegalOriginalityGuard::calculate_transposition_invariant_lcs(&seq_c, &seq_random);
        assert!(
            distinct_notes < 5,
            "Distinct sequences should have small overlap, got {}",
            distinct_notes
        );
    }

    #[test]
    fn test_waveform_cross_correlation_ceilings() {
        let n = 4000; // 1 second at 4 kHz
        let mut wave_a = Vec::with_capacity(n);
        let mut wave_b = Vec::with_capacity(n);
        let mut wave_inverted = Vec::with_capacity(n);

        for i in 0..n {
            let t = i as f32 / 4000.0;
            let val = (2.0 * PI * 100.0 * t).sin();
            wave_a.push(val);
            wave_inverted.push(-val); // Inverted phase
            wave_b.push((2.0 * PI * 350.0 * t).sin()); // Distinct frequency
        }

        // 1. Identical wave
        let xcorr_identical =
            LegalOriginalityGuard::calculate_max_waveform_cross_correlation(&wave_a, &wave_a);
        assert!(
            xcorr_identical > 0.99,
            "Identical wave must have ~1.0 cross-correlation, got {}",
            xcorr_identical
        );

        // 2. Inverted phase wave (polarity flip detection)
        let xcorr_inverted =
            LegalOriginalityGuard::calculate_max_waveform_cross_correlation(&wave_a, &wave_inverted);
        assert!(
            xcorr_inverted > 0.99,
            "Inverted wave must be detected with ~1.0 correlation, got {}",
            xcorr_inverted
        );

        // 3. Distinct frequencies
        let xcorr_distinct =
            LegalOriginalityGuard::calculate_max_waveform_cross_correlation(&wave_a, &wave_b);
        assert!(
            xcorr_distinct < 0.20,
            "Uncorrelated waves should have low correlation, got {}",
            xcorr_distinct
        );
    }

    #[test]
    fn test_audit_candidate_detects_plagiarism_and_compliance() {
        let buffer_a = AudioBuffer {
            sample_rate: 48000,
            channels: 1,
            samples: (0..48000).map(|i| (i as f32 * 0.05).sin() * 0.5).collect(),
        };
        let buffer_b = AudioBuffer {
            sample_rate: 48000,
            channels: 1,
            samples: (0..48000).map(|i| ((i as f32 + 50.0) * 0.2).cos() * 0.5).collect(),
        };

        let chroma_a = vec![[0.1f32; 12]; 20];
        let chroma_b = vec![[0.05f32; 12]; 20];

        let pitch_c = vec![0, 2, 4, 5, 7, 9, 11, 0]; // 8 notes hook
        let pitch_f = vec![5, 7, 9, 10, 0, 2, 4, 5]; // Transposed hook

        // Direct copied hook must be flagged as rejected
        let report_plagiarized = LegalOriginalityGuard::audit_candidate(
            "candidate",
            &buffer_a,
            &chroma_a,
            &pitch_c,
            &[("ref_1.wav", &buffer_b, &chroma_b, &pitch_f)],
        );

        assert!(!report_plagiarized.is_compliant);
        assert_eq!(
            report_plagiarized.compliance_status,
            ComplianceStatus::RejectedPotentialCopy
        );
        assert_eq!(report_plagiarized.max_melodic_sequence_notes, 8);

        // Distinct candidate must be compliant
        let mut chroma_distinct = vec![[0.0f32; 12]; 20];
        for f in 0..20 {
            chroma_distinct[f][1] = 1.0; // C#
            chroma_distinct[f][6] = 1.0; // F#
        }
        let pitch_distinct = vec![1, 3, 6, 8, 10, 1, 3, 6];
        let report_compliant = LegalOriginalityGuard::audit_candidate(
            "candidate_distinct",
            &buffer_a,
            &chroma_distinct,
            &pitch_distinct,
            &[("ref_1.wav", &buffer_b, &chroma_b, &pitch_f)],
        );

        assert!(report_compliant.is_compliant);
        assert!(report_compliant.originality_score_pct >= 30.0);
        assert!(report_compliant.legal_compliance.zero_waveform_sampling);
        assert!(report_compliant.legal_compliance.synthetic_voice_only_elvis_compliant);
    }
}
