# Feature 52: Active Chat UI Stop Button & In-Flight Cancellation Handoff Report

## 1. Observation

### 1.1 Frontend Elements and Current Wiring
- **`crates/quorum-ui/assets/index.html` (lines 185–196)**:
  ```html
  <form id="chat-form" class="input-capsule">
    <div class="input-row">
      <textarea id="prompt-input" class="prompt-textarea" rows="1" spellcheck="true" placeholder="Ask anything about your local documents..."></textarea>
      <button type="button" id="btn-spellcheck" class="btn-spellcheck" title="Correct spelling in chat box using local LLM" style="...">
        <span>✨</span><span>Spellcheck</span>
      </button>
      <button type="button" id="btn-stop" class="btn-stop hidden" title="Stop generating response" style="background:var(--danger, #ef4444); color:#fff; border:none; border-radius:8px; padding:6px 12px; font-weight:600; cursor:pointer; display:inline-flex; align-items:center; gap:5px;">
        <span>■</span><span>Stop</span>
      </button>
      <button type="submit" id="btn-send" class="btn-send" title="Send query">
        <span class="send-arrow">➔</span>
      </button>
    </div>
  ```
  Both `#btn-stop` and `#btn-send` exist in the DOM inside `.input-row`. `#btn-stop` has the class `btn-stop hidden`.

- **`crates/quorum-ui/assets/styles.css` (lines 623–642, line 797)**:
  ```css
  .btn-send {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background-color: var(--btn-send-bg);
    color: var(--btn-send-color);
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-weight: bold;
    transition: opacity 0.15s ease;
  }
  ...
  .hidden {
    display: none !important;
  }
  ```
  `.hidden` uses `display: none !important;`. However, `.btn-stop` is not styled in `styles.css` (it currently relies on inline styles on the HTML element).

- **`crates/quorum-ui/assets/app.js` (lines 213–223, 250–256, 374–380)**:
  ```javascript
  const btnStop = document.getElementById('btn-stop');
  let activeAbortController = null;

  if (btnStop) {
    btnStop.addEventListener('click', () => {
      if (activeAbortController) {
        activeAbortController.abort();
        activeAbortController = null;
      }
    });
  }
  ```
  In `submitPrompt()` (lines 250–256):
  ```javascript
  isGenerating = true;
  btnSend.disabled = true;
  btnSend.classList.add('hidden');
  if (btnStop) {
    btnStop.classList.remove('hidden');
  }
  activeAbortController = new AbortController();
  ```
  In `finally` (lines 374–380):
  ```javascript
  activeAbortController = null;
  isGenerating = false;
  btnSend.disabled = false;
  btnSend.classList.remove('hidden');
  if (btnStop) {
    btnStop.classList.add('hidden');
  }
  ```
  *Deficiency observed*:
  1. `submitPrompt()` does not generate or pass any `query_id` to the backend.
  2. `btnStop.addEventListener('click', ...)` calls `activeAbortController.abort()` but **never issues `POST /api/chat/cancel`**.
  3. If fetch fails due to `AbortError`, the retry loop (lines 277–307) can keep trying instead of breaking immediately on user abort.

---

### 1.2 Backend Routes and Handlers
- **`crates/quorum-daemon/src/server.rs` (lines 20–23)**:
  ```rust
  // Chat & Query Endpoints (SSE streaming & JSON)
  .route("/api/chat", post(chat::chat_sse_handler))
  .route("/api/chat/stream", get(chat_stream_compat_handler))
  .route("/api/chat/spellcheck", post(chat::spellcheck_handler))
  .route("/api/query", post(chat::query_handler))
  ```
  *Deficiency observed*: Route `.route("/api/chat/cancel", post(chat::cancel_chat_handler))` is **not registered**.

- **`crates/quorum-daemon/src/handlers/chat.rs` (lines 17–37, 41–106)**:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct ChatRequest {
      pub prompt: String,
      pub search_mode: Option<SearchMode>,
      pub model_family: Option<String>,
      pub top_k: Option<usize>,
      pub access_tier: Option<u8>,
      #[serde(default)]
      pub history: Vec<ChatTurnMessage>,
  }
  ```
  `ChatRequest` has no `query_id` field.
  In `chat_sse_handler`:
  ```rust
  let (tx, rx) = tokio::sync::mpsc::channel::<AgenticTelemetryEvent>(256);
  let coordinator = Arc::clone(&state.coordinator);

  tokio::spawn(async move {
      ...
      match coordinator
          .query_agentic_with_history(&prompt, history, family, top_k, Some(tx.clone()))
          .await
      { ... }
  });

  let stream = ReceiverStream::new(rx).map(|event| {
      let json_line = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
      Ok::<Event, Infallible>(Event::default().data(json_line))
  });

  Sse::new(stream).keep_alive(KeepAlive::default().interval(Duration::from_secs(15)))
  ```
  *Deficiency observed*:
  1. The spawned task's `JoinHandle` is dropped and not tracked in `DaemonState`.
  2. No cancellation token or registry exists to cancel in-flight queries by ID.
  3. No `cancel_chat_handler` function exists in `chat.rs`.

- **`crates/quorum-daemon/src/state.rs` (lines 13–30)**:
  `DaemonState` contains `start_time`, `storage`, `coordinator`, `reindex_worker`, `drift_healer`, `classifier`, `embedder`, `dedup`, `swarm_handle`, `ingest_config`, `default_model_family`, `port`, `host`, `data_dir`, and `shutdown_notify`.
  *Deficiency observed*: It lacks an `in_flight_queries` registry.

---

### 1.3 Coordinator State Graph & Streaming Loops
- **`crates/quorum-coordinator/src/agentic/mod.rs` (lines 115–125, lines 153–164, lines 340–355)**:
  The state graph execution loop:
  ```rust
  loop {
      match state_name {
          AgenticStateName::ClassifyIntent => { ... }
          AgenticStateName::RetrieveMesh => { ... }
          AgenticStateName::GradeRelevance => { ... }
          AgenticStateName::RewriteQuery => { ... }
          AgenticStateName::SynthesizeWithCitations => { ... }
      }
  }
  ```
  *Deficiency observed*:
  1. The top of the `loop` does not check `if tx.is_closed()`. If the client disconnects or clicks Stop, the coordinator continues running all remaining hops (RetrieveMesh, GradeRelevance, Synthesize).
  2. Cache hit streaming (line 157): sends `TokenDelta` with the entire cached string at once, rather than progressive word-streaming with `tx.is_closed()` break.

- **`crates/quorum-coordinator/src/offline_engine.rs` (lines 501–507, lines 734–738)**:
  In `synthesize_with_history` (lines 501–507):
  ```rust
  let synthesized_rag = sections.join("\n\n");
  if let Some(tx) = telemetry_tx {
      let _ = tx.try_send(AgenticTelemetryEvent::TokenDelta {
          delta: synthesized_rag.clone(),
      });
  }
  (synthesized_rag, citations)
  ```
  *Deficiency observed*: `synthesized_rag` is sent as a single monolithic `TokenDelta` block. There is no word-by-word streaming loop checking `if tx.is_closed() { break; }`.
  *Contrast*: In `query_local_ollama_chat` (lines 735–738), the code already has:
  ```rust
  // Immediately abort if the user clicked the Stop button on the chat box
  if tx.is_closed() {
      break;
  }
  ```

---

## 2. Logic Chain

1. **User Action & Browser Signal**:
   When a user clicks `#btn-stop` while a query is running, the frontend must immediately:
   - Call `activeAbortController.abort()` to close the active HTTP fetch request.
   - Send `POST /api/chat/cancel` with `{ "query_id": activeQueryId }` to ensure backend processes that may not immediately detect socket disconnect are explicitly aborted.
   - Transition the UI state (hide `#btn-stop`, show enabled `#btn-send`, stop thinking indicator, append `*(Generation stopped by user)*`).

2. **Backend Dual-Channel Cancellation**:
   Cancellation must be fail-safe and operate through two complementary channels:
   - **Channel 1 (HTTP Connection Drop & `tx.is_closed()`)**:
     When `activeAbortController.abort()` runs in the browser, the TCP connection is severed. Axum drops the `ReceiverStream(rx)` in `chat_sse_handler`. When `rx` is dropped, Tokio's `mpsc::Sender::is_closed()` immediately returns `true`.
     Any loop in the backend checking `tx.is_closed()` will immediately detect that the client has gone away and terminate.
   - **Channel 2 (Explicit `POST /api/chat/cancel` & `AbortHandle`)**:
     The browser concurrently fires `POST /api/chat/cancel` with the `query_id`.
     The daemon looks up the query in an `InFlightQueryRegistry`.
     Calling `abort_handle.abort()` immediately terminates the spawned Tokio task.
     Calling `token.cancel()` sets an atomic cancellation flag.
   - **Synergy**: If Channel 2 arrives first, Tokio halts the task immediately. If Channel 1 triggers first (e.g. before the second HTTP request reaches the server), `tx.is_closed()` halts the streaming loop and graph transitions immediately.

3. **Coordination Loop Termination**:
   - Adding `if let Some(ref tx) = telemetry_tx { if tx.is_closed() { return Ok(graph_state); } }` at the top of the state loop in `agentic/mod.rs` guarantees no hops (such as `RetrieveMesh` or `SynthesizeWithCitations`) are executed after cancellation.
   - Progressive word-streaming in `offline_engine.rs` (`for word in synthesized_rag.split_inclusive(' ')`) with `if tx.is_closed() { break; }` and a 6ms sleep guarantees that token emission is progressive, interactive, and stops instantaneously when the Stop button is clicked.

---

## 3. Caveats

1. **No External internet requests**: All endpoints and cancellation mechanisms operate 100% locally over loopback HTTP/SSE.
2. **Public API Invariants**: As required by `PROJECT.md`, the public API signature of `QueryCoordinator::query_agentic_with_history` (`prompt, history, family, top_k, telemetry_tx`) remains unchanged. Cancellation information is passed via `telemetry_tx` (`tx.is_closed()`) and Tokio task-level `AbortHandle`.
3. **Query ID Optionality**: If a client sends a request without a `query_id`, the backend generates a UUID. If `POST /api/chat/cancel` is called with `{}` (empty query ID), it cancels all active in-flight queries.

---

## 4. Conclusion & Concrete Implementation Plan

### Exact File Changes Required:

#### 1. `crates/quorum-ui/assets/styles.css`
- Add dedicated styling for `.btn-stop` around line 642 (next to `.btn-send`):
  ```css
  .btn-stop {
    background-color: var(--danger, #ef4444);
    color: #ffffff;
    border: none;
    border-radius: 8px;
    padding: 6px 12px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    transition: all 0.15s ease;
  }
  .btn-stop:hover {
    background-color: #dc2626;
    transform: translateY(-1px);
  }
  .btn-stop:active {
    transform: translateY(0);
  }
  ```

#### 2. `crates/quorum-ui/assets/app.js`
- **Line 18**: Add `let activeQueryId = null;`.
- **Lines 216–223**: Update `btnStop` listener:
  ```javascript
  if (btnStop) {
    btnStop.addEventListener('click', () => {
      if (activeAbortController) {
        activeAbortController.abort();
        activeAbortController = null;
      }
      if (activeQueryId) {
        fetch('/api/chat/cancel', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ query_id: activeQueryId })
        }).catch(() => {});
        activeQueryId = null;
      }
    });
  }
  ```
- **Lines 256–298**: In `submitPrompt()`:
  - Generate `activeQueryId = 'q-' + Date.now() + '-' + Math.random().toString(36).substring(2, 9);`
  - Pass `query_id: activeQueryId` in the JSON body to `/api/chat`.
  - In the retry loop (line 301): on `AbortError`, break immediately without retrying.
- **Lines 342–351**: In error handling:
  - When `err.name === 'AbortError'`, append `*(Generation stopped by user)*` to `bodyEl._rawMarkdown` or render `⏹ Generation stopped by user.`.
- **Lines 374–382**: In `finally`:
  - Reset `activeAbortController = null; activeQueryId = null; isGenerating = false;`
  - Show and enable `#btn-send`, hide `#btn-stop`.

#### 3. `crates/quorum-daemon/src/state.rs`
- Add `InFlightQueryRegistry` definition and include it in `DaemonState`:
  ```rust
  use tokio::task::AbortHandle;
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::RwLock;
  use std::collections::HashMap;

  #[derive(Clone, Default)]
  pub struct InFlightCancelToken(Arc<AtomicBool>);

  impl InFlightCancelToken {
      pub fn new() -> Self {
          Self(Arc::new(AtomicBool::new(false)))
      }
      pub fn cancel(&self) {
          self.0.store(true, Ordering::SeqCst);
      }
      pub fn is_cancelled(&self) -> bool {
          self.0.load(Ordering::SeqCst)
      }
  }

  #[derive(Clone)]
  pub struct InFlightEntry {
      pub token: InFlightCancelToken,
      pub abort_handle: Arc<std::sync::Mutex<Option<AbortHandle>>>,
  }

  impl InFlightEntry {
      pub fn set_abort_handle(&self, handle: AbortHandle) {
          if let Ok(mut lock) = self.abort_handle.lock() {
              *lock = Some(handle);
          }
      }
  }

  pub struct InFlightGuard {
      pub query_id: String,
      pub registry: Arc<RwLock<HashMap<String, InFlightEntry>>>,
  }

  impl Drop for InFlightGuard {
      fn drop(&mut self) {
          if let Ok(mut map) = self.registry.write() {
              map.remove(&self.query_id);
          }
      }
  }

  #[derive(Clone, Default)]
  pub struct InFlightQueryRegistry {
      queries: Arc<RwLock<HashMap<String, InFlightEntry>>>,
  }

  impl InFlightQueryRegistry {
      pub fn new() -> Self {
          Self {
              queries: Arc::new(RwLock::new(HashMap::new())),
          }
      }

      pub fn register(&self, query_id: String) -> (InFlightEntry, InFlightGuard) {
          let entry = InFlightEntry {
              token: InFlightCancelToken::new(),
              abort_handle: Arc::new(std::sync::Mutex::new(None)),
          };
          if let Ok(mut map) = self.queries.write() {
              map.insert(query_id.clone(), entry.clone());
          }
          let guard = InFlightGuard {
              query_id,
              registry: Arc::clone(&self.queries),
          };
          (entry, guard)
      }

      pub fn cancel(&self, query_id: &str) -> bool {
          if let Ok(map) = self.queries.read() {
              if let Some(entry) = map.get(query_id) {
                  entry.token.cancel();
                  if let Ok(mut lock) = entry.abort_handle.lock() {
                      if let Some(handle) = lock.take() {
                          handle.abort();
                      }
                  }
                  return true;
              }
          }
          false
      }

      pub fn cancel_all(&self) -> usize {
          let mut count = 0;
          if let Ok(map) = self.queries.read() {
              for entry in map.values() {
                  entry.token.cancel();
                  if let Ok(mut lock) = entry.abort_handle.lock() {
                      if let Some(handle) = lock.take() {
                          handle.abort();
                      }
                  }
                  count += 1;
              }
          }
          count
      }
  }
  ```
- In `DaemonState` struct (line 29):
  ```rust
  pub in_flight_queries: Arc<InFlightQueryRegistry>,
  ```

#### 4. `crates/quorum-daemon/src/handlers/chat.rs`
- In `ChatRequest` (line 37):
  ```rust
  pub query_id: Option<String>,
  ```
- Add `CancelChatRequest` and `cancel_chat_handler`:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct CancelChatRequest {
      pub query_id: Option<String>,
  }

  pub async fn cancel_chat_handler(
      State(state): State<Arc<DaemonState>>,
      Json(payload): Json<CancelChatRequest>,
  ) -> impl IntoResponse {
      let cancelled = if let Some(ref qid) = payload.query_id {
          state.in_flight_queries.cancel(qid)
      } else {
          state.in_flight_queries.cancel_all() > 0
      };

      info!(query_id = ?payload.query_id, cancelled, "Chat query cancellation requested via /api/chat/cancel");

      (
          axum::http::StatusCode::OK,
          Json(serde_json::json!({
              "status": "cancelled",
              "cancelled": cancelled,
              "query_id": payload.query_id
          }))
      )
  }
  ```
- In `chat_sse_handler`:
  ```rust
  let query_id = payload.query_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
  let (in_flight_entry, in_flight_guard) = state.in_flight_queries.register(query_id.clone());
  let task_token = in_flight_entry.token.clone();
  let task_tx = tx.clone();

  let join_handle = tokio::spawn(async move {
      let _guard = in_flight_guard;
      ...
      match coordinator
          .query_agentic_with_history(&prompt, history, family, top_k, Some(task_tx.clone()))
          .await
      {
          Ok(_result) => {}
          Err(e) => {
              if !task_token.is_cancelled() && !task_tx.is_closed() {
                  ...
              }
          }
      }
  });

  in_flight_entry.set_abort_handle(join_handle.abort_handle());
  ```

#### 5. `crates/quorum-daemon/src/server.rs`
- In `build_router` (line 24):
  ```rust
  .route("/api/chat/cancel", post(chat::cancel_chat_handler))
  ```

#### 6. `crates/quorum-coordinator/src/agentic/mod.rs`
- At the top of `loop` in `run_with_history` (line 115):
  ```rust
  if let Some(ref tx) = telemetry_tx {
      if tx.is_closed() {
          tracing::info!("Telemetry channel closed by client disconnect; terminating AgenticStateGraph");
          return Ok(graph_state);
      }
  }
  ```
- In cache-hit emission (lines 154–164):
  ```rust
  if let Some(ref resp_text) = graph_state.synthesized_response {
      for word in resp_text.split_inclusive(' ') {
          if tx.is_closed() {
              break;
          }
          let _ = tx.send(AgenticTelemetryEvent::TokenDelta { delta: word.to_string() }).await;
          tokio::time::sleep(tokio::time::Duration::from_millis(6)).await;
      }
  }
  ```
- Before `SynthesizeWithCitationsNode::execute` (line 350):
  ```rust
  if let Some(ref tx) = telemetry_tx {
      if tx.is_closed() {
          return Ok(graph_state);
      }
  }
  ```

#### 7. `crates/quorum-coordinator/src/offline_engine.rs`
- In `synthesize_with_history` (lines 502–506):
  ```rust
  let synthesized_rag = sections.join("\n\n");
  if let Some(tx) = telemetry_tx {
      for word in synthesized_rag.split_inclusive(' ') {
          if tx.is_closed() {
              tracing::debug!("Token streaming stopped: tx channel closed");
              break;
          }
          if tx.try_send(AgenticTelemetryEvent::TokenDelta {
              delta: word.to_string(),
          }).is_err() {
              break;
          }
          std::thread::sleep(std::time::Duration::from_millis(6));
      }
  }
  ```

#### 8. Updates to Existing Tests & `main.rs` Instantiating `DaemonState`
- Update `crates/quorum-daemon/src/main.rs:204` and tests:
  - `crates/quorum-daemon/tests/daemon_tests.rs:86`
  - `crates/quorum-daemon/tests/adversarial_endpoint_tests.rs:67`
  - `crates/quorum-daemon/tests/adversarial_recache_api_tests.rs:54`
  - `crates/quorum-daemon/tests/adversarial_web_routes_challenge.rs:71`
  Add:
  ```rust
  in_flight_queries: Arc::new(quorum_daemon::InFlightQueryRegistry::new()),
  ```

---

## 5. Verification Method

1. **Automated Unit & Integration Tests**:
   - Add test `test_chat_cancel_endpoint()` in `crates/quorum-daemon/tests/adversarial_endpoint_tests.rs`:
     - POST `/api/chat/cancel` with `{"query_id": "test-123"}` -> asserts HTTP 200 OK and `{"status": "cancelled", "cancelled": false, "query_id": "test-123"}`.
     - Start a streaming query via `/api/chat`, capture the first event, then immediately POST `/api/chat/cancel` -> asserts that query terminates and stream ends cleanly.
   - Run `cargo test -p quorum-daemon --test adversarial_endpoint_tests -- --test-threads=2`.
   - Run `cargo test -p quorum-ui --test ui_tests -- --test-threads=2` ensuring UI elements (`btn-stop`, `btn-send`, `styles.css`) pass all assertions.

2. **Browser Interactive Verification**:
   - Launch `cargo run --bin quorumrag`.
   - Submit a multi-document query in the browser (e.g. "what is universal mineral income").
   - Confirm `#btn-send` disappears and `#btn-stop` appears immediately with animated cycling phrases.
   - Click `#btn-stop` mid-generation:
     - Verify network tab shows `POST /api/chat/cancel` 200 OK.
     - Verify the SSE stream is aborted.
     - Verify `#btn-stop` disappears and `#btn-send` reappears enabled.
     - Verify the assistant message displays `*(Generation stopped by user)*` without page reload or broken state.

3. **Conditions That Invalidate This Assessment**:
   - If `tx.is_closed()` is not checked in the token streaming loop, queries will continue streaming on the daemon even after the browser disconnects.
   - If `InFlightQueryRegistry` is not shared across Axum handlers, `/api/chat/cancel` will not find active queries.
