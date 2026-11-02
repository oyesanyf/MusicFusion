# Handoff Report: Explorer Survey Gen2 — Requirements R3 & R4

## 1. Observation

### 1.1 Chat UI Stop Button & Streaming Cancellation Architecture
- **Web UI Chat Form & Button (`crates/quorum-ui/assets/index.html:200-206`)**:
  - The chat input is wrapped in `<form id="chat-form" class="input-capsule">` containing `<textarea id="prompt-input">` and `<button type="submit" id="btn-send" class="btn-send" title="Send query"><span class="send-arrow">➔</span></button>`.
  - There is currently no Stop button element in `index.html`.
- **Client Fetch Loop & Missing AbortController (`crates/quorum-ui/assets/app.js:171-249`)**:
  - In `app.js` line 171-172: When a query starts, `isGenerating = true; btnSend.disabled = true;`.
  - Lines 191-205: `resp = await fetch('/api/chat', { method: 'POST', ... })`.
  - **No `AbortController` is created or passed in the `fetch` options**. The user cannot abort or stop an in-flight query.
  - Lines 250-264: `btnSend.disabled` is only set back to `false` in the `finally` block when the stream is exhausted or throws.
- **Server Routes & In-Flight Query Handling (`crates/quorum-daemon/src/server.rs:20-22`, `crates/quorum-daemon/src/handlers/chat.rs:41-114`)**:
  - Server defines `POST /api/chat` and `GET /api/chat/stream`. There is no `POST /api/chat/cancel` endpoint.
  - In `crates/quorum-daemon/src/handlers/chat.rs:64-106`:
    - `let (tx, rx) = tokio::sync::mpsc::channel::<AgenticTelemetryEvent>(256);`
    - `tokio::spawn(async move { match coordinator.query_agentic_with_history(...).await { Ok(result) => { for word in words { let _ = tx.send(AgenticTelemetryEvent::TokenDelta { delta: word.to_string() }).await; tokio::time::sleep(Duration::from_millis(5)).await; } } ... } });`
    - `let stream = ReceiverStream::new(rx)...; Sse::new(stream)...`
  - In `chat.rs` line 87: `let _ = tx.send(...).await;` ignores errors and does NOT break if `tx.is_closed()`. If the user disconnects, the spawned tokio thread continues sleeping 5ms per word for thousands of words doing useless work.
- **Agentic Coordinator State Graph (`crates/quorum-coordinator/src/agentic/mod.rs:85-150`)**:
  - `AgenticStateGraphRunner::run_with_history` runs a loop transitioning across `ClassifyIntent -> RetrieveMesh -> GradeRelevance -> RewriteQuery -> SynthesizeWithCitations`.
  - It takes `telemetry_tx: Option<Sender<AgenticTelemetryEvent>>`.
  - In `crates/quorum-coordinator/src/offline_engine.rs:460-490`, `if tx.is_closed() { break; }` was recently added in the NDJSON stream loop, proving `tx.is_closed()` is a clean, non-invasive abort signal. However, `agentic/mod.rs` does not yet check `tx.is_closed()` between graph state hops.

### 1.2 Live Indexer Progress Indicators & Stale Status
- **Reindex API Endpoints (`crates/quorum-daemon/src/handlers/reindex.rs:71-129`)**:
  - `GET /api/reindex/status`:
    - Calls `state.reindex_worker.get_progress().await` and `state.storage.count().await`.
    - Lines 102-116: When `get_progress()` returns `None` (on startup or when no task is active), it defaults to:
      ```rust
      "phase": "scanning_filesystem",
      "percent": 0.0,
      "files_scanned": 0,
      "files_total_estimated": 0,
      "message": "Continuous background indexer initializing..."
      ```
    - Lines 120-129: `POST /api/reindex/cancel` calls `state.reindex_worker.cancel_reindex()`, which sets an `AtomicBool` (`self.cancel_flag.store(true, Ordering::SeqCst)`).
- **Misleading Percentage & Phase Handling in Web UI (`crates/quorum-ui/assets/app.js:622-714`)**:
  - In `app.js` lines 634-644:
    ```javascript
    let pct = Number(data.percent || 0);
    if (totalEst > 0) {
      pct = Math.min(100, Math.max(2, (scanned / totalEst) * 100));
    } else if (phase === 'completed') {
      pct = 100;
    } else {
      pct = 25; // <--- Hardcoded fake fallback!
    }
    ```
  - When `get_progress()` is `None`, `phase` is reported as `"scanning_filesystem"` and `totalEst == 0`. Therefore, `pct` is set to `25%` and `isRunning` evaluates to `true` (line 643).
  - As a result, the UI continuously flashes `"Indexing Drives... 25% (Scanning...)"` even when the system is completely idle!
- **Slice Undercounting Bug in Pipeline (`crates/quorum-ingest/src/reindex.rs:278-314`)**:
  - In `execute_pipeline`:
    ```rust
    for target in &targets {
        let candidates = crawler.crawl_path_sync(&target)?;
        {
            let mut lock = progress.write().await;
            if let Some(ref mut p) = *lock {
                p.files_total_estimated += candidates.len(); // Increment happens target-by-target
                p.phase = ReindexPhase::ChunkingAndVectorizing;
            }
        }
        for candidate in candidates { ... }
    }
    ```
  - `p.files_total_estimated` is only updated as each directory/drive target is crawled inside the loop. During target 1, `files_total_estimated` only reflects target 1. Progress jumps to 100% on target 1, then drops back down to 30% when target 2 is discovered.
- **12-Second Loop Progress Wipe (`crates/quorum-daemon/src/main.rs:201-245`)**:
  - A background loop triggers `bg_worker.start_reindex(pass_options)` every 12 seconds if not running.
  - In `crates/quorum-ingest/src/reindex.rs:118-135`, `start_reindex` resets `files_scanned: 0`, `files_total_estimated: 0`, wiping out the completed state and re-triggering the "scanning" phase.

### 1.3 Existing Test and Benchmark Suite
- **28 Test Suites in Crates (`crates/*/tests/`)**:
  - `quorum-classifier`: `adversarial_magika_tests.rs`, `corner_cases_multibyte_utf8.rs`
  - `quorum-contract`: `adversarial_contract_tests.rs`, `corner_cases_handshake_gating.rs`
  - `quorum-coordinator`: `adversarial_cache_challenge_tests.rs`, `adversarial_cache_m5_5_2_stress_tests.rs`, `adversarial_challenge_m4_tests.rs`, `adversarial_tier5_coordinator_daemon_stress.rs`, `agentic_tests.rs`, `coordinator_tests.rs`
  - `quorum-daemon`: `adversarial_endpoint_tests.rs`, `adversarial_recache_api_tests.rs`, `adversarial_web_routes_challenge.rs`, `daemon_tests.rs`
  - `quorum-ingest`: `adversarial_challenge_tests.rs`, `adversarial_reindex_and_drift.rs`, `ingest_tests.rs`
  - `quorum-network`: `adversarial_network_tests.rs`, `network_tests.rs`
  - `quorum-storage`: `adversarial_storage_tests.rs`, `adversarial_stress_tests.rs`, `adversarial_tier5_storage_ingest_stress.rs`, `storage_tests.rs`
  - `quorum-types`: `adversarial_challenge.rs`, `adversarial_types_tests.rs`, `corner_cases_timestamp_and_utf8.rs`
  - `quorum-ui`: `adversarial_ui_tests.rs`, `ui_tests.rs`
- **9 E2E Test Suites (`tests/e2e/`)**:
  - `test_storage_isolation.rs`, `test_gemma_contract.rs`, `test_ingestion_dedup.rs`, `test_federated_mesh.rs`, `test_web_endpoints.rs`, `test_cross_feature_combinations.rs`, `test_real_world_scenarios.rs`, `test_live_ide_execution.rs`.
- **Existing Benchmark Pattern (`crates/quorum-coordinator/tests/adversarial_cache_challenge_tests.rs:120-170`)**:
  - Deterministic timing loop over 1,000 iterations measuring Min, P50, P90, P99, Max, Average µs/ms, and hard threshold assertions without requiring external Criterion dependencies.

### 1.4 P2P Mesh Security & Inbound Signature Gating
- In `crates/quorum-coordinator/src/mesh.rs:201-245` (`OsoosiMeshInboundHandler::handle_query`):
  - Inbound queries (`req: FederatedQueryRequest`) check `p2p_enabled` and `is_quarantined`, but **never verify the Ed25519 signature** (`req.signature`) or anti-replay timestamp before returning chunks from shared folders.
- In `crates/quorum-coordinator/src/scatter_gather.rs:161-175`:
  - When remote candidate chunks are returned, `chunk.access_tier <= access_tier` is not checked on the receiving side, allowing a Byzantine peer to return Tier 3 chunks for a Tier 0 request.

### 1.5 Solo Developer Process
- `d:\harfile\OshoosiClaw\scripts\solo_pr_ship.ps1`:
  - Features 5 stages: feature branch creation (`pr/solo-<timestamp>`), git commit, push to origin, GitHub API PR creation via Python, polling review state (`APPROVED`), and squash-merging into `main`.
  - QuorumRAG remote: `https://github.com/oyesanyf/MusicFusion.git`, branch `main`.

### 1.6 Active Workspace Compiler Break Alert (`crates/quorum-coordinator/src/agentic/synthesize.rs:8`)
- Verification via `cargo check --workspace --tests` revealed:
  `error[E0061]: this function takes 6 arguments but 5 arguments were supplied` at `crates\quorum-coordinator\src\agentic\synthesize.rs:8:33`.
- Root cause: A recent change in `crates\quorum-coordinator\src\offline_engine.rs:99` added a 6th argument (`telemetry_tx: Option<&Sender<AgenticTelemetryEvent>>`) to `DeterministicOfflineSynthesisEngine::synthesize_with_history`.
- In `crates\quorum-coordinator\src\agentic\synthesize.rs:8`, only 5 arguments are passed:
  ```rust
  let (resp, citations) = DeterministicOfflineSynthesisEngine::synthesize_with_history(
      &state.original_prompt,
      &state.current_query,
      &state.conversation_history,
      &state.retrieved_chunks,
      &state.selected_model_tag,
  );
  ```
- Immediate Remedy for Implementer: Pass `None` (or plumb `telemetry_tx`) at line 14 of `synthesize.rs`.


---

## 2. Logic Chain

1. **User Interface Responsiveness & Stop Button (R3)**:
   - *Observation*: `app.js:172` disables `btnSend` without saving an `AbortController`. `chat.rs:87` ignores transmission failures and continues looping.
   - *Inference*: To provide immediate cancellation, the frontend must instantiate an `AbortController` and wire its abort signal to both a stop button click and Escape key. The button must transform from a send arrow into a stop square (`⏹`).
   - *Inference*: On the backend, when `ReceiverStream` drops, `tx.is_closed()` immediately becomes true. `chat.rs` must break its word-streaming loop when `tx.is_closed()`. Adding an explicit `POST /api/chat/cancel` endpoint backed by an active query registry (`HashMap<String, Arc<AtomicBool>>`) enables both network-level and application-level cancellation.
   - *Inference*: In `agentic/mod.rs`, checking `telemetry_tx.is_closed()` at the start of each graph loop allows multi-hop reasoning chains to terminate early if cancelled.

2. **Accurate Indexer Progress & State Reporting (R3)**:
   - *Observation*: `reindex_status_handler` emits `"phase": "scanning_filesystem"` when `get_progress()` is `None`, and `app.js:640` fabricates `pct = 25`.
   - *Inference*: When no reindex is active, the backend must report `"phase": "idle"` (or `"completed"`) and `"percent": 100.0`. `app.js` must eliminate the hardcoded `25%` fallback and display `"Idle (100%)"` or `"Discovering..."`.
   - *Observation*: `reindex.rs:296` increments `files_total_estimated` sequentially per drive inside the processing loop, causing slice undercounting.
   - *Inference*: Total candidate files across all configured targets must be counted in an initial discovery pass before entering the chunking/vectorizing loop.
   - *Observation*: `main.rs:242` triggers a new reindex every 12 seconds, resetting progress counts to zero.
   - *Inference*: The 12-second pass must only trigger if the filesystem watcher detected modified files, or preserve previous completion counts if no files were touched.

3. **Deterministic Benchmark Suite for R4**:
   - *Observation*: The repository already implements microsecond-accurate percentile timing in `adversarial_cache_challenge_tests.rs:120-170`.
   - *Inference*: We can create a dedicated deterministic benchmark suite (`benches/r4_performance_benchmarks.rs` or `crates/quorum-coordinator/tests/benchmarks_r4.rs`) measuring:
     1. Query Latency (Cold vs Warm hit P50/P90/P99 over 1,000 queries)
     2. Disk I/O (bytes read/written on cold ingest vs 0 bytes on Level-1 dedup pass)
     3. CPU Overhead (idle CPU consumption of debounced watcher < 1%)
     4. Memory Footprint (RSS usage baseline vs 100 concurrent queries via `sysinfo`)
     5. Document & Image Index Coverage (100% of discovered text/image assets indexed)
     6. Binary Exclusion Efficiency (100% of PE/ELF/DLL binaries excluded in < 1ms, 0 chunks)
     7. Mesh Consensus Metrics (Scatter-gather latency, Ed25519 verify ops/sec, RRF consistency).

4. **Simulated Byzantine Peer, Spoofing, and Drive Fault Tests**:
   - *Observation*: In `mesh.rs:201`, `OsoosiMeshInboundHandler` does not verify Ed25519 signatures, and `scatter_gather.rs:161` does not filter out Byzantine peer chunks exceeding requested access tiers.
   - *Inference*: Regression tests must verify:
     - Forged signatures and stale timestamps (> 60s) fail closed.
     - Mismatched BLAKE3 chunk hashes and unsolicited higher-tier chunks are rejected.
     - Drive faults (unmounted targets, Windows sharing violations, storage write errors) are isolated without pipeline crashes.

5. **Solo Developer Process**:
   - *Observation*: The script `scripts/solo_pr_ship.ps1` from `d:\harfile\OshoosiClaw` automates branching, atomic committing, PR opening with benchmark descriptions, bot review approval, and squash-merging.
   - *Inference*: Adopting `scripts/solo_pr_ship.ps1` for QuorumRAG (configured for remote `https://github.com/oyesanyf/MusicFusion.git`) ensures compliance with R4 and repository rules.

---

## 3. Caveats

1. **Read-Only Scope**: This survey investigated code and technical architecture without modifying any production source files. Implementation will be handled by the subsequent worker/implementer agents.
2. **Offline Constraint**: All proposed benchmarks and regression tests must execute strictly offline without external internet connectivity or live cloud model calls. The deterministic offline engine (`quorum_contract::DeterministicOfflineEngine`) and local LanceDB/Tantivy fixtures ensure 100% reproducibility.
3. **LanceDB & DataFusion PDB Size on Windows**: Full compilation with debug symbols can consume significant memory. The workspace profile settings (`debug = 0`) must be preserved during testing.

---

## 4. Conclusion & Proposed Architecture

### Proposed Modifications for Requirement R3

#### 1. Chat UI Active Stop Button
- **`crates/quorum-ui/assets/index.html`**:
  - Update `#chat-form` so `#btn-send` supports toggling into a Stop button, or add a dedicated `<button type="button" id="btn-stop" class="btn-stop hidden" title="Stop generating"><span class="stop-icon">⏹</span></button>`.
- **`crates/quorum-ui/assets/styles.css`**:
  - Add styles for `.btn-stop`:
    ```css
    .btn-stop {
      width: 34px;
      height: 34px;
      border-radius: 50%;
      background: var(--danger, #ef4444);
      color: #fff;
      display: flex;
      align-items: center;
      justify-content: center;
      border: none;
      cursor: pointer;
      transition: transform 0.15s ease;
    }
    .btn-stop:hover { transform: scale(1.05); }
    ```
- **`crates/quorum-ui/assets/app.js`**:
  - Maintain `let activeChatAbortController = null;` and `let activeQueryId = null;`.
  - In `handlePromptSubmit`:
    - Generate `activeQueryId = 'q_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);`.
    - Instantiate `activeChatAbortController = new AbortController();`.
    - Pass `signal: activeChatAbortController.signal` and `{ prompt, query_id: activeQueryId, ... }` to `fetch('/api/chat', ...)`.
    - Switch send button to stop button state (show `#btn-stop` and hide `#btn-send`, or toggle class/icon).
    - Wire Stop button click and `Escape` key to:
      1. `activeChatAbortController.abort();`
      2. `fetch('/api/chat/cancel', { method: 'POST', body: JSON.stringify({ query_id: activeQueryId }) });`
      3. `stopThinkingIndicator(bodyEl);`
      4. Render partial text with `[Stopped by user]`.
      5. Reset button state back to send arrow.
- **`crates/quorum-daemon/src/handlers/chat.rs` & `server.rs`**:
  - Add `query_id: Option<String>` to `ChatRequest`.
  - In `DaemonState`, track `active_queries: Arc<RwLock<HashMap<String, Arc<AtomicBool>>>>`.
  - Add route `POST /api/chat/cancel` handled by `chat_cancel_handler`.
  - In `chat_sse_handler`, register the query's cancel flag; in the word-streaming loop, immediately break if `cancel_flag.load(Ordering::Relaxed) || tx.is_closed()`.
  - In `crates/quorum-coordinator/src/agentic/mod.rs`, check `if telemetry_tx.as_ref().map(|tx| tx.is_closed()).unwrap_or(false) { return Err("Query cancelled".into()); }` at each graph step.

#### 2. Live Indexer Progress & Stale Status Fixes
- **`crates/quorum-daemon/src/handlers/reindex.rs`**:
  - In `reindex_status_handler`, change default fallback phase from `"scanning_filesystem"` to `"phase": "idle"`, `"percent": 100.0`.
- **`crates/quorum-ingest/src/reindex.rs`**:
  - In `execute_pipeline`, pre-compute `total_candidates` across all valid target paths before entering the chunking/vectorization loop to prevent slice undercounting.
- **`crates/quorum-daemon/src/main.rs`**:
  - Only launch the 12-second background pass if there are pending filesystem watcher events, avoiding progress wiping during idle periods.
- **`crates/quorum-ui/assets/app.js`**:
  - Remove `else { pct = 25; }`. Show `100% (Idle)` when phase is `idle` or `completed`. Accurately format `scanned / totalEst` files and LanceDB chunk counts.

### Proposed Modifications for Requirement R4

#### 1. Benchmark Suite (`tests/benchmarks_r4.rs`)
Implement deterministic before/after benchmarks covering the 7 key metrics:
- Metric 1: Query Latency (Cold LanceDB+Tantivy vs Warm QueryCache over 1,000 iterations).
- Metric 2: Disk I/O (Cold write bytes vs Level-1 dedup 0 write bytes).
- Metric 3: CPU Overhead (Idle watcher CPU < 1%).
- Metric 4: Memory Footprint (Baseline RSS vs 100 concurrent queries via `sysinfo`).
- Metric 5: Document and Image Index Coverage (100% of discovered text/image files indexed).
- Metric 6: Binary Exclusion Efficiency (100% of executables/PE/DLLs rejected, 0 chunks).
- Metric 7: Mesh Consensus Metrics (Scatter-gather latency, Ed25519 verify throughput, RRF ranking stability).

#### 2. Byzantine Peer, Spoofing, and Drive Fault Tests (`tests/adversarial_mesh_and_faults.rs`)
- Inbound Ed25519 signature enforcement in `OsoosiMeshInboundHandler`.
- Inbound chunk tier clamping in `ScatterGatherEngine`.
- Drive unmount, file lock (`ERROR_SHARING_VIOLATION`), and storage write error isolation tests.

#### 3. Solo Developer PR Process
- Port `d:\harfile\OshoosiClaw\scripts\solo_pr_ship.ps1` to `scripts/solo_pr_ship.ps1` in QuorumRAG:
  - Branch cleanly from `main`: `pr/solo-<timestamp>`.
  - Atomic commits with detailed descriptions.
  - Verification: Run `cargo test` and benchmark suite.
  - Automatically open PR with the 7-metric before/after table in the description.
  - Verify bot/peer review approval and squash-merge into `main`.

---

## 5. Verification Method

### How to Independently Verify:
1. **Compilation Check**:
   ```powershell
   cargo check --workspace --tests
   ```
2. **UI Asset Integrity & Endpoint Tests**:
   ```powershell
   cargo test -p quorum-ui
   cargo test -p quorum-daemon --test adversarial_endpoint_tests
   ```
3. **Indexer Progress & Reindex Tests**:
   ```powershell
   cargo test -p quorum-ingest --test adversarial_reindex_and_drift
   ```
4. **Mesh Security & Cryptographic Gating Tests**:
   ```powershell
   cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests
   cargo test --test test_federated_mesh
   ```
5. **E2E & Live IDE Execution Verification**:
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/verify_live_ide_execution.ps1
   ```
6. **R4 Benchmark Execution**:
   ```powershell
   cargo test --test benchmarks_r4 -- --nocapture
   ```
7. **Solo PR Shipping Verification**:
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/solo_pr_ship.ps1 -Title "feat(ui,mesh): R3 indexer controls and R4 verification suite"
   ```
