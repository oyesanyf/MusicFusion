//! Native Rust StateGraph DAG / Pregel execution engine.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use music_core::state::MusicProductionState;
use crate::error::{PipelineError, Result};
pub use crate::node::{GraphNode, GraphObserver};
use crate::checkpointer::CheckpointManager;

pub const START: &str = "__start__";
pub const END: &str = "__end__";

pub type ConditionFn = Arc<dyn Fn(&MusicProductionState) -> String + Send + Sync>;

pub enum Edge {
    Direct(String),
    Conditional {
        condition: ConditionFn,
        branches: HashMap<String, String>,
    },
}

pub struct StateGraph {
    nodes: HashMap<String, Arc<dyn GraphNode>>,
    edges: HashMap<String, Edge>,
    entry_point: Option<String>,
}

impl Default for StateGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl StateGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            entry_point: None,
        }
    }

    /// Registers a node in the graph.
    pub fn add_node<N>(&mut self, name: &str, node: N) -> &mut Self
    where
        N: GraphNode + 'static,
    {
        self.nodes.insert(name.to_string(), Arc::new(node));
        self
    }

    /// Explicitly designates the graph's entry node.
    pub fn set_entry_point(&mut self, node: &str) -> &mut Self {
        self.entry_point = Some(node.to_string());
        self
    }

    /// Registers a direct unconditional transition edge.
    pub fn add_edge(&mut self, from: &str, to: &str) -> &mut Self {
        if from == START {
            self.entry_point = Some(to.to_string());
        } else {
            self.edges.insert(from.to_string(), Edge::Direct(to.to_string()));
        }
        self
    }

    /// Registers a dynamic conditional edge branching off state evaluation.
    pub fn add_conditional_edges<F, S1, S2>(
        &mut self,
        from: &str,
        condition_fn: F,
        branches: HashMap<S1, S2>,
    ) -> &mut Self
    where
        F: Fn(&MusicProductionState) -> String + Send + Sync + 'static,
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        let string_branches = branches
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
            .collect();

        self.edges.insert(
            from.to_string(),
            Edge::Conditional {
                condition: Arc::new(condition_fn),
                branches: string_branches,
            },
        );
        self
    }

    /// Compiles and validates graph topology into an immutable execution engine.
    pub fn compile(self) -> Result<CompiledGraph> {
        let entry = self.entry_point.ok_or(PipelineError::MissingEntryPoint)?;
        if !self.nodes.contains_key(&entry) {
            return Err(PipelineError::NodeNotFound(entry));
        }

        // Validate all edges point to valid nodes or END
        for (from, edge) in &self.edges {
            if !self.nodes.contains_key(from) {
                return Err(PipelineError::NodeNotFound(from.clone()));
            }
            match edge {
                Edge::Direct(to) => {
                    if to != END && !self.nodes.contains_key(to) {
                        return Err(PipelineError::NodeNotFound(to.clone()));
                    }
                }
                Edge::Conditional { branches, .. } => {
                    for to in branches.values() {
                        if to != END && !self.nodes.contains_key(to) {
                            return Err(PipelineError::NodeNotFound(to.clone()));
                        }
                    }
                }
            }
        }

        Ok(CompiledGraph {
            nodes: self.nodes,
            edges: self.edges,
            entry_point: entry,
            max_iterations: 50,
        })
    }
}

pub struct CompiledGraph {
    nodes: HashMap<String, Arc<dyn GraphNode>>,
    edges: HashMap<String, Edge>,
    entry_point: String,
    max_iterations: usize,
}

impl CompiledGraph {
    /// Returns current max iterations.
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    /// Configures max iterations.
    pub fn set_max_iterations(&mut self, max: usize) {
        self.max_iterations = max;
    }

    /// Checks if a node exists in compiled graph.
    pub fn has_node(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }

    /// Returns list of registered node names.
    pub fn node_names(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    /// Returns the entry point node name.
    pub fn entry_point(&self) -> &str {
        &self.entry_point
    }

    /// Executes the full graph from entry point to END.
    pub async fn execute(
        &self,
        mut initial_state: MusicProductionState,
    ) -> Result<MusicProductionState> {
        self.execute_internal(&mut initial_state, None, None, None, None).await?;
        Ok(initial_state)
    }

    /// Executes the full graph with automated state checkpointing.
    pub async fn execute_with_checkpointer(
        &self,
        mut initial_state: MusicProductionState,
        checkpointer: &CheckpointManager,
    ) -> Result<MusicProductionState> {
        self.execute_internal(&mut initial_state, Some(checkpointer), None, None, None).await?;
        Ok(initial_state)
    }

    /// Targeted single-node or sub-graph execution (e.g. for stem regeneration or mastering).
    pub async fn execute_from_node(
        &self,
        start_node: &str,
        stop_node: Option<&str>,
        mut state: MusicProductionState,
        checkpointer: Option<&CheckpointManager>,
    ) -> Result<MusicProductionState> {
        self.execute_internal(&mut state, checkpointer, None, Some(start_node), stop_node).await?;
        Ok(state)
    }

    /// Full execution loop with optional checkpointer and observer hooks.
    pub async fn execute_with_observer(
        &self,
        mut state: MusicProductionState,
        checkpointer: Option<&CheckpointManager>,
        observer: Option<&Arc<dyn GraphObserver>>,
    ) -> Result<MusicProductionState> {
        self.execute_internal(&mut state, checkpointer, observer, None, None).await?;
        Ok(state)
    }

    async fn execute_internal(
        &self,
        state: &mut MusicProductionState,
        checkpointer: Option<&CheckpointManager>,
        observer: Option<&Arc<dyn GraphObserver>>,
        start_node: Option<&str>,
        stop_node: Option<&str>,
    ) -> Result<()> {
        let mut current_node = start_node.unwrap_or(&self.entry_point).to_string();
        let mut iterations = 0;

        while current_node != END {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(PipelineError::MaxIterationsExceeded {
                    node: current_node,
                    iterations,
                });
            }

            let node = self.nodes.get(&current_node).ok_or_else(|| {
                PipelineError::NodeNotFound(current_node.clone())
            })?;

            state.record_node_transition(&current_node);

            if let Some(obs) = observer {
                obs.on_node_start(&current_node, state).await;
            }

            let start_time = Instant::now();
            let exec_res = node.execute(state).await;

            match exec_res {
                Ok(()) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    if let Some(obs) = observer {
                        obs.on_node_complete(&current_node, state, duration_ms).await;
                    }

                    // Save checkpoint if enabled
                    if let Some(cp) = checkpointer {
                        if let Err(e) = cp.save_checkpoint(state, &current_node).await {
                            log::warn!("Checkpointer warning at node {}: {}", current_node, e);
                        }
                    }

                    // If caller requested a stop node, break here
                    if let Some(stop) = stop_node {
                        if current_node == stop {
                            break;
                        }
                    }

                    // Advance via registered edge
                    match self.edges.get(&current_node) {
                        Some(Edge::Direct(target)) => {
                            current_node = target.clone();
                        }
                        Some(Edge::Conditional { condition, branches }) => {
                            let branch_key = condition(state);
                            match branches.get(&branch_key) {
                                Some(target) => {
                                    current_node = target.clone();
                                }
                                None => {
                                    return Err(PipelineError::InvalidBranchKey {
                                        node: current_node,
                                        branch: branch_key,
                                        available: branches.keys().cloned().collect(),
                                    });
                                }
                            }
                        }
                        None => {
                            current_node = END.to_string();
                        }
                    }
                }
                Err(err) => {
                    if let Some(obs) = observer {
                        obs.on_node_error(&current_node, &err).await;
                    }
                    return Err(err);
                }
            }
        }

        Ok(())
    }
}
