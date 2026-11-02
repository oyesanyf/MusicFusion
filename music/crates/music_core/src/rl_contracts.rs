use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// High-dimensional acoustic feature profile extracted from a reference WAV.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcousticFeatureVector {
    pub file_name: String,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub channels: u16,
    pub bpm: f32,
    pub chroma_12bin: [f32; 12],
    pub key_signature: String,
    pub key_confidence: f32,
    pub spectral_centroid_hz: f32,
    pub crest_factor_db: f32,
    pub dynamic_range_db: f32,
    pub vocal_presence_ratio: f32,
    pub vocal_activity_ratio: f32,
    pub timbre_subbands: [f32; 8],
    pub style_embedding: Vec<f32>, // 64 dimensions, L2 normalized
    pub pitch_token_sequence: Vec<u8>, // Beat-synchronous dominant pitch class (0..11)
}

/// Metadata stored for an uploaded reference WAV.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceWavMeta {
    pub id: String,
    pub file_name: String,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub channels: u16,
    pub file_size_bytes: u64,
    pub bpm: f32,
    pub key_signature: String,
    pub uploaded_at: String,
}

/// Ensemble blended style distribution across up to 20 reference WAVs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlendedStyleProfile {
    pub reference_count: usize,
    pub file_weights: HashMap<String, f32>,
    pub blended_bpm: f32,
    pub blended_chroma: [f32; 12],
    pub consensus_key: String,
    pub blended_spectral_centroid_hz: f32,
    pub blended_vocal_presence: f32,
    pub blended_timbre_subbands: [f32; 8],
    pub blended_style_embedding: Vec<f32>,
    pub style_variance: f32,
}

/// Legal compliance status classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceStatus {
    Compliant100PctOriginal,
    CompliantStyleBlended,
    RejectedPotentialCopy,
}

/// Single-file similarity audit against one reference WAV.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SingleFileOriginalityAudit {
    pub reference_file: String,
    pub pitch_sequence_overlap_notes: usize,
    pub pitch_sequence_overlap_ratio: f32,
    pub waveform_cross_correlation: f32,
    pub chroma_cosine_similarity: f32,
    pub composite_similarity_score: f32,
    pub is_breached: bool,
}

/// Legal compliance attestations for statutory and judicial compliance
/// (Bridgeport Music sampling protection & Tennessee ELVIS Act vocal protection).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalComplianceInfo {
    /// Zero direct waveform splicing/sampling into output audio (Bridgeport Music safeguard)
    pub zero_waveform_sampling: bool,
    /// Verification flag confirming master audio was synthesized 100% from scratch
    pub no_master_sampling_verified: bool,
    /// Maximum single-file pitch/melody overlap percentage across all reference files (< 15%)
    pub max_single_file_melody_overlap_pct: f32,
    /// Synthetic voice only, zero biometric speaker cloning (ELVIS Act compliance)
    pub synthetic_voice_only_elvis_compliant: bool,
    /// Synthetic vocal timbre only, strictly mix targets used
    pub synthetic_vocal_timbre_only: bool,
    /// Overall originality score percentage (e.g. 94.0% - 100.0%)
    pub originality_score_pct: f32,
    /// Affirmative verification flag confirming all legal safeguards passed
    pub legal_safeguard_verified: bool,
}

impl Default for LegalComplianceInfo {
    fn default() -> Self {
        Self {
            zero_waveform_sampling: true,
            no_master_sampling_verified: true,
            max_single_file_melody_overlap_pct: 0.0,
            synthetic_voice_only_elvis_compliant: true,
            synthetic_vocal_timbre_only: true,
            originality_score_pct: 100.0,
            legal_safeguard_verified: true,
        }
    }
}

/// Verifiable Legal Originality & Anti-Plagiarism Report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalOriginalityReport {
    pub originality_score_pct: f32, // e.g. 98.4%
    pub max_single_file_overlap_pct: f32, // e.g. 14.2%
    pub most_similar_reference_file: String,
    pub max_melodic_sequence_notes: usize,
    pub max_waveform_cross_correlation: f32,
    pub max_chroma_cosine_similarity: f32,
    pub compliance_status: ComplianceStatus,
    pub is_compliant: bool,
    pub legal_compliance: LegalComplianceInfo,
    pub per_file_breakdown: Vec<SingleFileOriginalityAudit>,
}

/// Breakdown of the 6 components of the Composite Reward Function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositeRewardBreakdown {
    pub r_style: f32,
    pub r_harmony: f32,
    pub r_rhythm: f32,
    pub r_vocal_clarity: f32,
    pub r_human: f32,
    pub r_originality_guard: f32,
    pub r_total: f32,
    pub originality_report: LegalOriginalityReport,
}

/// Feedback pair for online DPO preference learning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DpoPreferencePair {
    pub song_id: String,
    pub chosen_version: u32,
    pub rejected_version: u32,
    pub context_vector: Vec<f32>,
    pub chosen_action: Vec<f32>,
    pub rejected_action: Vec<f32>,
    pub feedback_type: String, // "keep", "revert", "thumbs_up", "thumbs_down"
}

/// 95% Confidence Interval container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceInterval95 {
    pub mean: f32,
    pub standard_deviation: f32,
    pub standard_error: f32,
    pub lower_bound: f32,
    pub upper_bound: f32,
}

/// Empirical baseline evaluation summary for RL benchmark.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaselineBenchmarkSummary {
    pub baseline_name: String, // "Disabled", "Random", "Heuristic", "NoLearning", "Learned"
    pub backend: String, // "SyntheticAudioEnvironment"
    pub episodes: usize,
    pub reward_ci95: ConfidenceInterval95,
    pub originality_ci95: ConfidenceInterval95,
    pub breach_frequency_ci95: ConfidenceInterval95,
}
