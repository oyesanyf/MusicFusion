//! Pipeline runner and high-level execution coordinator for song generation, stem regeneration, and mastering.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use music_core::state::MusicProductionState;

use crate::checkpointer::CheckpointManager;
use crate::error::Result;
use crate::graph::{CompiledGraph, StateGraph, END};
use crate::node::GraphObserver;
use crate::nodes::{
    judge_gate_condition, AnalogMasteringNode, DspAndJudgeGateNode, GrooveAndYueSynthesisNode,
    OllamaProducerPanelNode, VirtuosoStemInpaintingNode, VocalTimbreFusionNode,
    ROUTE_EXHAUSTED, ROUTE_MASTERING, ROUTE_RETRY_GROOVE, ROUTE_RETRY_STEM, ROUTE_SUCCESS,
};

/// Configuration parameters for pipeline execution.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub storage_dir: PathBuf,
    pub max_iterations: usize,
    pub enable_sqlite: bool,
    pub db_name: String,
    pub pure_rust_mode: bool,
    pub ollama_host: String,
    pub judge_model: String,
    pub max_duration_secs: Option<f32>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            storage_dir: PathBuf::from("storage"),
            max_iterations: 50,
            enable_sqlite: true,
            db_name: "music_storage.db".to_string(),
            pure_rust_mode: false,
            ollama_host: "http://127.0.0.1:11434".to_string(),
            judge_model: "qwen2.5:7b".to_string(),
            max_duration_secs: None,
        }
    }
}

impl PipelineConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_storage_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.storage_dir = dir.into();
        self
    }

    pub fn with_pure_rust_mode(mut self, enabled: bool) -> Self {
        self.pure_rust_mode = enabled;
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_max_duration(mut self, secs: f32) -> Self {
        self.max_duration_secs = Some(secs);
        self
    }
}

/// Builds the compiled 6-stage production StateGraph and associated CheckpointManager.
pub fn build_production_pipeline(config: &PipelineConfig) -> Result<(CompiledGraph, CheckpointManager)> {
    let checkpointer = if config.enable_sqlite {
        CheckpointManager::with_sqlite(&config.storage_dir, &config.db_name)?
    } else {
        CheckpointManager::new(&config.storage_dir)
    };

    let node1 = OllamaProducerPanelNode::new().with_storage_dir(&config.storage_dir);

    let mut node2 = GrooveAndYueSynthesisNode::new().with_storage_dir(&config.storage_dir);
    if let Some(dur) = config.max_duration_secs {
        node2 = node2.with_max_duration(dur);
    }
    let node3 = VocalTimbreFusionNode::new().with_storage_dir(&config.storage_dir);
    let node4 = VirtuosoStemInpaintingNode::new().with_storage_dir(&config.storage_dir);
    let node5 = DspAndJudgeGateNode::new(&config.ollama_host, &config.judge_model);
    let node6 = AnalogMasteringNode::new().with_storage_dir(&config.storage_dir);

    let mut graph = StateGraph::new();

    graph.add_node("ollama_producer_panel_node", node1);
    graph.add_node("groove_and_yue_synthesis_node", node2);
    graph.add_node("vocal_timbre_fusion_node", node3);
    graph.add_node("virtuoso_stem_inpainting_node", node4);
    graph.add_node("dsp_and_judge_gate_node", node5);
    graph.add_node("analog_mastering_node", node6);

    graph.set_entry_point("ollama_producer_panel_node");

    // Sequential transitions
    graph.add_edge("ollama_producer_panel_node", "groove_and_yue_synthesis_node");
    graph.add_edge("groove_and_yue_synthesis_node", "vocal_timbre_fusion_node");
    graph.add_edge("vocal_timbre_fusion_node", "virtuoso_stem_inpainting_node");
    graph.add_edge("virtuoso_stem_inpainting_node", "dsp_and_judge_gate_node");
    graph.add_edge("analog_mastering_node", END);

    // Conditional routing edges from Node 5 (DSP and Judge Gate)
    let mut branches = HashMap::new();
    branches.insert(ROUTE_MASTERING.to_string(), "analog_mastering_node".to_string());
    branches.insert(ROUTE_SUCCESS.to_string(), "analog_mastering_node".to_string());
    branches.insert(ROUTE_EXHAUSTED.to_string(), "analog_mastering_node".to_string());
    branches.insert(ROUTE_RETRY_GROOVE.to_string(), "groove_and_yue_synthesis_node".to_string());
    branches.insert(ROUTE_RETRY_STEM.to_string(), "virtuoso_stem_inpainting_node".to_string());
    branches.insert("retry_solo".to_string(), "virtuoso_stem_inpainting_node".to_string());

    graph.add_conditional_edges("dsp_and_judge_gate_node", judge_gate_condition, branches);

    let mut compiled = graph.compile()?;
    compiled.set_max_iterations(config.max_iterations);

    Ok((compiled, checkpointer))
}

/// PipelineRunner coordinates end-to-end execution, stem regeneration, version iteration, and mastering.
pub struct PipelineRunner {
    compiled: CompiledGraph,
    checkpointer: CheckpointManager,
}

impl PipelineRunner {
    pub fn new(config: PipelineConfig) -> Result<Self> {
        let (compiled, checkpointer) = build_production_pipeline(&config)?;
        Ok(Self {
            compiled,
            checkpointer,
        })
    }

    pub fn from_graph_and_checkpointer(compiled: CompiledGraph, checkpointer: CheckpointManager) -> Self {
        Self {
            compiled,
            checkpointer,
        }
    }

    pub fn compiled_graph(&self) -> &CompiledGraph {
        &self.compiled
    }

    pub fn checkpointer(&self) -> &CheckpointManager {
        &self.checkpointer
    }

    /// Executes the full 6-node music production pipeline from scratch.
    pub async fn execute_song(&self, initial_state: MusicProductionState) -> Result<MusicProductionState> {
        self.compiled.execute_with_checkpointer(initial_state, &self.checkpointer).await
    }

    /// Executes full pipeline with real-time observer events (e.g. for SSE streaming in Tokio REST).
    pub async fn execute_song_with_observer(
        &self,
        initial_state: MusicProductionState,
        observer: &Arc<dyn GraphObserver>,
    ) -> Result<MusicProductionState> {
        self.compiled.execute_with_observer(initial_state, Some(&self.checkpointer), Some(observer)).await
    }

    /// Targeted stem regeneration (Node 4 to Node 6): useful for `/api/songs/regenerate-stem`.
    pub async fn regenerate_stem(
        &self,
        song_id: &str,
        version: u32,
        _stem_name: &str,
    ) -> Result<MusicProductionState> {
        let mut state = self.checkpointer.load_checkpoint(song_id, version).await?;
        state.advance_version();
        state.history.push(format!("Regenerating stems for v{}", state.version));

        self.compiled.execute_from_node(
            "virtuoso_stem_inpainting_node",
            None,
            state,
            Some(&self.checkpointer),
        ).await
    }

    /// Dedicated mastering execution (Node 6 only): useful for `/api/songs/master`.
    pub async fn master_track(
        &self,
        song_id: &str,
        version: u32,
    ) -> Result<MusicProductionState> {
        let state = self.checkpointer.load_checkpoint(song_id, version).await?;
        self.compiled.execute_from_node(
            "analog_mastering_node",
            Some("analog_mastering_node"),
            state,
            Some(&self.checkpointer),
        ).await
    }

    /// Song iteration: creates a new version branch with optional updated style prompt or lyrics.
    pub async fn iterate_song(
        &self,
        song_id: &str,
        from_version: u32,
        prompt_update: Option<&str>,
    ) -> Result<MusicProductionState> {
        let mut state = self.checkpointer.load_checkpoint(song_id, from_version).await?;
        state.advance_version();
        if let Some(p) = prompt_update {
            state.style_prompt = p.to_string();
        }
        state.history.push(format!("Iterating song from v{} to v{}", from_version, state.version));

        self.compiled.execute_with_checkpointer(state, &self.checkpointer).await
    }
}
