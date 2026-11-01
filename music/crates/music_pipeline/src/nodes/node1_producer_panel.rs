//! Node 1: Ollama Producer Panel Node (`ollama_producer_panel_node`).
//!
//! Responsibilities:
//! 1. Probes live hardware memory via `SystemMemory::detect_live()`.
//! 2. Configures `OllamaProducerPanel` adhering to the Dynamic Available Memory Law.
//! 3. Synthesizes bracketed lyric tags (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`,
//!    `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`).
//! 4. Cleans `<think>` traces and generates YuE clean `genre.txt` and `lyrics.txt`.
//! 5. Populates `SongBlueprint` and updates `MusicProductionState` fields.
//! 6. Emits `genre.txt` and `lyrics.txt` to `./storage/songs/<song_id>/v<version>/`.
//! 7. Transitions state active node and audit history.

use std::fs;
use std::path::PathBuf;
use async_trait::async_trait;

use music_core::genre::GenreTaxonomy;
use music_core::schema::{PipelineModelStack, SongBlueprint, SongSection, StemMixConfig};
use music_core::state::MusicProductionState;
use music_fusion::judge::{parse_judge_json, EvaluatedSongBlueprint};
use music_fusion::memory::SystemMemory;
use music_fusion::panel::OllamaProducerPanel;

use crate::error::PipelineError;
use crate::graph::GraphNode;

/// Helper function to resolve the song storage directory for a specific version.
pub fn get_song_version_dir(song_id: &str, version: u32) -> PathBuf {
    PathBuf::from(format!("storage/songs/{}/v{}", song_id, version))
}

/// Node 1: Ollama Producer Panel & Song Blueprint Synthesizer.
#[derive(Debug, Clone, Default)]
pub struct OllamaProducerPanelNode {
    pub override_genre: Option<String>,
    pub storage_dir: Option<PathBuf>,
}

impl OllamaProducerPanelNode {
    pub fn new() -> Self {
        Self {
            override_genre: None,
            storage_dir: None,
        }
    }

    pub fn with_genre(genre: impl Into<String>) -> Self {
        Self {
            override_genre: Some(genre.into()),
            storage_dir: None,
        }
    }

    pub fn with_storage_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.storage_dir = Some(dir.into());
        self
    }

    pub fn resolve_version_dir(&self, song_id: &str, version: u32) -> PathBuf {
        let base = self.storage_dir.clone().unwrap_or_else(|| PathBuf::from("storage"));
        base.join("songs").join(song_id).join(format!("v{}", version))
    }

    /// Primary execution method for Node 1.
    pub async fn execute_node(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        state.record_node_transition("ollama_producer_panel_node");

        // 1. Live hardware detection adhering to Dynamic Available Memory Law
        let memory = SystemMemory::detect_live();
        log::info!(
            "Node 1: Probed system memory - RAM: {:.1}/{:.1} GB Free, VRAM: {:.1}/{:.1} GB Free",
            memory.free_ram_gb, memory.total_ram_gb, memory.gpu_vram_free_gb, memory.gpu_vram_total_gb
        );

        // 2. Genre and style resolution
        if let Some(ref override_g) = self.override_genre {
            state.genre = override_g.clone();
        }
        if state.genre.is_empty() {
            if !state.style_prompt.is_empty() {
                let resolved = GenreTaxonomy::find_or_create(&state.style_prompt);
                state.genre = resolved.name;
            } else {
                state.genre = "Roots Reggae".to_string();
            }
        }

        let genre_preset = GenreTaxonomy::find_or_create(&state.genre);

        // 3. Fallback sync if lyrics or prompt are completely empty
        if state.style_prompt.is_empty() {
            state.style_prompt = format!(
                "1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, spiritual and uplifting, {} BPM",
                state.bpm
            );
        }

        // 4. In pure-Rust mode, bypass Ollama network queries completely and use deterministic blueprint
        let (yue_genre_txt, yue_lyrics_txt, judge_evaluation) = if state.is_pure_rust() {
            let eval = parse_judge_json("", &genre_preset, &state.style_prompt);
            let g_txt = eval.to_yue_genre_txt();
            let l_txt = eval.to_yue_lyrics_txt();
            (g_txt, l_txt, eval)
        } else {
            let panel = OllamaProducerPanel::new_auto_configured(&memory).await;
            let panel_output = panel.run_panel(state).await.map_err(|e| {
                PipelineError::FusionError(format!("Producer panel execution failed: {}", e))
            })?;
            (panel_output.yue_genre_txt, panel_output.yue_lyrics_txt, panel_output.judge_evaluation)
        };

        // 5. Enforce strict bracketed lyric tags
        let structured_lyrics = Self::enforce_structural_bracket_tags(
            &yue_lyrics_txt,
            &state.lyrics,
            &genre_preset.name,
        );
        state.lyrics = structured_lyrics.clone();

        // 6. Synchronize SongBlueprint into state
        let mut blueprint = Self::convert_evaluated_blueprint_to_schema(
            &judge_evaluation,
            state,
            &yue_genre_txt,
            &structured_lyrics,
        );

        // If user explicitly configured BPM, key, or time signature, preserve them in blueprint and state
        if state.bpm > 0 {
            blueprint.bpm = state.bpm as f64;
        } else {
            state.bpm = blueprint.bpm.round() as u32;
        }

        if !state.key_signature.is_empty() {
            blueprint.key_signature = state.key_signature.clone();
        } else {
            state.key_signature = blueprint.key_signature.clone();
        }

        if !state.time_signature.is_empty() {
            blueprint.time_signature = state.time_signature.clone();
        } else {
            state.time_signature = blueprint.time_signature.clone();
        }

        state.blueprint = Some(blueprint.clone());
        if state.style_prompt.is_empty() {
            state.style_prompt = blueprint.style_of_music.clone();
        }

        // 7. Persist genre.txt and lyrics.txt into versioned song storage
        let version_dir = self.resolve_version_dir(&state.song_id, state.version);
        fs::create_dir_all(&version_dir).map_err(PipelineError::IoError)?;

        let genre_file = version_dir.join("genre.txt");
        let lyrics_file = version_dir.join("lyrics.txt");

        fs::write(&genre_file, &yue_genre_txt).map_err(PipelineError::IoError)?;
        fs::write(&lyrics_file, &structured_lyrics).map_err(PipelineError::IoError)?;

        log::info!(
            "Node 1: Populated blueprint '{}' (score: {:.2}, passed: {}). Emitted {:?} and {:?}",
            blueprint.title, state.judge_score, state.quality_passed, genre_file, lyrics_file
        );

        state.history.push(format!(
            "Node 1 (Producer Panel) completed: lyrics structured into {} sections, genre tags synthesized",
            blueprint.sections.len()
        ));

        Ok(())
    }

    /// Enforces structural bracket tags in lyrics: [Intro], [Verse 1], [Chorus], etc.
    /// Strips `<think>` tags, verifies section tags, and auto-synthesizes mandatory sections if missing.
    pub fn enforce_structural_bracket_tags(
        yue_lyrics: &str,
        existing_lyrics: &str,
        _genre_name: &str,
    ) -> String {
        // Source selection
        let mut raw = if !yue_lyrics.trim().is_empty() {
            yue_lyrics.trim().to_string()
        } else if !existing_lyrics.trim().is_empty() {
            existing_lyrics.trim().to_string()
        } else {
            String::new()
        };

        // 1. Strip reasoning blocks: <think>...</think>
        while let Some(start) = raw.find("<think>") {
            if let Some(end_offset) = raw[start..].find("</think>") {
                let end = start + end_offset;
                raw = format!("{}{}", &raw[..start], &raw[end + 8..]);
            } else {
                raw = raw[..start].to_string();
                break;
            }
        }

        raw = raw.trim().to_string();

        // 2. If raw lyrics have no bracket tags, wrap into canonical sections
        if !raw.contains('[') || !raw.contains(']') {
            let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
            if lines.is_empty() {
                return "[Intro]\n\n[Verse 1]\nWalking together in harmony and grace,\nA smile of mercy on every human face.\n\n[Chorus]\nOne love shining through the morning light,\nHealing every sorrow, making all things right.\n\n[Outro]\nPeace and blessing.".to_string();
            }

            let half = lines.len() / 2;
            let verse_lines = lines[..half].join("\n");
            let chorus_lines = lines[half..].join("\n");

            return format!(
                "[Intro]\n\n[Verse 1]\n{}\n\n[Chorus]\n{}\n\n[Outro]",
                verse_lines, chorus_lines
            );
        }

        // 3. Normalize case of standard bracket tags
        let canonical_replacements = [
            ("[intro]", "[Intro]"),
            ("[verse 1]", "[Verse 1]"),
            ("[verse 2]", "[Verse 2]"),
            ("[verse]", "[Verse 1]"),
            ("[chorus]", "[Chorus]"),
            ("[instrumental interlude]", "[Instrumental Interlude]"),
            ("[instrumental]", "[Instrumental Interlude]"),
            ("[bridge]", "[Bridge]"),
            ("[guitar solo]", "[Guitar Solo]"),
            ("[solo]", "[Guitar Solo]"),
            ("[outro]", "[Outro]"),
        ];

        let mut normalized = raw;
        for (lower, canonical) in &canonical_replacements {
            normalized = normalized.replace(lower, canonical);
        }

        // 4. Ensure at least [Intro] and [Outro] exist
        if !normalized.contains("[Intro]") {
            normalized = format!("[Intro]\n\n{}", normalized);
        }
        if !normalized.contains("[Outro]") {
            normalized = format!("{}\n\n[Outro]", normalized.trim());
        }

        normalized
    }

    /// Converts EvaluatedSongBlueprint into full domain SongBlueprint.
    fn convert_evaluated_blueprint_to_schema(
        evaluated: &EvaluatedSongBlueprint,
        state: &MusicProductionState,
        genre_txt: &str,
        _lyrics_txt: &str,
    ) -> SongBlueprint {
        let now = chrono::Utc::now().to_rfc3339();
        let sections = evaluated.arrangement_map.iter().map(|sa| {
            SongSection {
                section_id: sa.section_id.clone(),
                yue_clean_tag: sa.yue_clean_tag.clone(),
                display_label: sa.display_label.clone(),
                production_cue: sa.production_cue.clone(),
                lyrics: sa.lyrics.clone(),
                bars: sa.bars,
                chords: sa.chords.clone(),
                guitar_solo_intensity: sa.lead_instrument_intensity,
                wah_pedal_depth: sa.lead_instrument_intensity * 0.75,
                drum_fill_complexity: sa.drum_fill_complexity,
                nyabinghi_polyrhythm: sa.drum_fill_complexity * 0.65,
                gospel_choir_level: if sa.display_label.contains("Chorus") { 0.95 } else { 0.20 },
                brass_swell_level: if sa.display_label.contains("Chorus") { 0.85 } else { 0.15 },
                dub_sub_bass_weight: sa.bass_weight,
            }
        }).collect();

        SongBlueprint {
            song_id: state.song_id.clone(),
            slug: state.song_id.replace('-', "_"),
            version: state.version,
            parent_version: if state.version > 1 { Some(state.version - 1) } else { None },
            title: evaluated.title.clone(),
            subtitle: format!("Universal AI Music Production in {}", state.genre),
            style_of_music: genre_txt.trim().trim_end_matches('.').to_string(),
            bpm: if state.bpm > 0 { state.bpm as f64 } else { evaluated.bpm },
            key_signature: if !state.key_signature.is_empty() { state.key_signature.clone() } else { evaluated.key_signature.clone() },
            time_signature: if !state.time_signature.is_empty() { state.time_signature.clone() } else { evaluated.time_signature.clone() },
            sections,
            stem_mix: StemMixConfig::default(),
            pipeline_models: PipelineModelStack::default(),
            iteration_notes: format!("Synthesized by Node 1 for genre {}", state.genre),
            created_at: now,
        }
    }
}

#[async_trait]
impl GraphNode for OllamaProducerPanelNode {
    async fn execute(&self, state: &mut MusicProductionState) -> Result<(), PipelineError> {
        self.execute_node(state).await
    }

    fn name(&self) -> &str {
        "ollama_producer_panel_node"
    }

    fn description(&self) -> &str {
        "Ollama Multi-LLM Producer Panel: Refines prompt, bracketed lyric tags, and SongBlueprint"
    }
}
