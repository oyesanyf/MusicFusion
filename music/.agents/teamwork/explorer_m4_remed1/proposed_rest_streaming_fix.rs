//! Proposed Drop-in Remediation for Audio Streaming in `crates/music_cli/src/rest_server.rs`
//!
//! Fixes:
//! 1. Checkpoint state lookup: Inspects `state.runner.checkpointer().load_checkpoint(&clean_id, version)`
//!    to resolve stems from `state.stem_paths` across version lineages (e.g. unregenerated stems).
//! 2. Path resolution: Checks `st.stem_paths` using canonical stem names and aliases (`final_master`, `master`,
//!    `raspy_tenor_vocals`, `yue_vocals`, etc.), falling back to probing `v{version}/stems/` and earlier versions.
//! 3. Strict containment verification: `is_path_contained_in_storage` prevents directory traversal.
//! 4. Dual route support: Supports both `/api/songs/:id/versions/:v/audio/:stem` and `/api/songs/:id/:version/audio/:file`.

use std::path::{Path, PathBuf};
use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use music_pipeline::checkpointer::CheckpointManager;
use crate::rest_server::{parse_range_header, AppState};

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

/// GET /api/songs/:id/versions/:v/audio/:stem
/// GET /api/songs/:id/:version/audio/:file
/// Streams 48 kHz WAV audio with Range support and cross-version lineage resolution.
pub async fn handle_stream_audio(
    State(state): State<AppState>,
    AxumPath((song_id, version, stem)): AxumPath<(String, u32, String)>,
    headers: HeaderMap,
) -> Response {
    // 1. Sanitize components to fail closed against traversal, NT ADS, and DOS device names
    let clean_id = match CheckpointManager::sanitize_path_component(&song_id) {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string(), "status": "bad_request"})),
            ).into_response();
        }
    };
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
                    // Probe current version stems dir
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
                    // Probe v1..version backwards
                    for v in (1..=version).rev() {
                        let cand_v = state.storage_dir
                            .join("songs")
                            .join(&clean_id)
                            .join(format!("v{}", v))
                            .join("stems")
                            .join(file_name);
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
        .body(Body::from(file_bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

// ---------------------------------------------------------------------------
// Proposed Router Extension in `create_router(state: AppState) -> Router`
// ---------------------------------------------------------------------------
//
// In `rest_server.rs:693`:
// ```rust
//         .route("/api/songs/:id/versions", get(handle_list_versions))
//         .route("/api/songs/:id/versions/:v/audio/:stem", get(handle_stream_audio))
//         .route("/api/songs/:id/:version/audio/:file", get(handle_stream_audio))
// ```
