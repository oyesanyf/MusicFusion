## 2026-11-01T08:36:15Z

# Task Assignment: Milestone 6 Storage Engine Remediation Exploration (Iteration 3)

- **Agent Name**: explorer_m6_remediation_2
- **Role**: Milestone 6 Storage Remediation Explorer
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_2\`
- **Project Scope**: `d:\harfile\QuorumRAG\PROJECT.md`
- **Original User Request**: `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`
- **Dead Ends to Avoid**: `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\DEAD_ENDS.md`
- **Reviewer Handoff Report**: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m6_remediation_1\handoff.md`
- **Current Target File**: `crates/quorum-storage/src/tantivy_fts.rs` (and `crates/quorum-storage/src/engine.rs` if needed)
- **Failing Test Target**: `crates/quorum-storage/tests/adversarial_tier5_storage_ingest_stress.rs`

## Mission & Investigation Objectives
`reviewer_m6_remediation_1` executed independent verification of commit `699170a` and issued `REQUEST_CHANGES` because 3 out of 5 consecutive runs of `cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2` failed with intermittent flakes:
1. `test_tier5_tantivy_reader_reload_coherence_under_rapid_cycles`:
   - `assertion left == right failed: Immediate search must find chunk at cycle N` (left: 0, right: 1).
2. `test_tier5_compaction_and_version_pruning_under_churn`:
   - `Tantivy segments must merge` (line 525) or `Surviving chunks must be retrievable` (line 533).
3. `test_tier5_hybrid_drift_bidirectional_healing`:
   - `assertion left == right failed: Tantivy must contain all 20 chunks` (left: 15, right: 20).
4. Storage & Dependent Crate Regression Suite:
   - `cargo test -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types -- --test-threads=2` failed with exit code 1.

## Root Causes Identified by Reviewer
1. **Circular Synchronization in `reload_with_retry` (`tantivy_fts.rs:594-645`)**:
   - `self.reader.reload()?` calls Tantivy's internal `Index::searchable_segments()`, which reads `meta.json`.
   - Comparing `self.index.load_metas()` with `self.reader.searcher().segment_readers()` is circular because both source from `meta.json`. If Windows file caching serves a stale `meta.json`, both match and the function falsely considers synchronization achieved.
   - When `min_opstamp == 0`, it returns `Ok(())` on attempt 1 without verifying disk synchronization.
   - Fail-open fallthrough: If the loop finishes without meeting the condition, line 643 unconditionally returns `Ok(())`.
2. **Async Tokio Thread Starvation via Synchronous `thread::sleep` (`tantivy_fts.rs:601, 639`)**:
   - Calling `std::thread::sleep` inside synchronous methods called by async Tokio tasks blocks worker threads under `--test-threads=2`, causing latency spikes and timing drift across parallel tests.
3. **Fragile Compaction Segment Consolidation (`tantivy_fts.rs:346-413` & `engine.rs:409-415`)**:
   - In `compact()`, `self.writer` mutex is locked and held while `reload_with_retry` is called.
   - `writer.merge(&segment_ids).wait()` fails if open readers hold Windows file locks on merged segments. When it fails, `compact()` returns `Err`, causing `stats.tantivy_segments_merged = false`.
4. **Reader Searcher Generation Alignment**:
   - Investigate how Tantivy 0.27 handles reader reloads under rapid commit/search cycles on Windows.
   - Investigate why `engine.insert_chunks` followed immediately by `engine.search_text` sometimes sees 0 hits or sees stale searcher state.

## Output Requirements
1. Deliver a comprehensive analysis and diagnosis in `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_2\handoff.md`.
2. Provide a concrete, drop-in replacement file `proposed_tantivy_fts.rs` (and any patches to `engine.rs` or tests if appropriate) that completely solves the reader reload desynchronization, circular validation, thread starvation, and compaction merge issues.
3. Verify via read-only analysis that the proposed fix achieves 100% deterministic reliability under concurrent execution (`--test-threads=2`) across all 5 consecutive runs.

## 2026-11-01T08:48:15Z
[Message from Parent e13403c5-b0e6-48a4-a1ec-a1f096f9f53e]
Status inquiry: Please share your current progress on the Tantivy reload and compaction analysis, and estimated completion time for handoff.md and proposed solution code.
