# Dispatch: Explorer M3-3 (Dedicated Reindex & Comprehensive 5-Drift Healing Engine)

## 2026-10-31T20:00:00Z
- **Role**: teamwork_explorer
- **Milestone**: Milestone 3 (Filesystem Ingestion, OS Watcher, Image & Reindex Engine)
- **Angle**: Dedicated Reindexing (`reindex`) & Comprehensive Drift Remediation (`fix-drift`)
- **Working Directory**: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_3\
- **Mandatory References**:
  - `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` (Read in full!)
  - `d:\harfile\QuorumRAG\PROJECT.md`
  - `d:\harfile\QuorumRAG\crates\quorum-types\`
  - `d:\harfile\QuorumRAG\crates\quorum-classifier\`
  - `d:\harfile\QuorumRAG\crates\quorum-contract\`
  - `d:\harfile\QuorumRAG\crates\quorum-storage\`
  - `d:\harfile\QuorumRAG\tests\e2e\test_ingestion_dedup.rs`
- **Objectives**:
  1. Design `crates/quorum-ingest/src/reindex.rs`:
     - Dedicated reindex worker supporting both incremental (mtime + Level 1 BLAKE3 comparison) and full rebuild (`--full`) modes.
     - Integration points for CLI (`quorumrag reindex [--path] [--full]`) and REST API (`POST /api/reindex`).
  2. Design `crates/quorum-ingest/src/drift_healer.rs`:
     - Dedicated drift remediation worker supporting CLI (`quorumrag fix-drift`) and REST API (`POST /api/drift/fix`).
     - Detect and repair ALL 5 drift types:
       (a) Filesystem <-> DB Merkle drift (deleted files purged, modified files updated).
       (b) LanceDB <-> Tantivy desync (missing chunks cross-checked and healed to 100% parity).
       (c) Vector contract drift (detects chunks with obsolete contract hash and re-embeds).
       (d) ACL policy drift (detects chunks whose access_tier no longer matches directory rules).
       (e) Storage fragmentation & tombstone drift (triggers LanceDB fragment compaction and Tantivy segment merging).
  3. Design `crates/quorum-ingest/src/engine.rs` / `lib.rs` coordinating crawler, watcher, reindex, dedup, and drift healing into a unified `IngestEngine`.
  4. Write a comprehensive technical report to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_3\handoff.md`.

## 2026-10-31T20:04:06Z
Received orchestrator activation message:
Investigate and provide a technical architecture for:
1. `reindex.rs`: Dedicated reindexing process/worker (`quorumrag reindex [--path <dir>] [--full]`, REST API `POST /api/reindex` + UI trigger) supporting both incremental (mtime + Level 1 BLAKE3 comparison) and full rebuild (`--full`) modes.
2. `drift_healer.rs`: Dedicated drift remediation process/worker (`quorumrag fix-drift`, REST API `POST /api/drift/fix`) detecting and repairing ALL 5 drift types:
   (a) Filesystem <-> DB Merkle drift (deleted files purged, modified files updated)
   (b) LanceDB <-> Tantivy desync (missing chunks cross-checked and healed to 100% parity)
   (c) Vector contract drift (detects chunks with obsolete contract hash and re-embeds)
   (d) ACL policy drift (detects chunks whose access_tier no longer matches directory rules)
   (e) Storage fragmentation & tombstone drift (triggers LanceDB fragment compaction and Tantivy segment merging)
3. `engine.rs` / `lib.rs`: Coordinating crawler, watcher, reindex, dedup, and drift healing into a unified `IngestEngine`.
Write full report to `handoff.md`. Read-only investigation (do NOT write source files to `crates/`).
