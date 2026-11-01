# Dispatch: Explorer M3-1 (Crawler, Watcher & Magika Routing)

## 2026-10-31T20:00:00Z
- **Role**: teamwork_explorer
- **Milestone**: Milestone 3 (Filesystem Ingestion, OS Watcher, Image & Reindex Engine)
- **Angle**: Crawler, Watcher, Google Magika Routing & File Filtering
- **Working Directory**: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_1\
- **Mandatory References**:
  - `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` (Read in full!)
  - `d:\harfile\QuorumRAG\PROJECT.md`
  - `d:\harfile\QuorumRAG\crates\quorum-types\`
  - `d:\harfile\QuorumRAG\crates\quorum-classifier\`
  - `d:\harfile\QuorumRAG\crates\quorum-storage\`
  - `d:\harfile\QuorumRAG\tests\e2e\test_ingestion_dedup.rs`
- **Objectives**:
  1. Design `crates/quorum-ingest/src/crawler.rs` using `ignore::WalkBuilder` / `WalkParallel` respecting `.gitignore`, `.ignore`, hidden files, configurable max file size caps (default 50MB), and routing through Google Magika (`quorum-classifier`).
  2. Design `crates/quorum-ingest/src/watcher.rs` using `notify` (or `notify-debouncer-full`) with configurable debounce (e.g., 300ms) handling Create, Modify, Rename, and Remove events. Ensure non-blocking async event channels into the ingestion coordinator.
  3. Formulate the exact integration with `quorum_classifier::FileTypeClassifier` for content-type detection before chunking or binary filtering.
  4. Write a comprehensive technical report to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_1\handoff.md`.
## 2026-10-31T20:03:51Z
You are Explorer M3-1 for QuorumRAG Milestone 3 (Filesystem Ingestion, OS Watcher, Image & Reindex Engine).
Your working directory is `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_1\`.
Read your task assignment in `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_1\DISPATCH.md`.
Read `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` (mandatory!), `PROJECT.md`, `crates/quorum-classifier`, `crates/quorum-storage`, and `tests/e2e/test_ingestion_dedup.rs`.

Investigate and provide a technical architecture for:
1. `crawler.rs`: High-capacity recursive directory walker using `ignore::WalkParallel` with `.gitignore`, `.ignore`, hidden file skipping, size limits (max 50MB), and routing through Google Magika (`quorum-classifier`).
2. `watcher.rs`: Continuous OS file watcher (`notify` or `notify-debouncer-full`) processing events (Create, Modify, Remove, Rename) with 300ms debouncing and non-blocking channels.
3. Content-type classification and binary filtering via `quorum_classifier::FileTypeClassifier`.

Write your full report to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_1\handoff.md` and send a message when done. Do NOT write source code files to `crates/`.
