# Milestone 6 Storage Engine Determinism Remediation Explorer Dispatch (Iteration 4)

## Context & Objectives
You are `explorer_m6_remediation_3`. Your role is **Milestone 6 Storage Remediation Explorer**.
Milestone 6 Phase 2 (Adversarial Coverage Hardening — Tier 5) failed Iteration 3 gating:
1. `auditor_m6_remediation_2` issued a binary veto **`INTEGRITY VIOLATION`**:
   - Check 3 (5 consecutive runs of `cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2`) achieved only 3/5 passes (60%).
   - Run 2 failed: 7/8 passed, 1 panic in `test_tier5_tantivy_reader_reload_coherence_under_rapid_cycles` at cycle 12 (`Immediate search must find chunk at cycle 12`, left: 0, right: 1).
   - Run 3 failed: 5/8 passed, 3 panics in `test_tier5_compaction_and_version_pruning_under_churn`, `test_tier5_hybrid_drift_bidirectional_healing`, and `test_tier5_tantivy_reader_reload_coherence_under_rapid_cycles` at cycle 16.
   - Isolated rerun panicked on iteration 1 at cycle 6.
2. `reviewer_m6_remediation_2` issued **`REQUEST_CHANGES`**:
   - Multi-run testing achieved 4/5 passes in first 5 runs (80%) and 6/8 across extended runs (75%).
   - Multi-crate test suite failed with exit code 1 (`test_tier5_tantivy_reader_reload_coherence_under_rapid_cycles` at cycle 16).
   - Isolated loop failed at cycle 0.
   - Run 8 failed at cycle 11 and line 430 in `test_tier5_acl_access_tier_drift_healing_and_isolation` (`Tier 1 must now retrieve declassified text chunk`, left: 0, right: 1).

## Mandatory Reference Documents
You MUST read the following files before beginning:
- `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (authoritative user request)
- `d:\harfile\QuorumRAG\PROJECT.md` (project architecture and milestone definitions)
- `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\DEAD_ENDS.md` (failed approaches to NEVER repeat)
- `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m6_remediation_2\handoff.md` (full forensic audit evidence)
- `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m6_remediation_2\handoff.md` (full reviewer findings)
- `d:\harfile\QuorumRAG\crates\quorum-storage\src\tantivy_fts.rs` (the target file to fix)
- `d:\harfile\QuorumRAG\crates\quorum-storage\tests\adversarial_tier5_storage_ingest_stress.rs` (the stress test suite)

## Root Causes Identified
1. **Premature Synchronization on Cycle 0**:
   - `last_commit_opstamp` starts at 0.
   - When cycle 0 inserts, `target_gen = 1`, `min_opstamp = 0`.
   - In `reload_with_retry()`, `if target_gen == 0 || meta.opstamp >= min_opstamp` evaluates to `true` (`0 >= 0`).
   - If NTFS file caching serves the pre-commit state on attempt 1, `disk_segments` is `{}` and `searcher_segments` is `{}`.
   - Empty set equality `{}` == `{}` evaluates to `true`! `synced_generation` is marked as 1 and `reload_with_retry()` returns `Ok(())` prematurely.
   - Subsequent `search_text` sees `synced_generation >= target_gen`, skips reload, searches empty index (`num_docs == 0`), and panics with `left: 0, right: 1`.
2. **Persistent Circularity in `disk_segments == searcher_segments`**:
   - Tantivy's `IndexReader::reload()` reads `meta.json` from disk to load segment readers.
   - `self.index.load_metas()` also reads `meta.json` from disk.
   - Comparing `disk_segments == searcher_segments` only proves both saw the same file, not that the writer's commit has flushed to disk or that the searcher has observed the committed data.
3. **Deletions Do Not Add/Change Segments**:
   - In Tantivy, document deletions write deletion bitsets (`.del` files). `meta.segments` does NOT change!
   - Comparing segment sets passes immediately following a deletion commit without verifying whether the reader has reloaded the new deletion bitsets.
4. **Rapid Delete + Insert Sequence (`test_tier5_acl_access_tier_drift_healing_and_isolation:430`)**:
   - `update_chunk_access_tier` performs `delete_by_chunk_id` followed immediately by `index_chunks`.
   - Rapid sequential operations cause subsequent searches to observe stale intermediate states if reload doesn't wait for both operations' commitments to be reflected.

## Scope of Investigation & Tasks
1. Investigate Tantivy 0.27 API and searcher lifecycle:
   - What is the exact contract of `IndexWriter::commit() -> Result<Opstamp>`?
   - How can `TantivyFtsIndex` track a strictly monotonic commit opstamp / generation?
   - In `reload_with_retry()`:
     - How to guarantee that on cycle 0 or whenever documents exist, reload waits until the searcher actually reflects the commit (e.g. `searcher.num_docs() >= expected_min` or verifying the searcher's segment readers match the committed opstamp)?
     - Can `commit_with_retry` return the actual `Opstamp` and save it? When does `index.load_metas()?.opstamp` advance?
     - How does Tantivy's `IndexReader` know when to reload? Tantivy's `IndexReader::reload()` rebuilds its internal `Searcher`. If we reload the reader, how can we test that the new searcher is active?
     - Can we store an internal monotonic atomic counter for commits (e.g., `last_commit_num: AtomicU64`) and ensure reader reload synchronizes deterministically?
2. Ensure `compact()` continues to query `searchable_segment_ids()?` directly from index metadata and retry merges cleanly without deadlock or lock contention.
3. Ensure no Tokio runtime thread starvation (use cooperative yield / backoff, no blocking millisecond sleep on async threads).
4. Author a complete, drop-in replacement file `proposed_tantivy_fts.rs` and unified patch in your working directory `.agents/teamwork/explorer_m6_remediation_3/`.
5. Author a detailed `handoff.md` with:
   - Observation: Analysis of the failure modes.
   - Logic Chain: Why the proposed synchronization guarantees 100% determinism.
   - Verification Method: Exact commands to verify clippy, 10 consecutive runs of `adversarial_tier5_storage_ingest_stress`, and multi-crate tests.


## 2026-11-01T09:24:55Z
From: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Content: You are explorer_m6_remediation_3. Your role is Milestone 6 Storage Remediation Explorer.
Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_3\.
Read your instructions in d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_3\DISPATCH.md and all references listed therein.
Your mission is to investigate crates/quorum-storage/src/tantivy_fts.rs and design a truly foolproof, non-flaky Tantivy reload synchronization mechanism that eliminates circular checks, fixes cycle 0 empty-set matching, guarantees deletion visibility, and achieves 100% deterministic passes across consecutive runs under --test-threads=2.
Deliver proposed_tantivy_fts.rs and handoff.md in your working directory, and notify parent via send_message.


## 2026-11-01T09:30:10Z
From: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Content: **Context**: Milestone 6 Iteration 4 Storage Engine Determinism Remediation
**Content**: Status check on your investigation of `crates/quorum-storage/src/tantivy_fts.rs`. How is the analysis of the non-circular Tantivy reload synchronization progressing?
**Action**: Please report your current progress, key insights, and estimated delivery time for `proposed_tantivy_fts.rs` and `handoff.md`.


## 2026-11-01T09:41:41Z
From: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Content: **Context**: Milestone 6 Iteration 4 Storage Engine Determinism Remediation
**Content**: Status inquiry following up on your 09:31Z update. Have you finalized the non-circular Tantivy searcher synchronization solution and verified the test suite?
**Action**: Please report your progress and estimated time to deliver `proposed_tantivy_fts.rs` and `handoff.md`.
