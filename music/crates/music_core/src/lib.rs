//! `music_core`: Core domain data structures, universal genre taxonomy, state management,
//! audio telemetry inspection, and song blueprints for the AI Music Producer Agent.

pub mod blueprint;
pub mod error;
pub mod genre;
pub mod rl_contracts;
pub mod schema;
pub mod state;
pub mod telemetry;

pub use error::{MusicCoreError, Result};
pub use genre::{GenreCategory, GenrePreset, GenreRegistry, GenreTaxonomy, ModalScale, RhythmGroove};
pub use rl_contracts::{
    AcousticFeatureVector, BaselineBenchmarkSummary, BlendedStyleProfile, ComplianceStatus,
    CompositeRewardBreakdown, ConfidenceInterval95, DpoPreferencePair, LegalComplianceInfo,
    LegalOriginalityReport, ReferenceWavMeta, SingleFileOriginalityAudit,
};
pub use schema::{PipelineModelStack, SongBlueprint, SongSection, StemMixConfig};
pub use state::{
    EngineMode, MusicProductionState, STEM_BASS, STEM_DRAFT_GROOVE, STEM_DRUMS, STEM_FINAL_MASTER,
    STEM_GUITAR, STEM_INPAINTED_DRUMS, STEM_INPAINTED_GUITAR, STEM_OTHER, STEM_RASPY_TENOR_VOCALS,
    STEM_YUE_INSTRUMENTALS, STEM_YUE_VOCALS,
};
pub use telemetry::StemTelemetry;
