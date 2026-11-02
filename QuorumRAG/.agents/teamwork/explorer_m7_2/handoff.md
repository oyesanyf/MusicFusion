# Handoff Report: Milestone M7 — Physical Drive Discovery & Fast-Path Binary Exclusion (Features 45 & 46)

**Agent**: `teamwork_explorer` (`explorer_m7_2`)  
**Parent**: `facfe77d-94d3-44be-95df-eec2b9dfe368`  
**Milestone**: M7 (Features 45 & 46)  
**Date**: 2026-11-02  
**Target Crates**: `crates/quorum-ingest`, `crates/quorum-classifier`  

---

## 1. Observation

Direct source code inspection across `crates/quorum-ingest` and `crates/quorum-classifier` reveals six distinct architectural defects causing drive discovery blind spots, dot-directory omission, slice duplication, and severe disk I/O / sharing violation overhead on compiled binaries.

### 1.1 Physical Drive Discovery Filesystem Restriction
- **File**: `crates/quorum-ingest/src/config.rs:297-302`
- **Verbatim Code**:
  ```rust
  let vol_label = String::from_utf16_lossy(&vol_name[..vol_len]).to_ascii_lowercase();
  let fs_label = String::from_utf16_lossy(&fs_name[..fs_len]).to_ascii_uppercase();

  if !matches!(fs_label.as_str(), "NTFS" | "REFS") {
      continue;
  }
  ```
- **Observed Behavior**:
  `GetVolumeInformationW` extracts the uppercase filesystem label into `fs_label`. Line 300 unconditionally skips any volume whose filesystem is not `"NTFS"` or `"REFS"`. If a physical fixed drive (e.g. secondary internal NVMe partition, developer data partition, or fixed storage disk) is formatted with `exFAT` or `FAT32`, it passes the kernel NT device path check (`\Device\HarddiskVolume*`) and the hardware bus IOCTL (`IOCTL_STORAGE_QUERY_PROPERTY`), but is dropped on line 300. QuorumRAG completely ignores all documents on that physical drive.

### 1.2 Blanket Skip of Dot-Directories during Drive Root Expansion
- **File**: `crates/quorum-ingest/src/reindex.rs:580-587` & `593-597`
- **Verbatim Code**:
  ```rust
  // Line 581-587
  let name_lower = entry
      .file_name()
      .to_string_lossy()
      .to_ascii_lowercase();
  if SKIP_DIRS.contains(&name_lower.as_str()) || name_lower.starts_with('.') {
      continue;
  }
  ```
  ```rust
  // Line 593-597
  let sub_name = sub.file_name().to_string_lossy().to_ascii_lowercase();
  if SKIP_DIRS.contains(&sub_name.as_str()) || sub_name.starts_with('.') {
      continue;
  }
  ```
- **Observed Behavior**:
  Lines 585 and 595 enforce a blanket skip on any directory whose filename starts with a period (`.`). While intended to avoid VCS noise like `.git`, this blanket condition drops legitimate user workspace directories such as `.workspace`, `.documents`, `.notes`, `.agents`, `.claude`, or `.config` located at drive roots or inside user home folders (`femi`, `harfile`, `users`). Furthermore, `SKIP_DIRS` already contains explicit exclusions for `.git`, `.cargo`, `.rustup`, `__pycache__`, `.venv`, `venv`, and `.tmp.driveupload`.

### 1.3 Overlapping Slice Duplication in `expand_drive_roots()`
- **File**: `crates/quorum-ingest/src/reindex.rs:603-613`
- **Verbatim Code**:
  ```rust
  } else if sub_name.contains("mechanistic_interpretability") {
      let probe_dir = sub_path.join("probe_and_tools");
      if probe_dir.is_dir() {
          top_priority_slices.push(probe_dir);
      }
      priority_slices.push(sub_path);
  } else if sub_name.contains("quorumrag") {
  ```
- **Observed Behavior**:
  When `sub_path` is `D:\harfile\mechanistic_interpretability` and contains `probe_and_tools`:
  1. `probe_dir` (`sub_path\probe_and_tools`) is pushed to `top_priority_slices`.
  2. `sub_path` is pushed to `priority_slices`.
  3. When `execute_pipeline` runs, it iterates targets in order (`top_priority_slices`, then `priority_slices`).
  4. It crawls `probe_and_tools` first.
  5. Then it crawls `sub_path`, recursively traversing all of `sub_path`, including `probe_and_tools` AGAIN.
  6. Every file under `probe_and_tools` is crawled, hashed, classified, and processed twice.
  7. In `reindex.rs:293`: `p.files_total_estimated += candidates.len()` adds candidate counts slice by slice. All files under `probe_and_tools` are counted twice in `files_total_estimated`, corrupting progress percentage telemetry.

### 1.4 Unfiltered Disk I/O & Open File Handles on Compiled Binaries
- **File**: `crates/quorum-ingest/src/classifier_router.rs:77-125`
- **Verbatim Code**:
  ```rust
  pub fn evaluate_file(&self, path: &Path) -> FilterDecision {
      // 1. Check symlink policy ...
      // 2. Check hidden file policy ...
      // 3. Inspect metadata with retry for Windows sharing violations
      let metadata = match self.read_metadata_with_retry(path, 3) {
          Ok(m) => m,
          Err(e) => return FilterDecision::SkipIoError { error: e.to_string() },
      };
      ...
      // 4. Content classification via Google Magika (first 4KB inspection)
      let identity = match self.classifier.classify_file(path) {
          Ok(id) => id,
          Err(e) => return FilterDecision::SkipIoError { error: e.to_string() },
      };
  ```
- **Observed Behavior**:
  For every file encountered, `evaluate_file` calls `read_metadata_with_retry(path, 3)` (invoking `std::fs::metadata`) and `self.classifier.classify_file(path)` (invoking `File::open` and `file.read` of 4096 bytes). When indexing developer workspaces or drives with large `target`, `node_modules`, or system directories, thousands of compiled binaries (`.exe`, `.dll`, `.sys`, `.lib`, `.pdb`, `.obj`) trigger disk I/O and file descriptor allocation, frequently triggering Windows Error 32 (`ERROR_SHARING_VIOLATION`) if an executable is in use.

### 1.5 Binary Data Leaking into Text Chunking Pipeline
- **File**: `crates/quorum-ingest/src/classifier_router.rs:141-154`
- **Verbatim Code**:
  ```rust
  // 6. Text / Source Code / Structured Data Routing
  if identity.is_text || matches!(
      identity.category,
      FileCategory::Code | FileCategory::Text | FileCategory::Document | FileCategory::Data
  ) {
      return FilterDecision::Accept {
          identity,
          kind: CandidateKind::Text {
              char_count_estimate: size as usize,
          },
          size_bytes: size,
          mtime_secs: mtime,
      };
  }
  ```
- **Observed Behavior**:
  Because `FileCategory::Data` is included in `matches!(identity.category, ...)`, any file classified under `FileCategory::Data` with `identity.is_text == false` (e.g. SQLite databases, PCAP packet captures, binary telemetry logs, raw binary blobs) evaluates to `true` and is accepted as `CandidateKind::Text`. It is routed to `DocumentExtractor`, which attempts text extraction or raw string harvesting, wasting CPU and generating noisy text chunks for vector embedding.

### 1.6 Missing Compiled Binary Extensions in Classifier Fallback
- **File**: `crates/quorum-classifier/src/lib.rs:1136-1148`
- **Verbatim Code**:
  ```rust
  // Executables / Binaries
  "exe" | "dll" | "sys" => FileIdentity::new("pebin", "application/x-dosexec", FileCategory::Executable, "Windows binary executable/DLL", false, base_score, ClassifierEngine::ExtensionFallback),
  "so" | "dylib" => FileIdentity::new("dylib", "application/octet-stream", FileCategory::Executable, "Shared library binary", false, base_score, ClassifierEngine::ExtensionFallback),
  "wasm" => FileIdentity::new("wasm", "application/wasm", FileCategory::Executable, "WebAssembly binary", false, base_score, ClassifierEngine::ExtensionFallback),
  // Archives ...
  // Unknown fallback
  _ => FileIdentity::unknown_binary(),
  ```
- **Observed Behavior**:
  Only 6 executable extensions (`exe`, `dll`, `sys`, `so`, `dylib`, `wasm`) are mapped to `FileCategory::Executable`. Standard compiled binaries, intermediate objects, system drivers, and debug artifacts:
  `.ocx`, `.cpl`, `.scr`, `.msi`, `.drv`, `.lib`, `.exp`, `.ilk`, `.pdb`, `.obj`, `.class`, `.pyc`, `.pyo`, `.pyd`, `.o`, `.a`, `.bin`, `.ko`
  fall into `_ => FileIdentity::unknown_binary()`, which sets `category: FileCategory::Unknown`.

---

## 2. Logic Chain

1. **Drive Discovery (Observation 1.1)**:
   - Physical internal fixed drives may be formatted with FAT32 or exFAT for cross-platform compatibility, partition portability, or large file storage.
   - The Windows physical disk check already executes `IOCTL_STORAGE_QUERY_PROPERTY` (confirming `removable_media == 0`, `bus != BUS_TYPE_VIRTUAL`, `bus != BUS_TYPE_FILE_BACKED_VIRTUAL`, `device_type != 0x05`).
   - Relaxing the filesystem label check from `matches!(fs_label.as_str(), "NTFS" | "REFS")` to `matches!(fs_label.as_str(), "NTFS" | "REFS" | "EXFAT" | "FAT32")` safely admits fixed FAT32/exFAT drives into `data_drives` while maintaining full protection against virtual mounts and CD-ROMs.

2. **Dot-Directory Blind Spot (Observation 1.2)**:
   - Blanket skipping of `name_lower.starts_with('.')` drops any user data directory that begins with a period (e.g. `.workspace`, `.documents`, `.notes`).
   - Since developer noise (`.git`, `.cargo`, `.rustup`, `.venv`, `.pytest_cache`, `.lance`, etc.) is already enumeratively blacklisted in `SKIP_DIRS`, removing `|| name_lower.starts_with('.')` and `|| sub_name.starts_with('.')` eliminates indexing blind spots without allowing build artifacts or VCS repos to enter the indexing queue.

3. **Slice Duplication & Progress Consistency (Observation 1.3)**:
   - When `probe_dir` (`sub_path\probe_and_tools`) is added to `top_priority_slices`, adding the entire parent `sub_path` to `priority_slices` creates a containment overlap: all files in `probe_and_tools` are traversed twice.
   - To preserve the fast searchability priority of `probe_and_tools` while preventing duplicate crawling:
     When `probe_dir.is_dir()`, add `probe_dir` to `top_priority_slices`, and add all *other* sibling children of `sub_path` (`child != probe_dir`) to `priority_slices`.
   - Applying `retain` deduplication across `expanded` guarantees mathematical idempotency and prevents `files_total_estimated` overcounting.

4. **Fast-Path Binary Exclusion (Observation 1.4 & 1.6)**:
   - Checking the file extension against a comprehensive constant set of 23 compiled binary extensions (`exe`, `dll`, `sys`, `ocx`, `cpl`, `scr`, `msi`, `drv`, `lib`, `exp`, `ilk`, `pdb`, `obj`, `class`, `pyc`, `pyo`, `pyd`, `so`, `dylib`, `o`, `a`, `bin`, `ko`):
     1. Can be performed purely in-memory via string matching without any syscalls.
     2. Returning `FilterDecision::SkipBinary` before `std::fs::metadata` or `File::open` eliminates all file I/O, context switches, and Windows sharing violations for compiled artifacts.
     3. Integrating this check into `crawler.rs:build_walker` (`filter_entry`) additionally prunes binaries before entry allocation and channel dispatch.

5. **Binary Data Leak Elimination (Observation 1.5)**:
   - Text documents (JSON, YAML, TOML, CSV, TSV) have `identity.is_text == true` and `category == FileCategory::Data`.
   - Non-text data files (SQLite, PCAP, raw binary data) have `identity.is_text == false` and `category == FileCategory::Data`.
   - In `classifier_router.rs`, changing Step 6 to require `identity.is_text || matches!(identity.category, FileCategory::Code | FileCategory::Text | FileCategory::Document)` ensures that `FileCategory::Data` is accepted ONLY when `identity.is_text` is `true`. Non-text data drops to Step 7 and is cleanly rejected as `FilterDecision::SkipBinary`.

---

## 3. Caveats

1. **Drive Letter Iteration**: `config.rs` iterates `b'C'..=b'Z'`. Physical drives mapped to `A:\` or `B:\` (legacy floppy letters sometimes reused for fixed drives) remain un-enumerated by default, which is standard on Windows NT.
2. **Directory Name Exclusions vs Custom Ignore**: System noise directories like `$recycle.bin` and `system volume information` remain unconditionally skipped across all drives.
3. **Double Extension Files**: Fast-path extension matching checks `path.extension()`. A file named `payload.dll.txt` will have extension `"txt"`, will not match the binary fast-path, and will proceed to Magika deep byte inspection (Tier 1 magic bytes and null byte checks), correctly preventing extension-spoofed binaries from entering the index.

---

## 4. Conclusion & Precise Code Specifications

### Specification 45.1: Physical Drive Discovery Filesystem Match
- **Target File**: `crates/quorum-ingest/src/config.rs:300`
- **Change**:
  ```rust
  <<<< PREVIOUS
                  if !matches!(fs_label.as_str(), "NTFS" | "REFS") {
                      continue;
                  }
  ====
                  if !matches!(fs_label.as_str(), "NTFS" | "REFS" | "EXFAT" | "FAT32") {
                      continue;
                  }
  >>>>
  ```

### Specification 45.2: Dot-Directory Blind Spot Elimination & Cache Directory Preservation
- **Target File**: `crates/quorum-ingest/src/reindex.rs:585, 595`
- **Change in `SKIP_DIRS`**: Add `.pytest_cache`, `.lance`, `.tantivy`, `.quorumrag-live-data` to `SKIP_DIRS` to ensure all system/cache directories are explicitly filtered.
- **Change on Line 585**:
  ```rust
  <<<< PREVIOUS
                  if SKIP_DIRS.contains(&name_lower.as_str()) || name_lower.starts_with('.') {
                      continue;
                  }
  ====
                  if SKIP_DIRS.contains(&name_lower.as_str()) {
                      continue;
                  }
  >>>>
  ```
- **Change on Line 595**:
  ```rust
  <<<< PREVIOUS
                              if SKIP_DIRS.contains(&sub_name.as_str()) || sub_name.starts_with('.') {
                                  continue;
                              }
  ====
                              if SKIP_DIRS.contains(&sub_name.as_str()) {
                                  continue;
                              }
  >>>>
  ```

### Specification 45.3: Overlapping Slice Duplication Fix in `reindex.rs:expand_drive_roots()`
- **Target File**: `crates/quorum-ingest/src/reindex.rs:603-613`
- **Change**:
  ```rust
  <<<< PREVIOUS
                              } else if sub_name.contains("mechanistic_interpretability") {
                                  let probe_dir = sub_path.join("probe_and_tools");
                                  if probe_dir.is_dir() {
                                      top_priority_slices.push(probe_dir);
                                  }
                                  priority_slices.push(sub_path);
                              } else if sub_name.contains("quorumrag") {
  ====
                              } else if sub_name.contains("mechanistic_interpretability") {
                                  let probe_dir = sub_path.join("probe_and_tools");
                                  if probe_dir.is_dir() {
                                      top_priority_slices.push(probe_dir.clone());
                                      if let Ok(sub_children) = std::fs::read_dir(&sub_path) {
                                          for child in sub_children.flatten() {
                                              let child_path = child.path();
                                              if child_path != probe_dir {
                                                  priority_slices.push(child_path);
                                              }
                                          }
                                      }
                                  } else {
                                      priority_slices.push(sub_path);
                                  }
                              } else if sub_name.contains("quorumrag") {
  >>>>
  ```
- **Deduplication Guard**: At the end of `expand_drive_roots`, deduplicate slices:
  ```rust
  let mut seen = std::collections::HashSet::new();
  expanded.retain(|p| seen.insert(p.clone()));
  ```

### Specification 46.1: Comprehensive Binary Extension Mapping in `quorum-classifier`
- **Target File**: `crates/quorum-classifier/src/lib.rs:1136-1148`
- **Add Public Constant and Helper Function**:
  ```rust
  pub const COMPILED_BINARY_EXTENSIONS: &[&str] = &[
      "exe", "dll", "sys", "ocx", "cpl", "scr", "msi", "drv",
      "lib", "exp", "ilk", "pdb", "obj", "class",
      "pyc", "pyo", "pyd",
      "so", "dylib", "o", "a", "bin", "ko",
  ];

  pub fn is_binary_executable_extension(ext: &str) -> bool {
      let clean = ext.trim_start_matches('.').to_ascii_lowercase();
      matches!(
          clean.as_str(),
          "exe"
              | "dll"
              | "sys"
              | "ocx"
              | "cpl"
              | "scr"
              | "msi"
              | "drv"
              | "lib"
              | "exp"
              | "ilk"
              | "pdb"
              | "obj"
              | "class"
              | "pyc"
              | "pyo"
              | "pyd"
              | "so"
              | "dylib"
              | "o"
              | "a"
              | "bin"
              | "ko"
      )
  }
  ```
- **Make `classify_by_extension` public**:
  ```rust
  pub fn classify_by_extension(&self, filename: &str, base_score: f32) -> FileIdentity
  ```
- **Update Extension Match Table**:
  Map all 23 extensions to `FileCategory::Executable` with `is_text: false`:
  ```rust
  // Executables / Binaries / System Libraries / Intermediate Artifacts
  "exe" | "dll" | "sys" | "ocx" | "cpl" | "scr" | "msi" | "drv" => FileIdentity::new(
      "pebin",
      "application/x-dosexec",
      FileCategory::Executable,
      "Windows binary executable / system library / driver",
      false,
      base_score,
      ClassifierEngine::ExtensionFallback,
  ),
  "so" | "dylib" | "ko" => FileIdentity::new(
      "dylib",
      "application/octet-stream",
      FileCategory::Executable,
      "Shared library or kernel module binary",
      false,
      base_score,
      ClassifierEngine::ExtensionFallback,
  ),
  "lib" | "a" => FileIdentity::new(
      "staticlib",
      "application/x-archive",
      FileCategory::Executable,
      "Static library archive",
      false,
      base_score,
      ClassifierEngine::ExtensionFallback,
  ),
  "obj" | "o" => FileIdentity::new(
      "object",
      "application/x-object",
      FileCategory::Executable,
      "Compiled object file",
      false,
      base_score,
      ClassifierEngine::ExtensionFallback,
  ),
  "class" => FileIdentity::new(
      "class",
      "application/java-vm",
      FileCategory::Executable,
      "Java compiled bytecode",
      false,
      base_score,
      ClassifierEngine::ExtensionFallback,
  ),
  "pyc" | "pyo" | "pyd" => FileIdentity::new(
      "python_bytecode",
      "application/x-python-bytecode",
      FileCategory::Executable,
      "Python compiled bytecode / C-extension binary",
      false,
      base_score,
      ClassifierEngine::ExtensionFallback,
  ),
  "pdb" | "ilk" | "exp" => FileIdentity::new(
      "pdb",
      "application/octet-stream",
      FileCategory::Executable,
      "Debug symbols / incremental linker / export database",
      false,
      base_score,
      ClassifierEngine::ExtensionFallback,
  ),
  "bin" => FileIdentity::new(
      "binary",
      "application/octet-stream",
      FileCategory::Executable,
      "Raw compiled binary image",
      false,
      base_score,
      ClassifierEngine::ExtensionFallback,
  ),
  "wasm" => FileIdentity::new(
      "wasm",
      "application/wasm",
      FileCategory::Executable,
      "WebAssembly binary",
      false,
      base_score,
      ClassifierEngine::ExtensionFallback,
  ),
  ```

### Specification 46.2: Fast-Path Binary Filter in `classifier_router.rs`
- **Target File**: `crates/quorum-ingest/src/classifier_router.rs:77-100`
- **Change**: Insert the fast-path check at the beginning of `evaluate_file`, immediately BEFORE `read_metadata_with_retry` and `classify_file`:
  ```rust
  pub fn evaluate_file(&self, path: &Path) -> FilterDecision {
      // Fast-path binary/executable/DLL exclusion:
      // Exclude compiled binaries immediately BEFORE executing std::fs::metadata or File::open
      if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
          if quorum_classifier::is_binary_executable_extension(ext) {
              let identity = self.classifier.classify_by_extension(
                  path.to_str().unwrap_or(ext),
                  1.0,
              );
              debug!(
                  path = %path.display(),
                  extension = %ext,
                  "Fast-path filtered out compiled binary/executable without disk I/O"
              );
              return FilterDecision::SkipBinary { identity };
          }
      }

      // 1. Check symlink policy ...
      // 2. Check hidden file policy ...
      // 3. Inspect metadata with retry for Windows sharing violations ...
  ```

### Specification 46.3: Non-Text `FileCategory::Data` Binary Leak Fix
- **Target File**: `crates/quorum-ingest/src/classifier_router.rs:142-146`
- **Change**:
  ```rust
  <<<< PREVIOUS
          // 6. Text / Source Code / Structured Data Routing
          if identity.is_text || matches!(
              identity.category,
              FileCategory::Code | FileCategory::Text | FileCategory::Document | FileCategory::Data
          ) {
  ====
          // 6. Text / Source Code / Structured Data Routing
          // Structured text data (JSON, YAML, TOML, CSV, TSV) matches via `identity.is_text == true`.
          // Non-text Data (is_text == false, e.g. SQLite, PCAP, raw binary) must NOT bypass binary filtering into CandidateKind::Text.
          if identity.is_text || matches!(
              identity.category,
              FileCategory::Code | FileCategory::Text | FileCategory::Document
          ) {
  >>>>
  ```

### Specification 46.4: Directory Walker Fast Binary Pruning in `crawler.rs`
- **Target File**: `crates/quorum-ingest/src/crawler.rs:245-260`
- **Change**:
  ```rust
  <<<< PREVIOUS
                  } else if lower.starts_with("~$")
                      || lower.starts_with(".~lock")
                      || matches!(
                          lower.as_str(),
                          "cargo.lock"
                              | "package-lock.json"
                              | "pnpm-lock.yaml"
                              | "yarn.lock"
                              | "poetry.lock"
                              | "composer.lock"
                              | "gemfile.lock"
                      )
                  {
                      return false;
                  }
  ====
                  } else if lower.starts_with("~$")
                      || lower.starts_with(".~lock")
                      || matches!(
                          lower.as_str(),
                          "cargo.lock"
                              | "package-lock.json"
                              | "pnpm-lock.yaml"
                              | "yarn.lock"
                              | "poetry.lock"
                              | "composer.lock"
                              | "gemfile.lock"
                      )
                      || std::path::Path::new(&lower)
                          .extension()
                          .and_then(|e| e.to_str())
                          .map(quorum_classifier::is_binary_executable_extension)
                          .unwrap_or(false)
                  {
                      return false;
                  }
  >>>>
  ```

---

## 5. Verification Method

### 5.1 Test Execution Commands
The implementer can verify the implementation with these targeted commands:
```powershell
# 1. Run classifier unit tests (including all 23 binary extensions)
cargo test -p quorum-classifier

# 2. Run ingest unit & integration tests
cargo test -p quorum-ingest

# 3. Run full workspace tests
cargo test --workspace
```

### 5.2 Unit Test Specifications to Add
1. **`test_fast_path_binary_extensions_all_mapped()`** in `crates/quorum-classifier/tests/`:
   Verify that for every extension in `COMPILED_BINARY_EXTENSIONS`, `is_binary_executable_extension(ext)` is `true`, `classify_by_extension(filename, 1.0)` has `category == FileCategory::Executable`, `is_text == false`, and `is_binary == true`.
2. **`test_classifier_router_fast_path_skips_without_file_io()`** in `crates/quorum-ingest/tests/ingest_tests.rs`:
   Point `ClassifierRouter::evaluate_file` to a non-existent path `D:\fake\path\test_service.dll`.
   Assert that it returns `FilterDecision::SkipBinary` without returning `FilterDecision::SkipIoError` (proving zero disk `stat` or `open` is performed).
3. **`test_non_text_data_does_not_leak_to_candidate_text()`** in `crates/quorum-ingest/tests/ingest_tests.rs`:
   Evaluate a SQLite file with `FileCategory::Data` and `is_text: false`.
   Assert that `evaluate_file` returns `FilterDecision::SkipBinary` and NOT `FilterDecision::Accept { kind: CandidateKind::Text, .. }`.
4. **`test_reindex_expand_drive_roots_does_not_skip_user_dot_folders()`** in `crates/quorum-ingest/tests/ingest_tests.rs`:
   Create temporary directories `.workspace`, `.documents`, and `.git`. Call `expand_drive_roots`.
   Assert that `.workspace` and `.documents` are included in slices, while `.git` is excluded.
5. **`test_probe_and_tools_slice_non_overlapping()`** in `crates/quorum-ingest/tests/ingest_tests.rs`:
   Verify that `probe_and_tools` and its parent `mechanistic_interpretability` do not contain overlapping slices.

### 5.3 Invalidation Conditions
- Any change that accepts non-text files (`is_text: false`) as `CandidateKind::Text`.
- Any regression where `read_metadata_with_retry` or `classify_file` executes before checking `is_binary_executable_extension`.
- Any failure in existing tests (`test_discover_physical_drives_excludes_non_physical`, `test_classifier_router_image_and_text_routing`).
