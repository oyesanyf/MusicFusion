# Task Assignment: Milestone 6 Storage Engine Remediation Implementation

- **Agent Name**: worker_m6_remediation_1
- **Role**: Milestone 6 Remediation Implementation Worker
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_remediation_1\`
- **Project Scope**: `d:\harfile\QuorumRAG\PROJECT.md`
- **Original User Request**: `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`
- **Explorer Remediation Design**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\handoff.md`
- **Proposed Code File**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\proposed_tantivy_fts.rs`
- **Proposed Patch**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\proposed_tantivy_fts.patch`
- **Reviewer 1 Report**: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m6_1\handoff.md`

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## File Ownership
You exclusively own and may modify:
- `crates/quorum-storage/src/tantivy_fts.rs`

## Tasks to Implement

### Task 1: Apply `proposed_tantivy_fts.rs` / `proposed_tantivy_fts.patch`
Update `crates/quorum-storage/src/tantivy_fts.rs`:
1. **Import `SegmentId`**:
   `use tantivy::{Index, IndexReader, IndexWriter, SegmentId, TantivyDocument, Term};`
2. **Add `chunk_to_doc(&self, c: &DocumentChunk) -> TantivyDocument`**:
   Clean document creation helper.
3. **Implement Batch-Replay Loop in `index_chunks`**:
   Enclose chunk addition and commit inside `'batch_attempt: loop` with `MAX_RECOVERIES = 3`. If writer is killed or panics during `add_document` or `commit_with_retry`, re-instantiate writer and restart batch replay from chunk 0.
4. **Segment Consolidation in `compact()`**:
   Enumerate segments via `self.reader.searcher().segment_readers()`. If `segment_ids.len() >= 2`, call `writer.merge(&segment_ids).wait()`, then commit with retry and reload with retry.
5. **Replace All Bare and Unretried `self.reader.reload()` Calls**:
   Replace all bare `let _ = self.reader.reload();` and unretried `reload()` calls across all 10 call sites with `reload_with_retry(&self.reader)?;`.
6. **Implement `reload_with_retry`**:
   Implement helper with 10 attempts and `15ms * attempts` linear backoff to tolerate Windows transient file locks.

### Task 2: Verification Protocol
Execute and verify each of the following:
1. `cargo clippy -p quorum-storage --tests -- -D warnings` (must be 0 warnings, 0 errors).
2. `cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2` (run 5 consecutive times to confirm 100% deterministic reliability under concurrent execution).
3. `cargo test -p quorum-storage --test adversarial_stress_tests -- --nocapture` (verify all 13 tests pass cleanly).
4. `cargo test -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types -- --test-threads=2` (all 5 storage-related crates pass cleanly).
5. `git diff --stat` to verify only `crates/quorum-storage/src/tantivy_fts.rs` was modified.
6. Commit changes with a descriptive git message per project rules.

## Deliverables
Write your handoff report to:
`d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_remediation_1\handoff.md`
Report verification commands, exact results, and git commit hash, then send a completion message to the orchestrator.

## 2026-11-01T07:24:14Z
From: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Content:
You are worker_m6_remediation_1, the Milestone 6 Storage Remediation Worker.
Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_remediation_1\
Read d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_remediation_1\DISPATCH.md and d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

You exclusively own: `crates/quorum-storage/src/tantivy_fts.rs`.
Explorer proposed replacement code and unified patch are at:
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\proposed_tantivy_fts.rs`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\proposed_tantivy_fts.patch`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\handoff.md`

Tasks:
1. Apply the proposed fixes to `crates/quorum-storage/src/tantivy_fts.rs` (`reload_with_retry` across all 10 call sites, batch replay on writer recovery in `index_chunks`, segment consolidation in `compact()`, `SegmentId` import).
2. Run `cargo clippy -p quorum-storage --tests -- -D warnings` (must be 0 warnings, 0 errors).
3. Run `cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2` (run 5 consecutive times to confirm 100% deterministic reliability under concurrent execution).
4. Run `cargo test -p quorum-storage --test adversarial_stress_tests -- --nocapture` (verify all 13 tests pass cleanly).
5. Run `cargo test -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types -- --test-threads=2`.
6. Verify only `crates/quorum-storage/src/tantivy_fts.rs` was modified (`git diff --stat`). Commit changes.
7. Write your handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_remediation_1\handoff.md` and send a completion message to the orchestrator.

## 2026-11-01T07:40:28Z
From: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Content:
**Context**: Milestone 6 Storage Engine Remediation Verification
**Content**: Checking in on your verification test runs (5x consecutive passes of adversarial_tier5_storage_ingest_stress, adversarial_stress_tests, and storage crate test suite).
**Action**: Please report your current test pass counts, any observed failures or regressions, and estimated time to deliver handoff.md.
## 2026-11-01T07:54:47Z
From: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Content:
**Context**: Milestone 6 Storage Engine Remediation Verification
**Content**: Checking in on the 5 consecutive runs of adversarial_tier5_storage_ingest_stress and storage crate test suite execution.
**Action**: Please report how many of the 5 runs have completed, results so far, and your estimated time to deliver handoff.md.
## 2026-11-01T08:04:58Z
From: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Content:
**Context**: Milestone 6 Storage Engine Remediation Verification
**Content**: Checking in on the 5x consecutive verification test execution and handoff progress.
**Action**: Please report which run you are currently on, results so far, and estimated time to deliver handoff.md.
