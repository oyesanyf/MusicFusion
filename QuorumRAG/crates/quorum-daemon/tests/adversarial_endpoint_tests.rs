//! Empirical Adversarial Stress Suite for QuorumRAG Daemon Endpoints
//!
//! Areas Covered:
//! 1. Endpoint Concurrency Stress (simultaneous /api/health, /api/models, /api/query)
//! 2. SSE Streaming Protocol & Syntax Assertions (text/event-stream, data: line format, AgenticTelemetryEvent parsing)
//! 3. Google LangExtract Standalone Visualizer Integrity & Pastel Spans
//! 4. Fail-Closed Path Traversal Gating on /api/langextract and /api/langextract/visualize

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use futures_util::future::join_all;
use quorum_coordinator::{AgenticTelemetryEvent, SynthesisModelFamily};
use quorum_daemon::{build_router, DaemonState};
use std::sync::Arc;
use std::time::Instant;
use tempfile::tempdir;
use tower::ServiceExt;

async fn setup_test_app() -> axum::Router {
    let temp_dir = tempdir().expect("tempdir");
    let data_dir = temp_dir.path().to_path_buf();

    let storage_config = quorum_storage::StorageConfig::new(data_dir.clone());
    let storage = Arc::new(
        quorum_storage::StorageEngine::open(storage_config)
            .await
            .expect("storage open"),
    );
    let classifier = Arc::new(quorum_classifier::FileTypeClassifier::new());
    let embedder: Arc<dyn quorum_contract::EmbeddingGenerator> =
        Arc::new(quorum_contract::DeterministicOfflineEngine::new());
    let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());
    let ingest_config = quorum_ingest::IngestConfig::default();

    let reindex_worker = Arc::new(quorum_ingest::ReindexWorker::new(
        ingest_config.clone(),
        Arc::clone(&storage),
        Arc::clone(&classifier),
        Arc::clone(&embedder),
        Arc::clone(&dedup),
    ));

    let drift_healer = Arc::new(quorum_ingest::DriftHealer::new(
        ingest_config.clone(),
        Arc::clone(&storage),
        Arc::clone(&classifier),
        Arc::clone(&embedder),
        Arc::clone(&dedup),
    ));

    let coordinator_engine = Arc::new(quorum_coordinator::ScatterGatherEngine::new(
        Arc::clone(&storage) as Arc<dyn quorum_coordinator::StorageSearcher>,
        Arc::new(quorum_coordinator::LocalOnlyDispatcher),
        Arc::new(tokio::sync::RwLock::new(
            quorum_coordinator::PeerAclManager::default(),
        )),
        quorum_coordinator::CoordinatorConfig::default(),
        [0u8; 32],
        "adversarial_test".to_string(),
    ));

    let coordinator = Arc::new(quorum_coordinator::QueryCoordinator::new(
        coordinator_engine,
        quorum_coordinator::CoordinatorConfig::default(),
    ));

    let state = Arc::new(DaemonState {
        start_time: Instant::now(),
        storage,
        coordinator,
        reindex_worker,
        drift_healer,
        classifier,
        embedder,
        dedup,
        swarm_handle: None,
        ingest_config,
        default_model_family: SynthesisModelFamily::AutoFusion,
        port: 3030,
        host: "127.0.0.1".to_string(),
        data_dir,
    });

    build_router(state)
}

// ============================================================================
// 1. ENDPOINT CONCURRENCY STRESS
// ============================================================================

#[tokio::test]
async fn test_endpoint_concurrency_stress() {
    let app = setup_test_app().await;

    // Dispatch 30 simultaneous requests across health, models, and query endpoints
    let mut tasks = Vec::new();
    for i in 0..30 {
        let app_clone = app.clone();
        let task = tokio::spawn(async move {
            let req = match i % 3 {
                0 => Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
                1 => Request::builder()
                    .uri("/api/models")
                    .body(Body::empty())
                    .unwrap(),
                _ => {
                    let body = serde_json::json!({
                        "prompt": format!("Concurrent query test prompt {}", i),
                        "model_family": "AutoFusion"
                    });
                    Request::builder()
                        .method("POST")
                        .uri("/api/query")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(serde_json::to_vec(&body).unwrap()))
                        .unwrap()
                }
            };

            let response = app_clone.oneshot(req).await.expect("Request execution");
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Request {} failed with status: {:?}",
                i,
                response.status()
            );
        });
        tasks.push(task);
    }

    let results = join_all(tasks).await;
    for (idx, res) in results.into_iter().enumerate() {
        assert!(res.is_ok(), "Task {} panicked or failed", idx);
    }
}

// ============================================================================
// 2. SSE STREAMING PROTOCOL & SYNTAX STRESS
// ============================================================================

#[tokio::test]
async fn test_chat_sse_stream_format_and_event_types() {
    let app = setup_test_app().await;

    let chat_payload = serde_json::json!({
        "prompt": "How does Reciprocal Rank Fusion combine vector and BM25 scores?",
        "model_family": "Glm4Agentic",
        "top_k": 4
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&chat_payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.expect("Execute chat request");
    assert_eq!(response.status(), StatusCode::OK);

    // Verify Content-Type is text/event-stream
    let ct = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .expect("Content-Type header");
    assert!(
        ct.starts_with("text/event-stream"),
        "Expected text/event-stream, got: {}",
        ct
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Read SSE stream bytes");
    let stream_text = String::from_utf8_lossy(&body_bytes);

    // Parse lines according to SSE specification
    let mut data_events = Vec::new();
    for line in stream_text.lines() {
        if line.starts_with("data:") {
            let data_str = line.trim_start_matches("data:").trim();
            if !data_str.is_empty() {
                // Must deserialize into valid AgenticTelemetryEvent
                let event: AgenticTelemetryEvent = serde_json::from_str(data_str)
                    .unwrap_or_else(|e| panic!("Failed to parse SSE event JSON '{}': {}", data_str, e));
                data_events.push(event);
            }
        }
    }

    assert!(
        !data_events.is_empty(),
        "SSE stream must produce at least one data: event"
    );

    // Verify token deltas or stage transitions were emitted
    let has_token_or_state = data_events.iter().any(|e| {
        matches!(
            e,
            AgenticTelemetryEvent::TokenDelta { .. }
                | AgenticTelemetryEvent::StateTransition { .. }
                | AgenticTelemetryEvent::ExecutionFinished { .. }
        )
    });
    assert!(
        has_token_or_state,
        "SSE stream must contain TokenDelta, StateTransition, or ExecutionFinished events"
    );
}

#[tokio::test]
async fn test_chat_sse_empty_prompt_emits_error_cleanly() {
    let app = setup_test_app().await;

    let empty_payload = serde_json::json!({
        "prompt": "   ",
        "model_family": "AutoFusion"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&empty_payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.expect("Execute chat request");
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Read SSE stream bytes");
    let stream_text = String::from_utf8_lossy(&body_bytes);

    // Verify error event was emitted gracefully
    assert!(
        stream_text.contains("ExecutionError") || stream_text.contains("empty"),
        "Empty prompt must yield ExecutionError event without panicking"
    );
}

#[tokio::test]
async fn test_chat_stream_compat_get_endpoint() {
    let app = setup_test_app().await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/chat/stream?prompt=test%20query&model_family=AutoFusion")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.expect("Execute GET stream compat");
    assert_eq!(response.status(), StatusCode::OK);

    let ct = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(
        ct.starts_with("text/event-stream"),
        "GET /api/chat/stream must return text/event-stream, got: {}",
        ct
    );
}

// ============================================================================
// 3. GOOGLE LANGEXTRACT VISUALIZER & GROUNDING ACCURACY
// ============================================================================

#[tokio::test]
async fn test_langextract_visualizer_html_integrity() {
    let app = setup_test_app().await;

    let rust_code = r#"
pub struct RrfCombiner;
impl RrfCombiner {
    pub fn compute_rrf_score(rank: usize) -> f32 {
        1.0 / (60.0 + rank as f32)
    }
}
"#;

    let req_body = serde_json::json!({
        "text": rust_code
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/langextract/visualize")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.expect("Execute visualize request");
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Read HTML body");
    let html = String::from_utf8_lossy(&body_bytes);

    // 1. Must be complete standalone HTML
    assert!(html.contains("<!DOCTYPE html>"), "Must declare <!DOCTYPE html>");
    assert!(html.contains("<html") && html.contains("</html>"), "Must have html tags");
    assert!(html.contains("<head>") && html.contains("</head>"), "Must have head tags");
    assert!(html.contains("<body>") && html.contains("</body>"), "Must have body tags");

    // 2. Must contain embedded styling
    assert!(html.contains("<style>"), "Must have embedded style block");

    // 3. Must contain pastel highlight color codes
    let has_pastel = html.contains("#D2E3FC") || html.contains("#FFE082") || html.contains("#C8E6C9");
    assert!(has_pastel, "Must contain pastel entity highlight color codes");

    // 4. Must contain mark spans with data-idx attribute
    assert!(
        html.contains("<mark class=\"lx-span\"") && html.contains("data-idx="),
        "Must contain <mark class=\"lx-span\" data-idx=\"...\"> highlight spans"
    );

    // 5. Must escape HTML tags in text to prevent XSS
    let xss_test_code = "fn evil() { let _x = \"<script>alert('xss')</script>\"; }";
    let xss_req_body = serde_json::json!({ "text": xss_test_code });
    let xss_req = Request::builder()
        .method("POST")
        .uri("/api/langextract/visualize")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&xss_req_body).unwrap()))
        .unwrap();

    let xss_resp = setup_test_app().await.oneshot(xss_req).await.unwrap();
    let xss_bytes = axum::body::to_bytes(xss_resp.into_body(), 1024 * 1024).await.unwrap();
    let xss_html = String::from_utf8_lossy(&xss_bytes);

    assert!(
        !xss_html.contains("<script>alert"),
        "Raw unescaped script tag leaked into visualizer HTML!"
    );
    assert!(
        xss_html.contains("&lt;script&gt;alert"),
        "Script tag must be safely HTML escaped"
    );
}

// ============================================================================
// 4. FAIL-CLOSED DIRECTORY TRAVERSAL GATING ON LANGEXTRACT
// ============================================================================

#[tokio::test]
async fn test_langextract_directory_traversal_blocked_fail_closed() {
    let app = setup_test_app().await;

    let traversal_paths = [
        "../../Cargo.toml",
        "/../../etc/passwd",
        "..\\..\\Cargo.toml",
        "subdir/../../../windows/win.ini",
    ];

    for path in traversal_paths {
        // Test POST /api/langextract
        let req_body = serde_json::json!({
            "file_path": path
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/langextract")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(req).await.expect("Execute request");
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "Path '{}' on /api/langextract must return 403 Forbidden, got: {:?}",
            path,
            response.status()
        );

        // Test POST /api/langextract/visualize
        let req_vis = Request::builder()
            .method("POST")
            .uri("/api/langextract/visualize")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
            .unwrap();

        let response_vis = app.clone().oneshot(req_vis).await.expect("Execute request");
        assert_eq!(
            response_vis.status(),
            StatusCode::FORBIDDEN,
            "Path '{}' on /api/langextract/visualize must return 403 Forbidden, got: {:?}",
            path,
            response_vis.status()
        );
    }
}
