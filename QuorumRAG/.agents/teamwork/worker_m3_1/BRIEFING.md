# BRIEFING — 2026-10-31T20:25:00Z

## Mission
Implement Milestone 3 for QuorumRAG: `crates/quorum-ingest` containing the Filesystem Crawler (`ignore::WalkParallel`), Real-Time OS Watcher (`notify` with 300ms debouncing), Google Magika Content Router (preserving and routing images to Image Indexer, text/code to Chunker, filtering binaries), Two-Level BLAKE3 Deduplication Engine (Level 1 file catalog with rename/move detection, Level 2 chunk vector cache), Line-Aware UTF-8 Safe Chunker, Image Indexer (EXIF/metadata/visual text representations), Dedicated Reindex Worker (`reindex`), Comprehensive 5-Drift Remediation Worker (`fix-drift`), and Unified `IngestEngine` supervisor.

## 🔒 My Identity
- Archetype: teamwork_worker
- Roles: implementer, qa, specialist
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork\worker_m3_1\
- Original parent: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
- Milestone: Milestone 3 (Filesystem Ingestion, OS Watcher, Image & Reindex Engine)

## 🔒 Key Constraints
- Preserve `jobs = 2` in `.cargo/config.toml` and `debug = 0` in `Cargo.toml`.
- Always run tests with `-- --test-threads=2`.
- Do NOT leak Arrow types across `crates/quorum-storage` boundary.
- DO NOT CHEAT: No mocks, no hardcoded test outputs, genuine implementation of two-level dedup, chunker, image indexer, crawler, watcher, reindexer, and 5-drift healer.
- Comply with all user rules: automatic git commit/push invariant, zero environment mutation, windows AppContainer / path safety invariants.

## Current Parent
- Conversation ID: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
- Updated: 2026-10-31T20:25:00Z

## Task Summary
- **What to build**:
  - `crates/quorum-ingest/Cargo.toml` with dependencies
  - Add `crates/quorum-ingest` to root `Cargo.toml`
  - In `crates/quorum-ingest/src/`:
    - `error.rs`: IngestError enum
    - `config.rs`: IngestConfig, ChunkerConfig, ReindexOptions, DriftFixOptions
    - `classifier_router.rs`: Magika routing (Images -> ImageIndexer, Text/Code -> Chunker, Binaries -> Skip)
    - `crawler.rs`: Parallel crawler with `ignore::WalkParallel`
    - `watcher.rs`: OS file watcher with 300ms debouncing window
    - `dedup.rs`: Level-1 file catalog (mtime/size fast path, rename/move detection) and Level-2 chunk vector cache
    - `chunker.rs`: Line-aware, UTF-8 char boundary safe chunker with sliding window
    - `image_indexer.rs`: EXIF/metadata/token/dimension extractor and visual text representation generator
    - `reindex.rs`: ReindexWorker supporting incremental and full rebuild modes
    - `drift_healer.rs`: DriftHealer resolving all 5 drift types (Merkle, Lance<->Tantivy, Vector contract, ACL policy, Storage compaction)
    - `engine.rs`: Unified IngestEngine supervisor
    - `lib.rs`: Clean public exports
  - Unit/integration tests in `crates/quorum-ingest/tests/ingest_tests.rs`
  - Add minimal parity helpers in `crates/quorum-storage` if needed for drift detection without Arrow leakage
- **Success criteria**:
  - `cargo test -p quorum-ingest -- --test-threads=2` passes
  - `cargo test --manifest-path tests/e2e/Cargo.toml --test test_ingestion_dedup -- --test-threads=2` passes
  - `cargo test --workspace -- --test-threads=2` passes
  - `cargo clippy --workspace --all-targets -- -D warnings` passes
  - Handoff report in `handoff.md`

## Key Decisions Made
- Use `ignore = "0.4.22"`, `notify = "6.1.1"`, `notify-debouncer-full = "0.3.1"` for crawler and watcher.
- Image routing explicitly preserves all images (`png`, `jpeg`, `webp`, `gif`, `bmp`, `tiff`, `svg`) and routes them to `CandidateKind::Image`, preventing accidental skipping of binary images.
- Chunker uses newline prefix table `line_starts: Vec<usize>` for $O(\log L)$ binary-search line lookup, ensuring 1-indexed exact lines without panics.
- Deduplication maintains both Level-1 file catalog (mtime/size fast check + rename/move detection with 0 re-embeddings) and Level-2 chunk cache (`chunk_blake3 -> Vec<f32>` bounded LRU cache).
- Add domain-level parity query methods to `StorageEngine` (`scan_all_chunks`, `get_lance_chunk_catalog`, `get_tantivy_chunk_catalog`, `update_chunk_record`, `update_chunk_access_tier`) maintaining strict Arrow isolation.

## Artifact Index
- `crates/quorum-ingest/` — Ingestion, watcher, dedup, image, reindex & drift engine crate
- `crates/quorum-ingest/tests/ingest_tests.rs` — Ingest test suite
- `handoff.md` — Final handoff report

## Change Tracker
- **Files modified**:
  - `crates/quorum-ingest/Cargo.toml`: Package configuration with required dependencies
  - `Cargo.toml`: Added `crates/quorum-ingest` to workspace members
  - `crates/quorum-ingest/src/lib.rs`: Public API re-exports
  - `crates/quorum-ingest/src/error.rs`: Typed `IngestError` enum
  - `crates/quorum-ingest/src/config.rs`: IngestConfig, ChunkerConfig, ReindexOptions, DriftFixOptions
  - `crates/quorum-ingest/src/classifier_router.rs`: Magika routing, image preservation, size cap
  - `crates/quorum-ingest/src/crawler.rs`: Parallel crawler with gitignore, hidden filter, sync collector
  - `crates/quorum-ingest/src/watcher.rs`: Debounced OS watcher (300ms window) with overflow recovery
  - `crates/quorum-ingest/src/dedup.rs`: Two-Level BLAKE3 dedup (L1 file catalog + L2 chunk vector cache)
  - `crates/quorum-ingest/src/chunker.rs`: Multibyte UTF-8 safe chunker with 1-indexed line numbers
  - `crates/quorum-ingest/src/image_indexer.rs`: EXIF extraction and visual text representations
  - `crates/quorum-ingest/src/reindex.rs`: Dedicated reindex worker (incremental and --full)
  - `crates/quorum-ingest/src/drift_healer.rs`: Automated 5-drift remediation engine
  - `crates/quorum-ingest/src/engine.rs`: Unified IngestEngine supervisor
  - `crates/quorum-ingest/tests/ingest_tests.rs`: 10 integration and unit tests covering all components
  - `crates/quorum-storage/`: Added domain parity helpers without Arrow leakage
- **Build status**: All workspace tests passed cleanly (`10/10` in `quorum-ingest`, `10/10` in `test_ingestion_dedup`, `33/33` in `quorum-storage`, full workspace green)
- **Pending issues**: None

## Quality Status
- **Build/test result**: Pass (0 failures across workspace)
- **Lint status**: Clean (0 warnings under `cargo clippy --workspace --all-targets -- -D warnings`)
- **Tests added/modified**: 10 tests in `crates/quorum-ingest/tests/ingest_tests.rs` covering crawler, watcher, router, dedup, chunker, image indexer, reindex, 5-drift healer, and hybrid search.
