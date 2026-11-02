# Handoff Report: Explorer M8.5 3 (Test Suites & Acceptance Explorer)

## 1. Observation

### 1.1 Existing Test Coverage Across Crates
1. **`crates/quorum-classifier`**:
   - `crates/quorum-classifier/src/lib.rs` (lines 1118–1165): Accurately maps extensions (`.rs`, `.py`, `.js`, `.ts`, `.tsx`, `.jsx`, `.c`, `.cpp`, `.go`, `.java`, `.kt`, `.cs`, `.sh`, `.ps1`, `.html`, `.css`, `.ipynb`, `.sql`, `.bat`, etc.) to `FileCategory::Code`.
   - Verified via `cargo test -p quorum-classifier -- --test-threads=2`:
     - `src/lib.rs`: 10 passed (including `test_rust_code_deep_inspection` and `test_python_code_shebang_inspection` asserting `FileCategory::Code`).
     - `tests/adversarial_magika_tests.rs`: 11 passed.
     - `tests/binary_fast_path_tests.rs`: 3 passed.
     - `tests/corner_cases_multibyte_utf8.rs`: 8 passed.
     - `tests/sharing_violation_stress_tests.rs`: 2 passed.
     - Total: **34/34 tests pass cleanly** in 0.54s. `FileCategory::Code` identification is completely preserved.

2. **`crates/quorum-ingest` Test Coverage & Implementation Details**:
   - `crates/quorum-ingest/src/config.rs`:
     - Lines 58–61: `pub exclude_source_code: bool,` with `#[serde(default)]`.
     - Lines 81–83: `impl Default for IngestConfig` sets `exclude_source_code: false`.
   - `crates/quorum-ingest/src/classifier_router.rs`:
     - Lines 109–117: Filters `~$*`, `.~lock*`, `*.tmp`, `*.bak`, `*.wbk` by returning `FilterDecision::SkipHidden`.
     - Lines 118–132: Excludes 49 source/config extensions (`EXCLUDED_SOURCE_EXTS`) only when `self.config.exclude_source_code == true`.
     - Lines 180–182: Filters `identity.category == FileCategory::Code` only when `self.config.exclude_source_code == true`.
   - `crates/quorum-ingest/src/crawler.rs`:
     - Lines 249–263: Excludes directories `harfile`, `python311`, `tools`, `yara`, `rules`, `db`, `dataset`, `restored_attachments` when `exclude_source_code == true`.
     - Lines 264–285: Excludes `~$*`, `.~lock*`, `*.tmp`, `*.bak`, `*.wbk`, package lock files (`cargo.lock`, `package-lock.json`, etc.), and binary executables.
     - Lines 286–298: Excludes `EXCLUDED_SOURCE_EXTS` when `exclude_source_code == true`.
   - Verified baseline via `cargo test -p quorum-ingest --test ingest_tests -- --test-threads=2`:
     - `tests/ingest_tests.rs`: **10 passed, 0 failed** in 1.49s (all 10 tests passed: chunker, router image preserving, size caps, gitignore crawling, image indexer, ingest engine hybrid search, reindex worker, dedup L1/L2, drift healer).
   - **Critical Gap Found**: In all existing tests (`ingest_tests.rs`, `m7_integration_tests.rs`, `m7_adversarial_stress_tests.rs`, `m7_challenger_stress_tests.rs`, `adversarial_challenge_tests.rs`):
     - `ingest_tests.rs` line 176 & line 212: explicitly asserts `assert!(paths.contains(&"main.rs".to_string()), "main.rs must be crawled")` because `exclude_source_code` is `false` by default.
     - `m7_adversarial_stress_tests.rs:218–265`: uses `main.rs` and `sub/lib.rs` as sample files to test directory rename tracking.
     - `m7_integration_tests.rs:335–367`: uses `main.rs` and `lib.rs` to test recursive watching.
     - `m7_challenger_stress_tests.rs:36`: creates `source_doc_001.rs` to test hydration.
     - **Verification count**: **0 tests** exist for `exclude_source_code: true`. **0 tests** verify rejection of `~$*`, `.~lock*`, `*.bak`, or `*.wbk` lock/temp files in `quorum-ingest`.

3. **`crates/quorum-coordinator` Test Coverage & Implementation Details**:
   - `crates/quorum-coordinator/src/agentic/retrieve.rs`:
     - Lines 106–110: `daemon_active` is determined by `crate::dlp_mesh::global_mesh_dlp_controller().settings_file_path.read().map(|p| p.is_some()).unwrap_or(false)`.
     - Lines 148–154: `requests_code` checks if query contains `"code"`, `"unit test"`, `"implement"`, `"rrfcombiner"`, `"struct "`, `"fn "`.
     - Lines 155–195: Source code extensions, `~$*`, `.~lock*`, and markdown code blocks (` ```python `, ` ```rust `) are retained unless `daemon_active && !requests_code`.
   - `crates/quorum-coordinator/src/offline_engine.rs`:
     - Lines 246–272: Excludes temporary lock files (`~$*`, `.~lock*`, `chat.html`, `conversations-*.json`) and source code files from synthesized citations when `!requests_code`.
   - `crates/quorum-coordinator/tests/adversarial_tier5_coordinator_daemon_stress.rs`:
     - Lines 653–665: In `challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops`, the test fixture creates a mock chunk with `file_path: "crypto.rs"` containing Ed25519 specification text, and queries `"How are Ed25519 signatures verified in QuorumRAG?"` (which lacks the keyword `code`).
     - Lines 723–725: Asserts `!state.citations.is_empty()`.
     - When retrieval/synthesis code exclusion executes, `crypto.rs` is excluded as source code, leaving 0 citations and failing the test!
   - **Verification count**: **0 tests** verify RAG retrieval isolation (queries returning only user documents and images while rejecting code and temp chunks).

---

## 2. Logic Chain

1. **Classifier Invariant**:
   - Observation 1.1 shows `quorum-classifier` classifies `.rs`, `.py`, `.js` as `FileCategory::Code` and all 34 classifier tests pass.
   - Preserving `FileCategory::Code` in `quorum-classifier` is mandatory so classifier tests do not regress, while downstream crates (`quorum-ingest` and `quorum-coordinator`) filter this category.

2. **Ingest Default Rationale**:
   - IngestConfig sets `exclude_source_code: false` by default because M7 watcher/crawler tests used `.rs` files (`main.rs`, `lib.rs`) as sample test files.
   - However, in `quorum-daemon/src/main.rs:188`, `ingest_config.exclude_source_code = true` is set for the live production daemon.
   - Standalone CLI commands in `quorum-daemon/src/main.rs:414` (`run_standalone_reindex`) and `main.rs:467` (`run_standalone_fix_drift`) currently instantiate `IngestConfig::default()` without setting `exclude_source_code = true`, creating a loophole where standalone reindex/drift commands could re-index source code!

3. **Absence of Exclusion Tests in Ingest**:
   - Because `exclude_source_code: false` was the default for unit tests, no developer previously wrote dedicated unit tests verifying that setting `exclude_source_code: true` actually excludes `.rs`, `.py`, `.js`, etc., nor did anyone write tests verifying `~$*` and `.~lock*` lock file rejection.
   - A dedicated test suite in `crates/quorum-ingest/tests/m8_5_exclusion_tests.rs` is required to eliminate this blind spot.

4. **Root Cause of Tier 5 Test Conflict**:
   - `challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops` named its mock architecture document `crypto.rs` instead of `crypto_architecture.md` or `crypto_spec.pdf`.
   - When the user query `"How are Ed25519 signatures verified in QuorumRAG?"` is processed, `!requests_code` causes `crypto.rs` to be correctly excluded as source code under Feature 57.
   - The test fails solely because its mock document used a source code extension (`.rs`) for architectural prose. Renaming the test chunk to `crypto_spec.md` or `crypto.pdf` satisfies both Feature 57 (source code exclusion) and Tier 5 agentic graph verification.

5. **Decoupling Retrieval Exclusion in Coordinator**:
   - In `retrieve.rs:106`, retrieval code exclusion currently checks `daemon_active` via `dlp_mesh.settings_file_path`.
   - Adding `pub exclude_source_code: bool` (default: `true`) to `CoordinatorConfig` provides an explicit, clean contract for retrieval isolation that can be tested directly in unit tests without initializing the full daemon DLP controller.

---

## 3. Caveats

1. **Existing Test Fixture Dependencies**: Existing tests in `crates/quorum-ingest/tests` (`ingest_tests.rs`, `m7_integration_tests.rs`, `m7_adversarial_stress_tests.rs`) rely on `IngestConfig::default().exclude_source_code == false`. Changing `Default for IngestConfig` to `true` would break 4 existing M7 tests unless those tests are updated to set `exclude_source_code: false` or use `.txt` files. The safer approach is keeping `default: false` in `IngestConfig` while explicitly setting `exclude_source_code: true` in the production daemon and in all M8.5 acceptance tests.
2. **Reindex Storage Purging**: If a user runs `quorumrag reindex` on an existing database that previously had source code indexed, `ReindexWorker` skips code files during scan but does not delete previously stored `.rs`/`.py` rows from LanceDB unless `full_rebuild: true` is set or a dedicated purge step is executed.

---

## 4. Conclusion & Required Worker Implementation Plan

### 4.1 Production Code Fixes Required:
1. **`crates/quorum-daemon/src/main.rs`**:
   - Line 414 (`run_standalone_reindex`) and Line 467 (`run_standalone_fix_drift`): set `config.exclude_source_code = true` so CLI commands also enforce source code exclusion.
2. **`crates/quorum-coordinator/src/config.rs` & `retrieve.rs`**:
   - Add `pub exclude_source_code: bool` (default: `true`) to `CoordinatorConfig`.
   - In `retrieve.rs`, check `(daemon_active || self.config.exclude_source_code) && !requests_code` so retrieval isolation works reliably in both daemon mode and unit/integration test mode.
3. **`crates/quorum-coordinator/tests/adversarial_tier5_coordinator_daemon_stress.rs`**:
   - Line 657: Change mock chunk path from `"crypto.rs"` to `"crypto_spec.md"` so the architectural prose test document is not treated as a source code file.

### 4.2 Exact New Test Specifications for Milestone M8.5:
Create a dedicated acceptance test suite `crates/quorum-ingest/tests/m8_5_exclusion_tests.rs` containing 5 comprehensive tests:

1. **`test_classifier_router_excludes_all_35_source_code_extensions_when_active`**:
   - Creates 40+ files covering `.rs`, `.py`, `.js`, `.ts`, `.jsx`, `.tsx`, `.c`, `.cpp`, `.h`, `.hpp`, `.go`, `.java`, `.kt`, `.cs`, `.sh`, `.ps1`, `.bat`, `.sql`, `.css`, `.toml`, `.yaml`, `.lock`, `.dat`, `.log`, `.json`, etc.
   - Verifies `router.evaluate_file` returns `FilterDecision::SkipBinary { .. }` for every code file when `exclude_source_code: true`.
   - Verifies `exclude_source_code: false` accepts them as `CandidateKind::Text`.
2. **`test_classifier_router_excludes_office_temporary_lock_and_autosave_files`**:
   - Creates `~$Annual_Report_2026.docx`, `~$Strategy.xlsx`, `~$Slides.pptx`, `.~lock.Resume.docx#`, `draft.tmp`, `report.bak`, `Backup of Doc.wbk`.
   - Verifies `router.evaluate_file` returns `FilterDecision::SkipHidden` across both `exclude_source_code: true` and `false`.
3. **`test_crawler_directory_pruning_and_file_filtering_under_source_exclusion`**:
   - Creates a realistic folder hierarchy containing `documents/` (`.pdf`, `.docx`, `.txt`, `.md`), `images/` (`.png`, `.jpg`, `.svg`), `src/` (`.rs`, `.py`), `tools/`, `harfile/`, and `~$*.docx`.
   - Verifies `crawler.crawl_path_sync` discovers 100% of legitimate documents and images, and 0% of source code, lock files, or source directories.
4. **`test_magika_content_inspection_rejects_extensionless_scripts`**:
   - Creates extensionless script with `#!/usr/bin/env python3`.
   - Verifies Magika deep inspection classifies as `FileCategory::Code` and `router.evaluate_file` rejects with `FilterDecision::SkipBinary`.
5. **`test_m8_5_rag_retrieval_isolation_never_returns_code_or_temp_chunks`** (in `crates/quorum-coordinator/tests/m8_5_rag_retrieval_isolation_tests.rs`):
   - Seeds `MockStorage` with mixed chunks: `policy.pdf`, `architecture.png`, `main.rs`, `~$policy.docx`, `script.py`.
   - Executes retrieval for query `"what is the policy coverage?"`.
   - Asserts retrieved chunks and cited sources contain strictly `policy.pdf` and `architecture.png`, with 0 code chunks and 0 temp lock files.

---

## 5. Verification Method

To independently verify after Worker implements the changes:

1. **Verify Classifier Tests Pass**:
   ```powershell
   cargo test -p quorum-classifier -- --test-threads=2
   ```
   *Expected*: 34 passed, 0 failed.

2. **Verify New Ingest Exclusion Test Suite**:
   ```powershell
   cargo test -p quorum-ingest --test m8_5_exclusion_tests -- --test-threads=2
   ```
   *Expected*: All 4+ tests pass.

3. **Verify Coordinator Retrieval Isolation Test Suite**:
   ```powershell
   cargo test -p quorum-coordinator --test m8_5_rag_retrieval_isolation_tests -- --test-threads=2
   ```
   *Expected*: Retrieval returns 0 code chunks and 0 temp chunks.

4. **Verify Tier 5 Stress Test Suite**:
   ```powershell
   cargo test -p quorum-coordinator --test adversarial_tier5_coordinator_daemon_stress -- challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops -- --test-threads=2
   ```
   *Expected*: Passes with grounded citations on `crypto_spec.md`.

5. **Invalidation Conditions**:
   - If any `.rs`, `.py`, or `.js` file is accepted by `ClassifierRouter` when `exclude_source_code: true`.
   - If any `~$*.docx` or `.~lock*.docx#` lock file is accepted for indexing.
   - If a RAG query on an index containing mixed files cites a `.rs` or `.py` file when the query did not ask for code.
