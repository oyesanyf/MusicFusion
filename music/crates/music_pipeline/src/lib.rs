//! music_pipeline: Native Rust StateGraph DAG engine, CheckpointManager, and 6-stage sequential fusion nodes.

pub mod checkpointer;
pub mod error;
pub mod graph;
pub mod node;
pub mod nodes;
pub mod runner;

// Public re-exports
pub use checkpointer::CheckpointManager;
pub use error::{PipelineError, Result};
pub use graph::{CompiledGraph, Edge, StateGraph, ConditionFn, END, START};
pub use node::{FnNode, GraphNode, GraphObserver};
pub use nodes::*;
pub use runner::{build_production_pipeline, PipelineConfig, PipelineRunner};

/// Returns the current music_pipeline crate release version.
pub fn pipeline_version() -> &'static str {
    "0.1.0"
}
