use quorum_storage::SearchMode;
use quorum_types::{Citation, ScoredChunk, VolatileRemoteChunk};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use uuid::Uuid;

/// Unique identifiers for the agentic state machine nodes (LangGraph in Rust).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgenticStateName {
    ClassifyIntent,
    RetrieveMesh,
    GradeRelevance,
    RewriteQuery,
    SynthesizeWithCitations,
    Complete,
    Failed,
}

/// High-level classification of user query intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryIntentType {
    /// General knowledge or conversational QA (retrieval optional/empty)
    GeneralConversation,
    /// Targeted local file or code query
    LocalDocumentSearch,
    /// Cross-node federated mesh query
    FederatedMeshSearch,
    /// Complex multi-hop comparative reasoning across sources
    MultiHopReasoning,
}

/// Relevance assessment output from GradeRelevance node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelevanceGrade {
    pub is_relevant: bool,
    pub score: f32, // 0.0 .. 1.0
    pub reasoning: String,
    pub satisfactory: bool,
}

/// Synthesis model family options for GLM & ModelFusion routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisModelFamily {
    #[default]
    Glm4Agentic,
    Qwen25Code,
    GemmaProse,
    AutoFusion,
}

/// Full state context propagated through the agentic graph nodes.
#[derive(Debug)]
pub struct AgenticGraphState {
    pub session_id: Uuid,
    pub original_prompt: String,
    pub current_query: String,
    pub query_history: Vec<String>,
    pub hop_count: usize,
    pub max_hops: usize, // default: 3
    pub intent: Option<QueryIntentType>,
    pub search_mode: SearchMode,
    pub model_family: SynthesisModelFamily,
    pub selected_model_tag: String,
    pub routing_rationale: String,
    pub retrieved_chunks: Vec<ScoredChunk>,
    pub volatile_chunks: Vec<VolatileRemoteChunk>,
    pub relevance_grade: Option<RelevanceGrade>,
    pub relevance_score: f32,
    pub start_time: Instant,
    pub synthesized_response: Option<String>,
    pub citations: Vec<Citation>,
}

impl AgenticGraphState {
    pub fn new(prompt: &str, model_family: SynthesisModelFamily, max_hops: usize) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            original_prompt: prompt.to_string(),
            current_query: prompt.to_string(),
            query_history: vec![prompt.to_string()],
            hop_count: 0,
            max_hops,
            intent: None,
            search_mode: SearchMode::Hybrid,
            model_family,
            selected_model_tag: String::new(),
            routing_rationale: String::new(),
            retrieved_chunks: Vec::new(),
            volatile_chunks: Vec::new(),
            relevance_grade: None,
            relevance_score: 0.0,
            start_time: Instant::now(),
            synthesized_response: None,
            citations: Vec::new(),
        }
    }
}
