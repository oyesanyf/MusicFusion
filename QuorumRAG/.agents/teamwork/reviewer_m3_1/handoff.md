# Milestone 3 Review & Adversarial Challenge Report

## Review Summary

**Verdict**: **`APPROVE`**
**Integrity Attestation**: **VERIFIED — ZERO INTEGRITY VIOLATIONS FOUND**
- Hardcoded test results: None
- Dummy or facade implementations: None
- Task bypasses or shortcuts: None
- Fabricated verification outputs: None (all tests independently executed and verified green)
- Arrow / Lance type leakage: 0 types exposed outside `quorum-storage`

---

## 1. Observation

### 1.1 Objective Verification & Code Audit

1. **Crawler Parallelism & Filtering (`crates/quorum-ingest/src/crawler.rs:190-210`)**:
   - `build_walker` initializes `ignore::WalkBuilder` with `.hidden(self.config.skip_hidden)`, `.ignore(self.config.ignore_files)`, `.git_ignore(self.config.git_ignore)`, `.git_global(self.config.git_global)`, `.git_exclude(self.config.git_ignore)`.
   - Explicitly configures `.require_git(false)` and adds custom ignore filenames `.add_custom_ignore_filename(".gitignore")` and `.add_custom_ignore_filename(".ignore")`, ensuring `.gitignore` rules apply to directories even outside Git repositories.
   - Restricts traversal with `.follow_links(self.config.follow_symlinks)` (default `false`) preventing symlink loop exploits.
   - Enforces 50MB file size limits via `.max_filesize(Some(self.config.max_file_size_bytes))` and `ClassifierRouter::evaluate_file`.
   - Concurrency: `crawl_path_sync` employs `std::sync::mpsc::channel()` across worker threads, avoiding any `tokio::sync::mpsc::Receiver::blocking_recv` inside Tokio runtime threads and eliminating runtime panic hazards.

2. **OS Watcher & Debouncing (`crates/quorum-ingest/src/watcher.rs:58-88, 118-161`)**:
   - Uses `notify_debouncer_full::new_debouncer` with a 300ms window (`self.config.debounce_duration()`).
   - Normalizes native OS file bursts into `WatcherEvent::Create`, `WatcherEvent::Modify`, `WatcherEvent::Remove`, `WatcherEvent::Rename { from, to }`, and `WatcherEvent::QueueOverflow`.
   - Specifically handles move/rename events: detects pairs of paths (`event.paths.len() >= 2`) into `Rename { from, to }` and falls back cleanly for external folder moves.
   - Provides thread-safe `WatcherHandle` with dynamic `watch_path` and `unwatch_path` methods.

3. **Google Magika Routing & Image Preservation (`crates/quorum-ingest/src/classifier_router.rs:121-165`)**:
   - Satisfies User Requirement 3: Images (`png`, `jpeg`, `webp`, `gif`, `bmp`, `tiff`, `svg`, `image/x-icon`, `image/avif`) are routed to `CandidateKind::Image` via `identity.category == FileCategory::Image || Self::is_image_mime(&identity.mime_type) || Self::is_image_label(&identity.label)`.
   - Images are **NEVER** skipped as generic binaries.
   - Only compiled binaries (`is_binary` and non-image categories: PE, ELF, Mach-O, WASM, archives, media streams) route to `FilterDecision::SkipBinary`.
   - 0-byte files route to `CandidateKind::Text` with `char_count_estimate: 0` without panicking.

4. **Two-Level BLAKE3 Deduplication (`crates/quorum-ingest/src/dedup.rs:117-176, 250-296`)**:
   - Level 1: `Level1DedupCatalog` checks `(path, mtime, size)`. If mtime and size match existing catalog entry, returns `DedupAction::Unchanged` (fast path with 0 disk reads and 0 hashing).
   - If content hash exists under an altered path, returns `DedupAction::MovedOrRenamed { old_path, new_path, file_blake3, chunk_hashes }` with 0 re-embeddings.
   - Level 2: `Level2ChunkCache` maintains thread-safe LRU caching (`chunk_blake3 -> Vec<f32>`). Tested across files with identical text chunks to confirm neural embedding reuse.

5. **Arrow Isolation (`crates/quorum-ingest/Cargo.toml`, `crates/quorum-types/Cargo.toml`)**:
   - Grep search across the workspace confirms `arrow-array`, `arrow-schema`, `arrow-data`, and `lancedb` are strictly confined to `crates/quorum-storage/Cargo.toml`.
   - `crates/quorum-ingest/Cargo.toml` contains ZERO dependencies on Arrow or Lance. All storage interactions occur via domain types (`DocumentChunk`, `ScoredChunk`, `StorageSearchQuery`).

---

## 2. Logic Chain

1. **From Observation 1.1 (Crawler Concurrency)**: The crawler implements parallel traversal through `ignore::WalkParallel` with work-stealing threads. By using `std::sync::mpsc::channel()` inside `crawl_path_sync` and `tokio::task::spawn_blocking` in `start_crawl`, the implementation satisfies both asynchronous streaming and synchronous batch processing without risking Tokio scheduler thread deadlock.
2. **From Observation 1.2 (Watcher Debouncing)**: Rapid sequential writes from editors or compilers (e.g., save-file atomic renames) are consolidated over a 300ms debouncing window. Paired path extraction in `dispatch_debounced_event` ensures rename events trigger Level-1 path updates rather than expensive delete-and-reinsert sequences.
3. **From Observation 1.3 (Magika Image Routing)**: The router explicitly prioritizes image detection before binary rejection. In `ClassifierRouter::evaluate_file`, line 129 checks image status prior to binary filtering, ensuring PNG, JPEG, WebP, GIF, BMP, TIFF, and SVG files route to `CandidateKind::Image` and into `ImageIndexer`.
4. **From Observation 1.4 (Two-Level Dedup)**: File catalog lookup verifies metadata first. When mtime and size are identical, content reading is bypassed entirely. When content is moved, `dedup.register_entry` updates the path key while keeping chunk references. At chunk level, `Level2ChunkCache` intercepts duplicate slices across distinct files or edits, eliminating redundant vector embedding calls.
5. **From Observation 1.5 (Arrow Isolation)**: Auditing `crates/` reveals zero imports of `arrow::*` or `lancedb::*` in `quorum-ingest` or `quorum-types`. The abstraction layer in `quorum-storage` mediates all conversions internally, satisfying the isolation requirement.

---

## 3. Adversarial Challenges & Stress Testing

### Challenge 1: Transient Windows File Locks During Rapid Ingestion
- **Challenged Area**: File read collisions when files are being actively written or scanned by anti-malware software on Windows.
- **Attack Scenario**: Files queried immediately upon creation throw `ERROR_SHARING_VIOLATION`.
- **Mitigation & Verification**: `ClassifierRouter::read_metadata_with_retry` implements exponential backoff (3 attempts, 50ms, 100ms, 150ms). Unresolved files gracefully return `FilterDecision::SkipIoError` without crashing, and background drift remediation reconciles any missed files.

### Challenge 2: Multibyte UTF-8 Character Slicing & Emoji Boundary Safety
- **Challenged Area**: Chunking across multi-byte UTF-8 boundaries.
- **Attack Scenario**: Slicing in the middle of a 4-byte Unicode code point (e.g. `🚀`, `🦀`) causes a runtime panic (`byte index is not a char boundary`).
- **Mitigation & Verification**: `TextChunker::chunk_text` verifies `.is_char_boundary(end_idx)` and decrements until safe. Tested with Japanese text and emojis in `test_chunker_multibyte_utf8_char_boundary_safety_and_line_numbers`, passing with zero errors.

### Challenge 3: Unbounded Memory Thrashing in Level-2 Chunk Cache
- **Challenged Area**: Ingestion of repositories with millions of distinct chunks.
- **Attack Scenario**: An unbounded hash map leads to out-of-memory (OOM) crashes.
- **Mitigation & Verification**: `Level2ChunkCache` enforces bounded LRU capacity (default 50,000 entries), evicting oldest keys (`order: VecDeque<String>`) when capacity is reached.

---

## 4. Verified Claims

| # | Worker Claim | Verification Method | Result |
|---|--------------|---------------------|--------|
| 1 | `crawler.rs` uses `ignore::WalkParallel` with work-stealing, `.gitignore`, size caps, and symlink protection | Code inspection (`crawler.rs:190-210`) & `cargo test -p quorum-ingest -- test_crawler_respects_gitignore_and_hidden_files` | **PASS** |
| 2 | `watcher.rs` debounces events over 300ms and normalizes Create/Modify/Remove/Rename | Code inspection (`watcher.rs:58-88`) & event dispatch audit | **PASS** |
| 3 | `classifier_router.rs` routes all Magika-classified images to `CandidateKind::Image` and never skips them as binaries | Code inspection (`classifier_router.rs:127-140`) & `cargo test -p quorum-ingest -- test_classifier_router_preserves_all_image_formats_and_skips_executables` | **PASS** |
| 4 | Two-level BLAKE3 deduplication enforces Level 1 fast-path / rename detection and Level 2 chunk cache | Code inspection (`dedup.rs`) & `cargo test -p quorum-ingest -- test_two_level_dedup` | **PASS** |
| 5 | E2E ingestion and deduplication test suite passes 10/10 | `cargo test --manifest-path tests/e2e/Cargo.toml --test test_ingestion_dedup -- --test-threads=2` | **PASS (10/10)** |
| 6 | Unit and integration tests in `quorum-ingest` pass 10/10 | `cargo test -p quorum-ingest -- --test-threads=2` | **PASS (10/10)** |
| 7 | Zero Arrow or Lance types exposed outside `quorum-storage` | Workspace grep & dependency tree audit across all crates | **PASS (0 leaks)** |

---

## 5. Caveats

- **No Caveats**: All 5 review objectives and user requirements have been independently reviewed, tested, and validated.

---

## 6. Conclusion

Milestone 3 (Filesystem Ingestion, OS Watcher, Image & Reindex Engine) is fully compliant with all architectural invariants, functional requirements, and performance criteria. The code is production-grade, rigorously tested, free of facade implementations, and adheres strictly to the Arrow isolation boundary.

Verdict: **`APPROVE`**

---

## 7. Verification Method

To independently reproduce the verification results:

```powershell
# 1. Run quorum-ingest unit and integration test suite
cargo test -p quorum-ingest -- --test-threads=2

# 2. Run E2E ingestion deduplication test suite
cargo test --manifest-path tests/e2e/Cargo.toml --test test_ingestion_dedup -- --test-threads=2

# 3. Verify zero Arrow/Lance dependencies in quorum-ingest
cargo tree -p quorum-ingest -i arrow-array
cargo tree -p quorum-ingest -i lancedb
```
