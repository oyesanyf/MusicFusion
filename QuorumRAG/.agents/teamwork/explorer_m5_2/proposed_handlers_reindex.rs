use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use crate::state::DaemonState;
use quorum_ingest::ReindexOptions;

/// Payload to trigger reindexing via `POST /api/reindex`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReindexRequest {
    /// Optional specific directories to reindex.
    pub target_paths: Option<Vec<PathBuf>>,

    /// Bypass mtime and Level-1 BLAKE3 catalogs, re-verifying all files.
    pub full_rebuild: Option<bool>,

    /// Force re-embedding all chunks even if Level-2 hash matches.
    pub force_reembed: Option<bool>,

    /// Batch size for writes.
    pub batch_size: Option<usize>,
}

/// Start background reindexer: `POST /api/reindex`.
pub async fn start_reindex_handler(
    State(state): State<Arc<DaemonState>>,
    Json(payload): Json<ReindexRequest>,
) -> impl IntoResponse {
    let options = ReindexOptions {
        target_paths: payload
            .target_paths
            .unwrap_or_else(|| state.ingest_config.watch_paths.clone()),
        full_rebuild: payload.full_rebuild.unwrap_or(false),
        force_reembed: payload.force_reembed.unwrap_or(false),
        batch_size: payload.batch_size.unwrap_or(64),
        num_threads: 0,
        dry_run: false,
    };

    info!(
        targets = ?options.target_paths,
        full = options.full_rebuild,
        "Received POST /api/reindex request"
    );

    match state.reindex_worker.start_reindex(options).await {
        Ok(task_id) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "status": "started",
                "task_id": task_id.to_string(),
                "message": "Reindex worker task started in background"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "status": "error",
                "message": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Retrieve live reindex progress: `GET /api/reindex/status`.
pub async fn reindex_status_handler(
    State(state): State<Arc<DaemonState>>,
) -> impl IntoResponse {
    match state.reindex_worker.get_progress().await {
        Some(progress) => Json(serde_json::to_value(progress).unwrap_or_default()).into_response(),
        None => Json(serde_json::json!({
            "phase": "idle",
            "message": "No reindexing task has been executed yet"
        }))
        .into_response(),
    }
}

/// Cancel active reindex task: `POST /api/reindex/cancel`.
pub async fn cancel_reindex_handler(
    State(state): State<Arc<DaemonState>>,
) -> impl IntoResponse {
    let cancelled = state.reindex_worker.cancel_reindex();
    Json(serde_json::json!({
        "status": "cancel_requested",
        "cancelled": cancelled
    }))
}
