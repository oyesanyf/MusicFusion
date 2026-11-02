## 2026-11-02T07:01:58Z
You are Worker M9 1 (Worker M9 Implementation).
Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\worker_m9_1.

MANDATORY FIRST STEP:
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md and d:\harfile\QuorumRAG\PROJECT.md before doing anything else.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

INPUT EXPLORER REPORTS (Read these thoroughly):
1. Stop Button & Dual-Channel In-Flight Query Cancellation:
   Read `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m9_1\handoff.md`
2. Dynamic Thinking Phrases Engine & Static Cache Headers:
   Read `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m9_2\handoff.md`
3. Monotonic Live Indexer Progress Reporting (Eliminate 25% fake fallback):
   Read `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m9_3\handoff.md`

EXCLUSIVE WRITE OWNERSHIP:
- `crates/quorum-ui/assets/app.js`
- `crates/quorum-ui/assets/index.html`
- `crates/quorum-ui/assets/styles.css`
- `crates/quorum-ui/src/assets.rs`
- `crates/quorum-daemon/src/handlers/chat.rs`
- `crates/quorum-daemon/src/handlers/reindex.rs`
- `crates/quorum-daemon/src/server.rs`
- `crates/quorum-daemon/src/state.rs`
- `crates/quorum-coordinator/src/agentic/mod.rs`
- `crates/quorum-coordinator/src/offline_engine.rs`
- `crates/quorum-ingest/src/reindex.rs`
- Test files: `crates/quorum-ingest/tests/m9_monotonic_progress_test.rs`, `crates/quorum-ui/tests/m9_ui_cancellation_and_progress_tests.rs`

TASKS:
1. Feature 52 (Active Chat UI Stop Button & In-Flight Cancellation):
   - In `crates/quorum-ui/assets/styles.css`: Add styling for `.btn-stop`.
   - In `crates/quorum-ui/assets/index.html`: Ensure `#btn-stop` is positioned alongside `#btn-send`.
   - In `crates/quorum-ui/assets/app.js`:
     - Maintain `activeQueryId` and pass `query_id` in `/api/chat` payload.
     - On `#btn-stop` click, call `activeAbortController.abort()` AND `fetch('/api/chat/cancel', { method: 'POST', body: JSON.stringify({ query_id: activeQueryId }) })`.
     - In the retry loop on `AbortError`, break immediately without retrying.
     - When stopped by user, append `*(Generation stopped by user)*` cleanly.
     - Toggle `#btn-stop` visibility and `#btn-send` disabled state properly in start/finally.
   - In `crates/quorum-daemon/src/state.rs`: Add `InFlightQueryRegistry` to `DaemonState` to map query IDs to Tokio `AbortHandle` and cancel tokens.
   - In `crates/quorum-daemon/src/handlers/chat.rs`:
     - Track the spawned task in `InFlightQueryRegistry`.
     - Implement `cancel_chat_handler` (`POST /api/chat/cancel`) to abort Tokio handle and mark cancel token.
   - In `crates/quorum-daemon/src/server.rs`: Register `.route("/api/chat/cancel", post(chat::cancel_chat_handler))`.
   - In `crates/quorum-coordinator/src/agentic/mod.rs`: At the top of the state loop, check `if let Some(ref tx) = telemetry_tx { if tx.is_closed() { return Ok(graph_state); } }`.
   - In `crates/quorum-coordinator/src/offline_engine.rs`: In `synthesize_with_history`, stream tokens word-by-word with `if tx.is_closed() { break; }` so cancellation stops token emission immediately.

2. Feature 53 (Monotonic Live Indexer Progress Reporting):
   - In `crates/quorum-ingest/src/reindex.rs`:
     - In `execute_pipeline`, decouple discovery from chunking: first crawl all `targets` in `ScanningFilesystem` phase, accumulate candidates into a unified list, and set `p.files_total_estimated = total_candidates`.
     - Then transition `p.phase = ReindexPhase::ChunkingAndVectorizing` and iterate through candidates, incrementing `p.files_scanned += 1` on each file so `(files_scanned / files_total_estimated) * 100` advances monotonically without yo-yo oscillations between slice targets.
   - In `crates/quorum-daemon/src/handlers/reindex.rs`:
     - Compute `pct`: if `files_total_estimated > 0`, calculate genuine percent clamped to `[0.0, 100.0]`. If `files_total_estimated == 0 && (phase == Completed || total_stored_chunks > 0)`, return `100.0`. If 0 and idle with 0 chunks, return `0.0`. Never return fake 25%.
   - In `crates/quorum-daemon/src/server.rs`:
     - Map `.route("/api/index/status", get(reindex::reindex_status_handler))` as an alias for `/api/reindex/status`.
   - In `crates/quorum-ui/assets/app.js`:
     - In `checkReindexStatus`, eliminate `pct = 25` and `Math.max(2, ...)`.
     - Read `data.percent`, enforce monotonic client-side maximum during active tasks (`lastMonotonicPct`), and display accurate text.
   - In `crates/quorum-ui/assets/index.html`:
     - Change hardcoded widths `style="width:15%"` and `style="width:20%"` to `width:0%` initial state.

3. Feature 54 (Dynamic Thinking Phrases Engine & Cache Header Fix):
   - In `crates/quorum-ui/assets/app.js`:
     - Embed the comprehensive 72-item phrase pool from `explorer_m9_2/handoff.md § 4.1` covering all 8 architectural pillars.
     - Implement ~650ms interval cycling via uniform random selection without repeating consecutive phrases: `while (nextIdx === idx) { nextIdx = Math.floor(Math.random() * THINKING_PHRASES.length); }`.
     - Update text in-place (`.thinking-text.textContent = phrase`) preserving `.pulse-dot` animation.
     - Ensure `startThinkingAnimation` / `stopThinkingAnimation` are exported / aliased.
     - Handle telemetry events smoothly without stopping or resetting thinking animation before `TokenDelta`.
   - In `crates/quorum-ui/src/assets.rs`:
     - Set `header::CACHE_CONTROL, "public, max-age=3600"` for static assets (`/styles.css`, `/app.js`, `/favicon.svg`, `/langextract.html`), while keeping `index.html` at `no-cache, no-store, must-revalidate`.

4. Verification:
   - Create `crates/quorum-ingest/tests/m9_monotonic_progress_test.rs` asserting monotonic non-decreasing `files_scanned` and percentage.
   - Create/update UI tests in `crates/quorum-ui/tests/` to verify cancellation and cache headers.
   - Run verification commands:
     - `cargo check --workspace`
     - `cargo test -p quorum-ui -- --test-threads=2`
     - `cargo test -p quorum-daemon -- --test-threads=2`
     - `cargo test -p quorum-ingest --test m9_monotonic_progress_test -- --test-threads=2`

Write your report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m9_1\handoff.md` and send a completion message with summary.
