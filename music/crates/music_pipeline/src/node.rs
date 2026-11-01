//! Node traits and execution contracts for StateGraph.

use async_trait::async_trait;
use music_core::MusicProductionState;
use crate::error::PipelineError;

/// Asynchronous node trait in the native Rust StateGraph.
#[async_trait]
pub trait GraphNode: Send + Sync {
    /// Executes the stage, directly mutating the song production state.
    async fn execute(&self, state: &mut MusicProductionState) -> Result<(), PipelineError>;

    /// Unique identifier for this node (optional, default empty string).
    fn name(&self) -> &str {
        ""
    }

    /// Human-readable description of node functionality.
    fn description(&self) -> &str {
        ""
    }
}

/// Real-time observer interface for progress streaming (e.g., Tokio REST SSE events).
#[async_trait]
pub trait GraphObserver: Send + Sync {
    async fn on_node_start(&self, node: &str, state: &MusicProductionState);
    async fn on_node_complete(&self, node: &str, state: &MusicProductionState, duration_ms: u64);
    async fn on_node_error(&self, node: &str, error: &PipelineError);
}

/// Convenience struct to wrap a synchronous mutation function as a GraphNode.
pub struct FnNode<F>
where
    F: Fn(&mut MusicProductionState) -> Result<(), PipelineError> + Send + Sync + 'static,
{
    node_name: String,
    handler: F,
}

impl<F> FnNode<F>
where
    F: Fn(&mut MusicProductionState) -> Result<(), PipelineError> + Send + Sync + 'static,
{
    pub fn new(name: impl Into<String>, handler: F) -> Self {
        Self {
            node_name: name.into(),
            handler,
        }
    }
}

#[async_trait]
impl<F> GraphNode for FnNode<F>
where
    F: Fn(&mut MusicProductionState) -> Result<(), PipelineError> + Send + Sync + 'static,
{
    async fn execute(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        (self.handler)(state)
    }

    fn name(&self) -> &str {
        &self.node_name
    }
}
