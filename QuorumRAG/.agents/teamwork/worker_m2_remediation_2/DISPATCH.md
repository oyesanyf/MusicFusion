# Task Assignment: Worker M2 Remediation 2 — Top-K Bounds & Filename Assertions

## Mission
Apply the targeted final fixes for Milestone 2 (`crates/quorum-storage/`):
1. **Fix Tantivy `TopDocs` capacity overflow on `top_k = usize::MAX`**:
   In `crates/quorum-storage/src/tantivy_fts.rs:180`:
   Tantivy's `TopDocs::with_limit(top_k)` attempts to allocate a binary heap with capacity `top_k`. When `top_k == usize::MAX`, Rust standard library throws a `capacity overflow` panic.
   Fix:
   ```rust
   let num_docs = searcher.num_docs() as usize;
   let effective_limit = top_k.min(num_docs.max(1));
   let top_docs = searcher
       .search(&combined, &TopDocs::with_limit(effective_limit).order_by_score())
       .map_err(|e| StorageError::Tantivy(format!("Search execution failed: {}", e)))?;
   ```
2. **Cap `candidate_limit` in `StorageEngine::search_hybrid`**:
   In `crates/quorum-storage/src/engine.rs:175`:
   ```rust
   let candidate_limit = top_k.saturating_mul(2).min(50_000).max(10);
   ```
3. **Update `adversarial_stress_tests.rs`**:
   In `test_adv_top_k_overflow_in_hybrid`:
   Verify `engine.search_hybrid(..., usize::MAX).await` returns `Ok(hits)` cleanly without panicking.
   In `test_adv_filename_search_in_tantivy`:
   Add assertion `assert_eq!(hits.len(), 1, "Searching by filename keyword must return 1 hit");`.

## Mandatory Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Verification Commands
Execute sequentially with `-- --test-threads=2`:
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"

cargo test -p quorum-storage --test storage_tests -- --test-threads=2
cargo test -p quorum-storage --test adversarial_storage_tests -- --test-threads=2
cargo test -p quorum-storage --test adversarial_stress_tests -- --test-threads=2
cargo test --manifest-path tests/e2e/Cargo.toml --test test_storage_isolation -- --test-threads=2
cargo test --workspace -- --test-threads=2
cargo clippy --workspace --all-targets -- -D warnings
```

## Completion Criteria
Write your complete handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_2\handoff.md`, update `progress.md`, and notify orchestrator via `send_message`.


## 2026-10-31T18:53:20Z
You are `worker_m2_remediation_2`, assigned to apply the final targeted fixes for Milestone 2 (`crates/quorum-storage/`).
Your working directory is: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_2\`
Your task assignment is in: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_2\DISPATCH.md`

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

MEMORY & EXECUTION INVARIANTS:
1. Windows Commit Memory Fix Applied: `.cargo/config.toml` has `jobs = 2` and `Cargo.toml` has `debug = 0`. Preserve these.
2. Run cargo tests sequentially with `-- --test-threads=2` to ensure low memory consumption and zero thread contention.
3. Do NOT invoke `taskkill.exe`.

TASKS:
1. In `crates/quorum-storage/src/tantivy_fts.rs:180`:
   Clamp `top_k` using the number of documents in the index to prevent Tantivy's `TopDocs` binary heap capacity overflow on `top_k == usize::MAX`:
   ```rust
   let num_docs = searcher.num_docs() as usize;
   let effective_limit = top_k.min(num_docs.max(1));
   let top_docs = searcher
       .search(&combined, &TopDocs::with_limit(effective_limit).order_by_score())
       .map_err(|e| StorageError::Tantivy(format!("Search execution failed: {}", e)))?;
   ```
2. In `crates/quorum-storage/src/engine.rs:175`:
   Cap `candidate_limit` to prevent unbounded limits:
   ```rust
   let candidate_limit = top_k.saturating_mul(2).min(50_000).max(10);
   ```
3. In `crates/quorum-storage/tests/adversarial_stress_tests.rs`:
   In `test_adv_top_k_overflow_in_hybrid`:
   Verify `engine.search_hybrid(..., usize::MAX).await` returns `Ok(hits)` cleanly without panicking.
   In `test_adv_filename_search_in_tantivy`:
   Add assertion: `assert_eq!(hits.len(), 1, "Searching by filename keyword must return 1 hit");`.
4. Verification:
   Run using process-scoped environment variables sequentially:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
   $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo test -p quorum-storage --test storage_tests -- --test-threads=2
   cargo test -p quorum-storage --test adversarial_storage_tests -- --test-threads=2
   cargo test -p quorum-storage --test adversarial_stress_tests -- --test-threads=2
   cargo test --manifest-path tests/e2e/Cargo.toml --test test_storage_isolation -- --test-threads=2
   cargo test --workspace -- --test-threads=2
   cargo clippy --workspace --all-targets -- -D warnings
   ```
5. Write your handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_2\handoff.md` and notify orchestrator (`e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`) via `send_message`.


## 2026-10-31T19:07:57Z
**Context**: Milestone 2 Final Remediation Status Check
**Content**: Checking in on sequential test suite execution progress (`storage_tests`, `adversarial_storage_tests`, `adversarial_stress_tests`, `test_storage_isolation`, `cargo test --workspace`, `cargo clippy`). How are the test runs progressing?
**Action**: Please update `progress.md` with your latest test milestones or handoff status.


## 2026-10-31T19:37:54Z
**Context**: Milestone 2 Final Remediation Workspace Tests Check
**Content**: Checking in on `cargo test --workspace -- --test-threads=2` and workspace clippy execution. Are workspace tests concluding?
**Action**: Please update `progress.md` with current workspace test status or handoff report.


## 2026-10-31T19:57:51Z
**Context**: Milestone 2 Final Remediation Test Execution Status Query
**Content**: Checking in on `cargo test --workspace` (task-198) and clippy. Has task-198 completed, and is clippy running?
**Action**: Please update `progress.md` with current workspace test / clippy results or handoff status.
