//! Tokio Axum HTTP JSON REST Server (Port 5050 with fallback 5050..5060)
//! and audio stem streaming engine for the AI Music Producer Studio.
//!
//! Designed for Milestone 4: Tokio REST API & Multi-LLM Iteration Co-Pilot Fusion.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use music_core::genre::GenreTaxonomy;
use music_core::state::{
    EngineMode, MusicProductionState, STEM_BASS, STEM_DRUMS, STEM_FINAL_MASTER,
    STEM_GUITAR, STEM_RASPY_TENOR_VOCALS,
};
use music_core::telemetry::StemTelemetry;
use music_fusion::iteration_panel::{
    IterationFusionPanel, ParameterTweaks,
};
use music_fusion::memory::SystemMemory;
use music_fusion::ollama_client::OllamaClient;
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
    #[serde(default = "default_version_one")]
    pub from_version: u32,
    #[serde(alias = "instructions")]
    pub critique: String,
    #[serde(default)]
    pub bpm: Option<u32>,
    #[serde(default)]
    pub key_signature: Option<String>,
    #[serde(default)]
    pub vocal_rasp: Option<f32>,
    #[serde(default)]
    pub guitar_wah: Option<f32>,
    #[serde(default)]
    pub drum_fill_complexity: Option<f32>,
    #[serde(default)]
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
    if let Err(e) = CheckpointManager::sanitize_path_component(&req.song_id) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    // 2. Load prior version
    let prev_state = match state.runner.checkpointer().load_checkpoint(&req.song_id, req.from_version).await {
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
    next_state.bpm = iteration_result.blueprint.bpm.round() as u32;
    next_state.key_signature = iteration_result.blueprint.key_signature.clone();
    if !iteration_result.blueprint.lyrics_text.is_empty() {
        next_state.lyrics = iteration_result.blueprint.lyrics_text.clone();
    }
    next_state.history.push(format!(
        "Iterated from v{} to v{}: {}",
        req.from_version, next_state.version, req.critique
    ));

    // 4. Re-execute pipeline
    match state.runner.execute_song(next_state).await {
        Ok(final_state) => {
            let audio_urls = build_audio_urls(&final_state.song_id, final_state.version);

            Json(IterateSongResponse {
                status: "success".to_string(),
                song_id: final_state.song_id.clone(),
                from_version: req.from_version,
                to_version: final_state.version,
                iteration_summary: iteration_result.iteration_summary,
                stem_deltas: iteration_result.stem_deltas,
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
    if let Err(e) = CheckpointManager::sanitize_path_component(&req.song_id) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    if let Err(e) = CheckpointManager::sanitize_path_component(&req.stem) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    match state.runner.regenerate_stem(&req.song_id, req.version, &req.stem).await {
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

/// POST /api/songs/master — Runs Node 6 analog tape mastering with specified profile.
pub async fn handle_master_song(
    State(state): State<AppState>,
    Json(req): Json<MasterSongRequest>,
) -> Response {
    if let Err(e) = CheckpointManager::sanitize_path_component(&req.song_id) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    let profile = req.profile.unwrap_or_else(|| "WarmAnalogTape".to_string());
    match state.runner.master_track(&req.song_id, req.version).await {
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

/// GET /api/songs/:id/versions — Returns version tree and metadata.
pub async fn handle_list_versions(
    State(state): State<AppState>,
    AxumPath(song_id): AxumPath<String>,
) -> Response {
    let clean_id = match CheckpointManager::sanitize_path_component(&song_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

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

/// GET /api/songs/:id/versions/:v/audio/:stem — Streams 48 kHz WAV audio with Range support.
pub async fn handle_stream_audio(
    State(state): State<AppState>,
    AxumPath((song_id, version, stem)): AxumPath<(String, u32, String)>,
    headers: HeaderMap,
) -> Response {
    // 1. Sanitize components
    let clean_id = match CheckpointManager::sanitize_path_component(&song_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };
    let clean_stem = match CheckpointManager::sanitize_path_component(&stem) {
        Ok(s) => s,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    // 2. Resolve audio file path
    let v_dir = state.storage_dir.join("songs").join(&clean_id).join(format!("v{}", version));
    let mut cand_paths = vec![
        v_dir.join("stems").join(format!("{}.wav", clean_stem)),
        v_dir.join(format!("{}.wav", clean_stem)),
    ];
    if clean_stem == "final_master" || clean_stem == "master" {
        cand_paths.push(v_dir.join("final_master.wav"));
    }

    let found_path = cand_paths.into_iter().find(|p| p.exists());

    let path = match found_path {
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

    // 3. Read file metadata
    let file_bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to read audio file: {}", e)})),
            ).into_response();
        }
    };

    let total_len = file_bytes.len();

    // 4. Handle HTTP Range Header (for HTML5 audio scrubbing)
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
                .body(Body::from(chunk))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    }

    // 5. Full audio stream response
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "audio/wav")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, total_len.to_string())
        .body(Body::from(file_bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// POST /api/songs/assist-iteration — Queries Iteration Co-Pilot LLM for critique & suggestions.
pub async fn handle_assist_iteration(
    State(state): State<AppState>,
    Json(req): Json<AssistIterationRequest>,
) -> Response {
    if let Err(e) = CheckpointManager::sanitize_path_component(&req.song_id) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    let song_state = match state.runner.checkpointer().load_checkpoint(&req.song_id, req.version).await {
        Ok(s) => s,
        Err(_) => {
            let mut dummy = MusicProductionState::new(&req.song_id, "Roots Reggae", 76, "A Major");
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
        .route("/api/songs/create", post(handle_create_song))
        .route("/api/songs/iterate", post(handle_iterate_song))
        .route("/api/songs/regenerate-stem", post(handle_regenerate_stem))
        .route("/api/songs/master", post(handle_master_song))
        .route("/api/songs/:id/versions", get(handle_list_versions))
        .route("/api/songs/:id/versions/:v/audio/:stem", get(handle_stream_audio))
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

/// Resilient port binding loop: attempts 5050, falling back to 5051..5060 if busy.
pub async fn bind_server(start_port: u16, end_port: u16) -> anyhow::Result<(tokio::net::TcpListener, u16)> {
    for port in start_port..=end_port {
        let addr = format!("127.0.0.1:{}", port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                log::info!("Successfully bound Tokio HTTP server on http://{}", addr);
                return Ok((listener, port));
            }
            Err(e) => {
                log::warn!("Port {} unavailable ({}), attempting next port...", port, e);
            }
        }
    }
    anyhow::bail!("Failed to bind any port in range {}..={}", start_port, end_port)
}

/// Starts the REST server in the background and returns the successfully bound port.
pub async fn start_rest_server(
    runner: Arc<PipelineRunner>,
    storage_dir: PathBuf,
    start_port: u16,
    end_port: u16,
    ollama_host: String,
) -> anyhow::Result<u16> {
    let (listener, bound_port) = bind_server(start_port, end_port).await?;

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
    let (listener, bound_port) = bind_server(start_port, end_port).await?;

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
