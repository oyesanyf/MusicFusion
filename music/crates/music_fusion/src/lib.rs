//! Standalone hardware memory sizing, dynamic Ollama model selection,
//! and multi-model producer panel for the AI Music Producer Agent.

pub mod decision_model;
pub mod error;
pub mod hf_model_discovery;
pub mod iteration_panel;
pub mod judge;
pub mod lyricist_studio;
pub mod memory;
pub mod ollama_client;
pub mod ollama_lifecycle;
pub mod originality_guard;
pub mod panel;
pub mod prompt_templates;
pub mod rl_policy;
pub mod tts_engine;
pub mod vision_mood;

// Re-export primary types
pub use decision_model::{
    calibrate_logits, DecisionRequest, DecisionResult, DecisionSchema, NimbleDecisionEngine,
    NimbleStatusSnapshot, DEFAULT_COMMERCIAL_NIMBLE_MODEL, DEFAULT_RESEARCH_NIMBLE_MODEL,
    HITL_CONFIDENCE_THRESHOLD,
};
pub use error::{FusionError, Result};
pub use hf_model_discovery::{
    default_models_cache_dir, is_hf_repo_cached, HfModelDiscoveryEngine, HfModelEntry,
    HfModelRegistry, ModelLicenseTier, OpenWeightCatalogItem, STAGE_DECISION, STAGE_FUSION_TTS,
    STAGE_GROOVE, STAGE_INPAINTING, STAGE_MASTERING, STAGE_VOICE_CONVERSION, STAGE_YUE,
};
pub use iteration_panel::{
    default_genre_suggestions, parse_bpm_from_critique, parse_key_from_critique, sanitize_llm_json,
    AssistIterationResponse, IterationFusionPanel, IterationResult, IterationSpecialistOutputs,
    ParameterTweaks,
};
pub use judge::{
    parse_judge_json, EvaluatedSongBlueprint, JudgeAnalysis, JudgeEvaluator, SectionArrangement,
};
pub use lyricist_studio::{
    compose_offline_structured_lyrics, format_sections, parse_lyrics_sections,
    structure_user_lyrics_verbatim, write_lyrics_from_prompt, ParsedLyricSection,
    StructuredLyricsResult,
};
pub use vision_mood::{analyze_visual_mood_local, VisionMoodSuggestion};
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
