//! Standalone hardware memory sizing, dynamic Ollama model selection,
//! and multi-model producer panel for the AI Music Producer Agent.

pub mod error;
pub mod hf_model_discovery;
pub mod iteration_panel;
pub mod judge;
pub mod memory;
pub mod ollama_client;
pub mod ollama_lifecycle;
pub mod originality_guard;
pub mod panel;
pub mod prompt_templates;
pub mod rl_policy;
pub mod tts_engine;

// Re-export primary types
pub use error::{FusionError, Result};
pub use hf_model_discovery::{
    HfModelDiscoveryEngine, HfModelEntry, HfModelRegistry, STAGE_FUSION_TTS, STAGE_GROOVE,
    STAGE_INPAINTING, STAGE_MASTERING, STAGE_VOICE_CONVERSION, STAGE_YUE,
};
pub use iteration_panel::{
    default_genre_suggestions, parse_bpm_from_critique, parse_key_from_critique, sanitize_llm_json,
    AssistIterationResponse, IterationFusionPanel, IterationResult, IterationSpecialistOutputs,
    ParameterTweaks,
};
pub use judge::{
    parse_judge_json, EvaluatedSongBlueprint, JudgeAnalysis, JudgeEvaluator, SectionArrangement,
};
pub use memory::{
    create_hidden_command, ExecutionRoute, NeuralModelChoice, PipelineStageKind, StageType, SystemMemory,
};
pub use ollama_client::{
    ChatMessage, ChatOptions, ChatResponse, MissingModelGuidance, ModelTag, OllamaClient,
};
pub use ollama_lifecycle::{OllamaLifecycleManager, OllamaStatus};
pub use originality_guard::{
    LegalOriginalityGuard, CHROMA_COSINE_CEILING, HOOK_NOTE_LIMIT, PLAGIARISM_BREACH_THRESHOLD,
    WAVEFORM_CORRELATION_CEILING,
};
pub use panel::{ModelAnswer, OllamaPanelOutput, OllamaProducerPanel, SpecialistOutputs};
pub use prompt_templates::PromptPair;
pub use rl_policy::{
    PolicyMode, RlPolicy, SynthesisAction, DEFAULT_BETA_TEMPERATURE, DEFAULT_LEARNING_RATE,
    WEIGHT_HARMONY, WEIGHT_HUMAN, WEIGHT_ORIGINALITY, WEIGHT_RHYTHM, WEIGHT_STYLE, WEIGHT_VOCAL,
};
pub use tts_engine::{FusionTtsEngine, FusionTtsModel, TtsVocalRequest, TtsVocalResponse};
