use axum::extract::{Json, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

use crate::state::DaemonState;
use quorum_coordinator::{AgenticTelemetryEvent, SynthesisModelFamily};
use quorum_storage::SearchMode;
use quorum_types::QueryResult;

/// Request payload for `/api/chat` and `/api/query`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// User prompt query string.
    pub prompt: String,

    /// Optional search mode override: Keyword, Vector, Hybrid (default: Hybrid).
    pub search_mode: Option<SearchMode>,

    /// Model family selector: AutoFusion, Glm4Agentic, Qwen25Code, GemmaProse.
    pub model_family: Option<String>,

    /// Top candidate chunks count (default: 8).
    pub top_k: Option<usize>,

    /// Authorized upper bound access tier (0=Public..3=Restricted, default: 0).
    pub access_tier: Option<u8>,
}

/// SSE Streaming chat endpoint: `POST /api/chat`.
/// Emits live Agentic State Graph events, GLM selection telemetry, token deltas, and citations.
pub async fn chat_sse_handler(
    State(state): State<Arc<DaemonState>>,
    Json(payload): Json<ChatRequest>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    info!(prompt = %payload.prompt, "Received /api/chat SSE streaming request");

    let prompt = payload.prompt.trim().to_string();
    let top_k = payload.top_k.unwrap_or(8);
    let _access_tier = payload.access_tier.unwrap_or(0);

    let family = match payload.model_family.as_deref() {
        Some("Glm4Agentic") | Some("glm4") | Some("glm-4") => SynthesisModelFamily::Glm4Agentic,
        Some("Qwen25Code") | Some("qwen") | Some("qwen2.5") => SynthesisModelFamily::Qwen25Code,
        Some("GemmaProse") | Some("gemma") | Some("gemma3") => SynthesisModelFamily::GemmaProse,
        _ => state.default_model_family,
    };

    let (tx, rx) = tokio::sync::mpsc::channel::<AgenticTelemetryEvent>(256);
    let coordinator = Arc::clone(&state.coordinator);

    tokio::spawn(async move {
        if prompt.is_empty() {
            let _ = tx
                .send(AgenticTelemetryEvent::ExecutionError {
                    failed_state: quorum_coordinator::types::AgenticStateName::ClassifyIntent,
                    error_message: "Prompt query string cannot be empty".to_string(),
                    fallback_action: "Aborting execution".to_string(),
                })
                .await;
            return;
        }

        match coordinator
            .query_agentic(&prompt, family, top_k, Some(tx.clone()))
            .await
        {
            Ok(result) => {
                // If offline engine produced full text, stream word deltas for typewriter effect
                let words: Vec<&str> = result.synthesis_prompt.split_inclusive(' ').collect();
                for word in words {
                    let _ = tx
                        .send(AgenticTelemetryEvent::TokenDelta {
                            delta: word.to_string(),
                        })
                        .await;
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
            Err(e) => {
                error!("Error in query_agentic: {}", e);
                let _ = tx
                    .send(AgenticTelemetryEvent::ExecutionError {
                        failed_state: quorum_coordinator::types::AgenticStateName::SynthesizeWithCitations,
                        error_message: e.to_string(),
                        fallback_action: "Returned partial graph state".to_string(),
                    })
                    .await;
            }
        }
    });

    let stream = ReceiverStream::new(rx).map(|event| {
        let json_line = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok::<Event, Infallible>(Event::default().data(json_line))
    });

    Sse::new(stream).keep_alive(KeepAlive::default().interval(Duration::from_secs(15)))
}

/// One-shot query endpoint: `POST /api/query`.
/// Returns structured JSON `QueryResult` for REST and programmatic consumers.
pub async fn query_handler(
    State(state): State<Arc<DaemonState>>,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    let prompt = payload.prompt.trim();
    if prompt.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Prompt cannot be empty" })),
        )
            .into_response();
    }

    let top_k = payload.top_k.unwrap_or(8);
    let family = match payload.model_family.as_deref() {
        Some("Glm4Agentic") => SynthesisModelFamily::Glm4Agentic,
        Some("Qwen25Code") => SynthesisModelFamily::Qwen25Code,
        Some("GemmaProse") => SynthesisModelFamily::GemmaProse,
        _ => state.default_model_family,
    };

    match state.coordinator.query_agentic(prompt, family, top_k, None).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
