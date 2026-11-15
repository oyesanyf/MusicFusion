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
use music_fusion::decision_model::NimbleDecisionEngine;
use music_fusion::judge::{parse_judge_json, EvaluatedSongBlueprint};
use music_fusion::lyricist_studio::{
    compose_offline_structured_lyrics, format_sections, parse_lyrics_sections,
    structure_user_lyrics_verbatim, write_lyrics_from_prompt,
};
use music_fusion::memory::SystemMemory;
use music_fusion::ollama_client::OllamaClient;
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

        // 3. Fallback style prompt derived from state genre when empty (never force Reggae on other genres)
        if state.style_prompt.is_empty() {
            if state.genre.to_ascii_lowercase().contains("reggae") {
                state.style_prompt = format!(
                    "1977 roots reggae anthem, soulful raspy Jamaican male tenor vocals, spiritual and uplifting, {} BPM",
                    state.bpm
                );
            } else {
                state.style_prompt = format!(
                    "Studio {} production, expressive dynamic performance, {} BPM in {}",
                    state.genre, state.bpm, state.key_signature
                );
            }
        }

        // 4. Local Nimble Decision Gates 1 (Synthesis Model), 2 (Lyrics Routing), and 5 (License Compliance)
        let nimble = NimbleDecisionEngine::default();
        let effective_dec_mode = if state.is_pure_rust()
            && state.decision_mode == music_core::state::DecisionMode::Auto
        {
            music_core::state::DecisionMode::FastDeterministic
        } else {
            state.decision_mode
        };
        let req_model: Option<String> = state
            .selected_model
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
            .map(str::to_string);
        let pref_dec: Option<String> = state
            .decision_model
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let gate1 = nimble
            .gate1_select_generation_model(
                &state.style_prompt,
                state.lyrics_mode,
                !state.lyrics.trim().is_empty(),
                req_model.as_deref(),
                state.commercial_only,
                state.lora_adapter_path.as_deref(),
                effective_dec_mode,
                pref_dec.as_deref(),
            )
            .await;
        state.record_decision_trace(gate1.trace.clone());
        if req_model.is_none() || !gate1.blocked_candidates.is_empty() {
            state.selected_model = Some(gate1.selected_choice.clone());
        }

        let gate2 = nimble
            .gate2_resolve_lyrics_and_structure(
                &state.style_prompt,
                &state.lyrics,
                &state.lyrics_topic,
                state.lyrics_mode,
                effective_dec_mode,
                pref_dec.as_deref(),
            )
            .await;
        state.record_decision_trace(gate2.trace.clone());

        let active_synth_model = state
            .selected_model
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "ACE-Step/acestep-v15-turbo".to_string());
        let gate5 = nimble
            .gate5_license_and_export_gate(
                &active_synth_model,
                pref_dec.as_deref(),
                state.commercial_only,
                effective_dec_mode,
            )
            .await;
        state.record_decision_trace(gate5.trace.clone());
        if gate5.selected_choice == "BlockedLicenseConflict" {
            return Err(PipelineError::CompilationError(format!(
                "License compliance blocked export under commercial_only=true: {}",
                gate5.rationale
            )));
        }

        // 5. Dual-mode songwriting & blueprint resolution
        // Snapshot verbatim user input BEFORE running panel/judge so auto/offline blueprints never overwrite user lyrics.
        let original_user_lyrics = state.lyrics.clone();
        let (yue_genre_txt, yue_lyrics_txt, judge_evaluation) = if state.is_pure_rust() {
            let eval = parse_judge_json("", &genre_preset, &state.style_prompt);
            let g_txt = eval.to_yue_genre_txt();
            let l_txt = match gate2.selected_choice.as_str() {
                "UseUserLyricsVerbatim" => structure_user_lyrics_verbatim(&original_user_lyrics).formatted_lyrics,
                "InstrumentalOnly" => "[Instrumental]\n(Pure instrumental arrangement — no vocals)\n".to_string(),
                _ => {
                    let topic = if !state.lyrics_topic.trim().is_empty() {
                        state.lyrics_topic.as_str()
                    } else {
                        state.genre.as_str()
                    };
                    compose_offline_structured_lyrics(topic, &state.style_prompt).formatted_lyrics
                }
            };
            (g_txt, l_txt, eval)
        } else {
            let panel = OllamaProducerPanel::new_auto_configured(&memory).await;
            let eval = match panel.run_panel(state).await {
                Ok(out) => (out.yue_genre_txt, out.yue_lyrics_txt, out.judge_evaluation),
                Err(_) => {
                    let fallback_eval = parse_judge_json("", &genre_preset, &state.style_prompt);
                    let g_txt = fallback_eval.to_yue_genre_txt();
                    (g_txt, String::new(), fallback_eval)
                }
            };
            let resolved_lyrics = match gate2.selected_choice.as_str() {
                "UseUserLyricsVerbatim" => structure_user_lyrics_verbatim(&original_user_lyrics).formatted_lyrics,
                "InstrumentalOnly" => "[Instrumental]\n(Pure instrumental arrangement — no vocals)\n".to_string(),
                _ => {
                    let ollama = OllamaClient::from_env();
                    let topic = if !state.lyrics_topic.trim().is_empty() {
                        state.lyrics_topic.as_str()
                    } else {
                        state.style_prompt.as_str()
                    };
                    write_lyrics_from_prompt(&ollama, &memory, topic, &state.style_prompt, None)
                        .await
                        .formatted_lyrics
                }
            };
            (eval.0, resolved_lyrics, eval.2)
        };

        // 6. Enforce clean structural tags while preserving user-provided words 100% verbatim
        let structured_lyrics = match gate2.selected_choice.as_str() {
            "UseUserLyricsVerbatim" => structure_user_lyrics_verbatim(&original_user_lyrics).formatted_lyrics,
            "InstrumentalOnly" => yue_lyrics_txt.clone(),
            _ => Self::enforce_structural_bracket_tags(
                &yue_lyrics_txt,
                "",
                &genre_preset.name,
            ),
        };
        state.lyrics = structured_lyrics.clone();

        // 6. Synchronize SongBlueprint into state
        let mut blueprint = Self::convert_evaluated_blueprint_to_schema(
            &judge_evaluation,
            state,
            &yue_genre_txt,
            &structured_lyrics,
        );

        // Preserve pre-configured stem mix and iteration notes if state already had a blueprint
        if let Some(ref existing) = state.blueprint {
            blueprint.stem_mix = existing.stem_mix.clone();
            if !existing.iteration_notes.is_empty() {
                blueprint.iteration_notes = existing.iteration_notes.clone();
            }
        }

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

    /// Enforces structural bracket tags cleanly while preserving user-provided lyrics 100% verbatim.
    /// Strips `<think>` reasoning traces, preserves explicit `[Section]` tags, and falls back to
    /// genre-matched songwriting only when input text is completely empty.
    pub fn enforce_structural_bracket_tags(
        yue_lyrics: &str,
        existing_lyrics: &str,
        genre_name: &str,
    ) -> String {
        fn strip_think_blocks(input: &str) -> String {
            let mut s = input.to_string();
            while let Some(start) = s.find("<think>") {
                if let Some(end_offset) = s[start..].find("</think>") {
                    let end = start + end_offset;
                    s = format!("{}{}", &s[..start], &s[end + 8..]);
                } else {
                    s = s[..start].to_string();
                    break;
                }
            }
            s.trim().to_string()
        }

        let cleaned_existing = strip_think_blocks(existing_lyrics);
        if !cleaned_existing.is_empty() {
            return structure_user_lyrics_verbatim(&cleaned_existing).formatted_lyrics;
        }

        let cleaned_yue = strip_think_blocks(yue_lyrics);
        if !cleaned_yue.is_empty() {
            let sections = parse_lyrics_sections(&cleaned_yue);
            if !sections.is_empty() {
                return format_sections(&sections);
            }
        }

        compose_offline_structured_lyrics(genre_name, genre_name).formatted_lyrics
    }

    /// Converts EvaluatedSongBlueprint into full domain SongBlueprint.
    fn convert_evaluated_blueprint_to_schema(
        evaluated: &EvaluatedSongBlueprint,
        state: &MusicProductionState,
        genre_txt: &str,
        lyrics_txt: &str,
    ) -> SongBlueprint {
        let now = chrono::Utc::now().to_rfc3339();
        let parsed_sections = parse_lyrics_sections(lyrics_txt);
        let sections: Vec<SongSection> = if !parsed_sections.is_empty() && !state.lyrics.trim().is_empty() {
            parsed_sections
                .iter()
                .enumerate()
                .map(|(idx, ps)| {
                    let fallback = evaluated.arrangement_map.get(idx);
                    SongSection {
                        section_id: format!("sec_{}", idx + 1),
                        yue_clean_tag: ps.tag.clone(),
                        display_label: ps.display_label.clone(),
                        production_cue: fallback
                            .map(|f| f.production_cue.clone())
                            .unwrap_or_else(|| format!("Dynamic {} section", state.genre)),
                        lyrics: ps.lines.clone(),
                        bars: fallback.map(|f| f.bars).unwrap_or(8),
                        chords: fallback
                            .map(|f| f.chords.clone())
                            .unwrap_or_else(|| vec![state.key_signature.clone()]),
                        guitar_solo_intensity: fallback.map(|f| f.lead_instrument_intensity).unwrap_or(0.35),
                        wah_pedal_depth: fallback.map(|f| f.lead_instrument_intensity * 0.75).unwrap_or(0.25),
                        drum_fill_complexity: fallback.map(|f| f.drum_fill_complexity).unwrap_or(0.45),
                        nyabinghi_polyrhythm: 0.25,
                        gospel_choir_level: if ps.tag.contains("chorus") { 0.85 } else { 0.20 },
                        brass_swell_level: if ps.tag.contains("chorus") { 0.65 } else { 0.15 },
                        dub_sub_bass_weight: fallback.map(|f| f.bass_weight).unwrap_or(0.75),
                    }
                })
                .collect()
        } else {
            evaluated.arrangement_map.iter().map(|sa| {
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
            }).collect()
        };

        let mut pipeline_models = PipelineModelStack::default();
        if let Some(m) = state
            .selected_model
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto"))
        {
            pipeline_models.stage2_yue_semantic = m.to_string();
            pipeline_models.stage1_groove_foundation = m.to_string();
        }
        if let Some(d) = state
            .decision_model
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            pipeline_models.decision_engine = d.to_string();
        }

        SongBlueprint {
            song_id: state.song_id.clone(),
            slug: state.song_id.replace('-', "_"),
            version: state.version,
            parent_version: if state.version > 1 { Some(state.version - 1) } else { None },
            title: evaluated.title.clone(),
            subtitle: format!("Open-Weight Studio Production in {}", state.genre),
            style_of_music: genre_txt.trim().trim_end_matches('.').to_string(),
            bpm: if state.bpm > 0 { state.bpm as f64 } else { evaluated.bpm },
            key_signature: if !state.key_signature.is_empty() { state.key_signature.clone() } else { evaluated.key_signature.clone() },
            time_signature: if !state.time_signature.is_empty() { state.time_signature.clone() } else { evaluated.time_signature.clone() },
            sections,
            stem_mix: StemMixConfig::default(),
            pipeline_models,
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
