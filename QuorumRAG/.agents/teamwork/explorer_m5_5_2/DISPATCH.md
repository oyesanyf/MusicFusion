# Dispatch — Explorer M5.5-2: Ingestion, Storage & Cache Coherence

## Objective
Investigate and design cache invalidation, cache coherence, and index epoch management across `crates/quorum-ingest` and `crates/quorum-storage`:
1. `index_epoch` Management:
   - Where should `index_epoch` (or `cache_epoch`) live? (e.g., in `StorageEngine` metadata or an atomic counter in shared state/coordinator).
   - How to increment `index_epoch` whenever LanceDB / Tantivy indices change:
     - On completion of `quorumrag reindex` (`/api/reindex` or CLI).
     - On completion of `quorumrag fix-drift` (`/api/drift/fix` or CLI).
     - On real-time OS file watcher (`watcher.rs`) detecting and indexing file create/modify/delete events.
2. Invalidation & Cache Warming:
   - Invalidation strategy: Incrementing `cache_epoch` instantly invalidates all queries keyed by older epochs.
   - Selective vs full cache clear: support both `clear()` (drop all entries) and `increment_epoch()` (soft invalidation).
   - Cache Warming Routine (`warm`): How to pre-warm hot embeddings/queries during `reindex` or `recache` by scanning top files / recent queries.
3. Storage & Ingest Parity:
   - Ensure LanceDB and Tantivy FTS operations maintain cache coherence without deadlocks or race conditions.

## Input Files to Inspect
- `crates/quorum-storage/src/lib.rs`
- `crates/quorum-storage/src/engine.rs`
- `crates/quorum-storage/src/tantivy_fts.rs`
- `crates/quorum-ingest/src/lib.rs`
- `crates/quorum-ingest/src/reindex.rs`
- `crates/quorum-ingest/src/drift_healer.rs`
- `crates/quorum-ingest/src/watcher.rs`
- `ORIGINAL_REQUEST.md`

## Output
Write your findings and design report to `.agents/teamwork/explorer_m5_5_2/handoff.md`. Include concrete Rust code snippets for epoch tracking, watcher hook integration, and pre-warming routines.


## 2026-11-01T03:59:47Z
Investigate crates/quorum-storage and crates/quorum-ingest to design cache coherence, index_epoch tracking, invalidation hooks for reindex, fix-drift, and real-time watcher events, plus pre-warming routines. Write your detailed design to d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_2\handoff.md and notify me when complete.
