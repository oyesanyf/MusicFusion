use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::handlers::{chat, drift, health, langextract, reindex};
use crate::state::DaemonState;

/// Constructs the complete Axum 0.7 web server Router.
pub fn build_router(state: Arc<DaemonState>) -> axum::Router {
    axum::Router::new()
        // Embedded Web UI
        .route("/", get(serve_index))
        .route("/assets/*path", get(serve_asset))
        .route("/favicon.ico", get(serve_favicon))

        // Chat & Query Endpoints (SSE streaming & JSON)
        .route("/api/chat", post(chat::chat_sse_handler))
        .route("/api/chat/stream", get(chat_stream_compat_handler))
        .route("/api/query", post(chat::query_handler))

        // Reindex Worker Endpoints
        .route("/api/reindex", post(reindex::start_reindex_handler))
        .route("/api/reindex/status", get(reindex::reindex_status_handler))
        .route("/api/reindex/cancel", post(reindex::cancel_reindex_handler))

        // Drift Healing Endpoints
        .route("/api/drift/fix", post(drift::fix_drift_handler))
        .route("/api/drift/status", get(drift::drift_status_handler))

        // Google LangExtract Endpoints
        .route("/api/langextract", post(langextract::extract_handler))
        .route("/api/langextract/visualize", post(langextract::visualize_handler))

        // Health & Models Telemetry
        .route("/api/health", get(health::health_handler))
        .route("/api/models", get(health::models_handler))

        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// GET / - Serves embedded ChatGPT-like interface.
async fn serve_index() -> impl IntoResponse {
    Html(quorum_ui::get_index_html())
}

/// GET /assets/*path - Serves embedded static assets.
async fn serve_asset(Path(path): Path<String>) -> Response {
    if let Some((bytes, mime)) = quorum_ui::get_asset(&path) {
        ([(header::CONTENT_TYPE, mime)], bytes).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Asset not found").into_response()
    }
}

/// GET /favicon.ico - Serves favicon.
async fn serve_favicon() -> Response {
    if let Some((bytes, mime)) = quorum_ui::get_asset("favicon.ico") {
        ([(header::CONTENT_TYPE, mime)], bytes).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// GET /api/chat/stream?prompt=... query compatibility endpoint (matching test_web_endpoints.rs).
async fn chat_stream_compat_handler(
    State(state): State<Arc<DaemonState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let prompt = params.get("prompt").cloned().unwrap_or_default();
    let family = params.get("model_family").cloned();
    let req = chat::ChatRequest {
        prompt,
        search_mode: None,
        model_family: family,
        top_k: Some(8),
        access_tier: Some(0),
    };
    chat::chat_sse_handler(State(state), axum::Json(req)).await
}
