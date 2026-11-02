//! Pure-Rust 48 kHz Acoustic Feature Analyzer & Multi-File Ensemble Blender.
//!
//! Extracts BPM, 12-bin Krumhansl-Schmuckler chroma & key signature,
//! spectral timbre envelope, centroid, crest factor, dynamic range,
//! 1-4 kHz vocal presence, and 64-dimensional style embeddings.
//! Enforces anti-monopoly blending (<=60% per file when N >= 3) across 1..=20 files.

use std::collections::HashMap;
use std::f32::consts::PI;

use music_core::rl_contracts::{AcousticFeatureVector, BlendedStyleProfile};

use crate::error::{DspError, Result};
use crate::telemetry_calc::fft_radix2;
use crate::wav::AudioBuffer;

pub const REFERENCE_CAP_LIMIT: usize = 20;
pub const ANTI_MONOPOLY_CEILING: f32 = 0.60;

// Standard Krumhansl-Kessler 12-element Key Profiles
// Major: C, C#, D, D#, E, F, F#, G, G#, A, A#, B
const KRUMHANSL_MAJOR: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];
// Minor: C, C#, D, D#, E, F, F#, G, G#, A, A#, B
const KRUMHANSL_MINOR: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub struct AcousticAnalyzer;

impl AcousticAnalyzer {
    /// Extracts a comprehensive acoustic feature vector from an audio buffer.
    pub fn analyze_buffer(file_name: &str, buffer: &AudioBuffer) -> Result<AcousticFeatureVector> {
        let sample_rate = buffer.sample_rate;
        let channels = buffer.channels;
        let duration_secs = buffer.duration_secs();

        if buffer.samples.is_empty() || duration_secs < 0.05 {
            return Err(DspError::ParameterError(
                "Audio buffer is empty or duration < 50ms".to_string(),
            ));
        }

        // Downmix to mono for analysis
        let full_mono = buffer.to_mono().samples;
        if full_mono.len() < 1024 {
            return Err(DspError::ParameterError(
                "Audio buffer is too short for 1024-point FFT".to_string(),
            ));
        }

        // For multi-minute reference tracks (> 20s), analyze a representative 16s musical window
        // (starting 15% into the track to skip silent intros) so 70-100 MB WAVs analyze in < 100ms.
        let max_window_samples = (sample_rate as usize) * 16;
        let mono_samples: &[f32] = if full_mono.len() > (sample_rate as usize) * 20 {
            let start = (full_mono.len() * 15 / 100).min(full_mono.len().saturating_sub(max_window_samples));
            let end = (start + max_window_samples).min(full_mono.len());
            &full_mono[start..end]
        } else {
            &full_mono
        };
        let n_samples = mono_samples.len();

        // 1. Dynamic range and crest factor (time-domain)
        let (crest_factor_db, dynamic_range_db) = Self::calculate_dynamics(mono_samples);

        // 2. STFT frame processing (fine musical pitch resolution, 10ms hops)
        let fft_size = if n_samples >= 4096 {
            4096
        } else if n_samples >= 2048 {
            2048
        } else {
            1024
        };
        let hop_size = (sample_rate / 100).max(1) as usize; // 10ms hop

        let num_frames = (n_samples - fft_size) / hop_size + 1;
        let mut stft_magnitudes: Vec<Vec<f32>> = Vec::with_capacity(num_frames);
        let mut frame_energies: Vec<f32> = Vec::with_capacity(num_frames);
        let mut frame_chromas: Vec<[f32; 12]> = Vec::with_capacity(num_frames);

        let mut real = vec![0.0f32; fft_size];
        let mut imag = vec![0.0f32; fft_size];

        for f in 0..num_frames {
            let start = f * hop_size;
            // Apply Hann window
            for i in 0..fft_size {
                let sample = mono_samples[start + i];
                let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / (fft_size - 1) as f32).cos());
                real[i] = sample * w;
                imag[i] = 0.0;
            }

            fft_radix2(&mut real, &mut imag);

            let mut mag = vec![0.0f32; fft_size / 2];
            let mut total_energy = 0.0f32;
            for i in 0..fft_size / 2 {
                let m = (real[i] * real[i] + imag[i] * imag[i]).sqrt();
                mag[i] = m;
                total_energy += m * m;
            }

            let chroma_frame = Self::extract_frame_chroma(&mag, sample_rate, fft_size);
            frame_chromas.push(chroma_frame);
            frame_energies.push(total_energy);
            stft_magnitudes.push(mag);
        }

        // 3. BPM extraction via onset flux autocorrelation with log-Gaussian tempo prior
        let bpm = Self::detect_bpm(&stft_magnitudes, hop_size, sample_rate);

        // 4. Krumhansl-Schmuckler 12-bin chroma profile and key signature
        let (chroma_12bin, key_signature, key_confidence) =
            Self::detect_key_profile(&frame_chromas, &frame_energies);

        // 5. Dominant pitch sequence (frame-level dominant pitch class)
        let pitch_token_sequence = Self::extract_pitch_sequence(&frame_chromas, &frame_energies);

        // 6. Spectral centroid, timbre subbands, and vocal presence
        let (spectral_centroid_hz, timbre_subbands, vocal_presence_ratio, vocal_activity_ratio) =
            Self::analyze_spectrum(&stft_magnitudes, sample_rate, fft_size);

        // 7. 64-dimensional style embedding vector (L2 normalized)
        let style_embedding = Self::construct_style_embedding(
            &chroma_12bin,
            &timbre_subbands,
            spectral_centroid_hz,
            crest_factor_db,
            dynamic_range_db,
            vocal_presence_ratio,
            vocal_activity_ratio,
            bpm,
            &stft_magnitudes,
        );

        Ok(AcousticFeatureVector {
            file_name: file_name.to_string(),
            duration_secs,
            sample_rate,
            channels,
            bpm,
            chroma_12bin,
            key_signature,
            key_confidence,
            spectral_centroid_hz,
            crest_factor_db,
            dynamic_range_db,
            vocal_presence_ratio,
            vocal_activity_ratio,
            timbre_subbands,
            style_embedding,
            pitch_token_sequence,
        })
    }

    /// Blends multiple acoustic feature profiles into an ensemble profile.
    /// Strictly limits references to 1..=20 files and applies an anti-monopoly
    /// ceiling (<=60% per file when N >= 3).
    pub fn blend_ensemble(
        profiles: &[AcousticFeatureVector],
        user_weights: Option<&HashMap<String, f32>>,
    ) -> Result<BlendedStyleProfile> {
        let count = profiles.len();
        if count == 0 {
            return Err(DspError::ParameterError(
                "Cannot blend ensemble with 0 reference files".to_string(),
            ));
        }
        if count > REFERENCE_CAP_LIMIT {
            return Err(DspError::ParameterError(format!(
                "Reference file count {} exceeds maximum cap of {}",
                count, REFERENCE_CAP_LIMIT
            )));
        }

        // 1. Compute raw weights
        let mut raw_weights = Vec::with_capacity(count);
        for p in profiles {
            let w = user_weights
                .and_then(|m| m.get(&p.file_name))
                .copied()
                .unwrap_or(1.0)
                .max(0.001);
            raw_weights.push(w);
        }

        // 2. Anti-monopoly capping when N >= 3
        let final_weights: Vec<f32> = if count >= 3 {
            Self::clamp_and_normalize_weights(&raw_weights, ANTI_MONOPOLY_CEILING)
        } else {
            let sum_raw: f32 = raw_weights.iter().sum();
            raw_weights.iter().map(|&w| w / sum_raw).collect()
        };

        // 3. Assemble file weights map
        let mut file_weights = HashMap::with_capacity(count);
        for (i, p) in profiles.iter().enumerate() {
            file_weights.insert(p.file_name.clone(), final_weights[i]);
        }

        // 4. Blended BPM (convex combination)
        let blended_bpm: f32 = profiles
            .iter()
            .enumerate()
            .map(|(i, p)| p.bpm * final_weights[i])
            .sum();

        // 5. Blended Chroma (convex combination + L2 renormalization)
        let mut blended_chroma = [0.0f32; 12];
        for (i, p) in profiles.iter().enumerate() {
            let w = final_weights[i];
            for k in 0..12 {
                blended_chroma[k] += p.chroma_12bin[k] * w;
            }
        }
        let chroma_norm: f32 = blended_chroma.iter().map(|&v| v * v).sum::<f32>().sqrt();
        if chroma_norm > 1e-7 {
            for v in blended_chroma.iter_mut() {
                *v /= chroma_norm;
            }
        }

        // 6. Consensus Key from blended chroma
        let (consensus_key, _) = Self::match_krumhansl(&blended_chroma);

        // 7. Blended Spectral Centroid, Vocal Presence, Timbre Subbands
        let mut blended_spectral_centroid_hz = 0.0f32;
        let mut blended_vocal_presence = 0.0f32;
        let mut blended_timbre_subbands = [0.0f32; 8];

        for (i, p) in profiles.iter().enumerate() {
            let w = final_weights[i];
            blended_spectral_centroid_hz += p.spectral_centroid_hz * w;
            blended_vocal_presence += p.vocal_presence_ratio * w;
            for b in 0..8 {
                blended_timbre_subbands[b] += p.timbre_subbands[b] * w;
            }
        }

        // 8. Blended Style Embedding (convex combination + L2 renormalization)
        let dim = profiles[0].style_embedding.len();
        let mut blended_style_embedding = vec![0.0f32; dim];
        for (i, p) in profiles.iter().enumerate() {
            let w = final_weights[i];
            for d in 0..dim {
                blended_style_embedding[d] += p.style_embedding.get(d).copied().unwrap_or(0.0) * w;
            }
        }
        let emb_norm: f32 = blended_style_embedding.iter().map(|&v| v * v).sum::<f32>().sqrt();
        if emb_norm > 1e-7 {
            for v in blended_style_embedding.iter_mut() {
                *v /= emb_norm;
            }
        }

        // 9. Style Variance / Dispersion: \sum w_i (1.0 - cos(e_i, e_ens))
        let mut style_variance = 0.0f32;
        for (i, p) in profiles.iter().enumerate() {
            let w = final_weights[i];
            let dot: f32 = p
                .style_embedding
                .iter()
                .zip(blended_style_embedding.iter())
                .map(|(&a, &b)| a * b)
                .sum();
            style_variance += w * (1.0 - dot.clamp(-1.0, 1.0));
        }

        Ok(BlendedStyleProfile {
            reference_count: count,
            file_weights,
            blended_bpm,
            blended_chroma,
            consensus_key,
            blended_spectral_centroid_hz,
            blended_vocal_presence,
            blended_timbre_subbands,
            blended_style_embedding,
            style_variance,
        })
    }

    // ------------------------------------------------------------------------
    // Internal Helper Algorithms
    // ------------------------------------------------------------------------

    fn calculate_dynamics(samples: &[f32]) -> (f32, f32) {
        let mut peak = 0.0f32;
        let mut sum_sq = 0.0f32;

        for &s in samples {
            let abs_s = s.abs();
            if abs_s > peak {
                peak = abs_s;
            }
            sum_sq += s * s;
        }

        let rms = (sum_sq / samples.len().max(1) as f32).sqrt().max(1e-7);
        let crest_factor_db = 20.0 * (peak.max(1e-7) / rms).log10();

        // 50ms block dynamic range calculation
        let block_size = 2400; // 50ms at 48kHz
        let mut block_rms_db = Vec::new();
        for chunk in samples.chunks(block_size) {
            let c_rms = (chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32)
                .sqrt()
                .max(1e-7);
            let db = 20.0 * c_rms.log10();
            block_rms_db.push(db);
        }
        block_rms_db.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let dynamic_range_db = if block_rms_db.len() >= 10 {
            let idx_95 = (block_rms_db.len() as f32 * 0.95) as usize;
            let idx_05 = (block_rms_db.len() as f32 * 0.05) as usize;
            (block_rms_db[idx_95] - block_rms_db[idx_05]).clamp(0.0, 60.0)
        } else {
            12.0
        };

        (crest_factor_db.clamp(0.0, 40.0), dynamic_range_db)
    }

    fn clamp_and_normalize_weights(raw: &[f32], cap: f32) -> Vec<f32> {
        let n = raw.len();
        if n == 0 {
            return Vec::new();
        }
        let sum: f32 = raw.iter().sum();
        let mut w: Vec<f32> = if sum > 1e-7 {
            raw.iter().map(|&v| v / sum).collect()
        } else {
            vec![1.0 / n as f32; n]
        };

        for _ in 0..10 {
            let mut excess = 0.0f32;
            let mut unconstrained_sum = 0.0f32;
            let mut constrained_count = 0;

            for &val in w.iter() {
                if val > cap + 1e-6 {
                    excess += val - cap;
                    constrained_count += 1;
                } else {
                    unconstrained_sum += val;
                }
            }

            if excess <= 1e-6 || constrained_count == n {
                break;
            }

            for val in w.iter_mut() {
                if *val > cap {
                    *val = cap;
                } else if unconstrained_sum > 1e-7 {
                    *val += excess * (*val / unconstrained_sum);
                } else {
                    *val += excess / (n - constrained_count) as f32;
                }
            }
        }

        let sum_w: f32 = w.iter().sum();
        if sum_w > 1e-7 {
            for val in w.iter_mut() {
                *val /= sum_w;
            }
        }
        for val in w.iter_mut() {
            if *val > cap {
                *val = cap;
            }
        }
        w
    }

    fn extract_frame_chroma(mag: &[f32], sample_rate: u32, fft_size: usize) -> [f32; 12] {
        let mut chroma = [0.0f32; 12];
        let freq_per_bin = sample_rate as f32 / fft_size as f32;

        let max_mag = mag.iter().copied().fold(0.0f32, f32::max).max(1e-7);
        let threshold = max_mag * 0.05;

        // Map bins from 65.4 Hz (C2) to 2093 Hz (C7)
        let min_bin = (65.4 / freq_per_bin).floor() as usize;
        let max_bin = ((2093.0 / freq_per_bin).ceil() as usize).min(mag.len().saturating_sub(1));

        for k in min_bin.max(1)..max_bin {
            let m_curr = mag[k];
            let m_prev = mag[k - 1];
            let m_next = mag[k + 1];

            // Only extract prominent spectral peaks
            if m_curr > m_prev && m_curr >= m_next && m_curr > threshold {
                let denom = 2.0 * (2.0 * m_curr - m_prev - m_next);
                let delta = if denom.abs() > 1e-7 {
                    (m_next - m_prev) / denom
                } else {
                    0.0
                };
                let k_refined = (k as f32 + delta.clamp(-0.5, 0.5)).max(1.0);
                let f_eff = k_refined * freq_per_bin;

                if f_eff > 20.0 {
                    let midi = 12.0 * (f_eff / 440.0).log2() + 69.0;
                    let pitch_class = ((midi.round() as i32 % 12) + 12) % 12;
                    chroma[pitch_class as usize] += m_curr * m_curr;
                }
            }
        }

        chroma
    }

    fn detect_bpm(stft_magnitudes: &[Vec<f32>], hop_size: usize, sample_rate: u32) -> f32 {
        if stft_magnitudes.len() < 2 {
            return 120.0;
        }

        // 1. Spectral flux onset envelope
        let num_frames = stft_magnitudes.len();
        let mut flux = vec![0.0f32; num_frames];
        for f in 1..num_frames {
            let prev = &stft_magnitudes[f - 1];
            let curr = &stft_magnitudes[f];
            let mut diff_sum = 0.0f32;
            for i in 0..curr.len() {
                let diff = curr[i] - prev[i];
                if diff > 0.0 {
                    diff_sum += diff;
                }
            }
            flux[f] = diff_sum;
        }

        // Subtract moving average to remove DC trend
        let win = 16;
        let mut centered_flux = vec![0.0f32; num_frames];
        for i in 0..num_frames {
            let start = i.saturating_sub(win);
            let end = (i + win + 1).min(num_frames);
            let avg: f32 = flux[start..end].iter().sum::<f32>() / (end - start) as f32;
            centered_flux[i] = (flux[i] - avg).max(0.0);
        }

        // 2. Autocorrelation over lag range corresponding to 40..240 BPM
        let fps = sample_rate as f32 / hop_size as f32;
        let min_bpm = 40.0f32;
        let max_bpm = 240.0f32;
        let min_lag = (fps * 60.0 / max_bpm).floor() as usize;
        let max_lag = ((fps * 60.0 / min_bpm).ceil() as usize).min(num_frames / 2);

        if max_lag <= min_lag {
            return 120.0;
        }

        let mut best_score = -1.0f32;
        let mut best_lag = min_lag;
        let mut lag_scores = vec![0.0f32; max_lag + 2];

        for lag in min_lag..=max_lag {
            let mut sum = 0.0f32;
            let mut norm_a = 0.0f32;
            let mut norm_b = 0.0f32;
            let n = num_frames - lag;
            for i in 0..n {
                let a = centered_flux[i];
                let b = centered_flux[i + lag];
                sum += a * b;
                norm_a += a * a;
                norm_b += b * b;
            }
            let denom = (norm_a * norm_b).sqrt();
            let raw_corr = if denom > 1e-7 { sum / denom } else { 0.0 };

            // Apply log-Gaussian tempo prior centered at 120 BPM (resolves octave ambiguity)
            let raw_bpm = fps * 60.0 / lag as f32;
            let log_ratio = (raw_bpm / 120.0).log2();
            let prior = (-0.5 * (log_ratio / 0.5).powi(2)).exp();
            let score = raw_corr * prior;

            lag_scores[lag] = score;
            if score > best_score {
                best_score = score;
                best_lag = lag;
            }
        }

        // Parabolic interpolation around peak lag for fractional BPM accuracy
        let refined_lag = if best_lag > min_lag && best_lag < max_lag {
            let y_left = lag_scores[best_lag - 1];
            let y_mid = lag_scores[best_lag];
            let y_right = lag_scores[best_lag + 1];
            let denom = 2.0 * (2.0 * y_mid - y_left - y_right);
            if denom.abs() > 1e-7 {
                best_lag as f32 + (y_left - y_right) / denom
            } else {
                best_lag as f32
            }
        } else {
            best_lag as f32
        };

        let detected_bpm = fps * 60.0 / refined_lag;
        detected_bpm.clamp(40.0, 240.0)
    }

    fn detect_key_profile(
        frame_chromas: &[[f32; 12]],
        frame_energies: &[f32],
    ) -> ([f32; 12], String, f32) {
        let mut total_chroma = [0.0f32; 12];

        for (i, c) in frame_chromas.iter().enumerate() {
            let w = frame_energies.get(i).copied().unwrap_or(1.0).sqrt();
            for k in 0..12 {
                total_chroma[k] += c[k] * w;
            }
        }

        // Normalize chroma vector
        let norm = total_chroma.iter().map(|&v| v * v).sum::<f32>().sqrt();
        if norm > 1e-7 {
            for v in total_chroma.iter_mut() {
                *v /= norm;
            }
        }

        let (key_signature, confidence) = Self::match_krumhansl(&total_chroma);
        (total_chroma, key_signature, confidence)
    }

    fn match_krumhansl(chroma: &[f32; 12]) -> (String, f32) {
        let mut best_corr = -2.0f32;
        let mut best_name = "C Major".to_string();

        let mean_c: f32 = chroma.iter().sum::<f32>() / 12.0;
        let var_c: f32 = chroma.iter().map(|&x| (x - mean_c).powi(2)).sum();

        // 12 Major keys
        for root in 0..12 {
            let mut rotated_profile = [0.0f32; 12];
            for i in 0..12 {
                rotated_profile[(root + i) % 12] = KRUMHANSL_MAJOR[i];
            }
            let mean_p: f32 = rotated_profile.iter().sum::<f32>() / 12.0;
            let var_p: f32 = rotated_profile.iter().map(|&x| (x - mean_p).powi(2)).sum();

            let mut cov = 0.0f32;
            for i in 0..12 {
                cov += (chroma[i] - mean_c) * (rotated_profile[i] - mean_p);
            }
            let denom = (var_c * var_p).sqrt();
            let corr = if denom > 1e-7 { cov / denom } else { 0.0 };

            if corr > best_corr {
                best_corr = corr;
                best_name = format!("{} Major", NOTE_NAMES[root]);
            }
        }

        // 12 Minor keys
        for root in 0..12 {
            let mut rotated_profile = [0.0f32; 12];
            for i in 0..12 {
                rotated_profile[(root + i) % 12] = KRUMHANSL_MINOR[i];
            }
            let mean_p: f32 = rotated_profile.iter().sum::<f32>() / 12.0;
            let var_p: f32 = rotated_profile.iter().map(|&x| (x - mean_p).powi(2)).sum();

            let mut cov = 0.0f32;
            for i in 0..12 {
                cov += (chroma[i] - mean_c) * (rotated_profile[i] - mean_p);
            }
            let denom = (var_c * var_p).sqrt();
            let corr = if denom > 1e-7 { cov / denom } else { 0.0 };

            if corr > best_corr {
                best_corr = corr;
                best_name = format!("{} Minor", NOTE_NAMES[root]);
            }
        }

        let confidence = ((best_corr + 1.0) * 0.5).clamp(0.0, 1.0);
        (best_name, confidence)
    }

    fn extract_pitch_sequence(
        frame_chromas: &[[f32; 12]],
        frame_energies: &[f32],
    ) -> Vec<u8> {
        let mut sequence = Vec::with_capacity(frame_chromas.len());
        let max_energy = frame_energies
            .iter()
            .copied()
            .fold(0.0f32, f32::max)
            .max(1e-7);

        let mut last_bin: Option<u8> = None;

        // For long polyphonic recordings (> 300 frames / > 3.0s), aggregate chroma over 20-frame
        // (~200ms musical eighth-note) windows so harmonic STFT frame jitter does not produce
        // thousands of spurious sub-10ms transitions, and cap at 32 representative contour notes.
        let chunk_frames = if frame_chromas.len() > 300 { 20 } else { 1 };
        let max_notes = if frame_chromas.len() > 300 { 32 } else { usize::MAX };

        let mut idx = 0;
        while idx < frame_chromas.len() && sequence.len() < max_notes {
            let end = (idx + chunk_frames).min(frame_chromas.len());
            let mut avg_energy = 0.0f32;
            let mut agg_chroma = [0.0f32; 12];
            for j in idx..end {
                let e = frame_energies.get(j).copied().unwrap_or(0.0);
                avg_energy += e;
                for k in 0..12 {
                    agg_chroma[k] += frame_chromas[j][k];
                }
            }
            avg_energy /= (end - idx).max(1) as f32;
            idx = end;

            if avg_energy / max_energy < 0.02 {
                // Low energy frame (silence / pause): reset note transition tracker
                last_bin = None;
                continue;
            }
            let mut best_bin = 0u8;
            let mut best_val = -1.0f32;
            for k in 0..12 {
                if agg_chroma[k] > best_val {
                    best_val = agg_chroma[k];
                    best_bin = k as u8;
                }
            }
            // Only record note transitions (distinct pitch changes or onsets after pause)
            if last_bin != Some(best_bin) {
                sequence.push(best_bin);
                last_bin = Some(best_bin);
            }
        }

        sequence
    }

    fn analyze_spectrum(
        stft_magnitudes: &[Vec<f32>],
        sample_rate: u32,
        fft_size: usize,
    ) -> (f32, [f32; 8], f32, f32) {
        let freq_per_bin = sample_rate as f32 / fft_size as f32;
        let mut sum_centroid_num = 0.0f32;
        let mut sum_centroid_den = 0.0f32;

        let mut subband_energy = [0.0f32; 8];
        let mut total_spectrum_energy = 0.0f32;
        let mut vocal_band_energy = 0.0f32;
        let mut vocal_active_frames = 0;

        // Subband boundaries: 20, 60, 250, 500, 2000, 4000, 6000, 12000, 20000 Hz
        let band_bounds = [20.0, 60.0, 250.0, 500.0, 2000.0, 4000.0, 6000.0, 12000.0, 20000.0];

        for mag in stft_magnitudes {
            let mut frame_total = 0.0f32;
            let mut frame_vocal = 0.0f32;

            for k in 0..mag.len() {
                let freq = k as f32 * freq_per_bin;
                let energy = mag[k] * mag[k];
                sum_centroid_num += freq * mag[k];
                sum_centroid_den += mag[k];
                frame_total += energy;

                // Subband accumulation
                for b in 0..8 {
                    if freq >= band_bounds[b] && freq < band_bounds[b + 1] {
                        subband_energy[b] += energy;
                        break;
                    }
                }

                // 1-4 kHz vocal formant cluster
                if freq >= 1000.0 && freq <= 4000.0 {
                    frame_vocal += energy;
                }
            }

            total_spectrum_energy += frame_total;
            vocal_band_energy += frame_vocal;

            if frame_total > 1e-7 && (frame_vocal / frame_total) > 0.20 {
                vocal_active_frames += 1;
            }
        }

        let spectral_centroid_hz = if sum_centroid_den > 1e-7 {
            sum_centroid_num / sum_centroid_den
        } else {
            1000.0
        };

        let mut normalized_subbands = [0.0f32; 8];
        if total_spectrum_energy > 1e-7 {
            for b in 0..8 {
                normalized_subbands[b] = subband_energy[b] / total_spectrum_energy;
            }
        }

        let vocal_presence_ratio = if total_spectrum_energy > 1e-7 {
            (vocal_band_energy / total_spectrum_energy).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let vocal_activity_ratio = if !stft_magnitudes.is_empty() {
            (vocal_active_frames as f32 / stft_magnitudes.len() as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        (
            spectral_centroid_hz,
            normalized_subbands,
            vocal_presence_ratio,
            vocal_activity_ratio,
        )
    }

    fn construct_style_embedding(
        chroma: &[f32; 12],
        subbands: &[f32; 8],
        centroid_hz: f32,
        crest_factor_db: f32,
        dynamic_range_db: f32,
        vocal_presence: f32,
        vocal_activity: f32,
        bpm: f32,
        stft_magnitudes: &[Vec<f32>],
    ) -> Vec<f32> {
        let mut embedding = Vec::with_capacity(64);

        // 12 chroma dims
        embedding.extend_from_slice(chroma);
        // 8 timbre subband dims
        embedding.extend_from_slice(subbands);
        // Scalar acoustic descriptors (5 dims)
        embedding.push((centroid_hz / 10000.0).clamp(0.0, 2.0));
        embedding.push((crest_factor_db / 20.0).clamp(0.0, 2.0));
        embedding.push((dynamic_range_db / 30.0).clamp(0.0, 2.0));
        embedding.push(vocal_presence.clamp(0.0, 1.0));
        embedding.push(vocal_activity.clamp(0.0, 1.0));
        // Tempo descriptor (1 dim)
        embedding.push(((bpm - 40.0) / 200.0).clamp(0.0, 1.0));

        // 38 DCT-II spectral envelope coefficients (to reach 64 dims: 12 + 8 + 5 + 1 + 38 = 64)
        let num_dct_coeffs = 64 - embedding.len();
        let mut mean_spectrum = vec![0.0f32; 64];
        if !stft_magnitudes.is_empty() {
            let n_frames = stft_magnitudes.len() as f32;
            let bins_to_use = stft_magnitudes[0].len().min(64);
            for mag in stft_magnitudes {
                for i in 0..bins_to_use {
                    mean_spectrum[i] += mag[i] / n_frames;
                }
            }
        }

        // Compute DCT-II on mean log-spectrum
        for k in 0..num_dct_coeffs {
            let mut sum = 0.0f32;
            let n = mean_spectrum.len();
            for n_idx in 0..n {
                let log_val = (mean_spectrum[n_idx] + 1e-4).ln();
                let factor = ((n_idx as f32 + 0.5) * k as f32 * PI / n as f32).cos();
                sum += log_val * factor;
            }
            embedding.push(sum);
        }

        // L2 normalize the complete 64-dim vector
        let norm = embedding.iter().map(|&v| v * v).sum::<f32>().sqrt();
        if norm > 1e-7 {
            for v in embedding.iter_mut() {
                *v /= norm;
            }
        }

        embedding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acoustic_analyzer_pure_440hz_a4() {
        let sample_rate = 48000;
        let duration = 0.5; // 500ms
        let n_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(n_samples);
        for i in 0..n_samples {
            let t = i as f32 / sample_rate as f32;
            samples.push((2.0 * PI * 440.0 * t).sin() * 0.8);
        }

        let buffer = AudioBuffer {
            sample_rate,
            channels: 1,
            samples,
        };

        let feature = AcousticAnalyzer::analyze_buffer("test_a4.wav", &buffer)
            .expect("Analysis should succeed on 440Hz sine wave");

        assert_eq!(feature.file_name, "test_a4.wav");
        assert_eq!(feature.channels, 1);
        assert_eq!(feature.sample_rate, 48000);
        assert!(feature.duration_secs >= 0.49);

        // A is pitch class 9
        let max_bin = feature
            .chroma_12bin
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert_eq!(max_bin, 9, "440 Hz tone must peak in pitch class 9 (A)");
        assert_eq!(feature.style_embedding.len(), 64);
    }

    #[test]
    fn test_krumhansl_chord_classification() {
        // C Major chord (C4 = 261.63, E4 = 329.63, G4 = 392.0)
        let sample_rate = 48000;
        let duration = 0.4;
        let n_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(n_samples);
        for i in 0..n_samples {
            let t = i as f32 / sample_rate as f32;
            let c = (2.0 * PI * 261.63 * t).sin();
            let e = (2.0 * PI * 329.63 * t).sin();
            let g = (2.0 * PI * 392.0 * t).sin();
            samples.push((c + e + g) * 0.25);
        }

        let buffer = AudioBuffer {
            sample_rate,
            channels: 1,
            samples,
        };

        let feature = AcousticAnalyzer::analyze_buffer("c_major.wav", &buffer)
            .expect("Analysis should succeed on C Major chord");

        assert!(
            feature.key_signature.contains("C Major"),
            "Expected C Major key detection, got {}",
            feature.key_signature
        );
    }

    #[test]
    fn test_ensemble_blending_capping_and_anti_monopoly() {
        let sample_rate = 48000;
        let duration = 0.3;
        let n_samples = (sample_rate as f32 * duration) as usize;
        let buffer = AudioBuffer {
            sample_rate,
            channels: 1,
            samples: (0..n_samples)
                .map(|i| (2.0 * PI * 440.0 * (i as f32 / 48000.0)).sin() * 0.5)
                .collect(),
        };

        let feature = AcousticAnalyzer::analyze_buffer("file_0.wav", &buffer).unwrap();

        // 1. Single file blending
        let ensemble_1 = AcousticAnalyzer::blend_ensemble(&[feature.clone()], None).unwrap();
        assert_eq!(ensemble_1.reference_count, 1);
        assert_eq!(ensemble_1.file_weights.get("file_0.wav"), Some(&1.0));

        // 2. 20 files blending
        let mut twenty_files = Vec::new();
        for i in 0..20 {
            let mut f = feature.clone();
            f.file_name = format!("file_{}.wav", i);
            twenty_files.push(f);
        }
        let ensemble_20 = AcousticAnalyzer::blend_ensemble(&twenty_files, None).unwrap();
        assert_eq!(ensemble_20.reference_count, 20);

        // 3. 21 files blending strictly rejected with error
        let mut twenty_one_files = twenty_files.clone();
        let mut extra = feature.clone();
        extra.file_name = "file_21.wav".to_string();
        twenty_one_files.push(extra);
        let err = AcousticAnalyzer::blend_ensemble(&twenty_one_files, None);
        assert!(err.is_err(), "21 files must exceed the 20-file cap limit");

        // 4. Anti-monopoly ceiling enforcement (N >= 3)
        let mut three_files = Vec::new();
        for i in 0..3 {
            let mut f = feature.clone();
            f.file_name = format!("stem_{}.wav", i);
            three_files.push(f);
        }
        let mut user_weights = HashMap::new();
        user_weights.insert("stem_0.wav".to_string(), 99.0); // Attempt 99% monopoly
        user_weights.insert("stem_1.wav".to_string(), 0.5);
        user_weights.insert("stem_2.wav".to_string(), 0.5);

        let blended = AcousticAnalyzer::blend_ensemble(&three_files, Some(&user_weights)).unwrap();
        let w0 = *blended.file_weights.get("stem_0.wav").unwrap();
        assert!(
            w0 <= ANTI_MONOPOLY_CEILING + 1e-5,
            "Monopoly weight {} must be capped at <= {}",
            w0,
            ANTI_MONOPOLY_CEILING
        );
    }
}
