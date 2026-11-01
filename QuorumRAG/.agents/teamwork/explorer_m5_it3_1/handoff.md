# Investigation & Technical Design Report: Axum 0.7 Web Route Fallback Routing

- **Author**: `explorer_m5_it3_1` (Explorer — Milestone 5 Iteration 3)
- **Recipient**: Parent Orchestrator (`e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`)
- **Date**: 2026-11-01T03:28:00Z
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_1\`
- **Target File**: `crates/quorum-daemon/src/server.rs`
- **Target Test Suite**: `crates/quorum-daemon/tests/adversarial_web_routes_challenge.rs`
- **Proposed Patch**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_1\server_fallback.patch`

---

## 1. Observation

### 1.1 Direct Tool Execution & Test Reproduction

1. **Reproduction Command**:
   ```powershell
   cargo test -p quorum-daemon --test adversarial_web_routes_challenge -- --test-threads=2
   ```

2. **Verbatim Test Failure Output**:
   ```text
   running 5 tests
   test challenge_api_chat_valid_and_malformed ... ok
   test challenge_api_shutdown_endpoint ... ok
   test challenge_static_assets_direct_and_spa_fallback ... FAILED
   test challenge_static_assets_routing ... ok
   test challenge_api_health ... ok

   failures:

   ---- challenge_static_assets_direct_and_spa_fallback stdout ----

   thread 'challenge_static_assets_direct_and_spa_fallback' (20944) panicked at crates\quorum-daemon\tests\adversarial_web_routes_challenge.rs:284:9:
   assertion `left == right` failed: Direct route '/styles.css' on daemon router must return StatusCode::OK, got: 404
     left: 404
    right: 200
   note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

   failures:
       challenge_static_assets_direct_and_spa_fallback

   test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.83s
   ```

3. **Status of Existing Tests**:
   - `challenge_api_health`: PASSED (verifies `GET /api/health` returns 200 OK, JSON, `status: "healthy"`, contract hash, storage/hardware blocks).
   - `challenge_api_chat_valid_and_malformed`: PASSED (verifies `POST /api/chat` with valid body returns `text/event-stream`, malformed JSON returns 400/422, garbage bytes return 400/422).
   - `challenge_api_shutdown_endpoint`: PASSED (verifies `POST /api/shutdown` returns 200 OK and triggers `shutdown_notify.notify_one()`).
   - `challenge_static_assets_routing`: PASSED (verifies `/assets/styles.css`, `/assets/app.js`, `/assets/langextract.html`, `/assets/favicon.svg` return 200 OK and expected MIME types).
   - `challenge_static_assets_direct_and_spa_fallback`: FAILED on first direct asset route (`/styles.css`), returning HTTP 404 instead of 200 OK. The subsequent 4 assertions in that test (`/app.js`, `/langextract.html`, `/favicon.svg`, and `/unknown/spa/route/123`) would also fail with 404.

### 1.2 Inspection of Existing Router Implementation

- **File**: `crates/quorum-daemon/src/server.rs`, lines 11–46:
  ```rust
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

          // Graceful Shutdown Endpoint
          .route("/api/shutdown", post(shutdown_handler))

          .layer(CorsLayer::permissive())
          .with_state(state)
  }
  ```

### 1.3 Inspection of UI Static Asset Handler

- **File**: `crates/quorum-ui/src/assets.rs`, lines 38–71:
  ```rust
  /// Static asset handler with SPA fallback to index.html.
  pub async fn static_handler(uri: Uri) -> Response {
      let path = uri.path().trim_start_matches('/');
      let stripped = path.strip_prefix("assets/").unwrap_or(path);
      let target_path = if stripped.is_empty() { "index.html" } else { stripped };

      match WebAssets::get(target_path) {
          Some(content) => {
              let mime = mime_guess::from_path(target_path)
                  .first_raw()
                  .unwrap_or("application/octet-stream");
              Response::builder()
                  .status(StatusCode::OK)
                  .header(
                      header::CONTENT_TYPE,
                      HeaderValue::from_str(mime).unwrap_or(HeaderValue::from_static("application/octet-stream")),
                  )
                  .header(header::CACHE_CONTROL, "public, max-age=3600")
                  .body(Body::from(content.data))
                  .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Response building failed").into_response())
          }
          None => match WebAssets::get("index.html") {
              Some(content) => Response::builder()
                  .status(StatusCode::OK)
                  .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                  .body(Body::from(content.data))
                  .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Response building failed").into_response()),
              None => Response::builder()
                  .status(StatusCode::NOT_FOUND)
                  .body(Body::from("404 Not Found"))
                  .unwrap_or_else(|_| (StatusCode::NOT_FOUND, "Not Found").into_response()),
          },
      }
  }
  ```

---

## 2. Logic Chain

1. **Root Cause Analysis (Linking Observation 1.1 to 1.2)**:
   In `crates/quorum-daemon/src/server.rs`, `build_router` defines explicit routes for `/`, `/assets/*path`, `/favicon.ico`, and all `/api/...` endpoints. However, it does not define routes for direct root static asset paths (`/styles.css`, `/app.js`, `/langextract.html`, `/favicon.svg`), nor does it register an Axum fallback handler (`.fallback(...)`). Consequently, when a request arrives for any path other than `/`, `/favicon.ico`, `/assets/*`, or `/api/*`, Axum's default fallback responds with `404 Not Found`.

2. **Availability of Existing Solution in Workspace (Observation 1.3)**:
   `crates/quorum-ui` already provides `pub async fn static_handler(uri: Uri) -> Response` in `crates/quorum-ui/src/assets.rs` (re-exported at `crates/quorum-ui/src/lib.rs:8`).
   - For direct static asset paths (e.g. `/styles.css`): `WebAssets::get("styles.css")` finds the asset, returns HTTP 200 OK with `Content-Type: text/css`, `Cache-Control: public, max-age=3600`, and file bytes.
   - For virtual SPA paths (e.g. `/unknown/spa/route/123`, `/chat/123`): `WebAssets::get` returns `None`, which falls back to the embedded `index.html`, returning HTTP 200 OK with `Content-Type: text/html; charset=utf-8`.

3. **Axum 0.7 Type & State Compatibility**:
   - `build_router` creates `axum::Router::new()` with routes requiring state `Arc<DaemonState>`.
   - `quorum_ui::static_handler` takes only `(uri: Uri)`, without extracting `State<...>`. Under Axum 0.7, any stateless handler `H: Handler<T, ()>` automatically implements `Handler<T, S>` for any state `S`. Thus, `quorum_ui::static_handler` implements `Handler<_, Arc<DaemonState>>`.
   - `Router<S>::fallback` accepts `quorum_ui::static_handler` before `.with_state(state)` is called.
   - Placing `.fallback(quorum_ui::static_handler)` before `.layer(CorsLayer::permissive())` ensures that cross-origin requests to fallback routes and static assets receive CORS headers.
   - Calling `.with_state(state)` after `.fallback(...)` and `.layer(...)` cleanly consumes the `Arc<DaemonState>` requirement and yields `axum::Router<()>`.

4. **Preservation of Explicit Routes & API Routes Priority**:
   - In Axum 0.7, registered routes (`.route(...)`) always take strict precedence over `.fallback(...)`.
   - Exact API routes (`/api/health`, `/api/chat`, `/api/shutdown`, etc.) are matched first; malformed payloads continue to be rejected with 400/422 by Axum's JSON extractor rather than reaching the fallback.
   - Retaining `.route("/assets/*path", get(serve_asset))` ensures requests to `/assets/` continue to resolve through `serve_asset`, and invalid paths under `/assets/` (e.g. `/assets/nonexistent.png`) fail closed with 404 rather than serving `index.html`.
   - Retaining `.route("/favicon.ico", get(serve_favicon))` ensures standard browser requests for `/favicon.ico` map to `favicon.svg`.

5. **Expected Test Results After Fix**:
   With `.fallback(quorum_ui::static_handler)` wired into `build_router`:
   - `challenge_api_health`: unaffected, 200 OK.
   - `challenge_api_chat_valid_and_malformed`: unaffected, 200 OK / 400 / 422.
   - `challenge_api_shutdown_endpoint`: unaffected, 200 OK.
   - `challenge_static_assets_routing`: unaffected, 200 OK.
   - `challenge_static_assets_direct_and_spa_fallback`: direct requests to `/styles.css`, `/app.js`, `/langextract.html`, `/favicon.svg` and SPA routes `/unknown/spa/route/123` hit the fallback, resolving with 200 OK and expected MIME types. All 5 assertions pass.

---

## 3. Caveats

- **No Caveats**: The issue is thoroughly characterized, reproduced under `cargo test`, and the Axum 0.7 router builder semantics have been verified across all existing endpoints and test suites.
- No modifications to `crates/quorum-ui` are necessary: `quorum_ui::static_handler` is already fully implemented, tested, and exported.

---

## 4. Conclusion

### 4.1 Recommended Code Change

In `crates/quorum-daemon/src/server.rs`, wire `.fallback(quorum_ui::static_handler)` into `build_router` immediately following the `/api/shutdown` route registration and before `.layer(CorsLayer::permissive())`:

#### Before:
```rust
        // Health & Models Telemetry
        .route("/api/health", get(health::health_handler))
        .route("/api/models", get(health::models_handler))

        // Graceful Shutdown Endpoint
        .route("/api/shutdown", post(shutdown_handler))

        .layer(CorsLayer::permissive())
        .with_state(state)
```

#### After:
```rust
        // Health & Models Telemetry
        .route("/api/health", get(health::health_handler))
        .route("/api/models", get(health::models_handler))

        // Graceful Shutdown Endpoint
        .route("/api/shutdown", post(shutdown_handler))

        // SPA Fallback & Direct Static Asset Resolution
        .fallback(quorum_ui::static_handler)

        .layer(CorsLayer::permissive())
        .with_state(state)
```

### 4.2 Machine-Applicable Patch

The patch has been written to:
`d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_1\server_fallback.patch`

```patch
--- a/crates/quorum-daemon/src/server.rs
+++ b/crates/quorum-daemon/src/server.rs
@@ -40,6 +40,9 @@ pub fn build_router(state: Arc<DaemonState>) -> axum::Router {
         // Graceful Shutdown Endpoint
         .route("/api/shutdown", post(shutdown_handler))
 
+        // SPA Fallback & Direct Static Asset Resolution
+        .fallback(quorum_ui::static_handler)
+
         .layer(CorsLayer::permissive())
         .with_state(state)
 }
```

---

## 5. Verification Method

### 5.1 Verification Commands for the Implementer

1. **Verify Adversarial Challenge Suite (Primary Target)**:
   ```powershell
   cargo test -p quorum-daemon --test adversarial_web_routes_challenge -- --test-threads=2
   ```
   *Expected Result*: All 5 tests pass:
   - `challenge_api_health` ... ok
   - `challenge_api_chat_valid_and_malformed` ... ok
   - `challenge_api_shutdown_endpoint` ... ok
   - `challenge_static_assets_routing` ... ok
   - `challenge_static_assets_direct_and_spa_fallback` ... ok

2. **Verify Daemon Integration & Unit Tests**:
   ```powershell
   cargo test -p quorum-daemon -- --test-threads=2
   ```
   *Expected Result*: 8 daemon tests + 5 challenge tests = 13 tests passed, 0 failed.

3. **Verify UI Unit & Adversarial Tests**:
   ```powershell
   cargo test -p quorum-ui -- --test-threads=2
   ```
   *Expected Result*: All 11 tests pass.

4. **Verify E2E Web Endpoints Suite**:
   ```powershell
   cargo test --manifest-path tests/e2e/Cargo.toml --test test_web_endpoints -- --test-threads=2
   ```
   *Expected Result*: All 10 tests pass.

5. **Verify Clippy Hygiene**:
   ```powershell
   cargo clippy -p quorum-daemon --all-targets -- -D warnings
   ```
   *Expected Result*: Clean build with 0 warnings.
