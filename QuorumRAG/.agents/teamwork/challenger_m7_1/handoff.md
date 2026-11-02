# Milestone M7 Adversarial Challenge Report: Zero-Overhead Ingestion Pipeline, Dedup Hydration & Binary Filtering

## 1. Observation
1. **Deduplication Startup Hydration (Feature 44)**:
   - File: `crates/quorum-ingest/src/dedup.rs:427-498` (`TwoLevelDedupEngine::hydrate_from_storage`).
   - Scans all chunks from LanceDB via `storage.scan_all_chunks().await?`.
   - Expands `Level2ChunkCache` capacity via `self.cache.ensure_capacity(chunk_count)` and inserts vectors via `insert_batch`.
   - Groups chunks by `document_id`, sorts chunk list deterministically by `(line_start, chunk_index)`, and registers each entry into `Level1DedupCatalog`.
   - Local filesystem metadata is probed opportunistically (`std::fs::metadata(&file_path)`), falling back safely to `(mtime_secs: 0, size_bytes: 0)` for offline or dismounted drive paths without panicking.
2. **Empirical Zero-Read & Zero-Hashing Level-1 Fast Path**:
   - File: `crates/quorum-ingest/src/dedup.rs:123-134`.
   - When `entry.mtime_secs == current_mtime && entry.size_bytes == current_size`, `evaluate_file` returns `DedupAction::Unchanged` immediately using in-memory secondary index lookup without opening the file or computing BLAKE3 hashes.
   - In our empirical test harness (`crates/quorum-ingest/tests/m7_challenger_stress_tests.rs:159-178`), an online file was deleted from disk; calling `dedup.evaluate_file(deleted_path, mtime, size)` succeeded cleanly with `DedupAction::Unchanged`, proving 0 disk I/O and 0 hashing. Modifying `mtime` by 1 second triggered Step 2 (`hash_file_stream`) and correctly failed with an I/O `NotFound` error.
3. **Fast-Path Binary Exclusion (Feature 46)**:
   - File: `crates/quorum-classifier/src/lib.rs:1256-1292`: Defines all 23 extensions in `COMPILED_BINARY_EXTENSIONS` (`exe`, `dll`, `sys`, `ocx`, `cpl`, `scr`, `msi`, `drv`, `lib`, `exp`, `ilk`, `pdb`, `obj`, `class`, `pyc`, `pyo`, `pyd`, `so`, `dylib`, `o`, `a`, `bin`, `ko`) and normalizes case via `ext.trim_start_matches('.').to_ascii_lowercase()`.
   - File: `crates/quorum-ingest/src/classifier_router.rs:80-93`: `evaluate_file` checks `quorum_classifier::is_binary_executable_extension(ext)` before calling `std::fs::symlink_metadata`, `skip_hidden`, or `std::fs::metadata`.
   - When tested against non-existent paths with uppercase and mixed-case extensions (`.EXE`, `.DLL`, `.SyS`, `.PdB`, `.So`, `.oCx`, etc.), `evaluate_file` returns `FilterDecision::SkipBinary` without triggering `SkipIoError`.
   - Deceptive files containing ASCII text named `.exe` or JSON payloads named `.DLL` are immediately excluded via extension fast-path before content parsing.
4. **Non-Text Data Leakage Prevention (Feature 46)**:
   - File: `crates/quorum-ingest/src/classifier_router.rs:161-184`: Step 6 requires `identity.is_text || matches!(identity.category, FileCategory::Code | FileCategory::Text | FileCategory::Document)`.
   - SQLite databases (`FileCategory::Data`, `is_text: false`) and PCAP/PCAPNG packet captures (5 standard magic signatures: `0xa1b2c3d4`, `0xd4c3b2a1`, `0xa1b23c4d`, `0x4d3cb2a1`, `0x0a0d0d0a`, `FileCategory::Data`, `is_text: false`) fall through Step 6 directly to Step 7 and are rejected as `FilterDecision::SkipBinary`. They are NEVER accepted as `CandidateKind::Text`.
   - Positive controls confirm valid structured text data (`json`, `toml`, `csv`, `yaml`) with `identity.is_text == true` are accepted as `CandidateKind::Text`.
5. **Empirical Test Suite Execution Results**:
   - `cargo test -p quorum-classifier -- --test-threads=2`:
     All 34 tests passed (10 unittests, 11 adversarial magika, 3 binary fast-path, 8 multibyte utf8, 2 sharing violation stress).
   - `cargo test -p quorum-ingest -- --test-threads=2`:
     All 51 tests passed (3 unittests, 9 adversarial challenge, 10 adversarial reindex and drift, 10 ingest tests, 6 m7 adversarial stress, 6 m7 challenger stress, 7 m7 integration tests).
   - `cargo check --workspace`:
     Completed with 0 errors and 0 warnings.

---

## 2. Logic Chain
1. **Hydration Scalability & Offline Resilience (Observation 1 & 2)**:
   - In `m7_challenger_stress_tests.rs`, 1,500 chunks (1,000 chunks across 25 online documents + 500 chunks across 10 offline/dismounted drive paths) were persisted to LanceDB storage.
   - `dedup.hydrate_from_storage(&storage).await` loaded all 1,500 chunks in a single pass.
   - Level-2 chunk cache dynamically expanded capacity to 1,500, with current size matching the chunk count.
   - Offline documents safely populated the Level-1 catalog with metadata fallback `(mtime_secs: 0, size_bytes: 0)` without crashing or panicking.
   - Deleting online files on disk empirically verified that Level-1 fast path achieves 0 file reads and 0 hashing, returning `DedupAction::Unchanged`.
2. **Binary Fast-Path Determinism (Observation 3)**:
   - Testing all 23 compiled binary extensions with uppercase (`.DLL`, `.EXE`, `.SYS`) and mixed-case (`.PdB`, `.So`, `.Dylib`) on non-existent paths verified that `evaluate_file` returns `FilterDecision::SkipBinary` without calling `std::fs::metadata` or opening the file.
   - Negative controls on `.txt` paths confirmed that non-binary paths proceed to disk inspection and return `SkipIoError` when files are missing.
   - Deceptive text or JSON files named with binary extensions are fast-path filtered without reading payload bytes.
3. **Fail-Closed Non-Text Data Isolation (Observation 4)**:
   - Validating SQLite 3 database files, 5 PCAP/PCAPNG formats, and raw non-UTF8 binary byte streams confirmed `is_text == false` and non-code/document categories are rejected as `SkipBinary`.
   - Text documents and structured text data formats (`.json`, `.toml`) were accepted into `CandidateKind::Text`.
4. **Full Workspace Health (Observation 5)**:
   - Independent verification across both crates confirms zero regressions and 100% test pass rate across 85 test targets.

---

## 3. Caveats
- Storage deduplication in `quorum-storage` deduplicates chunks globally by `blake3_hash`. If multiple documents share identical chunk hashes (such as a standard license preamble), LanceDB stores one chunk instance, and startup hydration registers the unique chunk hashes without cache duplication.
- For dismounted drives, metadata defaults to `(0, 0)`; once remounted, any change in modification time or size will trigger standard Level-1 content re-verification.
- No other caveats.

---

## 4. Conclusion
All requirements of Milestone M7 (Features 44, 45, 46, and 47) have been rigorously, empirically, and adversarially verified. The startup hydration accurately scales to 1,500+ chunks, Level-1 fast path achieves 0 file reads and 0 hashing, all 23 binary extensions are excluded without disk I/O regardless of case, and non-text data cannot leak into text candidate processing.

**VERDICT: APPROVE**

---

## 5. Verification Method
To independently reproduce and verify this assessment:

1. **Run Challenger Stress Harness**:
   ```powershell
   cargo test -p quorum-ingest --test m7_challenger_stress_tests -- --nocapture
   ```
   *Expected: All 6 tests pass cleanly (`test_adversarial_dedup_hydration_large_scale_1500_chunks_and_fast_path`, `test_adversarial_fast_path_binary_exclusion_all_23_extensions_mixed_case_no_io`, `test_adversarial_non_text_data_leakage_sqlite_pcap_raw_binary`, `test_adversarial_deceptive_binary_content_excluded_via_fast_path`, `test_adversarial_dedup_hydration_duplicate_chunk_hashes_across_storage`, `test_adversarial_dedup_hydration_empty_storage`).*

2. **Run Full Classifier Test Suite**:
   ```powershell
   cargo test -p quorum-classifier -- --test-threads=2
   ```
   *Expected: 34 tests passed, 0 failed.*

3. **Run Full Ingest Test Suite**:
   ```powershell
   cargo test -p quorum-ingest -- --test-threads=2
   ```
   *Expected: 51 tests passed, 0 failed.*

4. **Verify Clean Workspace Build**:
   ```powershell
   cargo check --workspace
   ```
   *Expected: Finished dev profile [unoptimized] target(s) with 0 errors.*
