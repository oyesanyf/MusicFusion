# Investigation & Implementation Report: Feature 47 (Watcher Resilience & Zero-Overhead Host Throttling)

## 1. Observation

Direct code observations from source inspection across `crates/quorum-ingest`, `crates/quorum-classifier`, and `crates/quorum-storage`:

### 1.1 Watcher Directory Event Dropping (`crates/quorum-ingest/src/watcher.rs:120-155`)
In `FilesystemWatcher::dispatch_debounced_event`:
```rust
121:         match event.kind {
122:             EventKind::Create(_) => {
123:                 for path in &event.paths {
124:                     if path.is_file() {
125:                         let _ = tx.try_send(WatcherEvent::Create(path.clone()));
126:                     }
127:                 }
128:             }
...
144:             EventKind::Modify(_) => {
145:                 for path in &event.paths {
146:                     if path.is_file() {
147:                         let _ = tx.try_send(WatcherEvent::Modify(path.clone()));
148:                     }
149:                 }
150:             }
```
- Line 124 and Line 146 test `if path.is_file()`.
- When a new directory is created or an existing directory is modified (e.g., a folder tree copied or pasted into a watched drive), `path.is_file()` returns `false`.
- The watcher drops the event entirely. None of the files inside the newly created directory are discovered or emitted as events.

### 1.2 Directory Rename Defect (`crates/quorum-ingest/src/engine.rs:259-273`)
In `IngestEngine::handle_watcher_event`:
```rust
259:             WatcherEvent::Rename { from, to } => {
260:                 let from_str = from.to_string_lossy().to_string();
261:                 let count = self.storage.delete_by_file_path(&from_str).await?;
262:                 self.dedup.remove_file(&from);
263:                 if count > 0 {
264:                     storage_mutated = true;
265:                 }
266:                 if to.is_file() {
267:                     if let Some(chunks) = self.ingest_file(&to).await? {
268:                         if !chunks.is_empty() {
269:                             storage_mutated = true;
270:                         }
271:                     }
272:                 }
273:             }
```
- In `crates/quorum-storage/src/engine.rs:395`, `delete_by_file_path` performs an exact match (`file_path = 'from_str'`).
- If `from` was a directory (e.g. `C:\docs\projectA`), stored chunks have paths prefixed with `C:\docs\projectA\...`. Exact match on `from_str` deletes 0 chunks.
- In Line 266, `if to.is_file()` is `false` because `to` is a directory (`to.is_dir()`).
- Result: none of the files under the new directory path are ingested, and all chunks under the old directory path remain orphaned in storage and the deduplication catalog.

### 1.3 Windows Error 32 Sharing Violation During Active Writes (`crates/quorum-classifier/src/lib.rs:218` & `crates/quorum-ingest/src/classifier_router.rs:122-125`)
In `crates/quorum-classifier/src/lib.rs:218`:
```rust
204:     pub fn classify_file(&self, path: impl AsRef<Path>) -> Result<FileIdentity, std::io::Error> {
205:         let path = path.as_ref();
...
218:         let mut file = File::open(path)?;
219:         let mut buffer = [0u8; 4096];
220:         let bytes_read = file.read(&mut buffer)?;
```
In `crates/quorum-ingest/src/classifier_router.rs:97-125`:
```rust
97:         let metadata = match self.read_metadata_with_retry(path, 3) {
98:             Ok(m) => m,
99:             Err(e) => return FilterDecision::SkipIoError { error: e.to_string() },
100:         };
...
122:         let identity = match self.classifier.classify_file(path) {
123:             Ok(id) => id,
124:             Err(e) => return FilterDecision::SkipIoError { error: e.to_string() },
125:         };
```
- While `read_metadata_with_retry` (lines 192-206) retries `std::fs::metadata(path)` 3 times, `classify_file` executes a direct `File::open(path)` with zero retries.
- On Windows, if a file is being actively written, downloaded, or copied, `File::open` fails immediately with Win32 Error 32 (`ERROR_SHARING_VIOLATION`, raw OS error 32).
- `evaluate_file` returns `FilterDecision::SkipIoError`, causing `IngestEngine::ingest_file` to drop the file permanently without recovery.

### 1.4 Silent Event Dropping on Internal Channel Saturation (`crates/quorum-ingest/src/watcher.rs:65-85, 125, 134, 147, 153`)
In `FilesystemWatcher::start_watching`:
```rust
61:         let (tx, rx) = mpsc::channel(10_000);
```
In `FilesystemWatcher::dispatch_debounced_event`:
```rust
125:         let _ = tx.try_send(WatcherEvent::Create(path.clone()));
134:         let _ = tx.try_send(WatcherEvent::Rename { from, to });
147:         let _ = tx.try_send(WatcherEvent::Modify(path.clone()));
153:         let _ = tx.try_send(WatcherEvent::Remove(path.clone()));
```
- `tx.try_send(...)` return values are unconditionally discarded with `let _ =`.
- When rapid bursts of filesystem changes exceed the 10,000-event queue capacity (e.g. large file unzipping or batch git checkouts), `try_send` fails and events are silently dropped.
- `WatcherEvent::QueueOverflow` is currently only emitted for native OS watcher driver errors (lines 78-81), never when the internal channel fills up.

### 1.5 Uncooperative Async Loop & Missing Thread Priority Throttling (`crates/quorum-ingest/src/reindex.rs:303-491`)
In `ReindexWorker::execute_pipeline`:
```rust
303:             for candidate in candidates {
304:                 if cancel_flag.load(Ordering::SeqCst) {
305:                     return Err(IngestError::Cancelled);
306:                 }
...
370:                         let raw_chunks = chunker.chunk_text(&content);
...
386:                                     let vec = embedder
387:                                         .embed_text(raw.text_slice)
...
479:                 if batch_chunks.len() >= options.batch_size {
480:                     if !options.dry_run {
481:                         let _ = storage.insert_chunks(&batch_chunks).await?;
482:                         let _ = storage.bump_index_epoch("reindex_micro_batch");
483:                     }
484:                     batch_chunks.clear();
485:                 }
```
- Lines 303-491 run a tight loop over thousands of candidates without calling `tokio::task::yield_now().await`.
- Sequential execution of heavy neural embeddings (Candle / Gemma) and storage writes monopolizes the Tokio worker thread, causing HTTP request latency spikes and UI freezes.
- Neither `FilesystemCrawler` nor `ReindexWorker` sets background thread or I/O priority on Windows (`THREAD_MODE_BACKGROUND_BEGIN` or `THREAD_PRIORITY_BELOW_NORMAL`), allowing indexing I/O to compete directly with host user disk operations.

---

## 2. Logic Chain

1. **Watcher Directory Event Traversal**:
   - Because `dispatch_debounced_event` checks `if path.is_file()` (Observation 1.1), directory events are discarded.
   - When a user moves or creates a directory containing 1,000 files, the OS notifies on the folder itself or its immediate children.
   - Replacing the single-file guard with a check `if path.is_dir()` that invokes `walkdir::WalkDir::new(path)` to traverse and dispatch individual file events ensures complete discovery of newly introduced directory trees.

2. **Directory Rename Chunk & Catalog Reconciliation**:
   - When a directory is renamed from `from` to `to` (Observation 1.2), `to.is_dir()` is true and `from` no longer exists on disk.
   - The stored chunks for files previously under `from` possess `file_path` values starting with the `from` path string.
   - To properly handle directory renames:
     1. Retrieve all tracked files from `TwoLevelDedupEngine` (and scan storage) that start with `from`.
     2. Delete each matching chunk via `storage.delete_by_file_path` and remove from dedup catalog.
     3. Recursively traverse `to` and call `ingest_file` on each newly located file.
   - This eliminates orphaned chunk state and guarantees seamless directory relocations.

3. **Windows Error 32 Resilience**:
   - Because Windows locks active write handles exclusively (Observation 1.3), `File::open` generates transient raw OS error 32.
   - Wrapping `File::open` in an exponential backoff loop with 3 retry attempts (25ms, 50ms, 100ms) specifically targeting sharing violations (`err.raw_os_error() == Some(32)` or `ErrorKind::PermissionDenied`) allows external processes (e.g. Word, Git, compilers) to finish closing their handles without dropping files from the ingestion pipeline.

4. **Queue Overflow Drift Healing**:
   - Because internal channel buffer saturation is currently silenced by `let _ =` (Observation 1.4), dropped events lead to silent index drift.
   - When `tx.try_send` fails, immediately emitting `WatcherEvent::QueueOverflow` triggers `IngestEngine::handle_watcher_event`, which calls `DriftHealer::fix_all_drifts` in the background.
   - If the channel is completely full, launching a background thread to deliver the `QueueOverflow` notification via non-blocking or blocking dispatch guarantees the drift healer will reconcile disk state with storage as soon as the queue drains.

5. **Host Throttling & Zero-Overhead Concurrency**:
   - Heavy embedding loops monopolize CPU cores and starve tokio tasks unless cooperative yield points are inserted (Observation 1.5).
   - Calling `tokio::task::yield_now().await` once per candidate file and after flushing each micro-batch gives immediate scheduling priority to Web UI queries and cancellation signals.
   - Calling Windows Win32 API `SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN)` (`0x00010000`) on the crawler/reindexing worker thread lowers both CPU scheduling priority and Windows disk I/O priority to `Very Low` (`IoPriorityVeryLow`), ensuring background indexing operations never interfere with foreground disk throughput.

---

## 3. Caveats

1. **Windows `THREAD_MODE_BACKGROUND_BEGIN` Availability**: This Win32 API has been available since Windows Vista and does not require administrative privileges. If running inside a minimal Windows container or Wine where background mode returns an error, the code safely falls back to `THREAD_PRIORITY_BELOW_NORMAL` (`-1`). On non-Windows platforms, it safely operates as a no-op.
2. **Directory Walk on Large Hierarchies**: When a directory containing tens of thousands of files is created or renamed, recursive walking in `dispatch_debounced_event` emits many events in sequence. The queue overflow handler guarantees that even if the 10,000-event queue is exceeded, the `DriftHealer` takes over and reconciles any excess without dropping files.
3. **Sharing Violation Maximum Timeout**: The exponential backoff (25ms + 50ms + 100ms = 175ms total) handles transient lock contention from standard file saving and compilation tools. If a file remains locked indefinitely by an exclusive process for minutes, it will fail after 3 retries and be picked up by the subsequent periodic drift healing pass once released.

---

## 4. Conclusion & Concrete Code Specifications

The following concrete implementations must be applied by the implementation agent.

### 4.1 Feature 47 Area 1: Watcher Directory & Rename Event Handling

#### File A: `crates/quorum-ingest/src/watcher.rs:118-161`
Replace `dispatch_debounced_event` with:
```rust
    /// Helper to safely dispatch events with QueueOverflow emission on channel saturation.
    fn send_event(tx: &mpsc::Sender<WatcherEvent>, event: WatcherEvent, fallback_root: &Path) {
        if let Err(e) = tx.try_send(event) {
            warn!(path = %fallback_root.display(), error = %e, "Watcher event channel full; emitting QueueOverflow");
            if tx.try_send(WatcherEvent::QueueOverflow {
                root: fallback_root.to_path_buf(),
                details: format!("Internal channel capacity saturated: {}", e),
            }).is_err() {
                let tx_clone = tx.clone();
                let root_buf = fallback_root.to_path_buf();
                let details = format!("Internal channel saturated: {}", e);
                std::thread::spawn(move || {
                    let _ = tx_clone.blocking_send(WatcherEvent::QueueOverflow {
                        root: root_buf,
                        details,
                    });
                });
            }
        }
    }

    /// Recursively emits individual file events for a directory.
    fn dispatch_dir_files<F>(dir: &Path, tx: &mpsc::Sender<WatcherEvent>, constructor: F)
    where
        F: Fn(PathBuf) -> WatcherEvent,
    {
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                Self::send_event(tx, constructor(entry.into_path()), dir);
            }
        }
    }

    /// Maps native `DebouncedEvent` into QuorumRAG `WatcherEvent`.
    fn dispatch_debounced_event(event: &DebouncedEvent, tx: &mpsc::Sender<WatcherEvent>) {
        use notify::event::ModifyKind;

        match event.kind {
            EventKind::Create(_) => {
                for path in &event.paths {
                    if path.is_file() {
                        Self::send_event(tx, WatcherEvent::Create(path.clone()), path);
                    } else if path.is_dir() {
                        Self::dispatch_dir_files(path, tx, WatcherEvent::Create);
                    }
                }
            }
            EventKind::Modify(ModifyKind::Name(_)) => {
                // Rename or move event
                if event.paths.len() >= 2 {
                    let from = event.paths[0].clone();
                    let to = event.paths[1].clone();
                    Self::send_event(tx, WatcherEvent::Rename { from, to }, &event.paths[0]);
                } else if let Some(path) = event.paths.first() {
                    // Single-path rename (e.g. rename from outside watch dir)
                    if path.exists() {
                        if path.is_file() {
                            Self::send_event(tx, WatcherEvent::Create(path.clone()), path);
                        } else if path.is_dir() {
                            Self::dispatch_dir_files(path, tx, WatcherEvent::Create);
                        }
                    } else {
                        Self::send_event(tx, WatcherEvent::Remove(path.clone()), path);
                    }
                }
            }
            EventKind::Modify(_) => {
                for path in &event.paths {
                    if path.is_file() {
                        Self::send_event(tx, WatcherEvent::Modify(path.clone()), path);
                    } else if path.is_dir() {
                        Self::dispatch_dir_files(path, tx, WatcherEvent::Modify);
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in &event.paths {
                    Self::send_event(tx, WatcherEvent::Remove(path.clone()), path);
                }
            }
            EventKind::Other => {
                debug!("Received non-actionable watcher event");
            }
            _ => {}
        }
    }
```

#### File B: `crates/quorum-ingest/src/engine.rs:250-275`
Replace `WatcherEvent::Remove` and `WatcherEvent::Rename` handling in `handle_watcher_event`:
```rust
            WatcherEvent::Remove(path) => {
                let path_str = path.to_string_lossy().to_string();
                let mut count = self.storage.delete_by_file_path(&path_str).await?;
                self.dedup.remove_file(&path);

                // Handle directory deletion: purge all tracked children under path
                let tracked = self.dedup.get_all_tracked_files();
                for tracked_file in tracked {
                    if tracked_file.starts_with(&path) {
                        let tracked_str = tracked_file.to_string_lossy().to_string();
                        let c = self.storage.delete_by_file_path(&tracked_str).await.unwrap_or(0);
                        self.dedup.remove_file(&tracked_file);
                        count += c;
                    }
                }

                if count > 0 {
                    storage_mutated = true;
                }
                debug!(path = %path.display(), count, "Removed deleted file or directory from storage and catalog");
            }
            WatcherEvent::Rename { from, to } => {
                if to.is_dir() {
                    // Directory Rename:
                    // 1. Delete all chunks and catalog entries with old path prefix
                    let tracked = self.dedup.get_all_tracked_files();
                    for tracked_file in tracked {
                        if tracked_file.starts_with(&from) {
                            let old_str = tracked_file.to_string_lossy().to_string();
                            let c = self.storage.delete_by_file_path(&old_str).await.unwrap_or(0);
                            self.dedup.remove_file(&tracked_file);
                            if c > 0 {
                                storage_mutated = true;
                            }
                        }
                    }

                    // 2. Recursively index all files under the new directory path
                    for entry in walkdir::WalkDir::new(&to).into_iter().filter_map(|e| e.ok()) {
                        if entry.file_type().is_file() {
                            let new_file_path = entry.into_path();
                            if let Some(chunks) = self.ingest_file(&new_file_path).await? {
                                if !chunks.is_empty() {
                                    storage_mutated = true;
                                }
                            }
                        }
                    }
                } else {
                    // Single File Rename:
                    let from_str = from.to_string_lossy().to_string();
                    let count = self.storage.delete_by_file_path(&from_str).await?;
                    self.dedup.remove_file(&from);
                    if count > 0 {
                        storage_mutated = true;
                    }
                    if to.is_file() {
                        if let Some(chunks) = self.ingest_file(&to).await? {
                            if !chunks.is_empty() {
                                storage_mutated = true;
                            }
                        }
                    }
                }
            }
```

---

### 4.2 Feature 47 Area 2: Windows Sharing Violation (Error 32) Resilience

#### File C: `crates/quorum-classifier/src/lib.rs:204-225`
Add retry loop to `classify_file` with exponential backoff (25ms, 50ms, 100ms):
```rust
    /// Helper to open a file with retry on transient Windows Error 32 sharing violations.
    pub fn open_file_with_retry(path: &Path) -> Result<File, std::io::Error> {
        const BACKOFFS_MS: [u64; 3] = [25, 50, 100];
        let mut last_err = None;

        for attempt in 0..=BACKOFFS_MS.len() {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(BACKOFFS_MS[attempt - 1]));
            }

            match File::open(path) {
                Ok(file) => return Ok(file),
                Err(err) => {
                    let is_sharing_violation = err.raw_os_error() == Some(32)
                        || err.kind() == std::io::ErrorKind::PermissionDenied;
                    last_err = Some(err);
                    if !is_sharing_violation && attempt > 0 {
                        break;
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| std::io::Error::other("Failed to open file after retries")))
    }

    /// Classify a file given its filesystem path.
    ///
    /// Reads up to the first 4096 bytes for content inspection.
    pub fn classify_file(&self, path: impl AsRef<Path>) -> Result<FileIdentity, std::io::Error> {
        let path = path.as_ref();
        if path.is_dir() {
            return Ok(FileIdentity::new(
                "directory",
                "inode/directory",
                FileCategory::Unknown,
                "Directory",
                false,
                1.0,
                ClassifierEngine::HeaderInspection,
            ));
        }

        let mut file = Self::open_file_with_retry(path)?;
        let mut buffer = [0u8; 4096];
        let bytes_read = file.read(&mut buffer)?;
        let filename_hint = path.file_name().and_then(|n| n.to_str());

        Ok(self.classify_bytes(&buffer[..bytes_read], filename_hint))
    }
```

#### File D: `crates/quorum-ingest/src/classifier_router.rs:122-125`
Update `evaluate_file` to use retry logic:
```rust
        // 4. Content classification via Google Magika (first 4KB inspection with sharing violation retry)
        let identity = match self.classify_file_with_retry(path, 3) {
            Ok(id) => id,
            Err(e) => return FilterDecision::SkipIoError { error: e.to_string() },
        };
```
And add method:
```rust
    fn classify_file_with_retry(&self, path: &Path, retries: usize) -> std::io::Result<FileIdentity> {
        const BACKOFFS_MS: [u64; 3] = [25, 50, 100];
        let mut last_err = None;
        for attempt in 0..retries {
            match self.classifier.classify_file(path) {
                Ok(id) => return Ok(id),
                Err(e) => {
                    let is_sharing_violation = e.raw_os_error() == Some(32)
                        || e.kind() == std::io::ErrorKind::PermissionDenied;
                    last_err = Some(e);
                    if !is_sharing_violation {
                        break;
                    }
                    if attempt + 1 < retries {
                        let delay = BACKOFFS_MS.get(attempt).copied().unwrap_or(100);
                        std::thread::sleep(std::time::Duration::from_millis(delay));
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| std::io::Error::other("Classification failed")))
    }
```

#### File E: `crates/quorum-ingest/src/dedup.rs:94-110`
In `Level1DedupCatalog::hash_file_stream`, also use `quorum_classifier::FileTypeClassifier::open_file_with_retry(path.as_ref())` so streaming hashing is resilient to Windows Error 32 sharing violations during file write finalization.

---

### 4.3 Feature 47 Area 3: Queue Overflow Drift Healing
- Addressed simultaneously in `crates/quorum-ingest/src/watcher.rs:send_event` (Section 4.1).
- When `tx.try_send` fails, `WatcherEvent::QueueOverflow { root, details }` is dispatched immediately.
- If queue is fully saturated, a background task performs `tx.blocking_send` to ensure the event is queued as soon as space opens.
- In `crates/quorum-ingest/src/engine.rs:274-280`, `WatcherEvent::QueueOverflow` automatically schedules `self.drift_healer.fix_all_drifts()`.

---

### 4.4 Feature 47 Area 4: Zero-Overhead Host Impact

#### File F: New Module `crates/quorum-ingest/src/throttling.rs`
Create zero-dependency thread throttling module:
```rust
//! Host Overhead Throttling & Background Thread Priority Control.

#[cfg(windows)]
mod platform {
    extern "system" {
        fn GetCurrentThread() -> isize;
        fn SetThreadPriority(hThread: isize, nPriority: i32) -> i32;
    }

    const THREAD_MODE_BACKGROUND_BEGIN: i32 = 0x00010000;
    const THREAD_MODE_BACKGROUND_END: i32 = 0x00020000;
    const THREAD_PRIORITY_BELOW_NORMAL: i32 = -1;

    /// Sets the current thread to background processing mode.
    /// Lowers CPU priority to background and disk I/O priority to IoPriorityVeryLow.
    pub fn set_background_thread_priority() -> bool {
        unsafe {
            // Prefer Windows background mode (both CPU and I/O priority lowered)
            if SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN) != 0 {
                return true;
            }
            // Fallback to below-normal CPU priority
            SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) != 0
        }
    }

    /// Restores thread priority back to normal.
    pub fn end_background_thread_priority() -> bool {
        unsafe {
            SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_END) != 0
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn set_background_thread_priority() -> bool {
        false
    }

    pub fn end_background_thread_priority() -> bool {
        false
    }
}

pub use platform::{end_background_thread_priority, set_background_thread_priority};
```
Export this in `crates/quorum-ingest/src/lib.rs`:
```rust
pub mod throttling;
pub use throttling::{end_background_thread_priority, set_background_thread_priority};
```

#### File G: `crates/quorum-ingest/src/crawler.rs:108, 126` & `reindex.rs:286`
At the beginning of each `tokio::task::spawn_blocking` closure:
```rust
crate::throttling::set_background_thread_priority();
```
This guarantees the OS-level crawling and drive traversal runs with `IoPriorityVeryLow` and background CPU priority.

#### File H: `crates/quorum-ingest/src/reindex.rs:303-491`
Insert cooperative async yields in candidate processing loop:
```rust
            for candidate in candidates {
                if cancel_flag.load(Ordering::SeqCst) {
                    return Err(IngestError::Cancelled);
                }

                // Cooperative multitasking yield point
                tokio::task::yield_now().await;
...
                // Inside raw_chunks loop: yield after every 8 chunks for large files
                if raw.chunk_index > 0 && raw.chunk_index % 8 == 0 {
                    tokio::task::yield_now().await;
                }
...
                if batch_chunks.len() >= options.batch_size {
                    if !options.dry_run {
                        let _ = storage.insert_chunks(&batch_chunks).await?;
                        let _ = storage.bump_index_epoch("reindex_micro_batch");
                    }
                    batch_chunks.clear();
                    tokio::task::yield_now().await;
                }
```

---

## 5. Verification Method

### 5.1 Verification Commands
Run the target test targets to verify compilation and test passing:
```powershell
# 1. Test quorum-classifier with retry logic
cargo test -p quorum-classifier --lib

# 2. Test quorum-ingest watcher, reindexing, and drift healing
cargo test -p quorum-ingest --test ingest_tests
cargo test -p quorum-ingest --test adversarial_reindex_and_drift
cargo test -p quorum-ingest --test adversarial_challenge_tests

# 3. Full project workspace compile and test
cargo test --workspace --exclude quorum-ui
```

### 5.2 Specific Regression Tests to Add
1. **Directory Recursive Ingestion Test** (`tests/watcher_resilience_tests.rs`):
   Create a nested folder `test_folder/sub_folder/` with two `.md` files. Dispatch `WatcherEvent::Create(test_folder)`. Assert both `.md` files are ingested and searchable in storage.
2. **Directory Rename Test**:
   Rename `test_folder` to `renamed_folder`. Dispatch `WatcherEvent::Rename { from: test_folder, to: renamed_folder }`. Assert storage returns 0 hits for queries matching old path strings and returns hits with updated paths.
3. **Windows Error 32 Retry Test**:
   Open a file exclusively with write access on Windows, invoke `FileTypeClassifier::classify_file` in a separate thread, release the file handle after 40ms. Assert `classify_file` successfully classifies the file on retry attempt 2 without error.
4. **Queue Overflow Emission Test**:
   Flood watcher channel with 10,000 dummy events, send 10,001st event. Assert a `WatcherEvent::QueueOverflow` event is received by the receiver.
5. **Cancellation & Yield Responsiveness Test**:
   Start a reindex over 500 files, immediately assert `cancel_flag.store(true)`. Assert the reindexer terminates with `IngestError::Cancelled` within 10ms due to `yield_now().await`.

### 5.3 Invalidation Conditions
This investigation would be invalidated if:
1. `FilesystemWatcher` was already recursively expanding directories in `dispatch_debounced_event` (refuted by line 124 of `watcher.rs`).
2. `delete_by_file_path` supported SQL glob/prefix matching out of the box (refuted by `VectorStore::delete_by_file_path` and `TantivyFtsIndex::delete_by_file_path`, which enforce exact equality).
3. `File::open` had built-in retry logic in `quorum-classifier` (refuted by line 218 of `quorum-classifier/src/lib.rs`).
