# Task Assignment: Milestone 5 Implementation Worker (`worker_m5_1`)

## 1. Identity & Role
- **Role**: Milestone 5 Implementation Worker
- **Archetype**: `teamwork_worker`
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_1\`
- **Parent Conversation ID**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`

## 2. Mandatory Inputs & References (Read These First!)
1. `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`: Read the user request verbatim, especially Feature 42 (lines 213–225).
2. `d:\harfile\QuorumRAG\PROJECT.md`: Read architecture, Milestone 5 scope, features 31, 32, 34, 38, 39, 40, 41, 42, and code layout.
3. `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_1\handoff.md`: UI and LangExtract visualizer architecture report.
4. `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_2\handoff.md`: Daemon and single executable launcher architecture report.
5. `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_3\handoff.md`: Feature 42 Live IDE Execution & Production Code Generation Gate report.
6. Reference implementations in `.agents/teamwork/explorer_m5_2/`:
   - `proposed_Cargo.toml`, `proposed_main.rs`, `proposed_cli.rs`, `proposed_state.rs`, `proposed_server.rs`, `proposed_browser.rs`, `proposed_shutdown.rs`, `proposed_handlers_*.rs`.
7. UI templates in `d:\harfile\ModelFusion\docs\screens\`:
   - `hugos_chatgpt_perfect_ui.html` and `hugos_chatgpt_browser_ui.html`.
8. Interactive visualizer in `d:\harfile\OshoosiClaw\target\langextract_audit_2603.02277.html`.

## 3. MANDATORY INTEGRITY WARNING
> DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## 4. Work Scope & Deliverables

### Phase 1: Implement `crates/quorum-ui`
Create `crates/quorum-ui` containing:
1. `Cargo.toml`: Package `quorum-ui`, dependencies on `axum = "0.7"`, `rust-embed = { version = "8.4", features = ["mime-guess"] }`, `mime_guess = "2.0"`, `tokio`, `serde`, `serde_json`, `tower-http`.
2. `assets/`:
   - `index.html`: Pixel-perfect ChatGPT layout with 260px fixed sidebar (`w-64 min-w-[260px]`), 768px reading column (`max-w-3xl mx-auto`), model selector dropdown/pill bar (`Auto-Fusion`, `GLM-4-9B`, `Qwen 2.5`, `Gemma 3`), 5 themes (`theme-white`, `theme-dark`, `theme-obsidian`, `theme-midnight`, `theme-warm`), 3-way search toggle (`BM25 Keyword`, `Semantic Vector`, `★ Hybrid RRF`), floating bottom input capsule with auto-resizing textarea, New Chat button, Reindex and Fix Drifts modals and sidebar status indicators, streaming typing cursor (`▍`), and structured citation cards (`[Peer:File:Lines]`).
   - `styles.css`: Full 5-theme CSS variable declarations, custom scrollbars, typography, and citation badges.
   - `app.js`: Real-time SSE stream consumer (`/api/chat`), typewriter token renderer, model switcher, search mode toggle, reindex modal controller, drift modal controller, and citation inspector.
   - `langextract.html`: Interactive Google LangExtract visualizer (adapted from `langextract_audit_2603.02277.html`) with color-coded highlight spans (`#D2E3FC`, `#C8E6C9`, `#FFE082`, `#E1BEE7`), playback steppers (`Play`, `Prev`, `Next`), slider scrubber, and attributes inspector drawer.
   - `favicon.svg`: Minimal SVG globe icon.
3. `src/assets.rs`: Static asset embedding via `rust-embed` with `mime_guess` and SPA fallback to `index.html`.
4. `src/lib.rs`: Public exports for `WebAssets`, `static_handler`, and helper router builders.
5. `tests/ui_tests.rs`: Unit tests verifying asset presence, MIME type resolution, and index HTML content.

### Phase 2: Implement `crates/quorum-daemon`
Create `crates/quorum-daemon` containing:
1. `Cargo.toml`: Package `quorum-daemon`, `[[bin]] name = "quorumrag" path = "src/main.rs"`, dependencies on all workspace crates (`quorum-types`, `quorum-classifier`, `quorum-contract`, `quorum-storage`, `quorum-ingest`, `quorum-network`, `quorum-coordinator`, `quorum-ui`), `axum = "0.7"`, `clap = { version = "4.5", features = ["derive", "env"] }`, `open = "5.3"`, `tokio`, `tower-http`, `sysinfo`, `tracing`, `serde`, `serde_json`.
2. `src/main.rs`: Entry point parsing `Cli` with `clap`, handling subcommands `reindex` and `fix-drift`, or initializing `DaemonState` and launching `axum::serve` with browser auto-launch and graceful shutdown.
3. `src/cli.rs`: Command line parsing with `--port` (default 3030), `--host` (127.0.0.1), `--data-dir`, `--watch-dir`, `--no-browser` (mandatory flag for test/headless runs), `--model-family`, `--local-only`, and subcommands:
   - `reindex`: `--path`, `--full`, `--force-reembed`, `--batch-size`.
   - `fix-drift`: `--dry-run`, `--drift-type`.
4. `src/state.rs`: `DaemonState` holding thread-safe `Arc` references to `StorageEngine`, `QueryCoordinator`, `ReindexWorker`, `DriftHealer`, and `NetworkSwarmHandle`.
5. `src/server.rs`: Axum 0.7 router builder registering all endpoints and embedding static UI.
6. `src/browser.rs`: Background browser auto-launch using `open::that` with Windows native fallback, suppressed when `--no-browser` is active.
7. `src/shutdown.rs`: Graceful shutdown signal listener (`Ctrl+C` / SIGINT / SIGTERM) canceling active reindexing, shutting down swarm actor, and calling `storage.compact()` to flush and merge LanceDB and Tantivy indexes.
8. `src/handlers/`:
   - `mod.rs`: Handler module declarations.
   - `chat.rs`: `POST /api/chat` and `GET /api/chat/stream` SSE streaming bridge converting `AgenticTelemetryEvent` to `text/event-stream` with typewriter token deltas, and `POST /api/query` one-shot JSON.
   - `reindex.rs`: `POST /api/reindex` (starts background async task with UUID), `GET /api/reindex/status`, `POST /api/reindex/cancel`.
   - `drift.rs`: `POST /api/drift/fix` (executes 5-drift remediation), `GET /api/drift/status` (dry run).
   - `langextract.rs`: `POST /api/langextract` (structured JSON with 4-arm consensus and 100% `char_interval` byte bounds), `POST /api/langextract/visualize` (returns standalone interactive HTML).
   - `health.rs`: `GET /api/health` and `GET /api/models` reporting hardware telemetry (RAM/VRAM), model families, storage metrics, and active swarm peers.
9. `tests/daemon_tests.rs`: Integration tests for CLI parsing, route construction, and health endpoints.

### Phase 3: Workspace Integration & Feature 42 Verification
1. Update root `Cargo.toml`: Add `"crates/quorum-ui"` and `"crates/quorum-daemon"` to `[workspace] members`.
2. Author `scripts/verify_live_ide_execution.ps1`: Production-grade PowerShell script implementing all 5 verification gates from `explorer_m5_3/handoff.md`.
3. Author `tests/e2e/test_live_ide_execution.rs`: Rust integration test in `tests/e2e/` verifying real code retrieval, model routing, compilable code generation, and LangExtract grounding.
4. Update `tests/e2e/Cargo.toml`: Register `[[test]] name = "test_live_ide_execution" path = "test_live_ide_execution.rs"`.

### Phase 4: Verification Suite & Commit
Run the following verification sequence sequentially (with memory safeguards `jobs = 2`, `debug = 0`, and `-- --test-threads=2`):
1. `cargo check -p quorum-ui -p quorum-daemon`
2. `cargo test -p quorum-ui -p quorum-daemon -- --test-threads=2`
3. `cargo test --manifest-path tests/e2e/Cargo.toml --test test_web_endpoints -- --test-threads=2`
4. `cargo test --manifest-path tests/e2e/Cargo.toml --test test_live_ide_execution -- --test-threads=2`
5. `cargo clippy -p quorum-ui -p quorum-daemon --all-targets -- -D warnings`
6. Run live verification script:
   `powershell -ExecutionPolicy Bypass -File scripts/verify_live_ide_execution.ps1`
7. Stage and commit changes:
   `git add -A`
   `git commit -m "feat(milestone-5): implement Web UI, Axum daemon, single executable launcher and Feature 42 live IDE gate"`
8. Write comprehensive `handoff.md` with:
   - Observation (files authored, test results, git commit hash)
   - Logic Chain
   - Caveats
   - Conclusion
   - Verification Commands executed and outputs.


## 2026-11-01T00:33:19Z
You are worker_m5_1 (Milestone 5 Implementation Worker).
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_1\
Your parent orchestrator is: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
MANDATORY FIRST STEPS:
1. Initialize your BRIEFING.md and progress.md in your working directory.
2. Read your assignment in d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_1\DISPATCH.md.
3. Read the verbatim user request in d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md (specifically Feature 42, lines 213–225).
4. Read d:\harfile\QuorumRAG\PROJECT.md (Milestone 5 scope, features 31, 32, 34, 38, 39, 40, 41, 42).
5. Read all 3 Explorer handoff reports.
6. Read reference files in explorer_m5_2.
7. Read UI templates in ModelFusion.
Scope: Phase 1 (quorum-ui), Phase 2 (quorum-daemon), Phase 3 (Workspace integration & Feature 42 Gate), Phase 4 (Verification suite & Commit).


## 2026-11-01T00:54:19Z
**Context**: Milestone 5 Implementation
**Content**: Status check on Phase 3 (Feature 42 script & test) and Phase 4 (test matrix, live IDE verification, clippy, git commit).
**Action**: Please report your current step in the verification suite and let us know if you need any assistance.


## 2026-11-01T01:04:46Z
**Context**: Milestone 5 Verification
**Content**: Status check on `cargo test -p quorum-ui -p quorum-daemon` and subsequent verification suites.
**Action**: Please report if test compilation/execution is progressing or if you encountered any test assertion or compilation issues.


## 2026-11-01T01:15:25Z
**Context**: Milestone 5 Verification
**Content**: Status check on `test_web_endpoints`, `test_live_ide_execution`, and `verify_live_ide_execution.ps1`.
**Action**: Please provide a quick update on current test progress or any build/runtime issues encountered.


## 2026-11-01T01:24:00Z
**Context**: Milestone 5 Implementation
**Content**: Status check on `verify_live_ide_execution.ps1` and git commit / handoff report.
**Action**: Please report if `verify_live_ide_execution.ps1` finished, and if you are currently writing `handoff.md` or need any assistance.
