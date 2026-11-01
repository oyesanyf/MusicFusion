//! Multi-Model Parallel Producer Panel coordinating Lyricist, Arranger, and Sound Engineer.

use crate::error::Result;
use crate::judge::{
    parse_judge_json, EvaluatedSongBlueprint, JudgeAnalysis, JudgeEvaluator,
};
use crate::memory::SystemMemory;
use crate::ollama_client::{ChatMessage, OllamaClient};
use crate::prompt_templates::{
    build_arranger_prompt, build_lyricist_prompt, build_sound_engineer_prompt,
};
use music_core::genre::{GenrePreset, GenreTaxonomy};
use music_core::state::MusicProductionState;
use serde::{Deserialize, Serialize};

/// Answer from an individual model specialist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAnswer {
    pub model_name: String,
    pub role: String,
    pub answer: String,
}

/// Raw outputs captured from the three parallel specialists.
#[derive(Debug, Clone)]
pub struct SpecialistOutputs {
    pub genre: GenrePreset,
    pub lyricist_text: String,
    pub arranger_text: String,
    pub engineer_text: String,
}

/// Consolidated output of the Ollama Producer Panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaPanelOutput {
    pub lyricist_output: String,
    pub arranger_output: String,
    pub engineer_output: String,
    pub judge_evaluation: EvaluatedSongBlueprint,
    pub judge_analysis: Option<JudgeAnalysis>,
    pub yue_genre_txt: String,
    pub yue_lyrics_txt: String,
}

/// Standalone Multi-Model Ollama Producer Panel.
pub struct OllamaProducerPanel {
    pub client: OllamaClient,
    pub memory: Option<SystemMemory>,
    pub lyricist_model: String,
    pub arranger_model: String,
    pub engineer_model: String,
    pub judge_model: String,
}

impl OllamaProducerPanel {
    /// Construct a new producer panel with automatic hardware-aware model configuration.
    pub async fn new_auto_configured(memory: &SystemMemory) -> Self {
        let client = OllamaClient::from_env();
        let installed = client.list_installed_tags().await.unwrap_or_default();
        let recommended = memory.recommend_ollama_model(&installed);

        let mut panel = Self {
            client,
            memory: Some(memory.clone()),
            lyricist_model: recommended.clone(),
            arranger_model: recommended.clone(),
            engineer_model: recommended.clone(),
            judge_model: recommended,
        };

        // If diverse specialized models are available, assign personas
        if (memory.free_ram_gb >= 8.0 || memory.gpu_vram_free_gb >= 4.0)
            && installed.iter().any(|m| m.contains("flint"))
        {
            if let Some(flint) = installed.iter().find(|m| m.contains("flint")) {
                panel.arranger_model = flint.clone();
            }
        }

        if (memory.free_ram_gb >= 8.0 || memory.gpu_vram_free_gb >= 6.0)
            && installed.iter().any(|m| m.contains("glm4"))
        {
            if let Some(glm) = installed.iter().find(|m| m.contains("glm4")) {
                panel.engineer_model = glm.clone();
            }
        }

        panel
    }

    /// Construct panel with explicit model assignments.
    pub fn new(
        client: OllamaClient,
        lyricist: impl Into<String>,
        arranger: impl Into<String>,
        engineer: impl Into<String>,
        judge: impl Into<String>,
    ) -> Self {
        Self {
            client,
            memory: None,
            lyricist_model: lyricist.into(),
            arranger_model: arranger.into(),
            engineer_model: engineer.into(),
            judge_model: judge.into(),
        }
    }

    /// Run the 3 specialist models concurrently on Ollama `/api/chat`.
    pub async fn run_specialists(
        &self,
        user_prompt: &str,
        genre_hint: Option<&str>,
    ) -> Result<SpecialistOutputs> {
        let genre = GenreTaxonomy::find_or_create(genre_hint.unwrap_or(user_prompt));

        log::info!(
            "Booting Ollama Producer Panel for '{}' with models: L={}, A={}, E={}",
            genre.name, self.lyricist_model, self.arranger_model, self.engineer_model
        );

        let lyric_p = build_lyricist_prompt(&genre, user_prompt);
        let arr_p = build_arranger_prompt(&genre, user_prompt);
        let eng_p = build_sound_engineer_prompt(&genre, user_prompt);

        let lyricist_task = self.client.chat(
            &self.lyricist_model,
            vec![ChatMessage::system(lyric_p.system_prompt), ChatMessage::user(lyric_p.user_prompt)],
            None,
        );

        let arranger_task = self.client.chat(
            &self.arranger_model,
            vec![ChatMessage::system(arr_p.system_prompt), ChatMessage::user(arr_p.user_prompt)],
            None,
        );

        let engineer_task = self.client.chat(
            &self.engineer_model,
            vec![ChatMessage::system(eng_p.system_prompt), ChatMessage::user(eng_p.user_prompt)],
            None,
        );

        // Execute in parallel
        let (lyric_res, arr_res, eng_res) = tokio::join!(lyricist_task, arranger_task, engineer_task);

        let lyricist_text = lyric_res
            .map(|r| r.message.content)
            .unwrap_or_else(|e| format!("Lyricist fallback: {}", e));

        let arranger_text = arr_res
            .map(|r| r.message.content)
            .unwrap_or_else(|e| format!("Arranger fallback: {}", e));

        let engineer_text = eng_res
            .map(|r| r.message.content)
            .unwrap_or_else(|e| format!("Engineer fallback: {}", e));

        Ok(SpecialistOutputs {
            genre,
            lyricist_text,
            arranger_text,
            engineer_text,
        })
    }

    /// Complete panel execution: inspects state, executes specialists or fallback, evaluates with Judge,
    /// and populates the MusicProductionState with YuE tags, bracketed lyrics, and arrangement map.
    pub async fn run_panel(&self, state: &mut MusicProductionState) -> Result<OllamaPanelOutput> {
        let prompt = if !state.style_prompt.is_empty() {
            &state.style_prompt
        } else if !state.lyrics.is_empty() {
            &state.lyrics
        } else {
            "spiritual compassion, unity, and uplifting human harmony"
        };

        let genre_query = if !state.genre.is_empty() {
            &state.genre
        } else {
            prompt
        };

        let genre = GenreTaxonomy::find_or_create(genre_query);
        let is_online = self.client.probe_health().await;

        let (lyricist_out, arranger_out, engineer_out, blueprint) = if is_online {
            let specialists = self.run_specialists(prompt, Some(&genre.name)).await?;
            let judge = JudgeEvaluator::new(self.client.clone(), &self.judge_model);
            let bp = judge.evaluate(prompt, &specialists).await?;
            judge.apply_to_state(&bp, state);
            (
                specialists.lyricist_text,
                specialists.arranger_text,
                specialists.engineer_text,
                bp,
            )
        } else {
            log::info!("Ollama daemon offline or unprobed; utilizing genre-adaptive fail-safe blueprint.");
            let bp = parse_judge_json("", &genre, prompt);
            let judge = JudgeEvaluator::new(self.client.clone(), &self.judge_model);
            judge.apply_to_state(&bp, state);
            (
                format!("Offline preset lyrics for {}", genre.name),
                format!("Offline harmonic arrangement: {} BPM, {}", genre.default_bpm, genre.default_key),
                format!("Offline sound engineering: {:?}, {}", genre.rhythm_groove, genre.mastering_profile),
                bp,
            )
        };

        let yue_genre_txt = blueprint.to_yue_genre_txt();
        let yue_lyrics_txt = blueprint.to_yue_lyrics_txt();

        Ok(OllamaPanelOutput {
            lyricist_output: lyricist_out,
            arranger_output: arranger_out,
            engineer_output: engineer_out,
            judge_evaluation: blueprint,
            judge_analysis: Some(JudgeAnalysis::default()),
            yue_genre_txt,
            yue_lyrics_txt,
        })
    }
}
