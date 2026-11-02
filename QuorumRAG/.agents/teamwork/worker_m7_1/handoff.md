# Milestone M7 Handoff Report: Zero-Overhead Ingestion Pipeline, Dedup Hydration & Binary Filtering

## 1. Observation
1. **Upstream Compiler Defect**:
   - `crates/quorum-coordinator/src/agentic/synthesize.rs:8:33`: `synthesize_with_history` expects 6 arguments (`config`, `plan`, `contract`, `executor`, `history`, `telemetry_tx`), but only 5 arguments were supplied (`telemetry_tx` missing).
2. **Feature 44 (Deduplication Startup Hydration)**:
   - `crates/quorum-ingest/src/dedup.rs`: `TwoLevelDedupEngine` had no persistence bridge to LanceDB storage. On daemon startup, `TwoLevelDedupEngine` initialized with empty catalogs, causing the crawler to re-read and re-hash existing files on every startup.
   - `crates/quorum-daemon/src/main.rs:72`: Daemon initialized `TwoLevelDedupEngine::new()` without invoking hydration before starting background crawler or reindexing.
3. **Feature 45 (Physical Drive Discovery & Blind Spot Elimination)**:
   - `crates/quorum-ingest/src/config.rs:300`: `detect_fixed_drives()` checked only `matches!(fs_label.as_str(), "NTFS" | "REFS")`, silently ignoring valid fixed physical drives formatted with FAT32 or EXFAT.
   - `crates/quorum-ingest/src/reindex.rs:598`: `expand_drive_roots` previously performed a blanket skip on all directories starting with `.`, skipping critical top-level hidden user directories like `D:\.workspace`.
   - `crates/quorum-ingest/src/reindex.rs:616-630`: Overlapping slice duplication occurred for `probe_and_tools` within `mechanistic_interpretability`.
4. **Feature 46 (Fast-Path Deterministic Binary/DLL/EXE Exclusion)**:
   - `crates/quorum-classifier/src/lib.rs`: Only 4 extensions were mapped to `Executable`. Ingest pipeline opened and read the first 512 bytes of every `.dll`, `.exe`, `.sys`, `.pdb`, and `.so` on disk.
   - `crates/quorum-ingest/src/classifier_router.rs`: Non-text files categorized as `FileCategory::Data` (such as SQLite `.db` or binary archives) leaked into `CandidateKind::Text` if `identity.is_text` was false.
5. **Feature 47 (Watcher Resilience & Zero-Overhead Host Throttling)**:
   - `crates/quorum-ingest/src/watcher.rs:163`: `tx.try_send` silently dropped events on channel saturation without raising `WatcherEvent::QueueOverflow`. Directory creation and deletion did not recursively discover or purge nested children.
   - `crates/quorum-ingest/src/engine.rs:271`: Directory renames were handled as single file removals, leaving stale chunks in storage and missing files under the new path.
   - Windows Error 32 sharing violations caused immediate ingestion aborts during active user editing or background file flushing.
   - Crawler and reindexer loops ran unthrottled without background thread priority or cooperative async yields.

---

## 2. Logic Chain
1. **Resolving Agentic Synthesize Compiler Breakage**:
   - Observation 1.1 showed a missing 6th argument `telemetry_tx` in `agentic/synthesize.rs`.
   - Passing `None` (`Option<&Sender<AgenticTelemetryEvent>>`) restores clean compilation without breaking any coordinator contract.
2. **Implementing Startup Dedup Hydration (Feature 44)**:
   - Observation 2.1 showed empty dedup catalogs on startup.
   - Adding `TwoLevelDedupEngine::hydrate_from_storage`:
     a. Scans all stored chunks from LanceDB via `storage.scan_all_chunks().await?`.
     b. Expands `Level2ChunkCache` capacity via `ensure_capacity` and inserts all chunk hashes and vectors via `insert_batch`.
     c. Groups chunks by `document_id`, sorting chunks deterministically by `(line_start, chunk_index)`.
     d. Probes filesystem metadata `(mtime_secs, size_bytes)` for online files (falling back to `(0, 0)` for offline drives) to prime Level-1 Fast Path hits.
     e. Wires `dedup.hydrate_from_storage(&storage).await?` into `crates/quorum-daemon/src/main.rs` and standalone reindexer/drift healing CLI entrypoints.
3. **Physical Drive Discovery & Blind Spot Elimination (Feature 45)**:
   - Updating `detect_fixed_drives()` in `config.rs` to match `"NTFS" | "REFS" | "EXFAT" | "FAT32"` allows exFAT/FAT32 physical data drives to be indexed.
   - In `reindex.rs:expand_drive_roots`, removing the blanket dot-prefix skip while adding `.git`, `.lance`, `.tantivy`, `.quorumrag-live-data`, and `.pytest_cache` to `SKIP_DIRS` ensures user project directories (`.workspace`) are fully indexed without indexing engine metadata or repository noise.
   - Excluding `probe_dir` from the parent slice in `mechanistic_interpretability` and adding `HashSet` deduplication in `expand_drive_roots` eliminates redundant work.
4. **Fast-Path Binary Exclusion & Non-Text Data Fix (Feature 46)**:
   - Defining `COMPILED_BINARY_EXTENSIONS` (23 extensions: `exe`, `dll`, `sys`, `ocx`, `cpl`, `scr`, `msi`, `drv`, `lib`, `exp`, `ilk`, `pdb`, `obj`, `class`, `pyc`, `pyo`, `pyd`, `so`, `dylib`, `o`, `a`, `bin`, `ko`) and `is_binary_executable_extension` in `quorum-classifier`.
   - In `classifier_router.rs`, checking `is_binary_executable_extension` at the very top of `evaluate_file` returns `FilterDecision::SkipBinary` before any file open or `std::fs::metadata`.
   - In `crawler.rs`, checking `is_binary_executable_extension` in `filter_entry` skips binary files during filesystem walking.
   - In `classifier_router.rs` Step 6, requiring `identity.is_text || matches!(identity.category, FileCategory::Code | FileCategory::Text | FileCategory::Document)` prevents non-text `Data` category files (e.g. SQLite `.db`) from leaking to `CandidateKind::Text`.
5. **Watcher Resilience & Zero-Overhead Throttling (Feature 47)**:
   - In `watcher.rs`, `send_event` emits `WatcherEvent::QueueOverflow` on channel saturation and spawns a fallback async task to prevent event loss. Added `dispatch_dir_files` for recursive directory emission.
   - In `engine.rs`, `handle_watcher_event` purges all tracked files starting with `path` upon directory deletion, and upon directory rename purges the old prefix from catalog/storage and recursively ingests new files.
   - In `quorum-classifier` and `quorum-ingest`, implemented `open_file_with_retry` and `classify_file_with_retry` with exponential backoff (`[25ms, 50ms, 100ms]`) for Windows Error 32 sharing violations.
   - Created `crates/quorum-ingest/src/throttling.rs` setting Win32 background thread priority (`THREAD_MODE_BACKGROUND_BEGIN` and `THREAD_PRIORITY_BELOW_NORMAL`) for crawler/reindexer threads.
   - Added cooperative `tokio::task::yield_now().await` in reindexer loops and batch flushes.

---

## 3. Caveats
- Host background priority (`set_background_thread_priority`) relies on Windows API `SetThreadPriority`. On non-Windows platforms, a safe no-op fallback is compiled.
- For offline storage chunks where the drive or volume is dismounted, `hydrate_from_storage` records `mtime: 0, size: 0` in the Level-1 catalog; when the drive is remounted, the first evaluation will trigger content hash verification and update metadata.
- No other caveats.

---

## 4. Conclusion
Milestone M7 is completely implemented, verified, and passing all tests without regressions across all workspace crates:
- Zero-overhead startup hydration reconstructs both Level-1 File Catalog and Level-2 Chunk Cache directly from LanceDB storage.
- Physical drives with FAT32/EXFAT are discovered; user dot-directories are indexed; slice duplication is eliminated.
- 23 compiled binary extensions are deterministically excluded via fast-path without file I/O or `metadata` overhead.
- Non-text Data files cannot leak into text candidate processing.
- OS watcher handles directory creation, removal, and rename recursively without dropping events.
- Background worker threads run under low priority and yield cooperatively without host saturation.

---

## 5. Verification Method
1. **Workspace Compilation**:
   ```powershell
   cargo check --workspace
   ```
   *Expected: Finished dev profile [unoptimized] target(s) with 0 errors.*
2. **Classifier Unit & Adversarial Test Suites**:
   ```powershell
   cargo test -p quorum-classifier
   ```
   *Expected: All 32 tests pass (unittests, adversarial_magika_tests, binary_fast_path_tests, corner_cases_multibyte_utf8).*
3. **M7 Integration Test Suite**:
   ```powershell
   cargo test -p quorum-ingest --test m7_integration_tests
   ```
   *Expected: All 7 tests pass (`test_dedup_startup_hydration_from_storage`, `test_dedup_startup_hydration_empty_storage`, `test_dedup_startup_hydration_offline_drive_fallback`, `test_classifier_router_fast_path_skips_without_file_io`, `test_non_text_data_does_not_leak_to_candidate_text`, `test_reindex_expand_drive_roots_does_not_skip_user_dot_folders`, `test_watcher_directory_recursive_ingest_and_rename`).*
4. **Complete Ingest Test Suites**:
   ```powershell
   cargo test -p quorum-ingest -- --test-threads=2
   ```
   *Expected: All 39 tests pass across `ingest_tests`, `adversarial_challenge_tests`, `adversarial_reindex_and_drift`, and `m7_integration_tests`.*
