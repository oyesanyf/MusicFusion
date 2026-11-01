# Handoff Report: `crates/quorum-ui` Web UI & LangExtract Visualizer Architecture

## 1. Observation

### 1.1 UI Templates in `d:\harfile\ModelFusion`
Direct inspection of `D:\harfile\ModelFusion\docs\screens\hugos_chatgpt_perfect_ui.html` and `D:\harfile\ModelFusion\docs\screens\hugos_chatgpt_browser_ui.html` reveals:
- **Layout Structure**:
  - **Sidebar**: Fixed 260px width (`w-64 min-w-[260px]`, `hugos_chatgpt_perfect_ui.html` line 90; `test_web_endpoints.rs` line 146). Features New Chat button, navigation list (Images, Plugins, Deep Research, Reindex / Drift Status, Settings), and a bottom Local Privacy badge (`100% Local & Private`, line 140).
  - **Reading Column**: Centered 768px maximum reading width (`max-w-3xl mx-auto`, `hugos_chatgpt_perfect_ui.html` line 212; `test_web_endpoints.rs` line 150).
  - **Header Bar**: Top 56px (`h-14`) navigation bar with Model Selector dropdown, engine status indicator (`animate-pulse`), and 5-Theme quick switcher.
  - **Floating Input Capsule**: Bottom-pinned pill capsule (`max-w-3xl`) over a gradient backdrop (`bg-gradient-to-t from-[var(--bg-canvas)] via-[var(--bg-canvas)] to-transparent`, line 267), featuring file attachment (`+` / `📎`), auto-resizing textarea (`max-h-40`), search mode toggles, and circular send button (`↑` or `➔`).
- **5-Theme CSS Custom Property System**:
  - `theme-white` (Default Light): `--bg-canvas: #ffffff; --bg-sidebar: #f9f9f9; --bg-capsule: #ffffff; --border-color: #ececec; --text-primary: #0d0d0d; --user-msg-bg: #f4f4f4;`
  - `theme-dark` (ChatGPT Slate Dark): `--bg-canvas: #212121; --bg-sidebar: #171717; --bg-capsule: #2f2f2f; --border-color: #303030; --text-primary: #ececec; --user-msg-bg: #2f2f2f;`
  - `theme-obsidian` (OLED Pure Black): `--bg-canvas: #000000; --bg-sidebar: #0a0a0a; --bg-capsule: #141414; --border-color: #1f1f1f; --text-primary: #ffffff; --user-msg-bg: #1c1c1e;`
  - `theme-midnight` (Midnight Navy): `--bg-canvas: #0b0f19; --bg-sidebar: #080b12; --bg-capsule: #111827; --border-color: #1f2937; --text-primary: #f8fafc; --user-msg-bg: #1e293b;`
  - `theme-warm` (Warm Sepia): `--bg-canvas: #faf8f5; --bg-sidebar: #f4efe6; --bg-capsule: #ffffff; --border-color: #ded6c6; --text-primary: #2d2a26; --user-msg-bg: #f4efe6;`
- **Streaming SSE Cursor & Real-Time Telemetry**:
  - Token cursor: `<span class="inline-block w-2 h-4 bg-current ml-0.5 animate-pulse">▍</span>`
  - Pulsing status indicator during reasoning (`Thinking...`, line 773)
  - Inline source citations: `<span class="citation-badge" data-citation="..."><span class="badge-idx">[1]</span></span>` (`hugos_chatgpt_browser_ui.html` lines 134-152, 785)

### 1.2 Interactive Google LangExtract Visualizer Template
Direct inspection of `d:\harfile\OshoosiClaw\target\langextract_audit_2603.02277.html` reveals:
- **Highlights Legend**: Color-coded badges for extraction classes (`breakout_vector` in `#D2E3FC`, `defense_countermeasure` in `#C8E6C9`, `mathematical_formulation` in `#FFE082`, `code_ast` in `#E1BEE7`).
- **Grounded Spans in Source Text**: `<span class="lx-highlight" data-idx="0" style="background-color:#D2E3FC;">text<span class="lx-tooltip">class: ... attributes: ...</span></span>`.
- **Playback Controls**: Step controller (`▶️ Play`, `⏮ Previous`, `⏭ Next`), slider `<input type="range" class="lx-progress-slider">`, and status counter `Entity X/N | Pos [startPos-endPos]`.
- **Attributes Inspector Panel**: Key-value property inspector dynamically rendering grounded attributes (`class`, `char_interval`, `attributes`, `source_chunk_id`, `source_file_path`).

### 1.3 Existing QuorumRAG Crate Implementations
- **`crates/quorum-coordinator`**:
  - `QueryCoordinator::query(&self, prompt: &str, top_k: usize) -> Result<QueryResult, CoordinatorError>` (`coordinator.rs:28-55`)
  - `QueryCoordinator::query_agentic(&self, prompt: &str, family: SynthesisModelFamily, top_k: usize, telemetry_tx: Option<Sender<AgenticTelemetryEvent>>) -> Result<QueryResult, CoordinatorError>` (`coordinator.rs:58-96`)
  - `AgenticTelemetryEvent`: 11 variants (`StateTransition`, `IntentClassified`, `HardwareProbed`, `RetrievalCompleted`, `RelevanceGraded`, `QueryRewritten`, `SynthesisStarted`, `TokenDelta`, `CitationEmitted`, `ExecutionFinished`, `ExecutionError`) with `to_sse_line(&self) -> String` producing standard `data: <json>\n\n` lines (`telemetry.rs:1-108`).
  - `SynthesisModelFamily`: `Glm4Agentic`, `Qwen25Code`, `GemmaProse`, `AutoFusion` (`types.rs:46-52`).
  - `langextract`: `Extraction`, `ground_extraction`, `filter_grounded`, `ConsensusExtractionResult`, `LlmFusionConsensus` with 4 evaluation arms (`langextract/mod.rs:1-50`, `consensus.rs:1-200`).
- **`crates/quorum-ingest`**:
  - `ReindexWorker`: Coordinates traversal, two-level BLAKE3 deduplication, and LanceDB/Tantivy indexing; tracks live `ReindexProgress` (`task_id`, `phase`, `files_scanned`, `files_indexed`, `chunks_reused_from_cache`, `chunks_newly_embedded`, `bytes_processed`, `elapsed_secs`) with cooperative cancellation via `cancel_flag` (`reindex.rs:41-88`).
  - `DriftHealer`: Detects and heals all 5 drifts (Merkle state drift, LanceDB <-> Tantivy desync, Vector contract drift, ACL policy drift, Storage fragmentation); returns `DriftReport` (`drift_healer.rs:35-66`).
- **`crates/quorum-storage`**:
  - `SearchMode`: `Keyword` (Tantivy BM25), `Vector` (LanceDB IVF_PQ), `Hybrid` (Reciprocal Rank Fusion) (`engine.rs:20-27`).
- **`tests/e2e/test_web_endpoints.rs`**:
  - Test harness asserts 260px sidebar, 5 themes, 768px reading column, `/api/health`, `/api/chat/stream`, Ask Anything router, and `--no-browser` CLI support (`test_web_endpoints.rs:137-244`).

---

## 2. Logic Chain

1. **Standalone Binary Invariant**:
   `PROJECT.md` specifies a self-contained single `.exe` daemon (`quorumrag.exe`). Therefore, all Web UI assets (HTML, CSS, JavaScript, SVG icons) must be compiled directly into the binary at build time. Using `rust-embed` with `mime_guess` satisfies this requirement cleanly: it bundles `assets/` into binary byte slices, resolves MIME types automatically (`text/html`, `text/css`, `application/javascript`, `image/svg+xml`), provides automated ETag support, and allows zero-touch fallback to `index.html` for client-side routing.

2. **Search Mode Toggle Integration**:
   The storage layer (`quorum-storage`) already exposes `SearchMode::Keyword`, `SearchMode::Vector`, and `SearchMode::Hybrid`. In the Web UI, an interactive 3-segment pill switcher (`[ BM25 | Vector | ★ Hybrid RRF ]`) allows users to explicitly toggle search strategy. This value is transmitted to `POST /api/chat/stream` and `GET /api/search` via query parameter or JSON body, binding directly to `AgenticGraphState.search_mode`.

3. **Multi-Model Selector Pill Bar**:
   `quorum-coordinator` already implements `SynthesisModelFamily` (`AutoFusion`, `Glm4Agentic`, `Qwen25Code`, `GemmaProse`) and `AutoFusionRouter`. A dedicated pill bar in the top navigation header renders:
   - `[ ✨ Auto-Fusion ]` (Default: dynamically selects model based on RAM/VRAM and Magika file categories)
   - `[ 🧠 GLM-4-9B ]` (Multi-hop agentic synthesis and 128K context reasoning)
   - `[ 💻 Qwen 2.5 ]` (Dynamic memory-sized coding specialist)
   - `[ 📜 Gemma 3 ]` (Fluent natural English prose)
   When `Auto-Fusion` is selected, `HardwareTelemetry` and `IntentClassified` events stream the resolved model and rationale back to the UI in SSE headers.

4. **Interactive Google LangExtract Visualizer**:
   `quorum-coordinator::langextract` implements the 4-arm `LlmFusionConsensus` and verbatim `ground_extraction` with 100% `char_interval` verification. Exposing two endpoints:
   - `POST /api/langextract`: Returns structured JSON `ConsensusExtractionResult`.
   - `POST /api/langextract/visualize`: Returns the interactive HTML visualization (derived from `langextract_audit_2603.02277.html`) with interactive span highlights, tooltips, animation stepper (`Play`, `Prev`, `Next`), slider, and attributes inspector.
   In the Web UI, an `@extract` slash command in the chat box or an "Inspect Grounding" button on citation cards triggers this visualization.

5. **Reindex and Drift Remediation UI Subsystems**:
   Both `ReindexWorker` and `DriftHealer` are implemented in `quorum-ingest`. The UI can expose their states via:
   - Live sidebar indicators: OS File Watcher status badge (`Active (300ms debounce)`), Storage Health (`100% parity`).
   - Trigger buttons: "Reindex Files" (`POST /api/reindex` with incremental vs full options) and "Fix Drifts" (`POST /api/drift/fix`).
   - Modals: Animated progress modal for reindexing (with cancel support) and summary modal for drift remediation reporting healed anomalies.

---

## 3. Caveats

1. **Axum Version Alignment**:
   `PROJECT.md` specifies Axum 0.7 (`axum = "0.7"`). Axum 0.7 uses `http-body-util` and `tokio-stream` for SSE streaming rather than legacy `async-stream` macros. Handlers must construct `Sse<impl Stream<Item = Result<Event, Infallible>>>`.
2. **Offline Local Model Execution**:
   If an external Ollama or GLM inference endpoint is offline or unavailable during tests/development, `quorum-coordinator`'s `DeterministicOfflineSynthesisEngine` is used as the deterministic fallback engine, ensuring zero crashes and 100% reproducible test runs.
3. **Tailwind & Asset Bundling**:
   To maintain 100% offline self-containment without external CDN downloads when the internet is disconnected, the embedded `assets/vendor/` folder must include `tailwindcss.min.js` and `marked.min.js` (or pure CSS utility classes matching the 5 themes).

---

## 4. Conclusion & Architecture Design

### 4.1 Crate Structure (`crates/quorum-ui`)
```
crates/quorum-ui/
├── Cargo.toml
├── assets/
│   ├── index.html              # Pixel-perfect ChatGPT layout (260px sidebar, 5 themes, 768px column)
│   ├── styles.css              # 5-theme CSS variables, custom scrollbars, citation card styles
│   ├── app.js                  # SSE stream parser, model selector, search mode toggle, citation modals
│   ├── langextract.html        # Interactive Google LangExtract visualizer
│   └── favicon.svg             # SVG mesh globe icon
├── src/
│   ├── lib.rs                  # Public exports: UiServer, UiConfig, build_router
│   ├── config.rs               # Server configuration (host, port, static directory, cors)
│   ├── server.rs               # Axum 0.7 server bootstrap & graceful shutdown handler
│   ├── router.rs               # Route builder registering all API endpoints and asset handlers
│   ├── assets.rs               # rust-embed static asset handler with mime_guess & SPA fallback
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── health.rs           # GET /api/health
│   │   ├── chat.rs             # POST /api/chat/stream (SSE streaming with telemetry & citations)
│   │   ├── search.rs           # GET /api/search & POST /api/search (SearchMode toggle)
│   │   ├── langextract.rs      # POST /api/langextract & POST /api/langextract/visualize
│   │   ├── reindex.rs          # POST /api/reindex, GET /api/reindex/status, POST /api/reindex/cancel
│   │   ├── drift.rs            # POST /api/drift/fix, GET /api/drift/status
│   │   └── models.rs           # GET /api/models (lists GLM-4-9B, Qwen 2.5, Gemma 3, hardware stats)
│   └── error.rs                # WebError and Axum IntoResponse conversions
└── tests/
    └── ui_tests.rs             # Unit and integration test suite
```

### 4.2 Cargo Manifest (`crates/quorum-ui/Cargo.toml`)
```toml
[package]
name = "quorum-ui"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "Axum 0.7 web server, embedded ChatGPT UI, SSE streaming, search toggle, LangExtract visualizer, and reindex/drift controls"

[dependencies]
quorum-types = { workspace = true }
quorum-contract = { workspace = true }
quorum-storage = { workspace = true }
quorum-classifier = { workspace = true }
quorum-ingest = { workspace = true }
quorum-coordinator = { workspace = true }

axum = { version = "0.7", features = ["macros"] }
tower = { version = "0.4", features = ["util"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
rust-embed = { version = "8.4", features = ["mime-guess"] }
mime_guess = "2.0"

tokio = { workspace = true, features = ["full"] }
tokio-stream = { version = "0.1", features = ["sync"] }
futures = { workspace = true }

serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }

thiserror = { workspace = true }
tracing = { workspace = true }
sysinfo = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

### 4.3 Static Asset Bundling Implementation (`src/assets.rs`)
```rust
use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct WebAssets;

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let target_path = if path.is_empty() { "index.html" } else { path };

    match WebAssets::get(target_path) {
        Some(content) => {
            let mime = mime_guess::from_path(target_path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap())
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(Body::from(content.data))
                .unwrap()
        }
        None => {
            // SPA fallback: return index.html for non-file routes
            match WebAssets::get("index.html") {
                Some(content) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(content.data))
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("404 Not Found"))
                    .unwrap(),
            }
        }
    }
}
```

### 4.4 API Routes Specification
| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/` and `/assets/*` | Embedded ChatGPT Web UI and static assets via `rust-embed` |
| `GET` | `/api/health` | System health, indexed chunk count, contract hash, active peers |
| `POST` | `/api/chat/stream` | Server-Sent Events (SSE) streaming with live agentic telemetry, tokens, and citations |
| `GET` / `POST` | `/api/search` | Direct search supporting BM25 Keyword, Semantic Vector, or Hybrid RRF |
| `GET` | `/api/models` | Available synthesis models (Auto-Fusion, GLM-4-9B, Qwen 2.5, Gemma 3) and hardware telemetry |
| `POST` | `/api/langextract` | Executes 4-Arm `LlmFusionConsensus` and returns grounded JSON `ConsensusExtractionResult` |
| `POST` | `/api/langextract/visualize` | Generates standalone interactive HTML visualizer with entity steppers |
| `POST` | `/api/reindex` | Triggers background reindexing (`full: bool`, `path: Option<String>`) |
| `GET` | `/api/reindex/status` | Returns current `ReindexProgress` telemetry |
| `POST` | `/api/reindex/cancel` | Cooperatively cancels active reindexing task |
| `POST` | `/api/drift/fix` | Executes 5-drift remediation worker and returns `DriftReport` |
| `GET` | `/api/drift/status` | Returns latest `DriftReport` |

### 4.5 Chat SSE Streaming Protocol (`/api/chat/stream`)
Incoming request body:
```json
{
  "prompt": "Explain the scatter-gather retrieval architecture",
  "search_mode": "hybrid",
  "model_family": "auto_fusion",
  "top_k": 5
}
```
Outbound SSE stream conforms to `AgenticTelemetryEvent`:
1. `data: {"event":"hardware_probed","data":{"free_ram_gb":28.4,"free_vram_mb":12288,"gpu_name":"NVIDIA RTX 4090","selected_tier":"Tier 1: GLM-4-9B"}}`
2. `data: {"event":"intent_classified","data":{"intent":"local_document_search","search_mode":"Hybrid","model_family":"glm4_agentic","model_name":"glm-4-9b-chat","routing_rationale":"Architectural local query","is_retrieval_required":true}}`
3. `data: {"event":"state_transition","data":{"from_state":"classify_intent","to_state":"retrieve_mesh","hop":1,"timestamp_ms":1761955200}}`
4. `data: {"event":"retrieval_completed","data":{"hop":1,"query":"scatter-gather architecture","search_mode":"Hybrid","retrieved_count":4,"local_count":3,"remote_count":1,"latency_ms":38}}`
5. `data: {"event":"relevance_graded","data":{"hop":1,"grade":{"is_relevant":true,"score":0.95,"reasoning":"Direct match","satisfactory":true},"score":0.95,"reasoning":"Direct match","will_rewrite":false}}`
6. `data: {"event":"synthesis_started","data":{"model_name":"glm-4-9b-chat","model_family":"glm4_agentic","context_chunks_count":4,"estimated_prompt_tokens":1240}}`
7. `data: {"event":"token_delta","data":{"delta":"QuorumRAG implements a "}}`
8. `data: {"event":"token_delta","data":{"delta":"400ms scatter-gather federated engine..."}}`
9. `data: {"event":"citation","data":{"citation":{"peer_id":"local","file_path":"crates/quorum-coordinator/src/scatter_gather.rs","line_start":25,"line_end":50,"score":0.98,"snippet":"pub async fn execute_query(...) -> (Vec<ScoredChunk>, ...)"}}}`
10. `data: {"event":"execution_finished","data":{"total_hops":1,"total_duration_ms":142,"total_tokens":85,"final_state":"complete","model_name":"glm-4-9b-chat"}}`

### 4.6 Interactive LangExtract Visualizer Architecture
- **HTML/CSS Design**:
  Embedded in `crates/quorum-ui/assets/langextract.html` and served via `/api/langextract/visualize`.
  - Color palette:
    - `breakout_vector` / `security_vector`: `#D2E3FC` (Soft Blue)
    - `defense_countermeasure`: `#C8E6C9` (Soft Mint)
    - `mathematical_formulation`: `#FFE082` (Warm Amber)
    - `code_ast` / `entity`: `#E1BEE7` (Soft Lavender)
  - Layout:
    - Top legend displaying active entity categories.
    - Centered monospace text window (`font-family: monospace; white-space: pre-wrap; line-height: 1.6`) with highlighted `<span class="lx-highlight" data-idx="...">` elements and hoverable tooltips.
    - Playback toolbar with Play/Pause button, Previous/Next buttons, interactive scrubber slider, and `Entity X/N | Pos [startPos-endPos]` indicator.
    - Attributes drawer displaying extracted JSON key-value attributes and 4-Arm Consensus confidence scores.
- **Web UI Chat Integration**:
  - `@extract <text or file>` chat command directly routes to `/api/langextract` and renders the visualizer inline or in an interactive modal.
  - Every assistant citation badge includes an "Inspect Grounding" button that opens `/api/langextract/visualize` scoped to that citation chunk.

### 4.7 Reindex & Drift Remediation UI Controls
- **Sidebar Indicators**:
  - `Watcher Status`: Live indicator showing OS filesystem watcher state (Green pulse for Active, Grey for Idle, with watched file count).
  - `Parity Score`: Vector <-> Tantivy BM25 alignment percentage (100% when synchronized).
- **Interactive Modals**:
  - **Reindex Modal**:
    - Mode selection: `Incremental (mtime + BLAKE3 cache)` vs `Full Rebuild (--full)`.
    - Live progress: Progress bar, current phase (`ScanningFilesystem` -> `ChunkingAndVectorizing` -> `WritingToStorage` -> `CompactingIndexes`), scanned file count, chunks reused from cache, and cooperative cancel button (`POST /api/reindex/cancel`).
  - **Fix Drifts Modal**:
    - Trigger button: "Run Drift Remediation" (`POST /api/drift/fix`).
    - Anomaly summary card displaying healed items across all 5 drift dimensions (Merkle files purged/indexed, hybrid index desyncs healed, contract drifted chunks re-embedded, ACL access tiers updated, and storage fragments compacted).

---

## 5. Verification Method

### 5.1 Independent Verification Commands
Once `crates/quorum-ui` is scaffolded and added to workspace `Cargo.toml`:
1. **Compilation Check**:
   ```powershell
   cargo check --package quorum-ui
   ```
2. **Unit & Route Tests**:
   ```powershell
   cargo test --package quorum-ui
   ```
3. **E2E Web Endpoints Verification Suite**:
   ```powershell
   cargo test --manifest-path tests/e2e/Cargo.toml --test test_web_endpoints
   ```
4. **Live Visual Inspection in Browser**:
   Run daemon with Web UI:
   ```powershell
   cargo run -p quorum-daemon
   ```
   Verify that:
   - Web UI automatically launches at `http://127.0.0.1:8080/` (or port specified in config).
   - Sidebar renders at 260px width with New Chat, Reindex, and Drift controls.
   - 5 themes switch dynamically via the top right dropdown without page reload.
   - Search mode toggle switches cleanly between `BM25`, `Vector`, and `Hybrid RRF`.
   - Model selector pill bar highlights active model (`Auto-Fusion`, `GLM-4-9B`, `Qwen 2.5`, `Gemma 3`).
   - Querying produces SSE token stream with animated typing cursor and structured citation cards.
   - Triggering `@extract` renders the interactive Google LangExtract visualizer with verbatim character-span highlights.
   - Clicking "Reindex Files" and "Fix Drifts" initiates background tasks and displays live progress modals.

### 5.2 Invalidation Criteria
- If UI assets are loaded from external unbundled filesystem paths at runtime rather than compiled inside the binary via `rust-embed` or `include_str!`.
- If search mode selection fails to route to Tantivy BM25 (`Keyword`), LanceDB (`Vector`), or Reciprocal Rank Fusion (`Hybrid`).
- If model selection does not map to `SynthesisModelFamily` or fails to stream routing telemetry in SSE headers.
- If `/api/langextract` returns ungrounded hallucinated strings where `char_interval` is missing.
- If reindexing blocks the Axum main loop instead of running asynchronously in a background tokio task.
