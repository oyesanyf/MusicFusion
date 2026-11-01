//! Standalone hardware memory sizing, dynamic Ollama model selection,
//! and multi-model producer panel for the AI Music Producer Agent.

pub mod error;
pub mod judge;
pub mod memory;
pub mod ollama_client;
pub mod panel;
pub mod prompt_templates;

// Re-export primary types
pub use error::{FusionError, Result};
pub use judge::{
    parse_judge_json, EvaluatedSongBlueprint, JudgeAnalysis, JudgeEvaluator, SectionArrangement,
};
pub use memory::{
    create_hidden_command, ExecutionRoute, PipelineStageKind, StageType, SystemMemory,
};
pub use ollama_client::{
    ChatMessage, ChatOptions, ChatResponse, MissingModelGuidance, ModelTag, OllamaClient,
};
pub use panel::{ModelAnswer, OllamaPanelOutput, OllamaProducerPanel, SpecialistOutputs};
pub use prompt_templates::PromptPair;
