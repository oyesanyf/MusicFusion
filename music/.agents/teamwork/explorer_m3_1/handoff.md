# Architectural Specification & Handoff Report: Milestone 3 (Native Rust StateGraph Engine & Checkpointer)

**Author**: Explorer M3-1  
**Milestone**: Milestone 3 — Native Rust StateGraph & 6 Fusion Nodes  
**Date**: 2026-11-01  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m3_1`  
**Target Crate**: `crates/music_pipeline`  

---

## 1. Observation

### 1.1 Existing Workspace & Repository State
Direct filesystem inspection confirms:
- **Workspace Configuration (`Cargo.toml`)**:
  Lines 3–9 define 5 member crates: `music_core`, `music_fusion`, `music_dsp`, `music_pipeline`, `music_cli`.
  Lines 12–33 define workspace dependencies: `tokio` (v1, "full"), `serde` (v1, "derive"), `serde_json` (v1), `anyhow` (v1), `thiserror` (v1), `reqwest` (v0.12), `rusqlite` (v0.31, "bundled"), `chrono` (v0.4, "serde"), `regex` (v1), `clap` (v4), `async-trait` (v0.1), `futures` (v0.3), `sysinfo` (v0.30), `uuid` (v1.10).
- **Milestone 1 & Milestone 2 Verification**:
  Execution of `cargo test --workspace` passed 150 unit, integration, and adversarial tests across `music_core`, `music_dsp`, and `music_fusion` with zero failures (exit code 0).
- **Target Crate State (`crates/music_pipeline`)**:
  - `crates/music_pipeline/Cargo.toml` contains only `music_core`, `tokio`, `serde`, `serde_json`, `anyhow`, `thiserror`. It currently lacks paths to `music_dsp`, `music_fusion`, `async-trait`, `log`, `chrono`, `uuid`, and `rusqlite`.
  - `crates/music_pipeline/src/lib.rs` currently contains only a stub:
    ```rust
    pub fn pipeline_version() -> &'static str {
        "0.1.0-stub"
    }
    ```

### 1.2 Authoritative Requirement Grounding
Direct inspection of `ORIGINAL_REQUEST.md`, `PROJECT.md`, and `TEST_INFRA.md` reveals:
1. **LangGraph & ModelFusion Elimination (`ORIGINAL_REQUEST.md §1 & §2`, `PROJECT.md Feature 1 & 5`)**:
   - Standalone 100% Rust architecture; zero Python LangGraph; zero runtime link to ModelFusion.
   - Self-contained async `tokio` Pregel / DAG `StateGraph<MusicProductionState>` engine with `add_node`, `add_edge`, `add_conditional_edges`, and state checkpointing per song version.
2. **State Container (`crates/music_core/src/state.rs`)**:
   - `MusicProductionState` contains 21 fields (including `song_id`, `version`, `engine_mode`, `style_prompt`, `lyrics`, `bpm`, `key_signature`, `vocal_ref_wav`, `master_ref_wav`, `stem_paths`, `telemetry`, `judge_score`, `quality_passed`, `retry_count`, `genre`, `time_signature`, `blueprint`, `active_node`, `history`, `created_at`, `updated_at`).
   - Canonical stem constants in `crates/music_core/src/state.rs`: `draft_groove`, `yue_vocals`, `yue_instrumentals`, `raspy_tenor_vocals`, `drums`, `bass`, `guitar`, `other`, `inpainted_guitar`, `inpainted_drums`, `final_master`.
3. **Execution Model & Node Pipeline (`ORIGINAL_REQUEST.md §3`, `PROJECT.md Feature 14–20`)**:
   - Sequential 6-node pipeline:
     - Node 1: `ollama_producer_panel_node`
     - Node 2: `groove_and_yue_synthesis_node`
     - Node 3: `vocal_timbre_fusion_node`
     - Node 4: `virtuoso_stem_inpainting_node`
     - Node 5: `dsp_and_judge_gate_node` (Quality gate & conditional router)
     - Node 6: `analog_mastering_node`
   - Node 5 conditional routing rule:
     - If `quality_passed == true` or `retry_count >= 2`: route to `analog_mastering_node`.
     - Else if `retry_count < 2`:
       - If `telemetry.tempo_drift_score > 0.5`: route to `groove_and_yue_synthesis_node`.
       - Else: route to `virtuoso_stem_inpainting_node`.
4. **Targeted Sub-Graph & Stem Regeneration (`ORIGINAL_REQUEST.md §4`, `PROJECT.md § Interface Contracts`)**:
   - Support full graph execution (`START` to `END`) AND targeted single-node / sub-graph re-execution for endpoints like `POST /api/songs/regenerate-stem` (Node 4) and `POST /api/songs/master` (Node 6).
5. **Checkpointer & Storage Invariant (`ORIGINAL_REQUEST.md §2`, `PROJECT.md § Interface Contracts`)**:
   - Saves state to `./storage/songs/<song_id>/v<version>/state.json`.
   - Persists intermediate WAV stems to `./storage/songs/<song_id>/v<version>/stems/<stem_name>.wav`.
   - SQLite version metadata and audit logging (`music_storage.db`).
   - Anti-breakout path traversal defense (`containment_ring_breakout_invariants.md`).
6. **VRAM Process Isolation Invariant (`ORIGINAL_REQUEST.md §3`, `PROJECT.md Feature 20`)**:
   - For external Hugging Face stages, spawn external worker processes via `tokio::process::Command` with `CREATE_NO_WINDOW`, await exit, and drop handle so Windows WDDM/CUDA driver reclaims 100% of GPU VRAM between stages.
   - Transparent pure-Rust DSP fallback when VRAM < 4.0 GB or `engine_mode == "pure_rust"`.

---

## 2. Logic Chain

1. **Standalone Requirement → Native Async Pregel / DAG Architecture**:
   Since Python LangGraph is prohibited and the architecture must be 100% Rust, we must implement a lightweight, robust, thread-safe asynchronous directed acyclic graph (DAG) engine with Pregel superstep semantics. The builder pattern (`StateGraph`) collects nodes, edges, conditional branches, and entry points, then produces an immutable `CompiledGraph` via `compile()`.
2. **In-Place State Mutation → Zero-Copy `&mut MusicProductionState` Node Trait**:
   Audio stems, blueprint section arrays, and lyric strings represent non-trivial allocations. Passing `&mut MusicProductionState` into `GraphNode::execute` allows nodes to update stem paths, telemetry, and scores in place without copying, guaranteeing optimal runtime performance and memory safety.
3. **Branching Flexibility → String-Keyed Conditional Edge Routing**:
   The conditional edge requirement in `PROJECT.md` specifies `add_conditional_edges(from: &str, condition_fn, branches: HashMap<String, String>)`. By parameterizing `condition_fn` as `Arc<dyn Fn(&MusicProductionState) -> String + Send + Sync>`, the gate node evaluates the state and outputs a semantic branch key (`"success"`, `"retry_stem"`, `"retry_groove"`, `"exhausted"`), which dynamically resolves to the next target node.
4. **Infinite Loop Prevention → Loop Iteration Threshold (`max_iterations`)**:
   In cyclic/retry graphs, an uncalibrated gate could theoretically route between Node 5 and Node 4 infinitely if retry counters fail to increment. Enforcing a hard execution limit (`max_iterations = 50`) in `CompiledGraph` prevents task hangs and returns `PipelineError::MaxIterationsExceeded`.
5. **REST API Streaming → Graph Observer Pattern**:
   To satisfy Milestone 4's SSE streaming requirement (`POST /api/songs/create` progress updates) without coupling to Axum, the execution engine should accept an optional `GraphObserver` trait or async channel sender that notifies listeners at node start, completion, and error.
6. **Security & Path Breakout Defense → Strict Path Normalization**:
   To comply with the workspace's multi-ring containment invariants, `CheckpointManager` must validate every `song_id` and stem name component against traversal attacks (`..`, `:`, `/`, `\`, null bytes, percent-encoded sequences), ensuring state snapshots and WAV files never write outside the designated `./storage/` root directory.
7. **Process Isolation → Asynchronous Subprocess Execution with Fallback**:
   To satisfy Feature 20 without leaking GPU memory, hybrid nodes check `SystemMemory::detect_live().should_use_dsp()`. If external weights are enabled and VRAM suffices, `tokio::process::Command` executes with `CREATE_NO_WINDOW`, awaiting child process exit before the graph step concludes. If VRAM is constrained or the child fails, the node transparently falls back to `music_dsp`.

---

## 3. Caveats

1. **Subprocess Binaries in Development**: In test environments or machines lacking NVIDIA CUDA GPUs or local Python model scripts (`YuE`, `ACE-Step`, `Seed-VC`, `HTDemucs`, `AudioSR`), all nodes fall back to the built-in 48 kHz pure-Rust DSP engine (`music_dsp`). This is expected behavior and guaranteed by the zero-mock invariant.
2. **SQLite Dependency**: `rusqlite` is declared in workspace dependencies with feature `["bundled"]`. If disk writes fail (e.g. read-only disk testing in Tier 2), `CheckpointManager` logs an error and allows in-memory state execution to continue without panicking.
3. **No Dynamic Plugin Loading**: All 6 sequential fusion nodes are compiled directly into `music_pipeline`, preventing any dynamic link or unsafe C ABI boundary.

---

## 4. Conclusion & Complete Architectural Specification

### 4.1 Crate Dependencies (`crates/music_pipeline/Cargo.toml`)
Update `crates/music_pipeline/Cargo.toml` to link workspace dependencies and sibling crates:

```toml
[package]
name = "music_pipeline"
version = "0.1.0"
edition = "2021"

[dependencies]
music_core = { path = "../music_core" }
music_dsp = { path = "../music_dsp" }
music_fusion = { path = "../music_fusion" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
log = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
rusqlite = { workspace = true }

[dev-dependencies]
tempfile = "3"
```

### 4.2 Module Layout (`crates/music_pipeline/src/`)
```
crates/music_pipeline/src/
├── lib.rs                  # Public exports: StateGraph, CompiledGraph, CheckpointManager, PipelineError, nodes
├── error.rs                # Strongly typed PipelineError enum with thiserror
├── node.rs                 # GraphNode trait, NodeExecutionResult, GraphObserver
├── graph.rs                # StateGraph builder, CompiledGraph execution engine, Edge routing
├── checkpointer.rs         # CheckpointManager, version snapshots, WAV stem persistence, path sanitization
├── runner.rs               # build_production_pipeline() helper, pipeline execution configuration
└── nodes/
    ├── mod.rs              # Re-exports of all 6 nodes
    ├── node1_producer_panel.rs   # Stage 0: Ollama Multi-LLM Producer Panel
    ├── node2_groove_yue.rs        # Stages 1 & 2: Groove Foundation & YuE Dual-Stem Synthesis
    ├── node3_vocal_timbre.rs      # Stage 3: Soulful Raspy Jamaican Tenor Voice Conversion
    ├── node4_stem_inpainting.rs   # Stage 4: 4-Stem Separation & Virtuosic Guitar Solo Inpainting
    ├── node5_dsp_judge_gate.rs    # Stage Quality Gate: Spectral Telemetry & AI Judge Gate
    └── node6_analog_mastering.rs  # Stage 5: 48 kHz Studer A800 Analog Tape & Sub-Bass Mastering
```

---

### 4.3 Strongly-Typed `PipelineError` (`crates/music_pipeline/src/error.rs`)

```rust
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

    #[error("External subprocess execution failed for stage '{stage}': exit code {code:?}, stderr: {stderr}")]
    SubprocessError {
        stage: String,
        code: Option<i32>,
        stderr: String,
    },

    #[error("Core domain error: {0}")]
    Core(#[from] music_core::MusicCoreError),

    #[error("Fusion error: {0}")]
    Fusion(#[from] music_fusion::FusionError),

    #[error("DSP error: {0}")]
    Dsp(#[from] music_dsp::DspError),

    #[error("Standard I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Pipeline execution cancelled: {0}")]
    Cancelled(String),

    #[error("General pipeline error: {0}")]
    General(String),
}

pub type Result<T> = std::result::Result<T, PipelineError>;
```

---

### 4.4 GraphNode Trait & Observer Interface (`crates/music_pipeline/src/node.rs`)

```rust
//! Node trait and execution contracts for StateGraph.

use async_trait::async_trait;
use music_core::MusicProductionState;
use crate::error::PipelineError;

/// Result status of a single node's execution step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeExecutionResult {
    /// Node executed successfully; execution advances to next edge.
    Continue,
    /// Node explicitly chose a named route.
    Branch(String),
}

/// Asynchronous node trait in the native Rust StateGraph.
#[async_trait]
pub trait GraphNode: Send + Sync {
    /// Unique identifier for this node (e.g. "ollama_producer_panel_node").
    fn name(&self) -> &str;

    /// Human-readable description of node functionality.
    fn description(&self) -> &str {
        ""
    }

    /// Executes the stage, directly mutating the song production state.
    async fn execute(&self, state: &mut MusicProductionState) -> Result<NodeExecutionResult, PipelineError>;
}

/// Real-time observer interface for progress streaming (e.g., Tokio REST SSE events).
#[async_trait]
pub trait GraphObserver: Send + Sync {
    async fn on_node_start(&self, node: &str, state: &MusicProductionState);
    async fn on_node_complete(&self, node: &str, state: &MusicProductionState, duration_ms: u64);
    async fn on_node_error(&self, node: &str, error: &PipelineError);
}
```

---

### 4.5 StateGraph Engine & CompiledGraph (`crates/music_pipeline/src/graph.rs`)

```rust
//! Native Rust StateGraph DAG / Pregel execution engine.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use music_core::MusicProductionState;
use crate::error::{PipelineError, Result};
use crate::node::{GraphNode, GraphObserver, NodeExecutionResult};
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
    pub fn add_conditional_edges<F>(
        &mut self,
        from: &str,
        condition_fn: F,
        branches: HashMap<String, String>,
    ) -> &mut Self
    where
        F: Fn(&MusicProductionState) -> String + Send + Sync + 'static,
    {
        self.edges.insert(
            from.to_string(),
            Edge::Conditional {
                condition: Arc::new(condition_fn),
                branches,
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
                Ok(result) => {
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

                    // Compute next node
                    match result {
                        NodeExecutionResult::Branch(explicit_target) => {
                            current_node = explicit_target;
                        }
                        NodeExecutionResult::Continue => {
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
```

---

### 4.6 Checkpoint Manager (`crates/music_pipeline/src/checkpointer.rs`)

```rust
//! Checkpoint manager for song versioning, state snapshotting, and WAV stem persistence.

use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use music_core::MusicProductionState;
use music_dsp::wav::AudioBuffer;
use crate::error::{PipelineError, Result};

pub struct CheckpointManager {
    base_dir: PathBuf,
    sqlite_conn: Option<Arc<Mutex<rusqlite::Connection>>>,
}

impl CheckpointManager {
    /// Creates a file-based checkpointer root at base_dir.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            sqlite_conn: None,
        }
    }

    /// Initializes checkpointer with embedded SQLite metadata persistence.
    pub fn with_sqlite(base_dir: impl Into<PathBuf>, db_name: &str) -> Result<Self> {
        let base = base_dir.into();
        std::fs::create_dir_all(&base)?;
        let db_path = base.join(db_name);
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| PipelineError::DatabaseError(e.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS song_versions (
                song_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                active_node TEXT,
                state_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (song_id, version)
            )",
            [],
        ).map_err(|e| PipelineError::DatabaseError(e.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS stem_records (
                song_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                stem_name TEXT NOT NULL,
                stem_path TEXT NOT NULL,
                PRIMARY KEY (song_id, version, stem_name)
            )",
            [],
        ).map_err(|e| PipelineError::DatabaseError(e.to_string()))?;

        Ok(Self {
            base_dir: base,
            sqlite_conn: Some(Arc::new(Mutex::new(conn))),
        })
    }

    /// Validates path component preventing directory traversal and NT ADS breakouts.
    pub fn sanitize_path_component(component: &str) -> Result<String> {
        if component.is_empty()
            || component.contains('/')
            || component.contains('\\')
            || component.contains(':')
            || component.contains('\0')
            || component.contains("..")
            || component.contains("%2e")
            || component.contains("%2f")
            || component.contains("%5c")
        {
            return Err(PipelineError::SecurityError(format!(
                "Invalid path component: '{}'",
                component
            )));
        }
        let p = Path::new(component);
        for comp in p.components() {
            if !matches!(comp, Component::Normal(_)) {
                return Err(PipelineError::SecurityError(format!(
                    "Path traversal attempt rejected in component: '{}'",
                    component
                )));
            }
        }
        Ok(component.to_string())
    }

    pub fn version_dir(&self, song_id: &str, version: u32) -> Result<PathBuf> {
        let clean_id = Self::sanitize_path_component(song_id)?;
        Ok(self.base_dir.join("songs").join(clean_id).join(format!("v{}", version)))
    }

    pub fn stems_dir(&self, song_id: &str, version: u32) -> Result<PathBuf> {
        Ok(self.version_dir(song_id, version)?.join("stems"))
    }

    /// Persists state snapshot to `./storage/songs/<song_id>/v<version>/state.json`.
    pub async fn save_checkpoint(
        &self,
        state: &MusicProductionState,
        active_node: &str,
    ) -> Result<PathBuf> {
        let v_dir = self.version_dir(&state.song_id, state.version)?;
        std::fs::create_dir_all(&v_dir).map_err(|e| PipelineError::CheckpointIoError {
            path: v_dir.clone(),
            source: e,
        })?;

        let state_path = v_dir.join("state.json");
        let json = state.to_json()?;
        std::fs::write(&state_path, &json).map_err(|e| PipelineError::CheckpointIoError {
            path: state_path.clone(),
            source: e,
        })?;

        // SQLite persistence
        if let Some(conn_lock) = &self.sqlite_conn {
            let conn = conn_lock.lock().map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
            conn.execute(
                "INSERT OR REPLACE INTO song_versions (song_id, version, active_node, state_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    state.song_id,
                    state.version,
                    active_node,
                    json,
                    chrono::Utc::now().to_rfc3339()
                ],
            ).map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
        }

        Ok(state_path)
    }

    /// Loads a specific version state snapshot.
    pub async fn load_checkpoint(&self, song_id: &str, version: u32) -> Result<MusicProductionState> {
        let v_dir = self.version_dir(song_id, version)?;
        let state_path = v_dir.join("state.json");
        let content = std::fs::read_to_string(&state_path).map_err(|e| PipelineError::CheckpointIoError {
            path: state_path,
            source: e,
        })?;
        let state = MusicProductionState::from_json(&content)?;
        Ok(state)
    }

    /// Saves an audio stem buffer directly into the version stems directory.
    pub async fn save_stem(
        &self,
        song_id: &str,
        version: u32,
        stem_name: &str,
        buffer: &AudioBuffer,
    ) -> Result<PathBuf> {
        let s_dir = self.stems_dir(song_id, version)?;
        std::fs::create_dir_all(&s_dir).map_err(|e| PipelineError::CheckpointIoError {
            path: s_dir.clone(),
            source: e,
        })?;

        let clean_name = Self::sanitize_path_component(stem_name)?;
        let stem_path = s_dir.join(format!("{}.wav", clean_name));
        buffer.save_to_wav_file(&stem_path)?;

        if let Some(conn_lock) = &self.sqlite_conn {
            let conn = conn_lock.lock().map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
            conn.execute(
                "INSERT OR REPLACE INTO stem_records (song_id, version, stem_name, stem_path)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    song_id,
                    version,
                    stem_name,
                    stem_path.to_string_lossy().to_string()
                ],
            ).map_err(|e| PipelineError::DatabaseError(e.to_string()))?;
        }

        Ok(stem_path)
    }
}
```

---

### 4.7 The 6 Sequential Fusion Nodes Architecture

#### Node 1: `ollama_producer_panel_node` (`crates/music_pipeline/src/nodes/node1_producer_panel.rs`)
- **Stage**: Stage 0.
- **Contract**: Multi-LLM Ollama Songwriter & Arranger Panel (`Lyricist`, `Arranger`, `Sound Engineer`, `Judge`).
- **Execution**:
  - Checks live hardware via `SystemMemory::detect_live()`.
  - Dispatches parallel queries to Ollama or transparently falls back to canonical genre blueprints (`SongBlueprint::default_one_love_shining_anthem()`) if the daemon is offline.
  - Ensures lyrics contain clean structural tags: `[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`, `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`.
  - Sets `state.style_prompt` and `state.lyrics`.

#### Node 2: `groove_and_yue_synthesis_node` (`crates/music_pipeline/src/nodes/node2_groove_yue.rs`)
- **Stage**: Stages 1 & 2.
- **Contract**: Foundation rhythm groove (`draft_groove.wav`) and full-song dual stems (`yue_vocals.wav`, `yue_instrumentals.wav`).
- **Execution**:
  - Checks `SystemMemory::detect_live().should_use_dsp(StageType::GrooveFoundation)`.
  - If pure-Rust DSP: calls `music_dsp::groove_synth::generate_groove(&state.genre, state.bpm, duration_secs)`.
  - Generates harmonic foundation for vocals and instrumentals.
  - Registers `draft_groove`, `yue_vocals`, and `yue_instrumentals` into `state.stem_paths`.

#### Node 3: `vocal_timbre_fusion_node` (`crates/music_pipeline/src/nodes/node3_vocal_timbre.rs`)
- **Stage**: Stage 3.
- **Contract**: Voice conversion into 1977 soulful raspy Jamaican tenor vocal timbre (or genre archetype).
- **Execution**:
  - Reads `yue_vocals` stem from disk.
  - Applies pure-Rust formant/rasp processing via `music_dsp::vocal_formant::apply_vocal_formant_and_rasp`.
  - If conditioned on `vocal_ref_wav`, adapts resonant frequencies.
  - Saves and registers `raspy_tenor_vocals` in `state.stem_paths`.

#### Node 4: `virtuoso_stem_inpainting_node` (`crates/music_pipeline/src/nodes/node4_stem_inpainting.rs`)
- **Stage**: Stage 4.
- **Contract**: 4-stem separation (`drums`, `bass`, `guitar`, `other`) + guitar solo & nyabinghi drum inpainting.
- **Execution**:
  - Separates stems using HTDemucs or pure-Rust spectral band splitting.
  - Synthesizes 16-bar electric guitar solo via `music_dsp::guitar_wah::synthesize_guitar_solo`.
  - Synthesizes polyrhythmic nyabinghi hand drums via `UniversalDrumSynth`.
  - Slices and splices with sample-accurate equal-power crossfades (`music_dsp::crossfader::equal_power_crossfade`).
  - Registers `drums`, `bass`, `guitar`, `other`, `inpainted_guitar`, and `inpainted_drums` in `state.stem_paths`.

#### Node 5: `dsp_and_judge_gate_node` (`crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs`)
- **Stage**: Quality Gate.
- **Contract**: Spectral waveform telemetry analysis and AI Judge evaluation.
- **Execution**:
  - Computes peak dBFS, RMS dBFS, dynamic range, tempo drift, and digital clipping across all stems via `music_dsp::telemetry_calc::calculate_telemetry`.
  - Runs `JudgeEvaluator` composite scoring.
  - Evaluates broadcast standards: zero clipping, peak $\le 0.0$ dBFS, judge score $\ge 7.5$.
  - Sets `state.quality_passed = true` or `false` and records `state.telemetry`.

#### Node 6: `analog_mastering_node` (`crates/music_pipeline/src/nodes/node6_analog_mastering.rs`)
- **Stage**: Stage 5.
- **Contract**: Final 48 kHz 24-bit PCM analog tape mastering.
- **Execution**:
  - Mixes all active stems according to `StemMixConfig`.
  - Applies Studer A800 tape saturation (`music_dsp::tape_mastering::AnalogTapeSaturator`).
  - Applies 55–85 Hz sub-bass warmth filter (`SubBassWarmthFilter`).
  - Applies brickwall lookahead limiter targeting $-0.3$ dBFS ceiling.
  - Persists and registers `final_master` in `state.stem_paths`.

---

### 4.8 Pipeline Runner Helper (`crates/music_pipeline/src/runner.rs`)

```rust
//! Pipeline assembly helper wiring the 6 sequential nodes into StateGraph.

use std::collections::HashMap;
use music_core::MusicProductionState;
use crate::error::Result;
use crate::graph::{CompiledGraph, StateGraph, START, END};
use crate::nodes::*;

pub fn build_production_pipeline() -> Result<CompiledGraph> {
    let mut graph = StateGraph::new();

    // Register all 6 sequential nodes
    graph.add_node("ollama_producer_panel_node", Node1ProducerPanel::default());
    graph.add_node("groove_and_yue_synthesis_node", Node2GrooveAndYue::default());
    graph.add_node("vocal_timbre_fusion_node", Node3VocalTimbre::default());
    graph.add_node("virtuoso_stem_inpainting_node", Node4StemInpainting::default());
    graph.add_node("dsp_and_judge_gate_node", Node5DspJudgeGate::default());
    graph.add_node("analog_mastering_node", Node6AnalogMastering::default());

    // Edges
    graph.add_edge(START, "ollama_producer_panel_node");
    graph.add_edge("ollama_producer_panel_node", "groove_and_yue_synthesis_node");
    graph.add_edge("groove_and_yue_synthesis_node", "vocal_timbre_fusion_node");
    graph.add_edge("vocal_timbre_fusion_node", "virtuoso_stem_inpainting_node");
    graph.add_edge("virtuoso_stem_inpainting_node", "dsp_and_judge_gate_node");

    // Dynamic conditional edge for Node 5
    let mut gate_branches = HashMap::new();
    gate_branches.insert("success".to_string(), "analog_mastering_node".to_string());
    gate_branches.insert("retry_stem".to_string(), "virtuoso_stem_inpainting_node".to_string());
    gate_branches.insert("retry_groove".to_string(), "groove_and_yue_synthesis_node".to_string());
    gate_branches.insert("exhausted".to_string(), "analog_mastering_node".to_string());

    graph.add_conditional_edges(
        "dsp_and_judge_gate_node",
        |state: &MusicProductionState| {
            if state.quality_passed {
                "success".to_string()
            } else if state.retry_count < 2 {
                if state.telemetry.tempo_drift_score > 0.5 {
                    "retry_groove".to_string()
                } else {
                    "retry_stem".to_string()
                }
            } else {
                "exhausted".to_string()
            }
        },
        gate_branches,
    );

    graph.add_edge("analog_mastering_node", END);

    graph.compile()
}
```

---

## 5. Verification Method

### 5.1 Compilation & Dependency Check
Execute from workspace root:
```powershell
cargo check -p music_pipeline
```

### 5.2 Unit Test Execution Plan for Milestone 3
Implementers must add unit tests to `crates/music_pipeline/tests/` covering:
1. **Graph Construction & Edge Traversal**:
   - `test_stategraph_linear_flow`: 3 test nodes executing in order; assert state mutation.
   - `test_conditional_edges_routing`: Branch key `"success"` routes forward; `"retry"` routes backward.
   - `test_max_iterations_infinite_loop_prevention`: Force retry without incrementing retry count; assert `PipelineError::MaxIterationsExceeded`.
   - `test_missing_entry_point_rejection`: Assert `compile()` returns `PipelineError::MissingEntryPoint`.
   - `test_targeted_node_execution`: Call `execute_from_node` starting at Node 4; assert Nodes 1–3 do not run.
2. **Checkpointer & Security Invariants**:
   - `test_checkpointer_save_and_load`: Save state, reload, assert all 21 fields identical.
   - `test_checkpointer_path_traversal_rejection`: Pass `song_id = "../../Windows"`; assert `PipelineError::SecurityError`.
   - `test_checkpointer_wav_stem_roundtrip`: Write WAV stem via checkpointer, read back with `AudioBuffer::load_from_wav_file`.
3. **End-to-End Pipeline Smoke Test**:
   ```powershell
   cargo test -p music_pipeline -- --nocapture
   ```
4. **Full Workspace Regression**:
   ```powershell
   cargo test --workspace -- --nocapture
   ```

### 5.3 Invalidation Conditions
The design is invalidated if:
- Any Python runtime or LangGraph dependency is introduced.
- Dynamic linking to `ModelFusion` is introduced.
- State mutation causes sample clipping or audio buffer duration mismatch.
- Path traversal escapes the `./storage/` root directory.
