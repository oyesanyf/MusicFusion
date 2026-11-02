# Exploration Report: Requirement R1 — Zero-Overhead Ingestion Pipeline, Deduplication Hydration, and Binary Filtering

## 1. Observation

Direct observations from source inspection across `crates/quorum-ingest`, `crates/quorum-classifier`, `crates/quorum-storage`, `crates/quorum-daemon`, and `crates/quorum-ui`:

### 1.1 Physical Drive Discovery & Traversal
- **Windows Drive Filtering** (`crates/quorum-ingest/src/config.rs:189-328`):
  - In `IngestConfig::discover_physical_drives()`, the loop iterates drive letters `b'C'..=b'Z'` (lines 189-190). Physical drives mapped to `A:\` or `B:\` are never checked.
  - Line 194: `GetDriveTypeW` requires `DRIVE_FIXED (3)`.
  - Line 215: `QueryDosDeviceW` requires `nt_target.starts_with("\\Device\\HarddiskVolume")`.
  - Lines 220-274: IOCTL `IOCTL_STORAGE_QUERY_PROPERTY` explicitly filters out `removable_media`, `device_type == 0x05 (CD-ROM)`, `BUS_TYPE_VIRTUAL (14)`, `BUS_TYPE_FILE_BACKED_VIRTUAL (15)`, and `BUS_TYPE_ATAPI (2)`.
  - Line 300: `if !matches!(fs_label.as_str(), "NTFS" | "REFS") { continue; }`. **Any internal fixed drive formatted as FAT32 or exFAT is unconditionally skipped and omitted from ingestion.**
  - Line 312: `if std::fs::read_dir(&path).is_ok()`. If root directory permission or security hook causes `read_dir` to return `Err`, the entire drive is omitted without error logging.
- **Non-Windows Drive Filtering** (`crates/quorum-ingest/src/config.rs:330-343`):
  - Hardcodes candidate directories: `&["/mnt", "/Volumes", "/home", "/Users"]`. Linux mounts in `/media`, `/run/media`, or root partitions are omitted unless fallback to `"/"` triggers.
- **Drive Slice Expansion & Hidden Directories** (`crates/quorum-ingest/src/reindex.rs:533-640`):
  - In `ReindexWorker::expand_drive_roots()`, drive roots are split into top-level directory slices.
  - Line 585: `if SKIP_DIRS.contains(&name_lower.as_str()) || name_lower.starts_with('.') { continue; }`. **Any top-level directory starting with a dot (e.g. `.documents`, `.workspace`, `.agents`) is completely omitted from drive root traversal.**
  - Lines 589-617: Hardcodes single-level expansion for specific directory names (`"femi" | "harfile" | "users"`), and prioritizes specific folder names (`"resume"`, `"transcripts"`, `"mechanistic_interpretability"`, `"quorumrag"`). Other subdirectories are pushed to `normal_slices`.
  - Lines 603-608: `probe_dir = sub_path.join("probe_and_tools")` is pushed to `top_priority_slices`, while `sub_path` is ALSO pushed to `priority_slices`. Files under `probe_and_tools` are crawled and evaluated twice.
- **Crawler Exclusions** (`crates/quorum-ingest/src/crawler.rs:194-263`):
  - `FilesystemCrawler::build_walker` enables `skip_hidden: true`, `ignore_files: true`, `git_ignore: true`, `git_global: true`.
  - Lines 213-245: Directory name blacklist excludes `$recycle.bin`, `system volume information`, `windows`, `program files`, `program files (x86)`, `programdata`, `recovery`, `perflogs`, `msocache`, `config.msi`, `appdata`, `node_modules`, `target`, `osoosi_target`, `site-packages`, `dist-packages`, `docker`, `chrome_temp_profile`, `.git`, `.cargo`, `.rustup`, `__pycache__`, `.venv`, `venv`, `.pytest_cache`, `.lance`, `.tantivy`, `.quorumrag-live-data`.
  - Lines 246-258: File-level filter ONLY excludes lockfiles (`cargo.lock`, `package-lock.json`, etc.) and temp lock prefixes (`~$`, `.~lock`). **Executable extensions (.exe, .dll, .sys, etc.) are NOT filtered in `crawler.rs`.**

### 1.2 Deduplication Hydration on Startup
- **Empty Initialization** (`crates/quorum-ingest/src/dedup.rs:328-333`):
  ```rust
  impl TwoLevelDedupEngine {
      pub fn new() -> Self {
          Self {
              catalog: Arc::new(RwLock::new(Level1DedupCatalog::new())),
              cache: Arc::new(Level2ChunkCache::new(50_000)),
          }
      }
  }
  ```
- **Daemon Startup Wiring** (`crates/quorum-daemon/src/main.rs:74-95`):
  ```rust
  let storage = Arc::new(quorum_storage::StorageEngine::open(storage_config).await?);
  let classifier = Arc::new(quorum_classifier::FileTypeClassifier::new());
  let embedder: Arc<dyn quorum_contract::EmbeddingGenerator> = Arc::new(
      quorum_contract::DeterministicOfflineEngine::new()
  );
  let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());
  ```
- **Storage Capability Already Available** (`crates/quorum-storage/src/engine.rs:435-447`):
  - `StorageEngine::scan_all_chunks(&self) -> Result<Vec<DocumentChunk>, StorageError>` retrieves all stored records from LanceDB.
  - Each `DocumentChunk` contains `chunk.chunk_id`, `chunk.blake3_hash`, `chunk.document_id` (the Level-1 file BLAKE3 digest), `chunk.file_path`, and `chunk.vector`.
- **Startup Impact**:
  - `TwoLevelDedupEngine` starts with 0 cached chunk vectors and 0 catalog entries.
  - When reindexing or continuous background crawl begins, `dedup.evaluate_file(path, mtime, size)` misses in the empty Level-1 catalog.
  - The crawler is forced to read each file from disk and compute `hash_file_stream`.
  - Since `catalog.entries` is empty, the file is classified as `ModifiedOrNew`.
  - The file is chunked, and each chunk is evaluated against `dedup.cache().get(&chunk_b3)`.
  - Because `cache` is empty, `embedder.embed_text(raw.text_slice)` is executed via neural Candle/Gemma for EVERY single chunk of previously indexed files.
  - When `storage.insert_chunks` is called (`quorum-storage/src/engine.rs:255`), `storage.has_chunk_hash(&chunk.blake3_hash)` detects the chunk already exists in LanceDB and skips the insert.
  - Result: Thousands of files are needlessly read, chunked, and embedded with neural models on every restart, wasting massive CPU and disk I/O.

### 1.3 File Estimation & Live Progress Reporting
- **Slice-by-Slice Incremental Addition** (`crates/quorum-ingest/src/reindex.rs:278-300`):
  ```rust
  for target in &targets {
      ...
      let candidates = tokio::task::spawn_blocking(move || {
          let crawler = FilesystemCrawler::new(cfg_clone);
          crawler.crawl_path_sync(&target_clone)
      }).await...;

      {
          let mut lock = progress.write().await;
          if let Some(ref mut p) = *lock {
              p.files_total_estimated += candidates.len();
              p.phase = ReindexPhase::ChunkingAndVectorizing;
          }
      }
  ```
- **Live Telemetry Percentage Calculation** (`crates/quorum-daemon/src/handlers/reindex.rs:87-94`):
  ```rust
  let pct = if progress.files_total_estimated > 0 {
      ((progress.files_scanned as f64 / progress.files_total_estimated as f64) * 100.0)
          .clamp(0.0, 100.0)
  } else if matches!(progress.phase, quorum_ingest::ReindexPhase::Completed) {
      100.0
  } else {
      0.0
  };
  ```
- **Web UI Oscillation** (`crates/quorum-ui/assets/app.js:634-642`):
  ```javascript
  let pct = Number(data.percent || 0);
  if (totalEst > 0) {
    pct = Math.min(100, Math.max(2, (scanned / totalEst) * 100));
  }
  ```
- **Undercounting Mechanism**:
  - `files_total_estimated` is initialized to 0 and only updated *after* each slice's crawler finishes.
  - In slice 1 (e.g. 15 files): `files_scanned: 15 / files_total_estimated: 15` -> 100%.
  - When slice 2 begins (e.g. 2,000 files): `files_total_estimated` jumps to 2,015; `files_scanned` is 16 -> 0.8%.
  - The UI progress bar constantly fluctuates and resets. The displayed `scanned / totalEst` displays misleading totals until the final slice is reached.

### 1.4 Deterministic Binary Filtering
- **Inspection Path** (`crates/quorum-ingest/src/classifier_router.rs:77-164`):
  - `ClassifierRouter::evaluate_file` runs on every file emitted by `crawler.rs`.
  - Line 97: Executes `read_metadata_with_retry(path, 3)`.
  - Line 122: Executes `self.classifier.classify_file(path)`.
  - `FileTypeClassifier::classify_file` (`crates/quorum-classifier/src/lib.rs:204-224`):
    - Opens the file (`File::open(path)`).
    - Reads up to 4096 bytes (`file.read(&mut buffer)`).
  - In `classify_bytes` (`crates/quorum-classifier/src/lib.rs:243-306, 344-404`):
    - `match_magic_headers`:
      - `b"MZ"` -> `pebin`, `FileCategory::Executable`, `is_text: false`.
      - `b"\x7fELF"` -> `elf`, `FileCategory::Executable`, `is_text: false`.
      - Mach-O magic (`0xfeedface`, etc.) -> `macho`, `FileCategory::Executable`, `is_text: false`.
      - `b"\0asm"` -> `wasm`, `FileCategory::Executable`, `is_text: false`.
      - `b"dex\n03"` -> `dex`, `FileCategory::Executable`, `is_text: false`.
    - If no magic match, `has_binary_null_bytes` checks for null bytes in first 1024 bytes -> `classify_by_extension`.
  - In `classify_by_extension` (`crates/quorum-classifier/src/lib.rs:1136-1148`):
    - `"exe" | "dll" | "sys"` -> `FileCategory::Executable`, `is_text: false`.
    - `"so" | "dylib"` -> `FileCategory::Executable`, `is_text: false`.
    - `"wasm"` -> `FileCategory::Executable`, `is_text: false`.
- **Vulnerabilities and Inefficiencies**:
  1. **Unnecessary File I/O on Executables**: QuorumRAG executes a `File::open` and a 4KB `file.read` on *every single* `.exe`, `.dll`, `.sys`, `.so`, and `.dylib` file on the filesystem instead of deterministically discarding them upfront by filename extension.
  2. **Missing Compiled Binary Extensions**: Extensions such as `.ocx`, `.cpl`, `.scr`, `.msi`, `.drv`, `.lib`, `.exp`, `.ilk`, `.pdb`, `.obj`, `.class`, `.pyc`, `.pyo`, `.pyd`, `.o`, `.a`, `.so.*` are missing from `classify_by_extension`.
  3. **Binary Data Leak into Text Pipeline**: Line 142 in `classifier_router.rs`:
     ```rust
     if identity.is_text || matches!(
         identity.category,
         FileCategory::Code | FileCategory::Text | FileCategory::Document | FileCategory::Data
     ) {
         return FilterDecision::Accept { kind: CandidateKind::Text { .. }, .. };
     }
     ```
     If an unreadable binary file is classified with `FileCategory::Data` (such as SQLite, PCAP, or unknown structured data) where `is_text == false`, it is accepted as `CandidateKind::Text` and forwarded to `DocumentExtractor`, which attempts text extraction or raw string harvesting.

### 1.5 Filesystem Race Conditions & Continuous Monitoring (Notify Watcher)
- **Inactive Watcher in Daemon** (`crates/quorum-daemon/src/main.rs:196-246`):
  - In `run_daemon_and_ui`, `FilesystemWatcher` is never instantiated or started.
  - Instead, lines 201-245 run a polling loop every 12 seconds calling `bg_worker.start_reindex(...)`.
  - Real-time event-driven continuous monitoring (`notify`) is dormant in `quorum-daemon`.
- **Directory Mutation Blind Spots** (`crates/quorum-ingest/src/watcher.rs:120-155`):
  - In `dispatch_debounced_event`:
    - Line 124: `if path.is_file() { let _ = tx.try_send(WatcherEvent::Create(path.clone())); }`
    - Line 146: `if path.is_file() { let _ = tx.try_send(WatcherEvent::Modify(path.clone())); }`
  - When a new folder is created, `path.is_file()` is `false`. No `Create` event is dispatched for the directory or its contents.
  - When a directory is renamed, in `crates/quorum-ingest/src/engine.rs:260-272`:
    - `delete_by_file_path(&from_str)` deletes chunks whose `file_path == from_str` (none, because `from_str` is a directory path).
    - `if to.is_file()` is `false`. None of the files under the renamed directory are indexed.
- **Windows File Sharing Violations during Write**:
  - In `ClassifierRouter::evaluate_file` (`crates/quorum-ingest/src/classifier_router.rs:97, 122`):
    - `read_metadata_with_retry` retries `std::fs::metadata(path)` 3 times.
    - However, `self.classifier.classify_file(path)` executes `File::open(path)` with zero retry.
    - If a file is being saved or copied by an external process, `File::open` fails immediately with Windows Error 32 (`ERROR_SHARING_VIOLATION`).
    - `evaluate_file` returns `FilterDecision::SkipIoError`, and `engine.ingest_file` drops the file without re-queueing or healing.
- **Silent Event Loss on Queue Overflow** (`crates/quorum-ingest/src/watcher.rs:125, 134, 147, 153`):
  - Events are dispatched via `tx.try_send(...)`.
  - When the 10,000-element channel buffer is full, `try_send` fails and the event is dropped silently with `let _ =`. `WatcherEvent::QueueOverflow` is only triggered on native OS watcher driver errors, not on internal channel saturation.
- **Decoupled Engine Instance in Watcher Loop** (`crates/quorum-ingest/src/engine.rs:317`):
  - In `IngestEngine::start_daemon`, a fresh `IngestEngine` is created inside `tokio::spawn`, which instantiates a separate, empty `TwoLevelDedupEngine` unrelated to `self.dedup`.

### 1.6 Host Overhead: CPU, I/O, and Memory
- **No Process/Thread Priority Throttling**:
  - QuorumRAG runs at Windows `NORMAL_PRIORITY_CLASS` with standard thread scheduling.
  - Background indexing does not set `BELOW_NORMAL_PRIORITY_CLASS`, `THREAD_PRIORITY_BELOW_NORMAL`, or EcoQoS background I/O (`IoPriorityVeryLow`).
  - Crawler and embedder compete directly with foreground user applications for CPU and disk bandwidth.
- **Tight Async Loop Without Yielding** (`crates/quorum-ingest/src/reindex.rs:303-491`):
  - The file candidate loop in `execute_pipeline` processes candidates sequentially without calling `tokio::task::yield_now().await` or inserting cooperative throttle intervals.
  - Heavy Candle/Gemma neural embedding calculations monopolize the runtime thread.
- **In-Memory Candidate Buffering**:
  - Line 288 in `reindex.rs`: `crawler.crawl_path_sync(&target_clone)` collects all discovered candidates into a `Vec<DiscoveredCandidate>` in memory.
  - On large disks with hundreds of thousands of files, this buffers massive arrays in memory instead of streaming through a bounded channel.

---

## 2. Logic Chain

1. **Physical Drive Coverage**:
   - Because `discover_physical_drives` enforces `matches!(fs_label.as_str(), "NTFS" | "REFS")` (Observation 1.1), non-NTFS volumes (such as fixed secondary exFAT or FAT32 partitions) are skipped.
   - Because `expand_drive_roots` skips any directory starting with `.` (Observation 1.1), top-level hidden directories on data drives containing target documents (e.g. `.workspace`, `.documents`) are omitted.
   - To eliminate indexing blind spots, `discover_physical_drives` must support all local fixed filesystems (`NTFS`, `REFS`, `FAT32`, `EXFAT`) while maintaining filtering of cloud mounts, virtual disks, and optical media.

2. **Deduplication Hydration**:
   - Because `TwoLevelDedupEngine::new()` starts with empty hash maps (Observation 1.2), the system loses all knowledge of previously indexed files upon daemon restart.
   - On startup, `dedup.evaluate_file` misses, forcing the system to read every file from disk, calculate its BLAKE3 hash, re-chunk it, and execute neural embeddings for every chunk.
   - Because `StorageEngine::scan_all_chunks()` already exists and returns all stored `DocumentChunk` records (including `blake3_hash`, `document_id`, `file_path`, and `vector`), calling a hydration routine on startup will immediately populate both the Level-1 File Catalog and Level-2 Chunk Cache.
   - Once hydrated, 99%+ of files hit the Level-1 Fast Path (`mtime` + `size` match) with 0 disk reads, 0 hashing, and 0 neural embeddings, reducing startup indexing CPU and I/O overhead to near zero.

3. **Progress Reporting & File Estimation**:
   - Because `files_total_estimated` is incremented slice-by-slice inside `execute_pipeline` (Observation 1.3), the total candidate count reflects only the current and past slices.
   - The UI calculates percentage as `(scanned / total_estimated) * 100`, which spikes to 100% at the end of each slice and drops to near 0% when the next slice begins.
   - Slices also overlap (e.g., `probe_and_tools` and its parent `mechanistic_interpretability`), resulting in duplicate counts.
   - To ensure accurate, monotonic progress reporting, the pipeline must either perform a fast upfront file count across all target roots before chunking begins or compute global estimates so `files_total_estimated` remains stable.

4. **Deterministic Binary Filtering**:
   - Because `crawler.rs` does not filter out binary extensions, every `.exe`, `.dll`, and `.sys` file is yielded to `classifier_router.rs`.
   - `classifier_router.rs` performs `std::fs::metadata` and `File::open` followed by a 4096-byte read on every executable on disk (Observation 1.4).
   - In addition, system libraries and build artifacts (`.pdb`, `.obj`, `.lib`, `.class`, `.pyc`, etc.) are missing from the extension table, and files classified as `FileCategory::Data` with `is_text == false` bypass binary filtering into the text extractor.
   - Implementing a deterministic extension check prior to file opening ensures that compiled binaries, executables, and system libraries are filtered out with zero disk I/O, while Google Magika magic byte inspection continues to safeguard against extension spoofing.

5. **Filesystem Watcher & Race Conditions**:
   - In `quorum-daemon/src/main.rs`, `FilesystemWatcher` is not active; only a 12-second reindex polling loop runs (Observation 1.5).
   - In `watcher.rs`, checking `path.is_file()` causes directory creations and directory renames to be ignored, leaving newly added or moved folder trees unindexed.
   - External file writes cause Windows Error 32 sharing violations that abort file ingestion without retry because `classifier.classify_file` does not employ retry logic.
   - Internal channel overflow drops events silently without initiating a drift healing pass.
   - Activating `FilesystemWatcher`, supporting recursive directory expansion on directory create/rename events, adding sharing violation retries to `classify_file`, and falling back to `DriftHealer` on queue saturation resolves these race conditions.

6. **Host Overhead Minimization**:
   - Without process priority reduction, thread throttling, or async yields, background reindexing consumes maximum host CPU and disk throughput.
   - Applying Windows background process/thread priority, inserting cooperative `tokio::task::yield_now().await` points, streaming candidates through bounded channels rather than allocating monolithic `Vec` buffers, and hydrating deduplication state on startup eliminates host performance interference.

---

## 3. Caveats

1. **Volume Formats on Windows**: While NTFS and ReFS are the dominant Windows filesystems, FAT32 and exFAT partitions do not support certain advanced NTFS metadata (such as USN change journals or alternate data streams). However, standard `std::fs` operations, file times, and `ignore::WalkBuilder` work identically across them.
2. **Offline Drive Hydration**: If a file was indexed on an external physical drive that is disconnected during daemon startup, hydration will register its chunks into the Level-2 Chunk Cache and its path into the Level-1 File Catalog (with placeholder mtime 0), preserving rename/move detection when the volume is remounted.
3. **Magika ONNX vs Heuristic Engine**: `quorum-classifier` contains both deep-inspection heuristics (magic bytes, token density, shebangs) and an optional `onnx` feature for the Google Magika neural model. In development mode without the `onnx` feature enabled, the heuristic inspection engine is the active path and successfully identifies PE, ELF, Mach-O, WASM, and archive magic headers.

---

## 4. Conclusion

Requirement R1 requires targeted enhancements across three crates (`quorum-ingest`, `quorum-classifier`, and `quorum-daemon`):

1. **Deduplication Hydration**:
   - Implement `TwoLevelDedupEngine::hydrate_from_storage(storage: &Arc<StorageEngine>) -> Result<usize, IngestError>` to load existing `DocumentChunk` records from LanceDB on daemon initialization.
   - Populate `Level2ChunkCache` with `(blake3_hash -> vector)` and `Level1DedupCatalog` with `(file_path -> FileCatalogEntry)` grouping chunks by `document_id`.
   - Call this method in `crates/quorum-daemon/src/main.rs` immediately after `StorageEngine::open`.

2. **Physical Drive Discovery & Traversal**:
   - In `IngestConfig::discover_physical_drives()`, expand Windows filesystem support to `matches!(fs_label.as_str(), "NTFS" | "REFS" | "EXFAT" | "FAT32")`.
   - In `reindex.rs:expand_drive_roots()`, remove the blanket `name_lower.starts_with('.')` exclusion on data drives for user folders, while retaining skips for system noise (`$recycle.bin`, `system volume information`).
   - Fix overlapping slice duplication in `expand_drive_roots()` (e.g. `probe_and_tools` within `mechanistic_interpretability`).

3. **Accurate File Estimation & Progress Reporting**:
   - In `ReindexWorker::execute_pipeline`, compute the total candidate file count upfront across all targets (or maintain a global estimated total) before starting chunking.
   - Set `files_total_estimated` once so `(files_scanned / files_total_estimated)` increases monotonically in both the REST API and the Web UI.

4. **Deterministic Binary & Library Filtering**:
   - In `ClassifierRouter::evaluate_file` and `crawler.rs`, introduce an immediate fast-path extension filter for compiled binaries, executables, system libraries, and intermediate artifacts:
     `.exe`, `.dll`, `.sys`, `.ocx`, `.cpl`, `.scr`, `.msi`, `.drv`, `.lib`, `.exp`, `.ilk`, `.pdb`, `.obj`, `.class`, `.pyc`, `.pyo`, `.pyd`, `.so`, `.dylib`, `.o`, `.a`, `.bin`, `.ko`.
   - If the extension matches, return `FilterDecision::SkipBinary` immediately without executing `std::fs::metadata` or `File::open`.
   - In `ClassifierRouter::evaluate_file`, ensure files with `is_text == false` and category `FileCategory::Data` do not bypass binary filtering into `CandidateKind::Text`.

5. **Continuous Filesystem Monitoring & Race Condition Elimination**:
   - Wire `FilesystemWatcher` into `quorum-daemon`'s background supervisor alongside incremental drift remediation.
   - In `watcher.rs:dispatch_debounced_event`, handle directory creation and rename events by recursively discovering candidate files in the new directory.
   - In `ClassifierRouter::evaluate_file` and `FileTypeClassifier::classify_file`, wrap `File::open` in a retry loop (e.g., 3 attempts with 50ms exponential backoff) to handle Windows Error 32 sharing violations during active writes.
   - If `WatcherEvent` dispatch channels fill, emit `WatcherEvent::QueueOverflow` to trigger an automatic `DriftHealer` reconciliation pass.

6. **Zero-Overhead Host Impact**:
   - Lower the priority of background crawler and reindexer threads (`THREAD_PRIORITY_BELOW_NORMAL` / EcoQoS).
   - Insert cooperative `tokio::task::yield_now().await` checkpoints in the reindexing candidate loop.
   - Stream candidates via bounded `mpsc` channels instead of buffering entire directory trees in `Vec<DiscoveredCandidate>`.

---

## 5. Verification Method

To independently verify the architecture, observations, and proposed resolutions:

### 5.1 Codebase Inspection Commands
```powershell
# 1. Inspect physical drive discovery logic and filesystem label filtering
view_file -AbsolutePath "d:\harfile\QuorumRAG\crates\quorum-ingest\src\config.rs" -StartLine 185 -EndLine 325

# 2. Inspect TwoLevelDedupEngine initialization and empty default caches
view_file -AbsolutePath "d:\harfile\QuorumRAG\crates\quorum-ingest\src\dedup.rs" -StartLine 325 -EndLine 345

# 3. Inspect reindex slice expansion and incremental files_total_estimated undercounting
view_file -AbsolutePath "d:\harfile\QuorumRAG\crates\quorum-ingest\src\reindex.rs" -StartLine 280 -EndLine 315

# 4. Inspect binary classification and file opening in classifier router
view_file -AbsolutePath "d:\harfile\QuorumRAG\crates\quorum-ingest\src\classifier_router.rs" -StartLine 95 -EndLine 165

# 5. Inspect notify watcher event filtering and directory omission
view_file -AbsolutePath "d:\harfile\QuorumRAG\crates\quorum-ingest\src\watcher.rs" -StartLine 120 -EndLine 160
```

### 5.2 Automated Test Execution
Run the existing automated test suite for ingestion, deduplication, and classifier components:
```powershell
cargo test -p quorum-ingest --test ingest_tests
cargo test -p quorum-classifier --lib
cargo test --test test_ingestion_dedup
```

### 5.3 Invalidation Conditions
This analysis would be invalidated if:
1. LanceDB already contained an automatic in-memory Level-1 file catalog with mtime tracking that bypassed `TwoLevelDedupEngine` (refuted by `crates/quorum-storage/src/vector_store.rs`, which stores only raw Arrow RecordBatches without mtime metadata).
2. `FilesystemWatcher` was already active and running inside `quorum-daemon` (refuted by `crates/quorum-daemon/src/main.rs:196-246`, which solely invokes `ReindexWorker` in a 12-second polling sleep loop).
3. The crawler already deterministically excluded `.exe` and `.dll` extensions prior to disk I/O (refuted by `crates/quorum-ingest/src/crawler.rs:246-263`, which only filters lockfiles and temporary office lock patterns).
