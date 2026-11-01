# Milestone 6 Storage Engine Determinism Implementation Dispatch (Iteration 4)

## Context & Objectives
You are `worker_m6_remediation_3`. Your role is **Milestone 6 Storage Remediation Worker**.
Your task is to apply the authoritative, mathematically-grounded Tantivy reload synchronization remediation designed by `explorer_m6_remediation_3`, verify that it compiles cleanly with zero clippy warnings, verify that it achieves 100% deterministic passes across 10 consecutive runs of the Tier 5 Adversarial Stress Suite under `--test-threads=2`, verify the multi-crate regression test suites, and commit the verified changes to git.

## Mandatory Reference Documents
You MUST read the following files before taking action:
- `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (authoritative user request)
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_3\handoff.md` (remediation design and architecture)
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_3\proposed_tantivy_fts.rs` (the complete drop-in source code)
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_3\tantivy_fts.patch` (the verified unified patch)
- `d:\harfile\QuorumRAG\crates\quorum-storage\src\tantivy_fts.rs` (your target file)

## Exclusive File Ownership
You have EXCLUSIVE write ownership of:
- `crates/quorum-storage/src/tantivy_fts.rs`

Do NOT touch any files outside this path without explicit reason.

## Implementation Details
1. Apply the changes from `proposed_tantivy_fts.rs` or `tantivy_fts.patch` to `crates/quorum-storage/src/tantivy_fts.rs`.
2. Core mechanism implemented:
   - `last_commit_seq: AtomicU64` starts from on-disk sequence (or 0) and strictly increments on every commit, delete, and compaction.
   - Tantivy commit payload monotonic sequence stamping: `writer.prepare_commit()?.set_payload(&format!("seq:{}", next_seq))`.
   - Inverted reload gate: polls `index.load_metas()` to assert `disk_seq >= target_seq && meta.opstamp >= min_opstamp` *before* calling `reader.reload()`.
   - Post-reload structural segment parity check capturing both segment composition and deletion opstamps.
   - Post-reload semantic invariant gate (`SyncCheck::DocPresent` / `SyncCheck::DocAbsent` / `FilePathAbsent` / `AllCleared`) enforcing that the searcher can actually query the newly committed document or confirm deletion before returning.
   - Lock-free atomic fast path (`synced_seq.load(Acquire) >= target_seq`) returning in ~5ns when no writes have occurred.
   - Fail-closed error return on `MAX_ATTEMPTS` exhaustion (returns `Err(StorageError::Tantivy(...))`).
   - Clean segment compaction merging via `searchable_segment_ids()?` with locks dropped before reload.
   - Cooperative yielding (`std::thread::yield_now()`) with microsecond backoff preventing Tokio thread starvation.

## Mandatory Verification Protocol
Execute the following verification steps in order:

### 1. Strict Compiler Warnings and Clippy
```powershell
cargo clippy -p quorum-storage --tests -- -D warnings
```
Must pass with exit code 0, 0 warnings, 0 errors.

### 2. 10 Consecutive Runs of Tier 5 Adversarial Stress Suite
```powershell
1..10 | ForEach-Object {
    Write-Host "=== TEST RUN $_ OF 10 ===" -ForegroundColor Cyan
    cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2
    if ($LASTEXITCODE -ne 0) { throw "Deterministic failure on Run $_" }
}
```
All 10 runs MUST pass 8/8 tests with 100% determinism (80/80 tests passed). Zero flakes, zero panics.

### 3. Isolated Rapid Reload Coherence 10-Loop Test
```powershell
1..10 | ForEach-Object {
    cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress test_tier5_tantivy_reader_reload_coherence_under_rapid_cycles -- --nocapture
    if ($LASTEXITCODE -ne 0) { throw "Isolated cycle flake on Run $_" }
}
```
All 10 runs MUST pass cleanly.

### 4. Storage Adversarial Stress Suite
```powershell
cargo test -p quorum-storage --test adversarial_stress_tests -- --nocapture
```
All 13 tests must pass.

### 5. Multi-Crate Regression Suite
```powershell
cargo test -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types -- --test-threads=2
```
Must exit with code 0 across all 5 crates.

### 6. Git Commit
Stage and commit:
```powershell
git add crates/quorum-storage/src/tantivy_fts.rs
git commit -m "fix(quorum-storage): monotonic payload sequence and inverted reload gate for Tantivy FTS"
```

### 7. Handoff Report
Write `handoff.md` in `.agents/teamwork/worker_m6_remediation_3/handoff.md` with full verification logs and notify parent via `send_message`.

## MANDATORY INTEGRITY WARNING
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.


## 2026-11-01T09:49:33Z
You are worker_m6_remediation_3. Your role is Milestone 6 Storage Remediation Worker.
Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_remediation_3\.
Read your instructions in d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_remediation_3\DISPATCH.md and all references listed therein.
Apply the Tantivy FTS reload synchronization fix from explorer_m6_remediation_3 to crates/quorum-storage/src/tantivy_fts.rs, verify clippy with 0 warnings, verify 10 consecutive runs of adversarial_tier5_storage_ingest_stress under --test-threads=2, verify multi-crate test suites, commit changes, write handoff.md, and notify parent via send_message.
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## 2026-11-01T10:27:54Z
Context: Milestone 6 Iteration 4 Storage Engine Determinism Remediation
Content: Status inquiry on the 10 consecutive runs of `adversarial_tier5_storage_ingest_stress` under `--test-threads=2` (task-355) and multi-crate regression tests.
Action: Please report your pass count and estimated time for git commit and `handoff.md`.
