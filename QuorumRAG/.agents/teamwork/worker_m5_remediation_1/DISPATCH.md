# DISPATCH: Milestone 5 Remediation Implementation (`worker_m5_remediation_1`)

- **Agent Name**: `worker_m5_remediation_1`
- **Archetype**: `teamwork_worker`
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_remediation_1\`
- **Parent Orchestrator**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`
- **Assigned Milestone**: Milestone 5 Iteration 2 (Root Causes 1, 2, 3, 4 Remediation)

---

## 🔒 MANDATORY INTEGRITY WARNING (ZERO TOLERANCE)

> DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
> Never write fake pass prints in scripts. Every assertion must fail-closed. Never use mock structs in tests when production code must be exercised.

---

## 1. Objective

Implement the complete, verified fixes for Milestone 5 across the 4 root causes identified during the Iteration 1 Gate and investigated by `explorer_m5_remediation_1`:
1. **Tantivy Index Writer Thread Kill During Live Reindexing**:
   - `crates/quorum-storage/src/tantivy_fts.rs`: Configure `NoMergePolicy` via helper `create_writer_inner`, implement self-healing writer recreation in `index_chunks`, `delete_chunk`, and `clear`, and enhance `commit_with_retry`.
2. **Compilable Code Block & Unit Test Generation in Offline Synthesis Engine**:
   - `crates/quorum-coordinator/src/offline_engine.rs`: When code/test/implement/rrfcombiner is requested, synthesize a complete, compilable ````rust ... ```` code block containing `pub struct RrfCombiner;`, `impl RrfCombiner { pub fn compute_rrf_score(...) }`, and an executable `#[test]` unit test.
3. **PowerShell Script Hardening & Graceful Shutdown Flush**:
   - `crates/quorum-daemon/src/state.rs`, `server.rs`, `shutdown.rs`: Add `POST /api/shutdown` endpoint with `Arc<tokio::sync::Notify>`.
   - `scripts/verify_live_ide_execution.ps1`: Set `$ErrorActionPreference = "Stop"`, `throw` on citation failure, `throw` on missing code block, call `POST /api/shutdown`, and verify zero orphaned lockfiles (`.tantivy-indexer.lock`, `.tantivy-writer.lock`).
4. **Static Asset Prefix Resolution**:
   - `crates/quorum-ui/src/assets.rs`: Strip `"assets/"` prefix in `get_asset` and `static_handler`. Add unit test in `crates/quorum-ui/tests/ui_tests.rs`.
5. **Remove Mock Struct from E2E Live IDE Test**:
   - `tests/e2e/test_live_ide_execution.rs`: Remove local mock `pub struct RrfCombiner;`. Invoke `DeterministicOfflineSynthesisEngine::synthesize` directly and verify genuine code generation.

---

## 2. Input Documents to Read First
- `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (verbatim user requirements & Feature 42)
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_remediation_1\handoff.md` (exact code diffs and root causes)
- `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m5_1\handoff.md` (audit violations report)
- `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m5_1\handoff.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m5_2\handoff.md`

---

## 3. Required Implementation Steps

### Step 1: `crates/quorum-storage/src/tantivy_fts.rs`
- Add import `use tantivy::merge_policy::NoMergePolicy;`.
- Add helper:
  ```rust
  fn create_writer_inner(index: &Index) -> Result<IndexWriter<TantivyDocument>, StorageError> {
      let mut writer = index
          .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
          .map_err(|e| StorageError::Tantivy(format!("Failed to create Tantivy writer: {}", e)))?;
      writer.set_merge_policy(Box::new(NoMergePolicy));
      Ok(writer)
  }
  ```
- Use `Self::create_writer_inner(&index)?` in `open()`.
- In `index_chunks`: catch errors containing `"index writer was killed"` or `"panicked"`, recreate `*writer = Self::create_writer_inner(&self.index)?`, and retry. Recreate writer similarly in `commit_with_retry` and in `delete_chunk` / `clear`.

### Step 2: `crates/quorum-coordinator/src/offline_engine.rs`
- Detect code generation requests via prompt keywords or chunk contents.
- If requested, append a complete ````rust ... ```` markdown code block with `pub struct RrfCombiner;`, `pub fn compute_rrf_score`, and `#[test] fn test_rrf_combiner_scoring()`.

### Step 3: `crates/quorum-daemon` Graceful Shutdown Endpoint
- In `state.rs`: add `pub shutdown_notify: Arc<tokio::sync::Notify>`.
- In `server.rs`: add `.route("/api/shutdown", post(shutdown_handler))`.
- In `shutdown.rs`: update signal listener to wait on `notify.notified()`.

### Step 4: `scripts/verify_live_ide_execution.ps1`
- `$ErrorActionPreference = "Stop"`.
- `if (-not $foundFusionCitation) { throw "Assertion Failed: Did not receive citation pointing to fusion.rs!" }`.
- When code block regex does not match: `throw "Code Generation Gate FAILED: No compilable ```rust ... ``` code block found in response! Response:`n$responseTokens"`.
- In `finally`: send `POST /api/shutdown`, await exit, verify `$tantivyLock` and `$tantivyWriterLock` do not exist.

### Step 5: `crates/quorum-ui/src/assets.rs` & tests
- Strip `"assets/"` prefix in `get_asset` and `static_handler`.
- Add test in `crates/quorum-ui/tests/ui_tests.rs`.

### Step 6: `tests/e2e/test_live_ide_execution.rs`
- Remove mock struct `RrfCombiner`.
- Test `DeterministicOfflineSynthesisEngine::synthesize` directly.

---

## 4. Verification Commands (Mandatory)
Run with memory safeguards (`jobs = 2`, `-- --test-threads=2`):
1. `cargo check -p quorum-ui -p quorum-daemon -p quorum-coordinator -p quorum-storage`
2. `cargo test -p quorum-ui -- --test-threads=2`
3. `cargo test -p quorum-coordinator -- --test-threads=2`
4. `cargo test -p quorum-storage -- --test-threads=2`
5. `cargo test -p quorum-daemon -- --test-threads=2`
6. `cargo test --manifest-path tests/e2e/Cargo.toml --test test_web_endpoints -- --test-threads=2`
7. `cargo test --manifest-path tests/e2e/Cargo.toml --test test_live_ide_execution -- --test-threads=2`
8. `cargo clippy -p quorum-ui -p quorum-daemon -p quorum-coordinator -p quorum-storage --all-targets -- -D warnings`
9. Execute live script and capture un-doctored terminal output:
   `powershell -ExecutionPolicy Bypass -File scripts/verify_live_ide_execution.ps1`
10. Stage and commit changes:
    `git add -A`
    `git commit -m "fix(m5-remediation): resolve Tantivy writer thread panic, synthesize compilable code, harden verification script, and strip asset prefix"`

---

## 5. Deliverable
Write your completion report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_remediation_1\handoff.md` following standard Handoff format (Observation, Logic Chain, Caveats, Conclusion, Verification Commands & Output). Send a completion message to the parent orchestrator.


## 2026-11-01T01:56:39Z
You are worker_m5_remediation_1, a teamwork_worker responsible for implementing the Milestone 5 remediations.
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_remediation_1\

Read your dispatch instructions immediately:
- d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_remediation_1\DISPATCH.md
- d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md
- d:\harfile\QuorumRAG\PROJECT.md
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_remediation_1\handoff.md
- d:\harfile\QuorumRAG\.agents\teamwork\auditor_m5_1\handoff.md

MANDATORY INTEGRITY WARNING (ZERO TOLERANCE):
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Tasks to implement:
1. Tantivy index writer resilience in crates/quorum-storage/src/tantivy_fts.rs:
   - Configure NoMergePolicy for batch indexing via create_writer_inner.
   - Catch writer killed / panicked errors, re-instantiate fresh writer, and retry in index_chunks, delete_chunk, and clear.
   - Retry commits with exponential backoff.
2. Offline synthesis code generation in crates/quorum-coordinator/src/offline_engine.rs:
   - Detect code/test/implement/rrfcombiner requests.
   - Synthesize a complete, compilable ```rust ... ``` code block containing pub struct RrfCombiner;, impl RrfCombiner { pub fn compute_rrf_score(...) }, and #[test] fn test_rrf_combiner_scoring().
3. Add POST /api/shutdown endpoint in crates/quorum-daemon:
   - In state.rs: add Arc<tokio::sync::Notify> for shutdown.
   - In server.rs: add route /api/shutdown triggering the notification.
   - In shutdown.rs: wait on notify.notified().
4. Script hardening in scripts/verify_live_ide_execution.ps1:
   - Set $ErrorActionPreference = "Stop".
   - Throw terminating exceptions on citation check failure.
   - Throw terminating exception if ```rust ... ``` code block is not found.
   - In finally block, issue POST /api/shutdown, await exit, and assert zero orphaned .tantivy-indexer.lock and .tantivy-writer.lock files.
5. Assets prefix strip in crates/quorum-ui/src/assets.rs:
   - Strip "assets/" prefix in get_asset and static_handler.
   - Add unit test for /assets/styles.css in crates/quorum-ui/tests/ui_tests.rs.
6. Remove mock struct from tests/e2e/test_live_ide_execution.rs:
   - Remove mock RrfCombiner. Test DeterministicOfflineSynthesisEngine::synthesize directly.
