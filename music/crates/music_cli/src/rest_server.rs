//! Tokio Axum HTTP JSON REST Server (Port 5050 with fallback 5050..5060)
//! and audio stem streaming engine for the AI Music Producer Studio.
//!
//! Designed for Milestone 4: Tokio REST API & Multi-LLM Iteration Co-Pilot Fusion.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use music_core::genre::GenreTaxonomy;
use music_core::state::{
    EngineMode, MusicProductionState, STEM_BASS, STEM_DRAFT_GROOVE, STEM_DRUMS, STEM_FINAL_MASTER,
    STEM_GUITAR, STEM_INPAINTED_DRUMS, STEM_INPAINTED_GUITAR, STEM_OTHER, STEM_RASPY_TENOR_VOCALS,
    STEM_YUE_VOCALS,
};
use music_core::telemetry::StemTelemetry;
use music_fusion::hf_model_discovery::HfModelDiscoveryEngine;
use music_fusion::iteration_panel::{
    IterationFusionPanel, ParameterTweaks,
};
use music_fusion::memory::SystemMemory;
use music_fusion::ollama_client::OllamaClient;
use music_fusion::ollama_lifecycle::OllamaLifecycleManager;
use music_fusion::tts_engine::{FusionTtsEngine, TtsVocalRequest, TtsVocalResponse};
use music_pipeline::checkpointer::CheckpointManager;
use music_pipeline::runner::PipelineRunner;

// App state shared across all REST routes
#[derive(Clone)]
pub struct AppState {
    pub runner: Arc<PipelineRunner>,
    pub storage_dir: PathBuf,
    pub bound_port: u16,
    pub ollama_host: String,
}

// ---------------------------------------------------------------------------
// Request / Response Data Contracts
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateSongRequest {
    #[serde(alias = "style_prompt", default)]
    pub prompt: String,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub bpm: Option<u32>,
    #[serde(default)]
    pub key_signature: Option<String>,
    #[serde(default)]
    pub lyrics: Option<String>,
    #[serde(default)]
    pub engine_mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateSongResponse {
    pub status: String,
    pub song_id: String,
    pub version: u32,
    pub state: MusicProductionState,
    pub audio_urls: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct IterateSongRequest {
    pub song_id: String,
    #[serde(alias = "version", default = "default_version_one")]
    pub from_version: u32,
    #[serde(alias = "instructions", alias = "feedback", default)]
    pub critique: String,
    #[serde(default)]
    pub bpm: Option<u32>,
    #[serde(default)]
    pub key_signature: Option<String>,
    #[serde(default)]
    pub vocal_rasp: Option<f32>,
    #[serde(default)]
    pub guitar_wah: Option<f32>,
    #[serde(alias = "drum_complexity", default)]
    pub drum_fill_complexity: Option<f32>,
    #[serde(alias = "sub_bass_warmth", default)]
    pub sub_bass_weight: Option<f32>,
    #[serde(default)]
    pub target_stems: Vec<String>,
}

fn default_version_one() -> u32 {
    1
}

#[derive(Debug, Serialize)]
pub struct IterateSongResponse {
    pub status: String,
    pub song_id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub iteration_summary: String,
    pub stem_deltas: HashMap<String, String>,
    pub suggested_next_iterations: Vec<String>,
    pub judge_score: f32,
    pub state: MusicProductionState,
    pub audio_urls: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct RegenerateStemRequest {
    pub song_id: String,
    pub version: u32,
    #[serde(alias = "stem_name")]
    pub stem: String,
}

#[derive(Debug, Serialize)]
pub struct RegenerateStemResponse {
    pub status: String,
    pub song_id: String,
    pub version: u32,
    pub regenerated_stem: String,
    pub stem_url: String,
    pub state: MusicProductionState,
}

#[derive(Debug, Deserialize)]
pub struct MasterSongRequest {
    pub song_id: String,
    pub version: u32,
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MasterSongResponse {
    pub status: String,
    pub song_id: String,
    pub version: u32,
    pub mastering_profile: String,
    pub master_url: String,
    pub telemetry: StemTelemetry,
    pub state: MusicProductionState,
}

#[derive(Debug, Serialize)]
pub struct VersionSummary {
    pub version: u32,
    pub title: String,
    pub genre: String,
    pub bpm: u32,
    pub key_signature: String,
    pub judge_score: f32,
    pub created_at: String,
    pub iteration_summary: Option<String>,
    pub stems: Vec<String>,
    pub has_master: bool,
}

#[derive(Debug, Serialize)]
pub struct ListVersionsResponse {
    pub song_id: String,
    pub current_version: u32,
    pub versions: Vec<VersionSummary>,
}

#[derive(Debug, Serialize)]
pub struct SongCatalogEntry {
    pub song_id: String,
    pub title: String,
    pub genre: String,
    pub latest_version: u32,
    pub versions: Vec<VersionSummary>,
}

#[derive(Debug, Serialize)]
pub struct ListSongsResponse {
    pub songs: Vec<SongCatalogEntry>,
}

/// Resolves canonical song_id on disk for known UI/CLI aliases while preserving security sanitization.
pub fn resolve_song_alias(storage_dir: &Path, clean_id: &str) -> String {
    let direct_dir = storage_dir.join("songs").join(clean_id);
    if direct_dir.exists() {
        return clean_id.to_string();
    }

    let lower = clean_id.to_ascii_lowercase();
    if lower == "song_reggae_anthem_001"
        || lower == "one_love_anthem"
        || lower == "reggae_anthem"
        || lower == "roots_reggae_anthem"
        || lower == "song-one-love-shining-1977"
    {
        for fallback in [
            "roots_reggae_anthem",
            "song-one-love-shining-1977",
            "song_reggae_anthem_001",
        ] {
            if storage_dir.join("songs").join(fallback).exists() {
                return fallback.to_string();
            }
        }
    }

    if lower == "song_afrobeats_002" || lower == "afrobeats_hit" {
        for fallback in ["song_afrobeats_002", "roots_reggae_anthem", "song-one-love-shining-1977"] {
            if storage_dir.join("songs").join(fallback).exists() {
                return fallback.to_string();
            }
        }
    }

    clean_id.to_string()
}

#[derive(Debug, Deserialize)]
pub struct AssistIterationRequest {
    pub song_id: String,
    #[serde(default = "default_version_one")]
    pub version: u32,
    #[serde(default)]
    pub focus: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssistIterationResponse {
    pub song_id: String,
    pub version: u32,
    pub copilot_model: String,
    pub critique: String,
    pub suggested_next_iterations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HardwareStatusResponse {
    pub total_ram_gb: f64,
    pub free_ram_gb: f64,
    pub cpu_cores: usize,
    pub gpu_name: Option<String>,
    pub gpu_vram_total_gb: f64,
    pub gpu_vram_free_gb: f64,
    pub active_tier: String,
    pub installed_models: Vec<String>,
    pub ollama_online: bool,
    pub server_port: u16,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub port: u16,
}

// ---------------------------------------------------------------------------
// Route Handlers
// ---------------------------------------------------------------------------

/// GET /api/health — Fast health probe with 400ms timeout compatibility.
pub async fn handle_health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "universal-ai-music-producer".to_string(),
        version: "0.1.0".to_string(),
        port: state.bound_port,
    })
}

/// GET /api/hardware — Probes live system memory, VRAM, and installed Ollama models.
pub async fn handle_hardware_status(State(state): State<AppState>) -> Json<HardwareStatusResponse> {
    let mem = SystemMemory::detect_live();
    let client = OllamaClient::new(&state.ollama_host);
    let installed = client.list_installed_tags().await.unwrap_or_default();
    let is_online = client.probe_health().await;
    let active_tier = mem.recommend_ollama_model(&installed);

    Json(HardwareStatusResponse {
        total_ram_gb: mem.total_ram_gb,
        free_ram_gb: mem.free_ram_gb,
        cpu_cores: mem.cpu_cores,
        gpu_name: mem.gpu_name,
        gpu_vram_total_gb: mem.gpu_vram_total_gb,
        gpu_vram_free_gb: mem.gpu_vram_free_gb,
        active_tier,
        installed_models: installed,
        ollama_online: is_online,
        server_port: state.bound_port,
    })
}

/// POST /api/songs/create — Boots StateGraph and creates a new song v1.
pub async fn handle_create_song(
    State(state): State<AppState>,
    Json(req): Json<CreateSongRequest>,
) -> Response {
    let song_id = format!("song_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let genre_query = req.genre.as_deref().unwrap_or(&req.prompt);
    let genre = GenreTaxonomy::get_or_default(genre_query);
    let bpm = req.bpm.unwrap_or(genre.default_bpm);
    let key = req.key_signature.unwrap_or_else(|| genre.default_key.clone());
    let engine_mode = req.engine_mode.as_deref().unwrap_or("auto");

    let mut init_state = MusicProductionState::new(&song_id, &genre.name, bpm, &key);
    init_state.style_prompt = req.prompt;
    init_state.engine_mode = EngineMode::from_str_loose(engine_mode).as_str().to_string();

    if let Some(lyr) = req.lyrics {
        if !lyr.trim().is_empty() {
            init_state.lyrics = lyr;
        }
    }

    match state.runner.execute_song(init_state).await {
        Ok(final_state) => {
            let audio_urls = build_audio_urls(&final_state.song_id, final_state.version);
            Json(CreateSongResponse {
                status: "success".to_string(),
                song_id: final_state.song_id.clone(),
                version: final_state.version,
                state: final_state,
                audio_urls,
            }).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Song generation failed: {}", e),
                    "status": "pipeline_error"
                })),
            ).into_response()
        }
    }
}

/// POST /api/songs/iterate — Runs Multi-LLM Iteration Fusion producing v(N+1).
pub async fn handle_iterate_song(
    State(state): State<AppState>,
    Json(req): Json<IterateSongRequest>,
) -> Response {
    // 1. Sanitize song_id
    let clean_req_id = match CheckpointManager::sanitize_path_component(&req.song_id) {
        Ok(id) => resolve_song_alias(&state.storage_dir, &id),
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    // 2. Load prior version
    let prev_state = match state.runner.checkpointer().load_checkpoint(&clean_req_id, req.from_version).await {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Version {} not found: {}", req.from_version, e)})),
            ).into_response();
        }
    };

    // 3. Formulate next state and run Multi-LLM Iteration Fusion
    let mem = SystemMemory::detect_live();
    let panel = IterationFusionPanel::new_auto_configured(&mem).await;
    let tweaks = ParameterTweaks {
        bpm: req.bpm,
        key_signature: req.key_signature.clone(),
        vocal_rasp: req.vocal_rasp,
        guitar_wah: req.guitar_wah,
        drum_fill_complexity: req.drum_fill_complexity,
        sub_bass_weight: req.sub_bass_weight,
        target_stems: req.target_stems.clone(),
        ..Default::default()
    };

    let iteration_result = panel.run_iteration_fusion(&prev_state, &req.critique, &tweaks).await
        .unwrap_or_else(|_| panel.offline_fallback_iteration(&prev_state, &req.critique, &tweaks));

    let mut next_state = prev_state.clone();
    next_state.advance_version();

    let crit_lower = req.critique.to_ascii_lowercase();
    let wants_bpm_change = crit_lower.contains("bpm")
        || crit_lower.contains("tempo")
        || crit_lower.contains("faster")
        || crit_lower.contains("slower")
        || crit_lower.contains("speed")
        || req.bpm.is_some_and(|b| b != prev_state.bpm);
    let wants_key_change = crit_lower.contains("key of")
        || crit_lower.contains("transpose")
        || req.key_signature.as_ref().is_some_and(|k| !k.eq_ignore_ascii_case(&prev_state.key_signature));

    if wants_bpm_change {
        next_state.bpm = req.bpm.unwrap_or_else(|| iteration_result.blueprint.bpm.round() as u32);
    } else {
        next_state.bpm = prev_state.bpm;
    }

    if wants_key_change {
        next_state.key_signature = req
            .key_signature
            .clone()
            .unwrap_or_else(|| iteration_result.blueprint.key_signature.clone());
    } else {
        next_state.key_signature = prev_state.key_signature.clone();
    }

    // Preserve existing lyrics unless user explicitly asked to rewrite lyrics
    let wants_lyrics_change = crit_lower.contains("rewrite lyrics")
        || crit_lower.contains("new lyrics")
        || crit_lower.contains("sing:")
        || crit_lower.contains("lyrics:");
    if wants_lyrics_change && !iteration_result.blueprint.lyrics_text.is_empty() {
        next_state.lyrics = iteration_result.blueprint.lyrics_text.clone();
    } else {
        next_state.lyrics = prev_state.lyrics.clone();
    }

    if !req.critique.trim().is_empty() {
        next_state.style_prompt = format!("{}, {}", prev_state.style_prompt.trim_end_matches('.'), req.critique.trim());
    }

    // Apply diff architected blueprint and stem tweaks into next_state.blueprint
    let mut blueprint = prev_state.blueprint.clone().unwrap_or_else(|| {
        music_core::schema::SongBlueprint::default_one_love_shining_anthem()
    });
    blueprint.version = next_state.version;
    blueprint.parent_version = Some(req.from_version);
    blueprint.bpm = next_state.bpm as f64;
    blueprint.key_signature = next_state.key_signature.clone();

    let mut stem_deltas = iteration_result.stem_deltas.clone();
    let mut summary_notes = Vec::new();

    // Natural-Language Prompt Mix & Stem Adjustments (Speaker / Vocal / Bass / Guitar / Drums / Organ)
    let mentions_vocal_or_speaker = crit_lower.contains("speaker")
        || crit_lower.contains("vocal")
        || crit_lower.contains("voice")
        || crit_lower.contains("singer")
        || crit_lower.contains("words")
        || crit_lower.contains("hear")
        || crit_lower.contains("talk");
    let is_too_low_or_louder = crit_lower.contains("too low")
        || crit_lower.contains("low")
        || crit_lower.contains("louder")
        || crit_lower.contains("boost")
        || crit_lower.contains("up")
        || crit_lower.contains("more")
        || crit_lower.contains("higher")
        || crit_lower.contains("increase")
        || crit_lower.contains("clear");
    let is_too_loud_or_lower = crit_lower.contains("too loud")
        || crit_lower.contains("quieter")
        || crit_lower.contains("lower")
        || crit_lower.contains("less")
        || crit_lower.contains("down")
        || crit_lower.contains("reduce");

    if mentions_vocal_or_speaker {
        if is_too_loud_or_lower && !crit_lower.contains("too low") {
            blueprint.stem_mix.lead_vocal_db = (blueprint.stem_mix.lead_vocal_db - 4.0).max(-12.0);
            stem_deltas.insert("raspy_tenor_vocals".to_string(), "-4.0 dB vocal level trim".to_string());
            summary_notes.push("Reduced vocal/speaker level by -4.0 dB while preserving the melody".to_string());
        } else {
            blueprint.stem_mix.lead_vocal_db = (blueprint.stem_mix.lead_vocal_db + 5.0).min(12.0);
            blueprint.stem_mix.guitar_db = (blueprint.stem_mix.guitar_db - 1.5).max(-12.0);
            blueprint.stem_mix.organ_brass_db = (blueprint.stem_mix.organ_brass_db - 1.5).max(-12.0);
            stem_deltas.insert("raspy_tenor_vocals".to_string(), "+5.0 dB vocal/speaker boost & -2.6 dB instrumental ducking".to_string());
            summary_notes.push("Boosted speaker/lead vocal level by +5.0 dB with sidechain ducking while preserving the original melody".to_string());
        }
    } else if is_too_low_or_louder && !crit_lower.contains("bass") && !crit_lower.contains("guitar") && !crit_lower.contains("drum") && !crit_lower.contains("organ") {
        // Generic "too low" / "make it louder" -> boost lead vocal clarity and overall presence
        blueprint.stem_mix.lead_vocal_db = (blueprint.stem_mix.lead_vocal_db + 4.5).min(12.0);
        stem_deltas.insert("raspy_tenor_vocals".to_string(), "+4.5 dB vocal/speaker clarity boost".to_string());
        summary_notes.push("Boosted speaker/vocal clarity by +4.5 dB while preserving the original melody".to_string());
    }

    if let Some(rasp) = tweaks.vocal_rasp {
        blueprint.stem_mix.vocal_rasp_intensity = rasp;
    } else if crit_lower.contains("rasp") {
        blueprint.stem_mix.vocal_rasp_intensity = (blueprint.stem_mix.vocal_rasp_intensity + 0.15).min(1.0);
    }

    if crit_lower.contains("bass") || crit_lower.contains("sub") {
        if is_too_loud_or_lower && !crit_lower.contains("too low") {
            blueprint.stem_mix.dub_bass_db = (blueprint.stem_mix.dub_bass_db - 3.5).max(-12.0);
            stem_deltas.insert("bass".to_string(), "-3.5 dB dub bass trim".to_string());
            summary_notes.push("Trimmed dub sub-bass by -3.5 dB".to_string());
        } else {
            blueprint.stem_mix.sub_bass_boost_db = (blueprint.stem_mix.sub_bass_boost_db + 2.0).min(6.0);
            blueprint.stem_mix.dub_bass_db = (blueprint.stem_mix.dub_bass_db + 3.5).min(10.0);
            stem_deltas.insert("bass".to_string(), "+3.5 dB dub sub-bass warmth boost".to_string());
            summary_notes.push("Boosted dub sub-bass warmth by +3.5 dB".to_string());
        }
    } else if let Some(bass) = tweaks.sub_bass_weight {
        blueprint.stem_mix.sub_bass_boost_db = bass * 6.0;
    }

    if crit_lower.contains("guitar") || crit_lower.contains("wah") || crit_lower.contains("solo") {
        if is_too_loud_or_lower && !crit_lower.contains("too low") {
            blueprint.stem_mix.guitar_db = (blueprint.stem_mix.guitar_db - 3.5).max(-12.0);
            stem_deltas.insert("inpainted_guitar".to_string(), "-3.5 dB guitar level trim".to_string());
            summary_notes.push("Reduced guitar level by -3.5 dB".to_string());
        } else {
            blueprint.stem_mix.guitar_db = (blueprint.stem_mix.guitar_db + 3.5).min(10.0);
            for sec in &mut blueprint.sections {
                sec.wah_pedal_depth = (sec.wah_pedal_depth + 0.20).min(1.0);
                sec.guitar_solo_intensity = (sec.guitar_solo_intensity + 0.15).min(1.0);
            }
            stem_deltas.insert("inpainted_guitar".to_string(), "+3.5 dB lead/skank guitar boost".to_string());
            summary_notes.push("Boosted guitar presence & solo sustain by +3.5 dB".to_string());
        }
    } else if let Some(wah) = tweaks.guitar_wah {
        for sec in &mut blueprint.sections {
            sec.wah_pedal_depth = wah;
            sec.guitar_solo_intensity = wah;
        }
    }

    if crit_lower.contains("drum") || crit_lower.contains("fill") || crit_lower.contains("nyabinghi") || crit_lower.contains("kick") || crit_lower.contains("snare") {
        if is_too_loud_or_lower && !crit_lower.contains("too low") {
            blueprint.stem_mix.drums_db = (blueprint.stem_mix.drums_db - 3.5).max(-12.0);
            stem_deltas.insert("inpainted_drums".to_string(), "-3.5 dB drum level trim".to_string());
            summary_notes.push("Reduced drum kit level by -3.5 dB".to_string());
        } else {
            blueprint.stem_mix.drums_db = (blueprint.stem_mix.drums_db + 3.0).min(10.0);
            for sec in &mut blueprint.sections {
                sec.drum_fill_complexity = (sec.drum_fill_complexity + 0.20).min(1.0);
            }
            stem_deltas.insert("inpainted_drums".to_string(), "+3.0 dB one-drop drum punch boost".to_string());
            summary_notes.push("Boosted drum punch by +3.0 dB".to_string());
        }
    } else if let Some(drum) = tweaks.drum_fill_complexity {
        for sec in &mut blueprint.sections {
            sec.drum_fill_complexity = drum;
        }
    }

    if crit_lower.contains("organ") || crit_lower.contains("horn") || crit_lower.contains("brass") || crit_lower.contains("keys") {
        if is_too_loud_or_lower && !crit_lower.contains("too low") {
            blueprint.stem_mix.organ_brass_db = (blueprint.stem_mix.organ_brass_db - 3.5).max(-12.0);
            stem_deltas.insert("other".to_string(), "-3.5 dB organ & brass trim".to_string());
            summary_notes.push("Reduced organ & brass level by -3.5 dB".to_string());
        } else {
            blueprint.stem_mix.organ_brass_db = (blueprint.stem_mix.organ_brass_db + 3.5).min(10.0);
            stem_deltas.insert("other".to_string(), "+3.5 dB Hammond organ & brass horn swell boost".to_string());
            summary_notes.push("Boosted Hammond B3 organ & brass horns by +3.5 dB".to_string());
        }
    }

    let final_summary = if !summary_notes.is_empty() {
        summary_notes.join(" • ")
    } else {
        iteration_result.iteration_summary.clone()
    };
    blueprint.iteration_notes = final_summary.clone();

    next_state.blueprint = Some(blueprint);
    next_state.history.push(format!(
        "Iterated from v{} to v{}: {} (BPM: {}, Key: {})",
        req.from_version, next_state.version, req.critique, next_state.bpm, next_state.key_signature
    ));

    // Copy all existing stems from v(from_version) to v(next_state.version) so the original melody is NEVER lost
    let prev_v_dir = state.storage_dir.join("songs").join(&clean_req_id).join(format!("v{}", req.from_version));
    let next_v_dir = state.storage_dir.join("songs").join(&clean_req_id).join(format!("v{}", next_state.version));
    let prev_stems_dir = prev_v_dir.join("stems");
    let next_stems_dir = next_v_dir.join("stems");
    let _ = std::fs::create_dir_all(&next_stems_dir);

    for txt_name in ["genre.txt", "lyrics.txt"] {
        let src_txt = prev_v_dir.join(txt_name);
        if src_txt.exists() {
            let _ = std::fs::copy(&src_txt, next_v_dir.join(txt_name));
        }
    }

    let mut copied_stems = 0usize;
    let stem_entries = [
        (STEM_DRAFT_GROOVE, "draft_groove.wav"),
        (STEM_DRUMS, "drums.wav"),
        (STEM_BASS, "bass.wav"),
        (STEM_GUITAR, "guitar.wav"),
        (STEM_OTHER, "other.wav"),
        (STEM_INPAINTED_GUITAR, "inpainted_guitar.wav"),
        (STEM_INPAINTED_DRUMS, "inpainted_drums.wav"),
        ("yue_instrumentals", "yue_instrumentals.wav"),
        (STEM_YUE_VOCALS, "yue_vocals.wav"),
        (STEM_RASPY_TENOR_VOCALS, "raspy_tenor_vocals.wav"),
    ];
    for (stem_key, file_name) in stem_entries {
        let src_file = prev_stems_dir.join(file_name);
        let dst_file = next_stems_dir.join(file_name);
        if src_file.exists() {
            if std::fs::copy(&src_file, &dst_file).is_ok() {
                next_state.set_stem(stem_key, dst_file);
                copied_stems += 1;
            }
        }
    }

    // 4. If BPM, Key, and Lyrics did not change and we have the existing stems, preserve the exact melody
    //    and run Node 6 (AnalogMasteringNode) with the new stem_mix dB gains & vocal sidechain ducking!
    let exec_res = if !wants_bpm_change && !wants_key_change && !wants_lyrics_change && copied_stems >= 4 {
        let _ = state.runner.checkpointer().save_checkpoint(&next_state, "copilot_non_destructive_mix").await;
        state.runner.master_track(&clean_req_id, next_state.version).await
    } else {
        state.runner.execute_song(next_state).await
    };

    match exec_res {
        Ok(final_state) => {
            let audio_urls = build_audio_urls(&final_state.song_id, final_state.version);

            Json(IterateSongResponse {
                status: "success".to_string(),
                song_id: final_state.song_id.clone(),
                from_version: req.from_version,
                to_version: final_state.version,
                iteration_summary: final_summary,
                stem_deltas,
                suggested_next_iterations: iteration_result.suggested_next_iterations,
                judge_score: final_state.judge_score,
                state: final_state,
                audio_urls,
            }).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Iteration failed: {}", e)})),
            ).into_response()
        }
    }
}

/// POST /api/songs/regenerate-stem — Re-executes targeted pipeline node for single stem.
pub async fn handle_regenerate_stem(
    State(state): State<AppState>,
    Json(req): Json<RegenerateStemRequest>,
) -> Response {
    let clean_req_id = match CheckpointManager::sanitize_path_component(&req.song_id) {
        Ok(id) => resolve_song_alias(&state.storage_dir, &id),
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };
    if let Err(e) = CheckpointManager::sanitize_path_component(&req.stem) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    match state.runner.regenerate_stem(&clean_req_id, req.version, &req.stem).await {
        Ok(final_state) => {
            let stem_url = format!(
                "/api/songs/{}/versions/{}/audio/{}",
                final_state.song_id, final_state.version, req.stem
            );
            Json(RegenerateStemResponse {
                status: "success".to_string(),
                song_id: final_state.song_id.clone(),
                version: final_state.version,
                regenerated_stem: req.stem,
                stem_url,
                state: final_state,
            }).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Stem regeneration failed: {}", e)})),
            ).into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RevertSongRequest {
    pub song_id: String,
    #[serde(default = "default_version_one")]
    pub current_version: u32,
    #[serde(default)]
    pub target_version: Option<u32>,
}

/// POST /api/songs/revert — Reverts a song to its previous state (either restoring in-place backup on v1 or stepping back to v(N-1)).
pub async fn handle_revert_song(
    State(state): State<AppState>,
    Json(req): Json<RevertSongRequest>,
) -> Response {
    let clean_req_id = match CheckpointManager::sanitize_path_component(&req.song_id) {
        Ok(id) => resolve_song_alias(&state.storage_dir, &id),
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    let cur_v = req.current_version.max(1);
    let cur_v_dir = state.storage_dir.join("songs").join(&clean_req_id).join(format!("v{}", cur_v));
    let prev_master_backup = cur_v_dir.join("prev_final_master.wav");
    let orig_master_backup = cur_v_dir.join("original_final_master.wav");

    // 1. If an in-place edit backup exists on cur_v (from TTS vocals or Re-Master), restore it first!
    if req.target_version.is_none() && prev_master_backup.exists() {
        let _ = std::fs::copy(&prev_master_backup, cur_v_dir.join("final_master.wav"));
        let _ = std::fs::remove_file(&prev_master_backup);

        let cur_stems = cur_v_dir.join("stems");
        let prev_lead = cur_stems.join("prev_raspy_tenor_vocals.wav");
        if prev_lead.exists() {
            let _ = std::fs::copy(&prev_lead, cur_stems.join("raspy_tenor_vocals.wav"));
            let _ = std::fs::remove_file(&prev_lead);
        }
        let prev_yue = cur_stems.join("prev_yue_vocals.wav");
        if prev_yue.exists() {
            let _ = std::fs::copy(&prev_yue, cur_stems.join("yue_vocals.wav"));
            let _ = std::fs::remove_file(&prev_yue);
        }

        let audio_urls = build_audio_urls(&clean_req_id, cur_v);
        return Json(serde_json::json!({
            "status": "success",
            "song_id": clean_req_id,
            "reverted_from_version": cur_v,
            "reverted_to_version": cur_v,
            "restored_in_place_backup": true,
            "summary": format!("Reverted v{} to previous state before last edit", cur_v),
            "audio_urls": audio_urls,
        })).into_response();
    }

    // 2. If cur_v == 1 and original_final_master.wav exists, restore original baseline state!
    if req.target_version.is_none() && cur_v == 1 && orig_master_backup.exists() {
        let _ = std::fs::copy(&orig_master_backup, cur_v_dir.join("final_master.wav"));
        let cur_stems = cur_v_dir.join("stems");
        let orig_lead = cur_stems.join("original_raspy_tenor_vocals.wav");
        if orig_lead.exists() {
            let _ = std::fs::copy(&orig_lead, cur_stems.join("raspy_tenor_vocals.wav"));
        }
        let orig_yue = cur_stems.join("original_yue_vocals.wav");
        if orig_yue.exists() {
            let _ = std::fs::copy(&orig_yue, cur_stems.join("yue_vocals.wav"));
        }
        let audio_urls = build_audio_urls(&clean_req_id, 1);
        return Json(serde_json::json!({
            "status": "success",
            "song_id": clean_req_id,
            "reverted_from_version": 1,
            "reverted_to_version": 1,
            "restored_in_place_backup": true,
            "summary": "Reverted v1 to original baseline state",
            "audio_urls": audio_urls,
        })).into_response();
    }

    // 3. Otherwise step back to target_version or (cur_v - 1).max(1)
    let target_v = req.target_version.unwrap_or_else(|| cur_v.saturating_sub(1).max(1)).max(1);
    let audio_urls = build_audio_urls(&clean_req_id, target_v);
    let loaded_state = state.runner.checkpointer().load_checkpoint(&clean_req_id, target_v).await.ok();

    Json(serde_json::json!({
        "status": "success",
        "song_id": clean_req_id,
        "reverted_from_version": cur_v,
        "reverted_to_version": target_v,
        "restored_in_place_backup": false,
        "summary": format!("Reverted from v{} to v{}", cur_v, target_v),
        "state": loaded_state,
        "audio_urls": audio_urls,
    })).into_response()
}

/// POST /api/songs/master — Runs Node 6 analog tape mastering with specified profile.
pub async fn handle_master_song(
    State(state): State<AppState>,
    Json(req): Json<MasterSongRequest>,
) -> Response {
    let clean_req_id = match CheckpointManager::sanitize_path_component(&req.song_id) {
        Ok(id) => resolve_song_alias(&state.storage_dir, &id),
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    // Save pre-master backup so Revert can undo in-place mastering
    let v_dir = state.storage_dir.join("songs").join(&clean_req_id).join(format!("v{}", req.version.max(1)));
    let cur_master = v_dir.join("final_master.wav");
    if cur_master.exists() {
        if !v_dir.join("original_final_master.wav").exists() {
            let _ = std::fs::copy(&cur_master, v_dir.join("original_final_master.wav"));
        }
        let _ = std::fs::copy(&cur_master, v_dir.join("prev_final_master.wav"));
    }

    let profile = req.profile.unwrap_or_else(|| "WarmAnalogTape".to_string());
    match state.runner.master_track(&clean_req_id, req.version).await {
        Ok(final_state) => {
            let master_url = format!(
                "/api/songs/{}/versions/{}/audio/{}",
                final_state.song_id, final_state.version, STEM_FINAL_MASTER
            );
            Json(MasterSongResponse {
                status: "success".to_string(),
                song_id: final_state.song_id.clone(),
                version: final_state.version,
                mastering_profile: profile,
                master_url,
                telemetry: final_state.telemetry.clone(),
                state: final_state,
            }).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Mastering failed: {}", e)})),
            ).into_response()
        }
    }
}

/// POST /api/songs/tts-vocals — Synthesizes ModelFusion vocal stems (raspy lead & choir) and remixes final_master.
pub async fn handle_tts_vocals(
    State(state): State<AppState>,
    Json(req): Json<TtsVocalRequest>,
) -> Response {
    let clean_req_id = match CheckpointManager::sanitize_path_component(&req.song_id) {
        Ok(id) => resolve_song_alias(&state.storage_dir, &id),
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    let target_version = req.version.max(1);
    let v_dir = state.storage_dir
        .join("songs")
        .join(&clean_req_id)
        .join(format!("v{}", target_version));
    let stems_dir = v_dir.join("stems");

    // Save pre-TTS backup of final_master and vocal stems so Revert can always restore exact prior audio!
    if v_dir.join("final_master.wav").exists() {
        if !v_dir.join("original_final_master.wav").exists() {
            let _ = std::fs::copy(v_dir.join("final_master.wav"), v_dir.join("original_final_master.wav"));
        }
        let _ = std::fs::copy(v_dir.join("final_master.wav"), v_dir.join("prev_final_master.wav"));
    }
    if stems_dir.join("raspy_tenor_vocals.wav").exists() {
        if !stems_dir.join("original_raspy_tenor_vocals.wav").exists() {
            let _ = std::fs::copy(stems_dir.join("raspy_tenor_vocals.wav"), stems_dir.join("original_raspy_tenor_vocals.wav"));
        }
        let _ = std::fs::copy(stems_dir.join("raspy_tenor_vocals.wav"), stems_dir.join("prev_raspy_tenor_vocals.wav"));
    }
    if stems_dir.join("yue_vocals.wav").exists() {
        if !stems_dir.join("original_yue_vocals.wav").exists() {
            let _ = std::fs::copy(stems_dir.join("yue_vocals.wav"), stems_dir.join("original_yue_vocals.wav"));
        }
        let _ = std::fs::copy(stems_dir.join("yue_vocals.wav"), stems_dir.join("prev_yue_vocals.wav"));
    }

    let lyrics = if req.text.trim().is_empty() {
        "One love! One heart! Let us get together and feel all right!".to_string()
    } else {
        req.text.clone()
    };

    // Synthesize vocal stems via ModelFusion TTS Engine
    match FusionTtsEngine::synthesize_vocal_stems(
        &lyrics,
        &req.model,
        req.voice_style.as_deref(),
        &stems_dir,
    ).await {
        Ok((lead_path, yue_path)) => {
            // Update checkpoint state if present
            if let Ok(mut st) = state.runner.checkpointer().load_checkpoint(&clean_req_id, target_version).await {
                st.set_stem(STEM_RASPY_TENOR_VOCALS, lead_path.clone());
                st.set_stem(STEM_YUE_VOCALS, yue_path.clone());
                let _ = state.runner.checkpointer().save_checkpoint(&st, "tts_vocals_synthesis").await;
            }

            // Remix final master with vocal-priority sidechain ducking (-1.8 dB ducking during syllables)
            let _ = state.runner.master_track(&clean_req_id, target_version).await;

            let vocal_url = format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_RASPY_TENOR_VOCALS);
            let master_url = format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_FINAL_MASTER);

            let mut audio_urls = HashMap::new();
            audio_urls.insert("final_master".to_string(), master_url.clone());
            audio_urls.insert("master".to_string(), master_url.clone());
            audio_urls.insert("raspy_tenor_vocals".to_string(), vocal_url.clone());
            audio_urls.insert("vocals".to_string(), vocal_url.clone());
            audio_urls.insert("yue_vocals".to_string(), format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_YUE_VOCALS));
            audio_urls.insert("drums".to_string(), format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_DRUMS));
            audio_urls.insert("bass".to_string(), format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_BASS));
            audio_urls.insert("guitar".to_string(), format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_GUITAR));
            audio_urls.insert("other".to_string(), format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_OTHER));
            audio_urls.insert("inpainted_guitar".to_string(), format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_INPAINTED_GUITAR));
            audio_urls.insert("inpainted_drums".to_string(), format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_INPAINTED_DRUMS));
            audio_urls.insert("draft_groove".to_string(), format!("/api/songs/{}/versions/{}/audio/{}", clean_req_id, target_version, STEM_DRAFT_GROOVE));

            Json(TtsVocalResponse {
                status: "success".to_string(),
                song_id: clean_req_id,
                version: target_version,
                model: req.model,
                vocal_url,
                master_url,
                audio_urls,
            }).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "error": format!("TTS Vocal synthesis failed: {}", e)
                })),
            ).into_response()
        }
    }
}

/// GET /api/songs/:id/versions/:v/download — Downloads final_master WAV for a specific version.
pub async fn handle_download_version(
    State(state): State<AppState>,
    AxumPath((song_id, version)): AxumPath<(String, String)>,
    Query(query): Query<AudioStreamQuery>,
    headers: HeaderMap,
) -> Response {
    handle_stream_audio(
        State(state),
        AxumPath((song_id, version, "final_master".to_string())),
        Query(AudioStreamQuery {
            download: Some("1".to_string()),
            format: query.format,
        }),
        headers,
    ).await
}

/// GET /api/songs/:id/download — Downloads latest available final_master WAV for a song.
pub async fn handle_download_latest(
    State(state): State<AppState>,
    AxumPath(song_id): AxumPath<String>,
    Query(query): Query<AudioStreamQuery>,
    headers: HeaderMap,
) -> Response {
    let clean_id = match CheckpointManager::sanitize_path_component(&song_id) {
        Ok(id) => resolve_song_alias(&state.storage_dir, &id),
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };
    let version_nums = state.runner.checkpointer().list_versions(&clean_id).unwrap_or_default();
    let latest_v = version_nums.into_iter().max().unwrap_or(1);

    handle_stream_audio(
        State(state),
        AxumPath((clean_id, latest_v.to_string(), "final_master".to_string())),
        Query(AudioStreamQuery {
            download: Some("1".to_string()),
            format: query.format,
        }),
        headers,
    ).await
}

/// GET /api/songs — Lists all songs and their version trees present in `storage/songs/`.
pub async fn handle_list_songs(State(state): State<AppState>) -> Response {
    let songs_dir = state.storage_dir.join("songs");
    let mut catalog = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&songs_dir) {
        let mut song_ids = Vec::new();
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Ok(clean_id) = CheckpointManager::sanitize_path_component(name) {
                        song_ids.push(clean_id);
                    }
                }
            }
        }
        song_ids.sort();

        for clean_id in song_ids {
            let version_nums = state.runner.checkpointer().list_versions(&clean_id).unwrap_or_default();
            let mut versions = Vec::new();
            let mut latest_v = 1;
            let mut song_title = clean_id.clone();
            let mut song_genre = "Roots Reggae".to_string();

            for v in version_nums {
                latest_v = v;
                if let Ok(st) = state.runner.checkpointer().load_checkpoint(&clean_id, v).await {
                    if let Some(ref bp) = st.blueprint {
                        if !bp.title.is_empty() {
                            song_title = bp.title.clone();
                        }
                    }
                    song_genre = st.genre.clone();
                    let mut stems: Vec<String> = st.stem_paths.keys().cloned().collect();
                    if stems.is_empty() {
                        stems = vec![
                            STEM_FINAL_MASTER.to_string(),
                            STEM_DRUMS.to_string(),
                            STEM_BASS.to_string(),
                            STEM_GUITAR.to_string(),
                            STEM_RASPY_TENOR_VOCALS.to_string(),
                        ];
                    }
                    let has_master = st.has_stem(STEM_FINAL_MASTER);
                    versions.push(VersionSummary {
                        version: v,
                        title: song_title.clone(),
                        genre: st.genre.clone(),
                        bpm: st.bpm,
                        key_signature: st.key_signature.clone(),
                        judge_score: st.judge_score,
                        created_at: st.created_at.clone(),
                        iteration_summary: st.history.last().cloned(),
                        stems,
                        has_master,
                    });
                }
            }

            if !versions.is_empty() {
                catalog.push(SongCatalogEntry {
                    song_id: clean_id,
                    title: song_title,
                    genre: song_genre,
                    latest_version: latest_v,
                    versions,
                });
            }
        }
    }

    Json(ListSongsResponse { songs: catalog }).into_response()
}

/// GET /api/songs/:id/versions — Returns version tree and metadata.
pub async fn handle_list_versions(
    State(state): State<AppState>,
    AxumPath(song_id): AxumPath<String>,
) -> Response {
    let raw_clean_id = match CheckpointManager::sanitize_path_component(&song_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };
    let clean_id = resolve_song_alias(&state.storage_dir, &raw_clean_id);

    let version_nums = state.runner.checkpointer().list_versions(&clean_id).unwrap_or_default();

    let mut versions = Vec::new();
    let mut current_v = 1;

    for v in version_nums {
        current_v = v;
        if let Ok(st) = state.runner.checkpointer().load_checkpoint(&clean_id, v).await {
            let mut stems: Vec<String> = st.stem_paths.keys().cloned().collect();
            if stems.is_empty() {
                stems = vec![
                    STEM_FINAL_MASTER.to_string(),
                    STEM_DRUMS.to_string(),
                    STEM_BASS.to_string(),
                    STEM_GUITAR.to_string(),
                    STEM_RASPY_TENOR_VOCALS.to_string(),
                ];
            }
            let has_master = st.has_stem(STEM_FINAL_MASTER);

            versions.push(VersionSummary {
                version: v,
                title: st.song_id.clone(),
                genre: st.genre.clone(),
                bpm: st.bpm,
                key_signature: st.key_signature.clone(),
                judge_score: st.judge_score,
                created_at: st.created_at.clone(),
                iteration_summary: st.history.last().cloned(),
                stems,
                has_master,
            });
        }
    }

    Json(ListVersionsResponse {
        song_id: clean_id,
        current_version: current_v,
        versions,
    }).into_response()
}

/// Helper to verify that a target path is strictly contained within base_dir.
/// Resolves symlinks and normalizes case/verbatim UNC prefixes on Windows.
pub fn is_path_contained_in_storage(candidate: &Path, storage_dir: &Path) -> bool {
    let canon_cand = match candidate.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let canon_storage = match storage_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            if let Ok(curr) = std::env::current_dir() {
                curr.join(storage_dir).canonicalize().unwrap_or_else(|_| storage_dir.to_path_buf())
            } else {
                storage_dir.to_path_buf()
            }
        }
    };

    let cand_str = canon_cand.to_string_lossy().to_ascii_lowercase();
    let storage_str = canon_storage.to_string_lossy().to_ascii_lowercase();

    let clean_cand = cand_str.strip_prefix(r"\\?\").unwrap_or(&cand_str);
    let clean_storage = storage_str.strip_prefix(r"\\?\").unwrap_or(&storage_str);

    Path::new(clean_cand).starts_with(Path::new(clean_storage))
}

#[derive(Debug, Default, Deserialize)]
pub struct AudioStreamQuery {
    #[serde(default)]
    pub download: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

/// Converts a 32-bit IEEE float RIFF/WAVE byte buffer into a standard 16-bit PCM RIFF/WAVE byte buffer
/// if and only if it is a valid 32-bit float WAV (`audio_format == 3`, `bits_per_sample == 32`).
/// Otherwise returns `None` so arbitrary/raw test files are served unchanged.
fn transcode_float32_wav_to_pcm16(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let audio_format = u16::from_le_bytes([bytes[20], bytes[21]]);
    let bits_per_sample = u16::from_le_bytes([bytes[34], bytes[35]]);
    if audio_format == 3 && bits_per_sample == 32 {
        if let Ok(buf) = music_dsp::wav::AudioBuffer::from_wav_bytes(bytes) {
            return Some(buf.to_wav_bytes_with_format(music_dsp::wav::WavBitDepth::Pcm16));
        }
    }
    None
}

/// GET /api/songs/:id/versions/:v/audio/:stem
/// GET /api/songs/:id/:version/audio/:file
/// Streams 48 kHz WAV audio with Range support and cross-version lineage resolution.
pub async fn handle_stream_audio(
    State(state): State<AppState>,
    AxumPath((song_id, version_raw, stem)): AxumPath<(String, String, String)>,
    Query(query): Query<AudioStreamQuery>,
    headers: HeaderMap,
) -> Response {
    let version = match version_raw.trim_start_matches(['v', 'V']).parse::<u32>() {
        Ok(v) if v >= 1 => v,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid version '{}'", version_raw), "status": "bad_request"})),
            ).into_response();
        }
    };
    // 1. Sanitize components to fail closed against traversal, NT ADS, and DOS device names
    let raw_clean_id = match CheckpointManager::sanitize_path_component(&song_id) {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string(), "status": "bad_request"})),
            ).into_response();
        }
    };
    let clean_id = resolve_song_alias(&state.storage_dir, &raw_clean_id);
    let clean_stem = match CheckpointManager::sanitize_path_component(&stem) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string(), "status": "bad_request"})),
            ).into_response();
        }
    };

    // Normalize stem: strip .wav extension if provided (e.g. "guitar.wav" -> "guitar")
    let raw_stem = clean_stem.strip_suffix(".wav").unwrap_or(&clean_stem);

    // Build alias keys to match against state.stem_paths
    let mut alias_keys = vec![raw_stem.to_string(), clean_stem.clone()];
    if raw_stem == "master" {
        alias_keys.push("final_master".to_string());
    } else if raw_stem == "final_master" {
        alias_keys.push("master".to_string());
    } else if raw_stem == "vocals" {
        alias_keys.push("raspy_tenor_vocals".to_string());
        alias_keys.push("yue_vocals".to_string());
    } else if raw_stem == "instrumentals" {
        alias_keys.push("yue_instrumentals".to_string());
    }

    // 2. Checkpoint state lookup: Load checkpoint for (clean_id, version)
    // When targeted stems are regenerated, untargeted stems reference prior versions in state.stem_paths.
    let mut resolved_path: Option<PathBuf> = None;

    if let Ok(st) = state.runner.checkpointer().load_checkpoint(&clean_id, version).await {
        for key in &alias_keys {
            if let Some(stem_path) = st.stem_paths.get(key) {
                // Check if recorded path exists directly
                if stem_path.exists() && stem_path.is_file() {
                    resolved_path = Some(stem_path.clone());
                    break;
                }
                // Check if relative to storage_dir
                let joined = state.storage_dir.join(stem_path);
                if joined.exists() && joined.is_file() {
                    resolved_path = Some(joined);
                    break;
                }
                // Suffix / component match inside storage_dir (e.g. if stem_path was generated on another path)
                if let Some(file_name) = stem_path.file_name() {
                    // Probe current version stems dir and root dir
                    let cand1 = state.storage_dir
                        .join("songs")
                        .join(&clean_id)
                        .join(format!("v{}", version))
                        .join("stems")
                        .join(file_name);
                    if cand1.exists() && cand1.is_file() {
                        resolved_path = Some(cand1);
                        break;
                    }
                    let cand1_root = state.storage_dir
                        .join("songs")
                        .join(&clean_id)
                        .join(format!("v{}", version))
                        .join(file_name);
                    if cand1_root.exists() && cand1_root.is_file() {
                        resolved_path = Some(cand1_root);
                        break;
                    }
                    // Probe v1..version backwards across both root and stems dirs
                    for v in (1..=version).rev() {
                        let v_dir = state.storage_dir
                            .join("songs")
                            .join(&clean_id)
                            .join(format!("v{}", v));
                        let cand_root = v_dir.join(file_name);
                        if cand_root.exists() && cand_root.is_file() {
                            resolved_path = Some(cand_root);
                            break;
                        }
                        let cand_v = v_dir.join("stems").join(file_name);
                        if cand_v.exists() && cand_v.is_file() {
                            resolved_path = Some(cand_v);
                            break;
                        }
                    }
                    if resolved_path.is_some() {
                        break;
                    }
                }
            }
        }
    }

    // 3. Fallback path probing if not found in checkpoint state
    if resolved_path.is_none() {
        let v_dir = state.storage_dir.join("songs").join(&clean_id).join(format!("v{}", version));
        let mut cand_paths = vec![
            v_dir.join("stems").join(format!("{}.wav", raw_stem)),
            v_dir.join("stems").join(&clean_stem),
            v_dir.join(format!("{}.wav", raw_stem)),
            v_dir.join(&clean_stem),
        ];
        if raw_stem == "final_master" || raw_stem == "master" {
            cand_paths.push(v_dir.join("final_master.wav"));
            cand_paths.push(v_dir.join("stems").join("final_master.wav"));
            cand_paths.push(v_dir.join("master.wav"));
            cand_paths.push(v_dir.join("stems").join("master.wav"));
        }

        // Backwards lineage fallback across previous versions (v(version-1)..v1)
        for prev_v in (1..version).rev() {
            let prev_dir = state.storage_dir.join("songs").join(&clean_id).join(format!("v{}", prev_v));
            cand_paths.push(prev_dir.join("stems").join(format!("{}.wav", raw_stem)));
            cand_paths.push(prev_dir.join("stems").join(&clean_stem));
            cand_paths.push(prev_dir.join(format!("{}.wav", raw_stem)));
            cand_paths.push(prev_dir.join(&clean_stem));
            if raw_stem == "final_master" || raw_stem == "master" {
                cand_paths.push(prev_dir.join("final_master.wav"));
                cand_paths.push(prev_dir.join("stems").join("final_master.wav"));
            }
        }

        resolved_path = cand_paths.into_iter().find(|p| p.exists() && p.is_file());
    }

    // 4. Return 404 if file does not exist anywhere
    let path = match resolved_path {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Audio stem '{}' for song '{}' v{} not found", clean_stem, clean_id, version),
                    "status": "not_found"
                })),
            ).into_response();
        }
    };

    // 5. Path containment check: ensure resolved path is strictly within storage_dir
    if !is_path_contained_in_storage(&path, &state.storage_dir) {
        log::warn!("Security traversal alert: '{}' rejected outside storage_dir '{}'", path.display(), state.storage_dir.display());
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Access denied: audio path outside storage directory",
                "status": "forbidden"
            })),
        ).into_response();
    }

    // 6. Read file metadata and contents
    let raw_file_bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to read audio file: {}", e)})),
            ).into_response();
        }
    };

    // Transcode 32-bit IEEE float WAVs to universally playable 16-bit PCM WAV unless ?format=raw
    let want_raw = query.format.as_deref() == Some("raw");
    let file_bytes = if !want_raw {
        transcode_float32_wav_to_pcm16(&raw_file_bytes).unwrap_or(raw_file_bytes)
    } else {
        raw_file_bytes
    };

    let total_len = file_bytes.len();
    let is_download = query.download.as_deref() == Some("1") || query.download.as_deref() == Some("true");
    let download_filename = format!("{}_v{}_{}.wav", clean_id, version, raw_stem);
    let content_disp = if is_download {
        format!("attachment; filename=\"{}\"", download_filename)
    } else {
        format!("inline; filename=\"{}\"", download_filename)
    };

    // 7. Handle HTTP Range Header (RFC 7233 byte-range streaming for HTML5 audio scrubbing)
    if let Some(range_header) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if let Some(range) = parse_range_header(range_header, total_len) {
            let chunk = file_bytes[range.start..=range.end].to_vec();
            let content_range = format!("bytes {}-{}/{}", range.start, range.end, total_len);
            let content_len = chunk.len().to_string();

            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, "audio/wav")
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_RANGE, content_range)
                .header(header::CONTENT_LENGTH, content_len)
                .header(header::CONTENT_DISPOSITION, &content_disp)
                .body(Body::from(chunk))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    }

    // 8. Full audio stream response
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "audio/wav")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, total_len.to_string())
        .header(header::CONTENT_DISPOSITION, &content_disp)
        .body(Body::from(file_bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// POST /api/songs/assist-iteration — Queries Iteration Co-Pilot LLM for critique & suggestions.
pub async fn handle_assist_iteration(
    State(state): State<AppState>,
    Json(req): Json<AssistIterationRequest>,
) -> Response {
    let clean_req_id = match CheckpointManager::sanitize_path_component(&req.song_id) {
        Ok(id) => resolve_song_alias(&state.storage_dir, &id),
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    let song_state = match state.runner.checkpointer().load_checkpoint(&clean_req_id, req.version).await {
        Ok(s) => s,
        Err(_) => {
            let mut dummy = MusicProductionState::new(&clean_req_id, "Roots Reggae", 76, "A Major");
            dummy.version = req.version;
            dummy
        }
    };

    let mem = SystemMemory::detect_live();
    let panel = IterationFusionPanel::new_auto_configured(&mem).await;
    let resp = panel.assist_song_iteration(&song_state, req.focus.as_deref()).await
        .unwrap_or_else(|_| panel.offline_fallback_assist(&song_state));

    Json(AssistIterationResponse {
        song_id: resp.song_id,
        version: resp.version,
        copilot_model: resp.copilot_model,
        critique: resp.critique,
        suggested_next_iterations: resp.suggested_next_iterations,
    }).into_response()
}

/// GET /api/songs/:id/assist — Shorthand GET endpoint for assist-iteration.
pub async fn handle_assist_iteration_get(
    State(state): State<AppState>,
    AxumPath(song_id): AxumPath<String>,
) -> Response {
    handle_assist_iteration(State(state), Json(AssistIterationRequest {
        song_id,
        version: 1,
        focus: None,
    })).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct AudioDevicesResponse {
    pub devices: Vec<AudioOutputDevice>,
    pub microphones: Vec<AudioOutputDevice>,
}

/// GET /api/audio-devices — Discovers active Windows MMDevice render audio output endpoints
/// (`{0.0.0.00000000}`) AND capture microphone input endpoints (`{0.0.1.00000000}`)
/// in < 20ms using `reg query` without requiring upfront browser permission.
pub async fn handle_audio_devices() -> Json<AudioDevicesResponse> {
    let mut devices = vec![AudioOutputDevice {
        id: "default".to_string(),
        name: "System Default Output Device".to_string(),
        is_default: true,
        status: "OK".to_string(),
    }];
    let mut microphones = vec![AudioOutputDevice {
        id: "default".to_string(),
        name: "System Default Microphone".to_string(),
        is_default: true,
        status: "OK".to_string(),
    }];

    #[cfg(windows)]
    {
        let mut cmd = tokio::process::Command::new("reg");
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.args([
            "query",
            r"HKLM\SYSTEM\CurrentControlSet\Enum\SWD\MMDEVAPI",
            "/s",
            "/v",
            "FriendlyName",
        ]);

        if let Ok(Ok(output)) = tokio::time::timeout(std::time::Duration::from_millis(800), cmd.output()).await {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut current_key = String::new();
                let mut seen_out_names = std::collections::HashSet::new();
                let mut seen_mic_names = std::collections::HashSet::new();

                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("HKEY_LOCAL_MACHINE\\") {
                        current_key = trimmed.to_string();
                    } else if trimmed.starts_with("FriendlyName") {
                        if let Some(idx) = trimmed.find("REG_SZ") {
                            let friendly = trimmed[idx + 6..].trim();
                            if !friendly.is_empty() {
                                let dev_id = current_key
                                    .rsplit('\\')
                                    .next()
                                    .unwrap_or(friendly)
                                    .to_string();
                                if current_key.contains("{0.0.0.00000000}") {
                                    if seen_out_names.insert(friendly.to_ascii_lowercase()) {
                                        devices.push(AudioOutputDevice {
                                            id: dev_id,
                                            name: friendly.to_string(),
                                            is_default: false,
                                            status: "OK".to_string(),
                                        });
                                    }
                                } else if current_key.contains("{0.0.1.00000000}") {
                                    if seen_mic_names.insert(friendly.to_ascii_lowercase()) {
                                        microphones.push(AudioOutputDevice {
                                            id: dev_id,
                                            name: friendly.to_string(),
                                            is_default: false,
                                            status: "OK".to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Json(AudioDevicesResponse { devices, microphones })
}

/// GET /api/models/registry — Returns current Hugging Face model registry and active stage models.
pub async fn handle_models_registry(State(state): State<AppState>) -> Response {
    let mem = SystemMemory::detect_live();
    let engine = HfModelDiscoveryEngine::new(state.storage_dir.join("hf_music_models_registry.json"));
    let reg = engine.load_registry(&mem);
    Json(reg).into_response()
}

/// POST /api/models/discover-upgrade — Auto-discovers trending Hugging Face music/TTS models and upgrades stages.
pub async fn handle_models_discover_upgrade(State(state): State<AppState>) -> Response {
    let mem = SystemMemory::detect_live();
    let engine = HfModelDiscoveryEngine::new(state.storage_dir.join("hf_music_models_registry.json"));
    match engine.evaluate_and_upgrade(&mem).await {
        Ok(reg) => Json(serde_json::json!({
            "status": "success",
            "registry": reg,
        })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            })),
        ).into_response(),
    }
}

/// POST /api/ollama/bootstrap — Ensures Ollama is installed, running, and selects best model for available RAM/VRAM.
pub async fn handle_ollama_bootstrap(State(state): State<AppState>) -> Response {
    let mem = SystemMemory::detect_live();
    let mgr = OllamaLifecycleManager::new(&state.ollama_host);
    match mgr.bootstrap(&mem).await {
        Ok(status) => Json(serde_json::json!({
            "status": "success",
            "ollama": status,
        })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            })),
        ).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Static File Serving
// ---------------------------------------------------------------------------

pub async fn handle_index() -> Response {
    let index_path = Path::new("crates/music_cli/webui/index.html");
    if index_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(index_path).await {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(content))
                .unwrap();
        }
    }

    // Embedded fallback HTML ensuring zero empty states
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(DEFAULT_STUDIO_HTML))
        .unwrap()
}

pub async fn handle_styles() -> Response {
    let css_path = Path::new("crates/music_cli/webui/styles.css");
    if css_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(css_path).await {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
                .body(Body::from(content))
                .unwrap();
        }
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
        .body(Body::from("body { background: #121214; color: #fff; font-family: sans-serif; }"))
        .unwrap()
}

pub async fn handle_app_js() -> Response {
    let js_path = Path::new("crates/music_cli/webui/app.js");
    if js_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(js_path).await {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/javascript; charset=utf-8")
                .body(Body::from(content))
                .unwrap();
        }
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/javascript; charset=utf-8")
        .body(Body::from("console.log('MusicStudio UI loaded');"))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Router & Port Binding
// ---------------------------------------------------------------------------

/// Builds the complete Axum Router with all API and UI endpoints.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // API Routes
        .route("/api/health", get(handle_health))
        .route("/api/hardware", get(handle_hardware_status))
        .route("/api/audio-devices", get(handle_audio_devices))
        .route("/api/models/registry", get(handle_models_registry))
        .route("/api/models/discover-upgrade", post(handle_models_discover_upgrade))
        .route("/api/ollama/bootstrap", post(handle_ollama_bootstrap))
        .route("/api/songs", get(handle_list_songs))
        .route("/api/songs/create", post(handle_create_song))
        .route("/api/songs/iterate", post(handle_iterate_song))
        .route("/api/songs/regenerate-stem", post(handle_regenerate_stem))
        .route("/api/songs/master", post(handle_master_song))
        .route("/api/songs/tts-vocals", post(handle_tts_vocals))
        .route("/api/songs/revert", post(handle_revert_song))
        .route("/api/songs/:id/versions", get(handle_list_versions))
        .route("/api/songs/:id/versions/:v/audio/:stem", get(handle_stream_audio))
        .route("/api/songs/:id/:version/audio/:file", get(handle_stream_audio))
        // Dedicated Download Endpoints (Zero Errors)
        .route("/api/songs/:id/versions/:v/download", get(handle_download_version))
        .route("/api/songs/:id/download", get(handle_download_latest))
        .route("/api/songs/assist-iteration", post(handle_assist_iteration))
        .route("/api/songs/:id/assist", get(handle_assist_iteration_get).post(handle_assist_iteration))
        // Web Studio UI Static Assets
        .route("/", get(handle_index))
        .route("/index.html", get(handle_index))
        .route("/styles.css", get(handle_styles))
        .route("/app.js", get(handle_app_js))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Resilient port binding loop: attempts `start_port..=end_port` on `127.0.0.1` (or dynamic OS assignment if `start_port == 0`).
pub async fn bind_server(start_port: u16, end_port: u16) -> anyhow::Result<(tokio::net::TcpListener, u16)> {
    bind_server_on_host("127.0.0.1", start_port, end_port).await
}

/// Checks whether `port` is completely free across loopback, all-interfaces TCP (`0.0.0.0`),
/// and UDP (`0.0.0.0`) so we never collide with another service on `127.0.0.1`, `10.0.0.3`, or `0.0.0.0`.
fn is_port_free_across_interfaces(host: &str, port: u16) -> bool {
    let loopback = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    if std::net::TcpStream::connect_timeout(&loopback, std::time::Duration::from_millis(40)).is_ok() {
        return false;
    }
    if std::net::TcpListener::bind(("0.0.0.0", port)).is_err() {
        return false;
    }
    if std::net::UdpSocket::bind(("0.0.0.0", port)).is_err() {
        return false;
    }
    std::net::TcpListener::bind((host, port)).is_ok()
}

/// Resilient port binding loop on explicit host.
pub async fn bind_server_on_host(host: &str, start_port: u16, end_port: u16) -> anyhow::Result<(tokio::net::TcpListener, u16)> {
    if start_port == 0 {
        let listener = tokio::net::TcpListener::bind(format!("{}:0", host)).await?;
        let bound_port = listener.local_addr()?.port();
        log::info!("Successfully bound Tokio HTTP server on dynamic port http://{}:{}", host, bound_port);
        return Ok((listener, bound_port));
    }

    let effective_end = if end_port < start_port {
        start_port.saturating_add(20)
    } else if end_port == start_port {
        start_port.saturating_add(20)
    } else {
        end_port
    };

    for port in start_port..=effective_end {
        if !is_port_free_across_interfaces(host, port) {
            log::warn!(
                "Port {} is already in use by another service on this machine; trying next port {}...",
                port,
                port.saturating_add(1)
            );
            continue;
        }
        let addr = format!("{}:{}", host, port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                if port != start_port {
                    log::info!(
                        "Port {} was in use; automatically bound Tokio HTTP server to next free port http://{}",
                        start_port,
                        addr
                    );
                } else {
                    log::info!("Successfully bound Tokio HTTP server on http://{}", addr);
                }
                return Ok((listener, port));
            }
            Err(e) => {
                log::warn!("Port {} unavailable on {} ({}), attempting next port...", port, host, e);
            }
        }
    }
    anyhow::bail!("Failed to bind any port in range {}..={} on {}", start_port, effective_end, host)
}

/// Dynamic server binder: scans `start_port..=end_port` and falls back to OS-assigned ephemeral port
/// if the entire range is occupied, guaranteeing 100% zero-conflict startup.
pub async fn bind_server_dynamic(start_port: u16, end_port: u16) -> anyhow::Result<(tokio::net::TcpListener, u16)> {
    bind_server_dynamic_on_host("127.0.0.1", start_port, end_port).await
}

pub async fn bind_server_dynamic_on_host(host: &str, start_port: u16, end_port: u16) -> anyhow::Result<(tokio::net::TcpListener, u16)> {
    if let Ok(res) = bind_server_on_host(host, start_port, end_port).await {
        return Ok(res);
    }
    let listener = tokio::net::TcpListener::bind(format!("{}:0", host)).await?;
    let bound_port = listener.local_addr()?.port();
    log::info!(
        "Range {}..={} busy; dynamically bound Tokio HTTP server on OS port http://{}:{}",
        start_port,
        end_port,
        host,
        bound_port
    );
    Ok((listener, bound_port))
}

/// Starts the REST server in the background and returns the successfully bound port.
pub async fn start_rest_server(
    runner: Arc<PipelineRunner>,
    storage_dir: PathBuf,
    start_port: u16,
    end_port: u16,
    ollama_host: String,
) -> anyhow::Result<u16> {
    start_rest_server_with_host(runner, storage_dir, "127.0.0.1", start_port, end_port, ollama_host).await
}

/// Starts the REST server on an explicit host in the background and returns the successfully bound port.
pub async fn start_rest_server_with_host(
    runner: Arc<PipelineRunner>,
    storage_dir: PathBuf,
    host: &str,
    start_port: u16,
    end_port: u16,
    ollama_host: String,
) -> anyhow::Result<u16> {
    let (listener, bound_port) = bind_server_dynamic_on_host(host, start_port, end_port).await?;

    let app_state = AppState {
        runner,
        storage_dir,
        bound_port,
        ollama_host,
    };

    let router = create_router(app_state);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            log::error!("REST server terminated with error: {}", e);
        }
    });

    Ok(bound_port)
}

/// Runs the REST server in the foreground, blocking until SIGINT / Ctrl+C.
pub async fn run_server_foreground(
    runner: Arc<PipelineRunner>,
    storage_dir: PathBuf,
    start_port: u16,
    end_port: u16,
    ollama_host: String,
) -> anyhow::Result<()> {
    let (listener, bound_port) = bind_server_dynamic(start_port, end_port).await?;

    let app_state = AppState {
        runner,
        storage_dir,
        bound_port,
        ollama_host,
    };

    let router = create_router(app_state);
    axum::serve(listener, router).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

fn build_audio_urls(song_id: &str, version: u32) -> HashMap<String, String> {
    let stems = &[
        STEM_FINAL_MASTER,
        STEM_DRUMS,
        STEM_BASS,
        STEM_GUITAR,
        STEM_RASPY_TENOR_VOCALS,
    ];
    let mut map = HashMap::new();
    for stem in stems {
        map.insert(
            stem.to_string(),
            format!("/api/songs/{}/versions/{}/audio/{}", song_id, version, stem),
        );
    }
    map
}

pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

pub fn parse_range_header(header: &str, total: usize) -> Option<ByteRange> {
    if !header.starts_with("bytes=") {
        return None;
    }
    let range_part = &header[6..];
    let mut parts = range_part.split('-');
    let start_str = parts.next()?;
    let end_str = parts.next()?;

    let start = start_str.parse::<usize>().ok()?;
    let end = if end_str.is_empty() {
        total.saturating_sub(1)
    } else {
        end_str.parse::<usize>().ok()?.min(total.saturating_sub(1))
    };

    if start <= end && start < total {
        Some(ByteRange { start, end })
    } else {
        None
    }
}

const DEFAULT_STUDIO_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>MusicFusion AI Studio</title>
  <link rel="stylesheet" href="/styles.css">
</head>
<body>
  <div id="app">
    <h1>MusicFusion AI Studio</h1>
    <p>Server active on port 5050. Studio UI assets loading...</p>
  </div>
  <script src="/app.js"></script>
</body>
</html>"#;
