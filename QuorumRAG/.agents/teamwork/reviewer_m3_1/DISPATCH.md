# Dispatch: Reviewer M3-1 (Crawler, Watcher, Magika Routing & Dedup)

## 2026-10-31T21:48:00Z
- **Role**: teamwork_reviewer
- **Milestone**: Milestone 3 (Filesystem Ingestion, OS Watcher, Image & Reindex Engine)
- **Angle**: Crawler, Watcher, Google Magika Routing, Two-Level BLAKE3 Dedup & Arrow Isolation
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m3_1\`
- **Mandatory References**:
  - `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` (Read in full!)
  - `d:\harfile\QuorumRAG\PROJECT.md`
  - `d:\harfile\QuorumRAG\.agents\teamwork\worker_m3_1\handoff.md`
  - `d:\harfile\QuorumRAG\crates\quorum-ingest\src\crawler.rs`
  - `d:\harfile\QuorumRAG\crates\quorum-ingest\src\watcher.rs`
  - `d:\harfile\QuorumRAG\crates\quorum-ingest\src\classifier_router.rs`
  - `d:\harfile\QuorumRAG\crates\quorum-ingest\src\dedup.rs`
  - `d:\harfile\QuorumRAG\tests\e2e\test_ingestion_dedup.rs`
- **Review Objectives**:
  1. Verify `crates/quorum-ingest/src/crawler.rs` properly uses `ignore::WalkParallel` with work-stealing, `.gitignore`, `.ignore`, hidden files, 50MB size caps, symlink defense, and non-blocking collection without runtime panics.
  2. Verify `crates/quorum-ingest/src/watcher.rs` debounces events over a 300ms window, normalizes events, and handles rename/move events properly.
  3. Verify `crates/quorum-ingest/src/classifier_router.rs` routes Google Magika-classified images (`png`, `jpeg`, `webp`, `gif`, `bmp`, `tiff`, `svg`) to `CandidateKind::Image` and NEVER skips them as generic binaries (User Requirement 3!).
  4. Verify `crates/quorum-ingest/src/dedup.rs` enforces Level 1 file catalog (mtime/size fast path, rename/move detection with 0 re-embeddings) and Level 2 chunk cache (`chunk_blake3 -> Vec<f32>`).
  5. Verify Arrow isolation: 0 Arrow or Lance types exposed outside `quorum-storage`.
- **Resource Invariant**: Do NOT run concurrent background builds if another test is running; you may inspect worker verification outputs or run `cargo test -p quorum-ingest --test ingest_tests test_crawler -- --test-threads=1`.
- **Verdict Requirements**: State your clear verdict: **`APPROVE`** or **`REQUEST_CHANGES`** with concrete evidence.
- **Output**: Write your review report to `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m3_1\handoff.md` and send a message with your verdict.
