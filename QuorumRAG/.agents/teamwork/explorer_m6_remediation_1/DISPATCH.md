# Task Assignment: Milestone 6 Storage Engine Remediation Exploration

- **Agent Name**: explorer_m6_remediation_1
- **Role**: Milestone 6 Storage Remediation Explorer
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\`
- **Project Scope**: `d:\harfile\QuorumRAG\PROJECT.md`
- **Original User Request**: `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`
- **Reviewer 1 Report**: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m6_1\handoff.md`

## Mission
Investigate the root causes identified by `reviewer_m6_1` in `crates/quorum-storage/src/tantivy_fts.rs` and design the exact code remediation for `worker_m6_remediation_1`:

### 1. Tantivy Reader Reload Silent Failure & Windows File Contention
- In `crates/quorum-storage/src/tantivy_fts.rs`:
  - `let _ = self.reader.reload();` is called at multiple locations (lines 154, 173, 236, 248, 382, 405, 436, 470, etc.).
  - Under concurrent execution (`--test-threads=2`), Windows file sharing violations (`ERROR_SHARING_VIOLATION` = 32 or `ERROR_ACCESS_DENIED` = 5) during `meta.json` replacements can cause `reader.reload()` to return transient errors. Ignoring the error causes the searcher to retain an outdated index snapshot, returning 0 hits.
  - Reviewer 1 recommended implementing `reload_with_retry(&self.reader)` (10 retry attempts with `15ms * attempts` exponential backoff, matching `commit_with_retry`).
  - Investigate all call sites of `self.reader.reload()` and `self.reader.searcher()` in `tantivy_fts.rs` and verify how to make reload robust across all queries and operations.

### 2. In-Flight Document Dropping upon Writer Recreation
- In `crates/quorum-storage/src/tantivy_fts.rs` lines 130–152:
  - If `writer.add_document()` or `commit_with_retry()` fails with `"index writer was killed"` or `"panicked"`, the code re-creates the writer:
    `*writer = Self::create_writer_inner(&self.index)?;`
  - For `add_document`, only the current document `doc` is re-added; all prior documents in the batch `chunks[..i]` were lost inside the killed writer's buffer.
  - For `commit_with_retry`, the new writer is committed empty, permanently losing the entire batch.
  - Despite dropping documents, `index_chunks` returns `Ok(count)` (reporting successful insertion).
  - Design the exact batch replay logic so that if writer recovery occurs, all chunks in the batch are replayed on the fresh writer without data loss.

### 3. Tantivy Compaction & Segment Consolidation
- In `crates/quorum-storage/src/tantivy_fts.rs` lines 286–308:
  - `compact()` only executes `commit_with_retry(&mut writer)` and `self.reader.reload()`.
  - In `open()`, the writer is initialized with `NoMergePolicy`.
  - Without calling `writer.merge(&segment_ids)` or scheduling segment merges, segment files are never merged or consolidated during `compact()`.
  - Investigate Tantivy 0.22 / 0.23 API for segment merging: inspect `self.reader.searcher().segment_readers()`, collect segment IDs, and design how `compact()` should trigger segment merging.

## Deliverables
Produce a comprehensive handoff report at:
`d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\handoff.md`
containing:
1. Exact file paths, line numbers, and existing code snippets in `tantivy_fts.rs`.
2. Ready-to-apply replacement code for:
   - `reload_with_retry` helper and call-site updates.
   - Batch replay recovery in `index_chunks`.
   - Tantivy segment consolidation in `compact()`.
3. Verification commands to validate that `cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2` passes 8/8 tests reliably across repeated runs.


## 2026-11-01T07:19:50Z
[Message from parent e13403c5-b0e6-48a4-a1ec-a1f096f9f53e]
**Context**: Milestone 6 Storage Engine Remediation Exploration
**Content**: Checking in on your analysis of crates/quorum-storage/src/tantivy_fts.rs and handoff report authoring.
**Action**: Please report your current progress, key code findings, and estimated time to deliver handoff.md.
