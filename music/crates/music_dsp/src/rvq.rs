//! Pure-Rust Multi-Stage Residual Vector Quantization (RVQ) Tokenizer.
//!
//! Provides deterministic orthogonal basis codebooks (DCT-II / Haar),
//! monotonic residual norm reduction (||r_m|| <= ||r_{m-1}||),
//! discrete token sequence encoding/decoding, and rolling N-gram hashing
//! for O(T) phrase overlap and originality detection.

use std::collections::HashSet;
use std::f32::consts::PI;

use crate::error::{DspError, Result};

pub const DEFAULT_RVQ_STAGES: usize = 4;
pub const DEFAULT_CODEBOOK_SIZE: usize = 256;
pub const DEFAULT_CODEBOOK_DIM: usize = 32;

/// Discrete representation of an audio feature sequence after RVQ tokenization.
#[derive(Debug, Clone, PartialEq)]
pub struct RvqTokenSequence {
    /// Token matrix of shape [T, M] where T is frames, M is stages
    pub tokens: Vec<Vec<u16>>,
    pub stages: usize,
    pub codebook_size: usize,
    pub dimension: usize,
    pub frame_hop_samples: usize,
    pub sample_rate: u32,
}

impl RvqTokenSequence {
    /// Computes rolling polynomial N-gram hashes for Stage-0 tokens (melodic contour).
    /// Uses 64-bit Rabin-Karp polynomial rolling hash: H = sum (tok_i * P^{n-1-i}) mod M.
    pub fn rolling_ngram_hashes(&self, n: usize) -> Vec<u64> {
        if self.tokens.len() < n || n == 0 {
            return Vec::new();
        }

        let p: u64 = 65537; // Prime larger than codebook size
        let num_ngrams = self.tokens.len() - n + 1;
        let mut hashes = Vec::with_capacity(num_ngrams);

        // Precompute P^(n-1)
        let mut p_pow = 1u64;
        for _ in 0..n - 1 {
            p_pow = p_pow.wrapping_mul(p);
        }

        // Initial window hash
        let mut curr_hash = 0u64;
        for i in 0..n {
            let tok = self.tokens[i][0] as u64;
            curr_hash = curr_hash.wrapping_mul(p).wrapping_add(tok);
        }
        hashes.push(curr_hash);

        // Rolling updates
        for i in 1..num_ngrams {
            let old_tok = self.tokens[i - 1][0] as u64;
            let new_tok = self.tokens[i + n - 1][0] as u64;
            curr_hash = curr_hash.wrapping_sub(old_tok.wrapping_mul(p_pow));
            curr_hash = curr_hash.wrapping_mul(p).wrapping_add(new_tok);
            hashes.push(curr_hash);
        }

        hashes
    }

    /// Computes the Jaccard overlap ratio of N-grams between this and another token sequence.
    pub fn ngram_overlap_ratio(&self, other: &Self, n: usize) -> f32 {
        let hashes_a: HashSet<u64> = self.rolling_ngram_hashes(n).into_iter().collect();
        let hashes_b: HashSet<u64> = other.rolling_ngram_hashes(n).into_iter().collect();

        if hashes_a.is_empty() || hashes_b.is_empty() {
            return 0.0;
        }

        let intersection_count = hashes_a.intersection(&hashes_b).count();
        let union_count = hashes_a.union(&hashes_b).count();

        if union_count == 0 {
            0.0
        } else {
            intersection_count as f32 / union_count as f32
        }
    }
}

/// Multi-stage Residual Vector Quantization codebook and tokenizer.
#[derive(Debug, Clone)]
pub struct RvqTokenizer {
    pub stages: usize,
    pub codebook_size: usize,
    pub dimension: usize,
    /// Codebooks: [stages][codebook_size][dimension]
    codebooks: Vec<Vec<Vec<f32>>>,
    /// Precomputed squared norms of codevectors for fast Euclidean distance: [stages][codebook_size]
    codevector_sq_norms: Vec<Vec<f32>>,
}

impl RvqTokenizer {
    /// Creates a new RVQ tokenizer with deterministic orthogonal basis codebooks.
    pub fn new(stages: usize, codebook_size: usize, dimension: usize) -> Result<Self> {
        if stages < 1 || stages > 8 {
            return Err(DspError::ParameterError(format!(
                "RVQ stages must be in 1..=8, got {}",
                stages
            )));
        }
        if codebook_size < 2 || codebook_size > 4096 {
            return Err(DspError::ParameterError(format!(
                "RVQ codebook_size must be in 2..=4096, got {}",
                codebook_size
            )));
        }
        if dimension < 4 || dimension > 256 {
            return Err(DspError::ParameterError(format!(
                "RVQ dimension must be in 4..=256, got {}",
                dimension
            )));
        }

        let mut codebooks = Vec::with_capacity(stages);
        let mut codevector_sq_norms = Vec::with_capacity(stages);

        for m in 0..stages {
            let mut stage_cb = Vec::with_capacity(codebook_size);
            let mut stage_norms = Vec::with_capacity(codebook_size);

            // Scale shrinks geometrically across stages: scale_m = 0.5^m
            let stage_scale = 0.5f32.powi(m as i32);

            for k in 0..codebook_size {
                let mut vec = vec![0.0f32; dimension];

                if k > 0 {
                    // Generate deterministic orthogonal basis vector via DCT-II and Haar wavelets
                    let basis_idx = (k - 1) % dimension;
                    let harmonic = ((k - 1) / dimension) + 1;
                    let phase_offset = (m as f32 * 0.785398) + (k as f32 * 0.1);

                    for d in 0..dimension {
                        let dct_term = ((d as f32 + 0.5) * basis_idx as f32 * PI / dimension as f32).cos();
                        let haar_term = if (d * harmonic) % 2 == 0 { 1.0 } else { -1.0 };
                        let phase_term = (phase_offset + d as f32 * 0.2).sin();

                        vec[d] = dct_term * 0.7 + haar_term * 0.2 + phase_term * 0.1;
                    }

                    let raw_norm = vec.iter().map(|&x| x * x).sum::<f32>().sqrt();
                    if raw_norm > 1e-7 {
                        for d in 0..dimension {
                            vec[d] = (vec[d] / raw_norm) * stage_scale;
                        }
                    }
                }

                // Compute squared Euclidean norm
                let sq_norm: f32 = vec.iter().map(|&x| x * x).sum();
                stage_norms.push(sq_norm);
                stage_cb.push(vec);
            }

            codebooks.push(stage_cb);
            codevector_sq_norms.push(stage_norms);
        }

        Ok(Self {
            stages,
            codebook_size,
            dimension,
            codebooks,
            codevector_sq_norms,
        })
    }

    /// Default configuration: 4 stages, 256 codevectors, 32 dimensions.
    pub fn default_config() -> Self {
        Self::new(DEFAULT_RVQ_STAGES, DEFAULT_CODEBOOK_SIZE, DEFAULT_CODEBOOK_DIM)
            .expect("Default RVQ tokenizer configuration must be valid")
    }

    /// Encodes a sequence of continuous feature vectors into a discrete RVQ token sequence.
    /// Monotonically reduces residual norm across every stage: ||r_m|| <= ||r_{m-1}||.
    pub fn encode(
        &self,
        features: &[Vec<f32>],
        hop_samples: usize,
        sample_rate: u32,
    ) -> Result<RvqTokenSequence> {
        let num_frames = features.len();
        let mut tokens: Vec<Vec<u16>> = Vec::with_capacity(num_frames);

        for (f_idx, feat) in features.iter().enumerate() {
            if feat.len() != self.dimension {
                return Err(DspError::ParameterError(format!(
                    "Feature frame {} has dimension {}, expected {}",
                    f_idx,
                    feat.len(),
                    self.dimension
                )));
            }

            let mut residual = feat.clone();
            let mut frame_tokens = Vec::with_capacity(self.stages);

            for m in 0..self.stages {
                let stage_cb = &self.codebooks[m];
                let stage_norms = &self.codevector_sq_norms[m];

                let mut best_dist = f32::MAX;
                let mut best_k = 0usize;

                // Find nearest centroid: argmin_k ||residual - c_k||^2
                // = ||residual||^2 + ||c_k||^2 - 2 * residual . c_k
                for k in 0..self.codebook_size {
                    let c_k = &stage_cb[k];
                    let sq_norm_k = stage_norms[k];

                    let mut dot = 0.0f32;
                    for d in 0..self.dimension {
                        dot += residual[d] * c_k[d];
                    }

                    let dist = sq_norm_k - 2.0 * dot;
                    if dist < best_dist {
                        best_dist = dist;
                        best_k = k;
                    }
                }

                // Residual update: r_m = r_{m-1} - c_best
                let best_codevector = &stage_cb[best_k];
                for d in 0..self.dimension {
                    residual[d] -= best_codevector[d];
                }

                frame_tokens.push(best_k as u16);
            }

            tokens.push(frame_tokens);
        }

        Ok(RvqTokenSequence {
            tokens,
            stages: self.stages,
            codebook_size: self.codebook_size,
            dimension: self.dimension,
            frame_hop_samples: hop_samples,
            sample_rate,
        })
    }

    /// Decodes a discrete token sequence back into continuous feature frames.
    pub fn decode(&self, tokens: &[Vec<u16>]) -> Result<Vec<Vec<f32>>> {
        let mut reconstructed = Vec::with_capacity(tokens.len());

        for (f_idx, frame_tok) in tokens.iter().enumerate() {
            if frame_tok.len() != self.stages {
                return Err(DspError::ParameterError(format!(
                    "Token frame {} has {} stages, expected {}",
                    f_idx,
                    frame_tok.len(),
                    self.stages
                )));
            }

            let mut feat = vec![0.0f32; self.dimension];
            for (m, &tok) in frame_tok.iter().enumerate() {
                let k = tok as usize;
                if k >= self.codebook_size {
                    return Err(DspError::ParameterError(format!(
                        "Token index {} exceeds codebook size {}",
                        k, self.codebook_size
                    )));
                }

                let c_k = &self.codebooks[m][k];
                for d in 0..self.dimension {
                    feat[d] += c_k[d];
                }
            }

            reconstructed.push(feat);
        }

        Ok(reconstructed)
    }

    /// Verifies that residual norm monotonically decreases across all stages for a given vector.
    pub fn verify_monotonic_residual_reduction(&self, input: &[f32]) -> (bool, Vec<f32>) {
        let mut residual = input.to_vec();
        let mut norms = Vec::with_capacity(self.stages + 1);

        let initial_norm = residual.iter().map(|&x| x * x).sum::<f32>().sqrt();
        norms.push(initial_norm);

        let mut monotonic = true;
        let mut prev_norm = initial_norm;

        for m in 0..self.stages {
            let stage_cb = &self.codebooks[m];
            let stage_norms = &self.codevector_sq_norms[m];

            let mut best_dist = f32::MAX;
            let mut best_k = 0usize;

            for k in 0..self.codebook_size {
                let c_k = &stage_cb[k];
                let sq_norm_k = stage_norms[k];
                let mut dot = 0.0f32;
                for d in 0..self.dimension {
                    dot += residual[d] * c_k[d];
                }
                let dist = sq_norm_k - 2.0 * dot;
                if dist < best_dist {
                    best_dist = dist;
                    best_k = k;
                }
            }

            let best_codevector = &stage_cb[best_k];
            for d in 0..self.dimension {
                residual[d] -= best_codevector[d];
            }

            let curr_norm = residual.iter().map(|&x| x * x).sum::<f32>().sqrt();
            // Small epsilon to account for floating point rounding
            if curr_norm > prev_norm + 1e-5 {
                monotonic = false;
            }
            norms.push(curr_norm);
            prev_norm = curr_norm;
        }

        (monotonic, norms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rvq_monotonic_residual_reduction() {
        let tokenizer = RvqTokenizer::default_config();
        assert_eq!(tokenizer.stages, 4);
        assert_eq!(tokenizer.codebook_size, 256);
        assert_eq!(tokenizer.dimension, 32);

        // Test with arbitrary vectors
        let test_vec = (0..32).map(|i| ((i as f32) * 0.1).sin()).collect::<Vec<_>>();
        let (monotonic, norms) = tokenizer.verify_monotonic_residual_reduction(&test_vec);

        assert!(
            monotonic,
            "Residual norms must strictly decrease monotonically: {:?}",
            norms
        );
        assert_eq!(norms.len(), 5);
        assert!(norms.last().unwrap() < norms.first().unwrap());
    }

    #[test]
    fn test_rvq_encode_decode_roundtrip_shape() {
        let tokenizer = RvqTokenizer::new(4, 256, 32).unwrap();
        let num_frames = 20;
        let mut frames = Vec::with_capacity(num_frames);
        for f in 0..num_frames {
            let frame = (0..32)
                .map(|i| ((f as f32 * 0.2) + (i as f32 * 0.05)).cos())
                .collect();
            frames.push(frame);
        }

        let token_seq = tokenizer.encode(&frames, 480, 48000).unwrap();
        assert_eq!(token_seq.tokens.len(), num_frames);
        assert_eq!(token_seq.stages, 4);

        let decoded = tokenizer.decode(&token_seq.tokens).unwrap();
        assert_eq!(decoded.len(), num_frames);
        assert_eq!(decoded[0].len(), 32);
    }

    #[test]
    fn test_rvq_rolling_ngram_hashes_and_overlap() {
        let tokenizer = RvqTokenizer::default_config();
        let frames: Vec<Vec<f32>> = (0..30)
            .map(|f| (0..32).map(|i| (f as f32 * 0.1 + i as f32 * 0.3).sin()).collect())
            .collect();

        let token_seq_1 = tokenizer.encode(&frames, 480, 48000).unwrap();
        let token_seq_2 = token_seq_1.clone();

        // 1. Identical sequences must yield 100% N-gram overlap
        let overlap_identical = token_seq_1.ngram_overlap_ratio(&token_seq_2, 4);
        assert!(
            (overlap_identical - 1.0).abs() < 1e-4,
            "Identical sequences must have 100% N-gram overlap, got {}",
            overlap_identical
        );

        // 2. Distinct sequences
        let distinct_frames: Vec<Vec<f32>> = (0..30)
            .map(|f| (0..32).map(|i| ((f as f32 + 100.0) * 0.7 + i as f32 * 1.5).cos()).collect())
            .collect();
        let token_seq_distinct = tokenizer.encode(&distinct_frames, 480, 48000).unwrap();
        let overlap_distinct = token_seq_1.ngram_overlap_ratio(&token_seq_distinct, 4);
        assert!(
            overlap_distinct < 0.20,
            "Distinct sequences should have low N-gram overlap, got {}",
            overlap_distinct
        );
    }
}
