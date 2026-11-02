# Scope: Milestone M7 — Zero-Overhead Ingestion Pipeline, Dedup Hydration & Binary Filtering

## Architecture & Module Boundaries
- Crate Ownership:
  - `crates/quorum-ingest/` (crawler, watcher, dedup, reindex, classifier_router, engine)
  - `crates/quorum-classifier/` (extension classification, fast-path binary rejection)
  - `crates/quorum-storage/` (`scan_all_chunks`, storage hydration reader)

## Features Assigned
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 44 | Dedup Engine Startup Hydration | Hydrate `TwoLevelDedupEngine` (catalog & chunk cache) from LanceDB on startup (`hydrate_from_storage`) | M7 | follow-up (R1) |
| 45 | Complete Physical Drive Discovery | Windows NTFS, ReFS, exFAT, FAT32 fixed volume support, dot-directory blind spot elimination | M7 | follow-up (R1) |
| 46 | Fast-Path Binary/DLL/EXE Exclusion | Deterministic file extension filtering (.exe, .dll, .sys, .so, .dylib, .pdb, .obj, .class, etc.) prior to file I/O | M7 | follow-up (R1) |
| 47 | Watcher Directory & Sharing Fault Resilience | Recursive directory creation/rename support in `watcher.rs` and retry on Windows sharing violations (Error 32) | M7 | follow-up (R1) |

## Implementation Directives
1. **Deduplication Hydration (`hydrate_from_storage`)**:
   - In `TwoLevelDedupEngine` (`crates/quorum-ingest/src/dedup.rs`), implement `hydrate_from_storage(storage: &Arc<StorageEngine>) -> Result<usize, IngestError>`.
   - Iterate over chunks from `storage.scan_all_chunks()`.
   - Insert `(blake3_hash -> vector)` into `Level2ChunkCache`.
   - Group chunks by `document_id` and insert into `Level1DedupCatalog` with `file_path` and `mtime` (or placeholder 0).
   - In `crates/quorum-daemon/src/main.rs`, call `dedup.hydrate_from_storage(&storage).await` right after opening storage.
2. **Physical Drive Discovery**:
   - In `crates/quorum-ingest/src/config.rs:300`, allow `matches!(fs_label.as_str(), "NTFS" | "REFS" | "EXFAT" | "FAT32")`.
   - In `crates/quorum-ingest/src/reindex.rs:585`, remove the blanket skip of `.dot` folders for user data paths, preserving skips for system directories (`$recycle.bin`, `system volume information`).
3. **Deterministic Binary Filtering**:
   - In `crates/quorum-ingest/src/classifier_router.rs` and `crawler.rs`, add immediate fast-path extension filtering for `.exe`, `.dll`, `.sys`, `.ocx`, `.cpl`, `.scr`, `.msi`, `.drv`, `.lib`, `.exp`, `.ilk`, `.pdb`, `.obj`, `.class`, `.pyc`, `.pyo`, `.pyd`, `.so`, `.dylib`, `.o`, `.a`, `.bin`, `.ko`.
   - Return `FilterDecision::SkipBinary` immediately without executing `std::fs::metadata` or `File::open`.
   - Ensure non-text files with `FileCategory::Data` do NOT bypass filtering into `CandidateKind::Text`.
4. **Filesystem Watcher Resilience**:
   - In `crates/quorum-ingest/src/watcher.rs`, handle directory creation and rename events by recursively scanning candidates in the directory.
   - In `classifier_router.rs` and `quorum-classifier`, wrap `File::open` in a retry loop (3 attempts with backoff) to handle Windows Error 32 sharing violations during active writes.
5. **Zero-Overhead Host Impact**:
   - Lower background thread priority (`THREAD_PRIORITY_BELOW_NORMAL` / EcoQoS).
   - Insert `tokio::task::yield_now().await` checkpoints in the reindexing loop.

## Interface Contracts & Layout
- Keep on-disk index storage schemas and public Rust APIs unchanged.
- Pure Rust domain types in `quorum-types` unchanged.

## Survey Reference
- Read `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_gen2_1\handoff.md` for complete file paths and line numbers.
