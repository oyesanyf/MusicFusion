//! Strongly typed error handling for the native Rust StateGraph engine and pipeline.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("Node '{node}' execution failed: {message}")]
    NodeExecutionFailed {
        node: String,
        message: String,
    },

    #[error("Routing error from node '{node}': {reason}")]
    RoutingError {
        node: String,
        reason: String,
    },

    #[error("Conditional edge error at node '{node}': branch key '{branch}' not found in registered branches: {available:?}")]
    InvalidBranchKey {
        node: String,
        branch: String,
        available: Vec<String>,
    },

    #[error("Graph compilation error: {0}")]
    CompilationError(String),

    #[error("Graph validation error: node '{0}' does not exist in graph")]
    NodeNotFound(String),

    #[error("Duplicate node error: node '{0}' is already registered in graph")]
    DuplicateNode(String),

    #[error("Dead-end node '{0}' in graph: non-terminal node has no outgoing edges (must route to another node or END)")]
    DeadEndNode(String),

    #[error("Missing entry point: graph has no entry node defined (call set_entry_point or add_edge(START, ...))")]
    MissingEntryPoint,

    #[error("Cycle detected in graph without conditional exit: {0}")]
    CycleDetected(String),

    #[error("Max iterations ({iterations}) exceeded in graph execution at node '{node}' (potential infinite loop)")]
    MaxIterationsExceeded {
        node: String,
        iterations: usize,
    },

    #[error("Checkpoint I/O error at '{path}': {source}")]
    CheckpointIoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Checkpoint serialization error: {0}")]
    CheckpointSerialization(#[from] serde_json::Error),

    #[error("SQLite database error: {0}")]
    DatabaseError(String),

    #[error("Path breakout or invalid path security violation: {0}")]
    SecurityError(String),

    #[error("VRAM process isolation failure for stage '{stage}': {message}")]
    VramProcessIsolationError {
        stage: String,
        message: String,
    },

    #[error("Subprocess execution error for stage '{stage}': {message}")]
    SubprocessError {
        stage: String,
        message: String,
    },

    #[error("Subprocess execution timed out for stage '{stage}' after {timeout_secs}s")]
    SubprocessTimeout {
        stage: String,
        timeout_secs: u64,
    },

    #[error("Missing expected stem: {0}")]
    MissingStem(String),

    #[error("State error: {0}")]
    StateError(String),

    #[error("Fusion error: {0}")]
    FusionError(String),

    #[error("DSP error: {0}")]
    DspError(String),

    #[error("Standard I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Core domain error: {0}")]
    Core(#[from] music_core::MusicCoreError),

    #[error("Fusion domain error: {0}")]
    Fusion(#[from] music_fusion::FusionError),

    #[error("DSP domain error: {0}")]
    Dsp(#[from] music_dsp::DspError),

    #[error("Pipeline execution cancelled: {0}")]
    Cancelled(String),

    #[error("General pipeline error: {0}")]
    General(String),
}

pub type Result<T> = std::result::Result<T, PipelineError>;
