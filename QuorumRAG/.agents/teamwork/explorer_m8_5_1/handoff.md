# Handoff Report: Milestone M8.5 Ingest & Router Investigation

**Agent**: Explorer M8.5 1 (Ingest & Router Explorer)  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_5_1`  
**Date**: 2026-11-02  
**Mission**: Investigate `crates/quorum-ingest` (`src/config.rs`, `src/classifier_router.rs`, `src/crawler.rs`, `src/watcher.rs`, `src/reindex.rs`) regarding `exclude_source_code` and Office temp/lock file filtering (`~$*`, `.~lock*`, `*.tmp`, `*.bak`, `*.wbk`).

---

## 1. Observation

### 1.1 `exclude_source_code` in `crates/quorum-ingest/src/config.rs`
- In `crates/quorum-ingest/src/config.rs` (lines 58-61 and 82):
  ```rust
  /// Whether to exclude source code files (`FileCategory::Code`), raw `.dat` blobs, and monolithic dumps during ingestion.
  #[serde(default)]
  pub exclude_source_code: bool,
  ...
  impl Default for IngestConfig {
      fn default() -> Self {
          Self {
              ...
              exclude_source_code: false,
          }
      }
  }
  ```
- Directly observed that `exclude_source_code` defaults to `false` in `IngestConfig::default()`.

### 1.2 Backwards Compatibility with Existing Ingest Unit Tests
- In `crates/quorum-ingest/tests/ingest_tests.rs` (lines 176, 201-212):
  ```rust
  let code_file = root.join("main.rs");
  std::fs::write(&code_file, "fn main() { println!(\"Hello\"); }").unwrap();
  ...
  let config = IngestConfig {
      watch_paths: vec![root.to_path_buf()],
      git_ignore: true,
      skip_hidden: true,
      ..Default::default()
  };
  let crawler = FilesystemCrawler::new(config);
  let candidates = crawler.crawl_path_sync(root).unwrap();
  ...
  assert!(paths.contains(&"main.rs".to_string()), "main.rs must be crawled");
  ```
- In `crates/quorum-ingest/tests/m7_integration_tests.rs` (lines 335-344, 366-369):
  ```rust
  let file1 = src_dir.join("main.rs");
  let file2 = sub_dir.join("lib.rs");
  ...
  let res1 = engine.ingest_file(&file1).await.unwrap();
  assert!(res1.is_some() && !res1.unwrap().is_empty());
  ```
- In `crates/quorum-ingest/tests/m7_adversarial_stress_tests.rs` (lines 218, 238, 251, 264):
  `main.rs` is used across stage transitions and asserted to exist in the deduplication catalog (`engine.dedup().get_file_entry(&stage2.join("main.rs")).is_some()`).
- If `IngestConfig::default().exclude_source_code` were set to `true`, these tests would immediately fail because `main.rs` and `lib.rs` would be rejected as `FilterDecision::SkipBinary` / filtered out by the crawler.

### 1.3 Daemon Ingestion Configuration in `crates/quorum-daemon/src/main.rs`
- In `crates/quorum-daemon/src/main.rs` (`run_daemon_and_ui`, lines 83-85):
  ```rust
  let mut ingest_config = quorum_ingest::IngestConfig::default();
  ingest_config.exclude_source_code = true;
  ingest_config.max_file_size_bytes = 15 * 1024 * 1024; // 15MB cap skips monolithic raw dumps
  ```
- Here, the live production daemon explicitly enables `exclude_source_code = true`.
- **Discrepancy / Gap Discovered**:
  - In `crates/quorum-daemon/src/main.rs` (`run_standalone_reindex`, lines 411-417):
    ```rust
    let worker = quorum_ingest::ReindexWorker::new(
        quorum_ingest::IngestConfig::default(),
        storage,
        classifier,
        embedder,
        dedup,
    );
    ```
  - In `crates/quorum-daemon/src/main.rs` (`run_standalone_fix_drift`, lines 464-477):
    ```rust
    let mut ingest_cfg = quorum_ingest::IngestConfig::default();
    if let Some(ref w) = cli.watch_dir {
        ingest_cfg.watch_paths = vec![w.clone()];
    } else {
        ingest_cfg.watch_paths = quorum_ingest::IngestConfig::discover_physical_drives();
    }
    let healer = quorum_ingest::DriftHealer::new(ingest_cfg, ...);
    ```
  - Both standalone CLI entry points (`reindex` and `fix-drift`) construct `IngestConfig::default()` and **omit** setting `exclude_source_code = true`.

### 1.4 Filtering of Temporary, Lock, and Binary Files
- In `crates/quorum-ingest/src/classifier_router.rs`:
  - **Fast-path executable/library filtering** (lines 80-93):
    ```rust
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if quorum_classifier::is_binary_executable_extension(ext) {
            let identity = self.classifier.classify_by_extension(
                path.to_str().unwrap_or(ext),
                1.0,
            );
            return FilterDecision::SkipBinary { identity };
        }
    }
    ```
    Uses `is_binary_executable_extension` from `quorum-classifier` covering `exe`, `dll`, `sys`, `ocx`, `cpl`, `scr`, `msi`, `drv`, `lib`, `exp`, `ilk`, `pdb`, `obj`, `class`, `pyc`, `pyo`, `pyd`, `so`, `dylib`, `o`, `a`, `bin`, `ko`.
  - **Office temporary lock and autosave filtering** (lines 109-117):
    ```rust
    let lower_name = file_name.to_ascii_lowercase();
    if lower_name.starts_with("~$")
        || lower_name.starts_with(".~lock")
        || lower_name.ends_with(".tmp")
        || lower_name.ends_with(".bak")
        || lower_name.ends_with(".wbk")
    {
        return FilterDecision::SkipHidden;
    }
    ```
  - **Source code exclusion when active** (lines 118-132, 180-182):
    ```rust
    if self.config.exclude_source_code {
        const EXCLUDED_SOURCE_EXTS: &[&str] = &[
            ".rs", ".py", ".js", ".ts", ".jsx", ".tsx", ".c", ".cpp", ".h", ".hpp",
            ".cc", ".cs", ".java", ".go", ".kt", ".rb", ".php", ".swift", ".scala",
            ".sh", ".bash", ".ps1", ".bat", ".cmd", ".sql", ".css", ".scss", ".less",
            ".vue", ".svelte", ".toml", ".yaml", ".yml", ".lock", ".dat", ".log",
            ".json", ".jsonl", ".xml", ".html", ".htm", ".tex", ".aux", ".bbl",
            ".blg", ".out", ".toc", ".synctex", ".crt", ".pem", ".key",
        ];
        if EXCLUDED_SOURCE_EXTS.iter().any(|ext| lower_name.ends_with(ext)) {
            let identity = self.classifier.classify_by_extension(file_name, 1.0);
            return FilterDecision::SkipBinary { identity };
        }
    }
    ...
    if self.config.exclude_source_code && identity.category == FileCategory::Code {
        return FilterDecision::SkipBinary { identity };
    }
    ```
- In `crates/quorum-ingest/src/crawler.rs` (`build_walker`, lines 264-299):
  - Skips lock/temp files (`lower.starts_with("~$") || lower.starts_with(".~lock") || lower.ends_with(".tmp") || lower.ends_with(".bak") || lower.ends_with(".wbk")`).
  - Skips package manager lock files (`cargo.lock`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `poetry.lock`, `composer.lock`, `gemfile.lock`).
  - Skips compiled binaries via `quorum_classifier::is_binary_executable_extension`.
  - When `exclude_source_code == true`, prunes directories like `harfile`, `python311`, `tools`, `yara`, `rules`, `db`, `dataset`, `restored_attachments`, and rejects files matching `EXCLUDED_SOURCE_EXTS`.

### 1.5 Reindex Subsystem in `crates/quorum-ingest/src/reindex.rs`
- `ReindexWorker::execute_pipeline` (lines 284-293) initializes `FilesystemCrawler::new(cfg_clone)`.
- If `config.exclude_source_code == true`, source code and temporary files are not discovered by the crawler and thus are not processed or indexed.
- **Chunk Purging State**:
  - `reindex.rs` does **not** query LanceDB or Tantivy to locate and delete previously indexed chunks matching source code or temporary file extensions.
  - Chunks already stored prior to enabling `exclude_source_code` remain in LanceDB and Tantivy until deleted or purged.
  - However, `crates/quorum-coordinator/src/agentic/retrieve.rs` (lines 155-196) actively intercepts queries at retrieval time when `daemon_active && !requests_code`, discarding any chunks whose file names start with `~`, `.~lock`, match `EXCLUDED_EXTS`, or contain code fences (` ```python `, ` ```rust `, etc.).

---

## 2. Logic Chain

1. **Step 1 (Config Default Decision)**:
   - Setting `IngestConfig::default().exclude_source_code = false` was a deliberate design choice that prevents breaking existing unit tests (`ingest_tests.rs:212`, `m7_integration_tests.rs:335`, `m7_adversarial_stress_tests.rs:218`) that synthesize and verify `main.rs` and `lib.rs` files without having to pass custom configuration objects.
2. **Step 2 (Runtime Enforcement in Daemon)**:
   - The user-facing daemon (`main.rs:83-85`) explicitly enables `ingest_config.exclude_source_code = true` for live background watching and crawling.
   - However, standalone CLI commands (`quorumrag reindex` and `quorumrag fix-drift`) instantiate `IngestConfig::default()` without flipping `exclude_source_code = true`, creating a divergence between daemon background ingestion and standalone CLI execution.
3. **Step 3 (Temp/Lock File Filtering Completeness)**:
   - Both `ClassifierRouter` and `FilesystemCrawler` explicitly reject `~$*`, `.~lock*`, `*.tmp`, `*.bak`, and `*.wbk`.
   - In `retrieve.rs:171`, the query filter uses `file_l.starts_with('~')`, which covers all tilde-prefixed files (e.g. `~WRL0001.tmp` or `~autosave.docx`). In contrast, `classifier_router.rs` and `crawler.rs` check `lower.starts_with("~$")`. Expanding router and crawler to `lower.starts_with('~')` guarantees 100% parity across the stack.
4. **Step 4 (Reindex Chunks Invariant)**:
   - Reindexing skips new/modified source code and temp files when `exclude_source_code` is `true`.
   - To completely isolate RAG from legacy source code chunks in existing stores, retrieval-time filtering in `retrieve.rs` ensures source code chunks are never returned to the user, even if they remain in historical LanceDB parquet fragments.

---

## 3. Caveats

1. **Read-Only Investigation**: In compliance with the Teamwork explorer archetype, no source files were modified.
2. **`EXCLUDED_SOURCE_EXTS` Inclusion of `.json`**: `EXCLUDED_SOURCE_EXTS` in `classifier_router.rs:124` and `crawler.rs:292` includes `.json`, `.jsonl`, `.xml`, `.html`. While this successfully blocks monolithic chat and web dumps, it also prevents user `.json` data files from being indexed when `exclude_source_code = true`. If indexing user JSON documents is desired, JSON exclusion could be restricted to specific dump filenames (e.g. `conversations-*.json`) as done in `retrieve.rs:174`.
3. **Background Test Concurrency**: Cargo test execution runs on a single lock on the `target` build directory, requiring sequential scheduling.

---

## 4. Conclusion

1. **Config Default**: `IngestConfig::default().exclude_source_code = false` is functioning as intended for test backward-compatibility, while `main.rs:84` enables `exclude_source_code = true` for the daemon.
2. **Actionable Fix 1 (Daemon CLI Workers)**:
   - In `crates/quorum-daemon/src/main.rs`:
     - In `run_standalone_reindex` (around line 412), change:
       ```rust
       let mut ingest_cfg = quorum_ingest::IngestConfig::default();
       ingest_cfg.exclude_source_code = true;
       ```
     - In `run_standalone_fix_drift` (around line 464), ensure:
       ```rust
       ingest_cfg.exclude_source_code = true;
       ```
3. **Actionable Fix 2 (Router/Crawler Tilde Expansion)**:
   - In `crates/quorum-ingest/src/classifier_router.rs` (line 110) and `crates/quorum-ingest/src/crawler.rs` (line 264), change:
     `lower_name.starts_with("~$")` -> `lower_name.starts_with('~')` to align with `retrieve.rs:171` and catch all temporary autosave files (`~*`).
4. **Actionable Fix 3 (Ingest Unit Test Coverage)**:
   - Add a dedicated test in `crates/quorum-ingest/tests/` (e.g., `test_source_code_and_temp_file_exclusion`) verifying that when `exclude_source_code = true`, `ClassifierRouter` and `FilesystemCrawler` reject `.rs`, `.py`, `.js`, `~$*.docx`, `*.tmp`, `*.bak`, while accepting `.docx`, `.pdf`, `.txt`, `.md`, `.png`.

---

## 5. Verification Method

To independently verify these findings:
1. **Config Verification**: Inspect `crates/quorum-ingest/src/config.rs:82` (`exclude_source_code: false`) and `crates/quorum-daemon/src/main.rs:84` (`ingest_config.exclude_source_code = true`).
2. **Filtering Inspection**: Inspect `crates/quorum-ingest/src/classifier_router.rs:109-132` and `crates/quorum-ingest/src/crawler.rs:264-299`.
3. **Retrieval Safeguard**: Inspect `crates/quorum-coordinator/src/agentic/retrieve.rs:155-196`.
4. **Test Suite Execution**:
   ```powershell
   cargo test -p quorum-ingest -- --test-threads=2
   ```
   **Verified Output** (Completed 2026-11-02T06:25:02Z):
   - `unittests src\lib.rs`: 3 passed, 0 failed
   - `adversarial_challenge_tests.rs`: 9 passed, 0 failed
   - `adversarial_reindex_and_drift.rs`: 10 passed, 0 failed
   - `ingest_tests.rs`: 10 passed, 0 failed
   - `m7_adversarial_stress_tests.rs`: 6 passed, 0 failed
   - `m7_challenger_stress_tests.rs`: 6 passed, 0 failed
   - `m7_integration_tests.rs`: 7 passed, 0 failed
   - Total: 51 passed; 0 failed.
   Invalidation conditions: If `IngestConfig::default().exclude_source_code` is changed to `true` without updating existing unit tests (`ingest_tests.rs:212`, `m7_integration_tests.rs:342`), the test suite will fail.
