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

    graph.add_node("ollama_producer_panel_node", node1)?;
    graph.add_node("groove_and_yue_synthesis_node", node2)?;
    graph.add_node("vocal_timbre_fusion_node", node3)?;
    graph.add_node("virtuoso_stem_inpainting_node", node4)?;
    graph.add_node("dsp_and_judge_gate_node", node5)?;
    graph.add_node("analog_mastering_node", node6)?;

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

    /// Song iteration: creates a new version branch with updated parameters and executed pipeline.
    pub async fn iterate_song(
        &self,
        song_id: &str,
        from_version: u32,
        prompt_update: Option<&str>,
    ) -> Result<MusicProductionState> {
        self.iterate_song_with_tweaks(song_id, from_version, prompt_update, None).await
    }

    /// Enhanced song iteration: accepts explicit ParameterTweaks or parses them from critique string,
    /// invokes IterationFusionPanel, applies diff blueprint & stem adjustments, and runs pipeline.
    pub async fn iterate_song_with_tweaks(
        &self,
        song_id: &str,
        from_version: u32,
        prompt_update: Option<&str>,
        explicit_tweaks: Option<music_fusion::iteration_panel::ParameterTweaks>,
    ) -> Result<MusicProductionState> {
        let prev_state = self.checkpointer.load_checkpoint(song_id, from_version).await?;
        let critique = prompt_update.unwrap_or("").trim();

        // 1. Prepare base ParameterTweaks
        let tweaks = explicit_tweaks.unwrap_or_default();

        // 2. Run IterationFusionPanel (or deterministic fallback)
        let mem = music_fusion::memory::SystemMemory::detect_live();
        let panel = music_fusion::iteration_panel::IterationFusionPanel::new_auto_configured(&mem).await;
        let iteration_result = panel.run_iteration_fusion(&prev_state, critique, &tweaks).await
            .unwrap_or_else(|_| panel.offline_fallback_iteration(&prev_state, critique, &tweaks));

        // 3. Modulate state parameters
        let mut next_state = prev_state.clone();
        next_state.advance_version();

        // 3a. BPM modulation: prioritize explicit tweak, extracted BPM, or genre-bounded delta
        let genre_preset = music_core::genre::GenreTaxonomy::find_or_create(&prev_state.genre);
        let next_bpm = if let Some(b) = tweaks.bpm {
            b
        } else if let Some(b) = music_fusion::iteration_panel::parse_bpm_from_critique(critique) {
            b
        } else {
            let candidate_bpm = iteration_result.blueprint.bpm.round() as u32;
            if candidate_bpm != prev_state.bpm && (genre_preset.bpm_range.0..=genre_preset.bpm_range.1).contains(&candidate_bpm) {
                candidate_bpm
            } else if !critique.is_empty() {
                let delta: i32 = if next_state.version.is_multiple_of(2) { 2 } else { -2 };
                let cand = (prev_state.bpm as i32 + delta)
                    .clamp(genre_preset.bpm_range.0 as i32, genre_preset.bpm_range.1 as i32) as u32;
                if cand != prev_state.bpm {
                    cand
                } else if prev_state.bpm + 2 <= genre_preset.bpm_range.1 {
                    prev_state.bpm + 2
                } else if prev_state.bpm >= genre_preset.bpm_range.0 + 2 {
                    prev_state.bpm - 2
                } else {
                    prev_state.bpm
                }
            } else {
                prev_state.bpm
            }
        };
        next_state.bpm = next_bpm;

        // 3b. Key signature modulation
        if let Some(ref k) = tweaks.key_signature {
            next_state.key_signature = k.clone();
        } else if let Some(k) = music_fusion::iteration_panel::parse_key_from_critique(critique) {
            next_state.key_signature = k;
        } else if !iteration_result.blueprint.key_signature.is_empty() && iteration_result.blueprint.key_signature != prev_state.key_signature {
            next_state.key_signature = iteration_result.blueprint.key_signature.clone();
        }

        // 3c. Style prompt & lyrics
        if let Some(p) = prompt_update {
            next_state.style_prompt = p.to_string();
        }
        if !iteration_result.blueprint.lyrics_text.is_empty() {
            next_state.lyrics = iteration_result.blueprint.lyrics_text.clone();
        }

        // 3d. Diff-architected blueprint and stem mix modulation
        let mut blueprint = prev_state.blueprint.clone().unwrap_or_else(|| {
            music_core::schema::SongBlueprint::default_one_love_shining_anthem()
        });
        blueprint.version = next_state.version;
        blueprint.parent_version = Some(from_version);
        blueprint.bpm = next_state.bpm as f64;
        blueprint.key_signature = next_state.key_signature.clone();
        blueprint.iteration_notes = iteration_result.iteration_summary.clone();

        // Modulate stem mix parameters based on tweaks or critique
        let crit_lower = critique.to_ascii_lowercase();
        if let Some(rasp) = tweaks.vocal_rasp {
            blueprint.stem_mix.vocal_rasp_intensity = rasp;
        } else if crit_lower.contains("rasp") || crit_lower.contains("vocal") {
            blueprint.stem_mix.vocal_rasp_intensity = (blueprint.stem_mix.vocal_rasp_intensity + 0.15).min(1.0);
        }

        if let Some(bass) = tweaks.sub_bass_weight {
            blueprint.stem_mix.sub_bass_boost_db = bass * 6.0;
            blueprint.stem_mix.dub_bass_db += 1.5;
        } else if crit_lower.contains("bass") || crit_lower.contains("sub") {
            blueprint.stem_mix.sub_bass_boost_db = (blueprint.stem_mix.sub_bass_boost_db + 1.8).min(6.0);
            blueprint.stem_mix.dub_bass_db += 1.2;
        }

        if let Some(wah) = tweaks.guitar_wah {
            blueprint.stem_mix.guitar_db += 1.0;
            for sec in &mut blueprint.sections {
                sec.wah_pedal_depth = wah;
                sec.guitar_solo_intensity = wah;
            }
        } else if crit_lower.contains("guitar") || crit_lower.contains("wah") || crit_lower.contains("solo") {
            blueprint.stem_mix.guitar_db += 1.5;
            for sec in &mut blueprint.sections {
                sec.wah_pedal_depth = (sec.wah_pedal_depth + 0.20).min(1.0);
                sec.guitar_solo_intensity = (sec.guitar_solo_intensity + 0.15).min(1.0);
            }
        }

        if let Some(drum) = tweaks.drum_fill_complexity {
            blueprint.stem_mix.drums_db += 0.8;
            for sec in &mut blueprint.sections {
                sec.drum_fill_complexity = drum;
            }
        } else if crit_lower.contains("drum") || crit_lower.contains("fill") || crit_lower.contains("nyabinghi") {
            blueprint.stem_mix.drums_db += 1.0;
            for sec in &mut blueprint.sections {
                sec.drum_fill_complexity = (sec.drum_fill_complexity + 0.20).min(1.0);
            }
        }

        next_state.blueprint = Some(blueprint);
        next_state.history.push(format!(
            "Iterating song from v{} to v{}: {} (BPM: {}, Key: {})",
            from_version, next_state.version, critique, next_state.bpm, next_state.key_signature
        ));

        self.compiled.execute_with_checkpointer(next_state, &self.checkpointer).await
    }
}
