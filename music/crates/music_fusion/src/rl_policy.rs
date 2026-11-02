//! Pure-Rust Reinforcement Learning (RL) Policy & Online DPO Preference Learner.
//!
//! Features:
//! 1. 6-Component Composite Reward Function:
//!    R_total = w_1*R_style + w_2*R_harmony + w_3*R_rhythm + w_4*R_vocal + w_5*R_human + w_6*R_originality
//! 2. Contextual Bandit / PPO Policy Parameterization over synthesis tweaks.
//! 3. Online Direct Preference Optimization (DPO) wired to Keep / Revert feedback pairs.
//! 4. 4 Baseline policy modes (Disabled, Random, Heuristic, NoLearning) for empirical superiority benchmarking.

use serde::{Deserialize, Serialize};

use music_core::rl_contracts::{
    BlendedStyleProfile, CompositeRewardBreakdown, DpoPreferencePair, LegalOriginalityReport,
};

pub const DEFAULT_LEARNING_RATE: f32 = 0.01;
pub const DEFAULT_BETA_TEMPERATURE: f32 = 0.1;

// Canonical 6-term weights (summing to 1.0)
pub const WEIGHT_STYLE: f32 = 0.20;
pub const WEIGHT_HARMONY: f32 = 0.15;
pub const WEIGHT_RHYTHM: f32 = 0.15;
pub const WEIGHT_VOCAL: f32 = 0.15;
pub const WEIGHT_HUMAN: f32 = 0.15;
pub const WEIGHT_ORIGINALITY: f32 = 0.20;

/// Action space of continuous synthesis parameter tweaks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SynthesisAction {
    pub delta_bpm: f32,          // [-10.0, +10.0] BPM
    pub vocal_rasp_delta: f32,   // [-0.3, +0.3]
    pub sub_bass_boost_db: f32,  // [0.0, 6.0] dB
    pub treble_presence_db: f32, // [0.0, 6.0] dB
    pub dynamic_compression: f32,// [0.0, 1.0]
    pub vocal_wet_ratio: f32,    // [0.1, 0.6]
}

impl Default for SynthesisAction {
    fn default() -> Self {
        Self {
            delta_bpm: 0.0,
            vocal_rasp_delta: 0.0,
            sub_bass_boost_db: 1.5,
            treble_presence_db: 1.0,
            dynamic_compression: 0.3,
            vocal_wet_ratio: 0.25,
        }
    }
}

impl SynthesisAction {
    pub fn to_vector(&self) -> Vec<f32> {
        vec![
            self.delta_bpm / 10.0,
            self.vocal_rasp_delta / 0.3,
            self.sub_bass_boost_db / 6.0,
            self.treble_presence_db / 6.0,
            self.dynamic_compression,
            self.vocal_wet_ratio / 0.6,
        ]
    }

    pub fn from_vector(v: &[f32]) -> Self {
        Self {
            delta_bpm: (v.get(0).copied().unwrap_or(0.0) * 10.0).clamp(-10.0, 10.0),
            vocal_rasp_delta: (v.get(1).copied().unwrap_or(0.0) * 0.3).clamp(-0.3, 0.3),
            sub_bass_boost_db: (v.get(2).copied().unwrap_or(0.25) * 6.0).clamp(0.0, 6.0),
            treble_presence_db: (v.get(3).copied().unwrap_or(0.16) * 6.0).clamp(0.0, 6.0),
            dynamic_compression: v.get(4).copied().unwrap_or(0.3).clamp(0.0, 1.0),
            vocal_wet_ratio: (v.get(5).copied().unwrap_or(0.4) * 0.6).clamp(0.1, 0.6),
        }
    }
}

/// Baseline policy mode for standardized empirical evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyMode {
    Disabled,
    Random,
    Heuristic,
    NoLearning,
    Learned,
}

/// Online Contextual Bandit and DPO RL Policy.
#[derive(Debug, Clone)]
pub struct RlPolicy {
    pub weights: Vec<f32>,
    pub ref_weights: Vec<f32>,
    pub feature_dim: usize,
    pub learning_rate: f32,
    pub beta: f32,
    pub mode: PolicyMode,
}

impl RlPolicy {
    /// Creates a new policy with initialized linear weights.
    pub fn new(feature_dim: usize, mode: PolicyMode) -> Self {
        // Deterministic initial orthogonal weights
        let mut weights = vec![0.0f32; feature_dim];
        for i in 0..feature_dim {
            weights[i] = ((i as f32 * 0.314).sin() * 0.1).clamp(-0.2, 0.2);
        }
        let ref_weights = weights.clone();

        Self {
            weights,
            ref_weights,
            feature_dim,
            learning_rate: DEFAULT_LEARNING_RATE,
            beta: DEFAULT_BETA_TEMPERATURE,
            mode,
        }
    }

    /// Selects a synthesis parameter action given a context state vector.
    pub fn select_action(&self, context: &[f32], seed: u64) -> SynthesisAction {
        match self.mode {
            PolicyMode::Disabled => SynthesisAction::default(),
            PolicyMode::Random => {
                // Pseudo-random sampling from matched seed
                let mut s = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let mut v = Vec::with_capacity(6);
                for _ in 0..6 {
                    s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let unit = (s >> 32) as f32 / u32::MAX as f32; // [0.0, 1.0]
                    v.push(unit * 2.0 - 1.0);
                }
                SynthesisAction::from_vector(&v)
            }
            PolicyMode::Heuristic => {
                // Fixed deterministic heuristic rules mapping context to adjustments
                let bpm_context = context.get(0).copied().unwrap_or(0.5);
                let dispersion = context.get(1).copied().unwrap_or(0.2);

                let delta_bpm = if bpm_context > 0.6 { -2.0 } else { 2.0 };
                let sub_bass = 1.0 + dispersion * 3.0;

                SynthesisAction {
                    delta_bpm,
                    vocal_rasp_delta: 0.05,
                    sub_bass_boost_db: sub_bass.clamp(0.0, 6.0),
                    treble_presence_db: 1.5,
                    dynamic_compression: 0.35,
                    vocal_wet_ratio: 0.25,
                }
            }
            PolicyMode::NoLearning | PolicyMode::Learned => {
                // Parametric action via weights
                let w = if self.mode == PolicyMode::NoLearning {
                    &self.ref_weights
                } else {
                    &self.weights
                };

                let mut action_v = vec![0.0f32; 6];
                for i in 0..6 {
                    let mut score = 0.0f32;
                    for c_idx in 0..context.len().min(self.feature_dim / 6) {
                        let w_idx = (i * (self.feature_dim / 6) + c_idx) % w.len();
                        score += context[c_idx] * w[w_idx];
                    }
                    action_v[i] = score.tanh();
                }

                SynthesisAction::from_vector(&action_v)
            }
        }
    }

    /// Online Direct Preference Optimization (DPO) update.
    /// Updates weights using Bradley-Terry gradient:
    /// \theta <- \theta + \eta * \beta * (1.0 - \hat{p}) * \Delta \phi
    pub fn update_dpo_preference(&mut self, pair: &DpoPreferencePair) {
        if self.mode != PolicyMode::Learned {
            return;
        }

        let phi_chosen = Self::construct_feature_phi(&pair.context_vector, &pair.chosen_action, self.feature_dim);
        let phi_rejected = Self::construct_feature_phi(&pair.context_vector, &pair.rejected_action, self.feature_dim);

        let mut delta_phi = vec![0.0f32; self.feature_dim];
        for i in 0..self.feature_dim {
            delta_phi[i] = phi_chosen[i] - phi_rejected[i];
        }

        // Implicit reward margin: beta * (theta - theta_ref)^T delta_phi
        let mut margin = 0.0f32;
        for i in 0..self.feature_dim {
            margin += (self.weights[i] - self.ref_weights[i]) * delta_phi[i];
        }
        margin *= self.beta;

        // Sigmoid probability: \hat{p} = \sigma(margin)
        let p_hat = 1.0 / (1.0 + (-margin).exp().clamp(1e-7, 1e7));

        // Feedback sign multiplier
        let sign = if pair.feedback_type == "revert" || pair.feedback_type == "thumbs_down" {
            -1.0f32
        } else {
            1.0f32
        };

        // Gradient update
        let step = self.learning_rate * self.beta * (1.0 - p_hat) * sign;
        for i in 0..self.feature_dim {
            self.weights[i] = (self.weights[i] + step * delta_phi[i]).clamp(-5.0, 5.0);
        }
    }

    /// Computes joint feature vector \phi(s, a) for policy scoring.
    pub fn construct_feature_phi(context: &[f32], action: &[f32], target_dim: usize) -> Vec<f32> {
        let mut phi = vec![0.0f32; target_dim];
        for (i, &a) in action.iter().enumerate() {
            for (j, &c) in context.iter().enumerate() {
                let idx = (i * context.len() + j) % target_dim;
                phi[idx] += a * c;
            }
        }
        let norm: f32 = phi.iter().map(|&x| x * x).sum::<f32>().sqrt();
        if norm > 1e-7 {
            for x in phi.iter_mut() {
                *x /= norm;
            }
        }
        phi
    }

    /// Evaluates the complete 6-component Composite Reward Function:
    /// R_total = w_1*R_style + w_2*R_harmony + w_3*R_rhythm + w_4*R_vocal + w_5*R_human + w_6*R_orig
    pub fn evaluate_composite_reward(
        candidate_embedding: &[f32],
        candidate_chroma: &[f32; 12],
        candidate_bpm: f32,
        tempo_drift_score: f32,
        has_clipping: bool,
        vocal_presence_ratio: f32,
        snr_db: f32,
        dynamic_range_db: f32,
        ensemble: &BlendedStyleProfile,
        originality_report: &LegalOriginalityReport,
        human_feedback: Option<f32>,
    ) -> CompositeRewardBreakdown {
        // 1. R_style in [0.0, 1.0]
        let emb_dot: f32 = candidate_embedding
            .iter()
            .zip(ensemble.blended_style_embedding.iter())
            .map(|(&a, &b)| a * b)
            .sum();
        let cos_sim = emb_dot.max(0.0);

        let chroma_dot: f32 = candidate_chroma
            .iter()
            .zip(ensemble.blended_chroma.iter())
            .map(|(&a, &b)| a * b)
            .sum();
        let chroma_sim = chroma_dot.max(0.0);

        let bpm_diff = (candidate_bpm - ensemble.blended_bpm).abs();
        let bpm_score = (-bpm_diff / 15.0).exp();

        let r_style = (0.50 * cos_sim + 0.30 * chroma_sim + 0.20 * bpm_score).clamp(0.0, 1.0);

        // 2. R_harmony in [0.0, 1.0]
        let r_harmony = (chroma_sim * 0.8 + 0.2).clamp(0.0, 1.0);

        // 3. R_rhythm in [0.0, 1.0]
        let clip_penalty = if has_clipping { 0.50 } else { 0.0 };
        let r_rhythm = ((-3.5 * tempo_drift_score).exp() * (1.0 - clip_penalty)).clamp(0.0, 1.0);

        // 4. R_vocal_clarity in [0.0, 1.0]
        let vocal_ratio_norm = (vocal_presence_ratio / 0.35).min(1.0);
        let snr_norm = ((snr_db.clamp(20.0, 90.0) - 20.0) / 70.0).clamp(0.0, 1.0);
        let dr_norm = ((dynamic_range_db.clamp(6.0, 18.0) - 6.0) / 12.0).clamp(0.0, 1.0);
        let r_vocal_clarity = (0.40 * vocal_ratio_norm + 0.30 * snr_norm + 0.30 * dr_norm).clamp(0.0, 1.0);

        // 5. R_human in [0.0, 1.0]
        let r_human = human_feedback.unwrap_or(0.50).clamp(0.0, 1.0);

        // 6. R_originality_guard in [-10.0, 1.0]
        let r_originality_guard = crate::originality_guard::LegalOriginalityGuard::calculate_originality_reward(originality_report);

        // Total Composite Reward
        let r_total = WEIGHT_STYLE * r_style
            + WEIGHT_HARMONY * r_harmony
            + WEIGHT_RHYTHM * r_rhythm
            + WEIGHT_VOCAL * r_vocal_clarity
            + WEIGHT_HUMAN * r_human
            + WEIGHT_ORIGINALITY * r_originality_guard;

        CompositeRewardBreakdown {
            r_style,
            r_harmony,
            r_rhythm,
            r_vocal_clarity,
            r_human,
            r_originality_guard,
            r_total,
            originality_report: originality_report.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use music_core::rl_contracts::{ComplianceStatus, LegalComplianceInfo};

    #[test]
    fn test_rl_policy_modes() {
        let policy_disabled = RlPolicy::new(36, PolicyMode::Disabled);
        let action_d = policy_disabled.select_action(&[0.5; 6], 1234);
        assert_eq!(action_d, SynthesisAction::default());

        let policy_random = RlPolicy::new(36, PolicyMode::Random);
        let action_r1 = policy_random.select_action(&[0.5; 6], 1001);
        let action_r2 = policy_random.select_action(&[0.5; 6], 1002);
        assert_ne!(action_r1, action_r2, "Random actions should vary across seeds");

        let policy_heuristic = RlPolicy::new(36, PolicyMode::Heuristic);
        let action_h = policy_heuristic.select_action(&[0.8, 0.4], 1234);
        assert_eq!(action_h.delta_bpm, -2.0);

        let policy_learned = RlPolicy::new(36, PolicyMode::Learned);
        let action_l = policy_learned.select_action(&[0.5; 6], 1234);
        assert!(action_l.delta_bpm >= -10.0 && action_l.delta_bpm <= 10.0);
    }

    #[test]
    fn test_dpo_preference_learning() {
        let mut policy = RlPolicy::new(12, PolicyMode::Learned);
        let initial_weights = policy.weights.clone();

        let pair = DpoPreferencePair {
            song_id: "song_123".to_string(),
            chosen_version: 2,
            rejected_version: 1,
            context_vector: vec![0.5, 0.2],
            chosen_action: vec![1.0, 0.5, 0.0, 0.0, 0.5, 0.2],
            rejected_action: vec![-1.0, -0.5, 0.0, 0.0, 0.1, 0.8],
            feedback_type: "keep".to_string(),
        };

        policy.update_dpo_preference(&pair);

        // Weights must update in non-zero direction
        let diff: f32 = policy
            .weights
            .iter()
            .zip(initial_weights.iter())
            .map(|(&a, &b)| (a - b).abs())
            .sum();
        assert!(diff > 1e-5, "DPO update must adjust weights, diff = {}", diff);
    }

    #[test]
    fn test_composite_reward_evaluation() {
        let ensemble = BlendedStyleProfile {
            reference_count: 2,
            file_weights: HashMap::new(),
            blended_bpm: 120.0,
            blended_chroma: [0.1; 12],
            consensus_key: "C Major".to_string(),
            blended_spectral_centroid_hz: 2500.0,
            blended_vocal_presence: 0.3,
            blended_timbre_subbands: [0.125; 8],
            blended_style_embedding: vec![0.1; 64],
            style_variance: 0.05,
        };

        let originality_report = LegalOriginalityReport {
            originality_score_pct: 95.0,
            max_single_file_overlap_pct: 12.0,
            most_similar_reference_file: "ref_1.wav".to_string(),
            max_melodic_sequence_notes: 3,
            max_waveform_cross_correlation: 0.2,
            max_chroma_cosine_similarity: 0.3,
            compliance_status: ComplianceStatus::Compliant100PctOriginal,
            is_compliant: true,
            legal_compliance: LegalComplianceInfo::default(),
            per_file_breakdown: Vec::new(),
        };

        let breakdown = RlPolicy::evaluate_composite_reward(
            &vec![0.1; 64],
            &[0.1; 12],
            120.0,
            0.01,
            false,
            0.3,
            45.0,
            12.0,
            &ensemble,
            &originality_report,
            Some(1.0), // Keep
        );

        assert!(breakdown.r_total >= 0.0 && breakdown.r_total <= 1.0);
        assert_eq!(breakdown.r_human, 1.0);
        assert!(breakdown.r_originality_guard > 0.8);
    }
}
