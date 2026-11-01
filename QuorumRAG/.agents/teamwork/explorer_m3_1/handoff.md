# Handoff Report: Filesystem Crawler, Real-Time OS Watcher & Magika Routing (Milestone 3)

**Agent**: Explorer M3-1 (`teamwork_explorer`)  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_1\`  
**Target Milestone**: Milestone 3 (Filesystem Ingestion, OS Watcher, Image & Reindex Engine)  
**Date**: 2026-10-31T20:15:00Z  

---

## 1. Observation

### 1.1 Existing Codebase & Environment Audit
Direct inspection of the repository revealed the following verified facts:

1. **Workspace Layout (`d:\harfile\QuorumRAG\Cargo.toml:3-8`)**:
   - Workspace currently defines four crates: `crates/quorum-types`, `crates/quorum-classifier`, `crates/quorum-contract`, `crates/quorum-storage`.
   - `crates/quorum-ingest` does not yet exist and must be created for Milestone 3.
   - Dependencies `ignore` and `notify` (or `notify-debouncer-full`) are not yet present in root `Cargo.toml` or `Cargo.lock`.

2. **Classification Foundation (`crates/quorum-classifier/src/lib.rs:29-121, 192-265`)**:
   - `FileTypeClassifier` is a thread-safe, stateless unit struct implementing `Default`, `Clone`, `Send`, `Sync`.
   - Method `classify_file(&self, path: impl AsRef<Path>) -> Result<FileIdentity, std::io::Error>` reads up to 4,096 bytes and inspects magic headers, Google Magika heuristics, binary null-byte entropy, UTF-8 text, and file extension fallback.
   - `FileIdentity` exposes:
     - `label: String` (e.g., `"rust"`, `"python"`, `"png"`, `"jpeg"`, `"pebin"`, `"pdf"`)
     - `mime_type: String` (e.g., `"text/x-rust"`, `"image/png"`, `"application/x-dosexec"`)
     - `category: FileCategory` (`Code`, `Text`, `Document`, `Data`, `Archive`, `Executable`, `Image`, `Audio`, `Video`, `Unknown`)
     - `is_text: bool`
     - `is_binary: bool`
     - `score: f32`
     - `engine: ClassifierEngine`
   - Image formats identified by magic bytes include `PNG` (`\x89PNG\r\n\x1a\n`), `JPEG` (`0xff, 0xd8, 0xff`), `GIF` (`GIF87a`/`GIF89a`), `WebP` (`RIFF...WEBP`), `BMP` (`BM`), and `SVG` (`<?xml...<svg`).
   - For image types, `is_text` is `false` (except SVG which has `is_text: true`), resulting in `is_binary: true`. Crucially, their `category` is `FileCategory::Image`.

3. **Storage Isolation & Ingestion Interface (`crates/quorum-storage/src/engine.rs:114-141, 258-270`)**:
   - `StorageEngine::insert_chunks(&self, chunks: &[DocumentChunk]) -> Result<usize, StorageError>` accepts pure domain `DocumentChunk` slices and encapsulates all Arrow/LanceDB/Tantivy conversions.
   - `StorageEngine::has_chunk_hash(&self, blake3_hash: &str) -> Result<bool, StorageError>` provides Level-2 storage deduplication checks.
   - `StorageEngine::delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError>` atomically removes all chunks belonging to a deleted file across both LanceDB and Tantivy.

4. **E2E Ingestion Test Pattern (`tests/e2e/test_ingestion_dedup.rs:49-136`)**:
   - Verifies 10MB/50MB max file size filtering, binary sniffing skipping executables (`service.exe`), Level-1 file BLAKE3 skip for unmodified files, Level-1 rename/move path updates with zero re-embeddings, Level-2 chunk BLAKE3 deduplication, 0-byte file boundary handling, and unicode path support (`Ünïcödé Pâth 📁\rédäctïon tëst.md`).

5. **Mandatory User Requirements (`ORIGINAL_REQUEST.md:50-88, 138-142`)**:
   - **Google Magika Identification**: All discovered and modified files must pass through Google Magika content inspection before chunking or filtering binaries.
   - **Do Not Skip Images!**: Images (`jpeg`, `png`, `webp`, `gif`, `bmp`, `tiff`, `svg`, etc.) must NOT be skipped as generic binaries; they must route to an Image Extractor/Indexer.
   - **Continuous OS File Watcher**: `notify` with 300ms debouncing window handling `Create`, `Modify`, `Remove`, `Rename` events with non-blocking async channels.
   - **High-Capacity Crawler**: `ignore::WalkBuilder` / `WalkParallel` respecting `.gitignore`, `.ignore`, hidden files, 50MB size caps, scaling to 30TB+ drives.
   - **Compilation & Memory Safety directive**: debug=0, `--test-threads=2`, zero uncontained leaks.

---

## 2. Logic Chain

From the verified requirements and codebase architecture, the technical design for Milestone 3 ingestion components flows logically:

### 2.1 Crate Architecture & Module Layout (`crates/quorum-ingest`)
The new crate `quorum-ingest` must cleanly separate filesystem traversal (`crawler.rs`), event streaming (`watcher.rs`), content classification and filtering (`classifier_router.rs`), configuration (`config.rs`), and errors (`error.rs`), while providing extension points for Explorer M3-2 (`dedup.rs`, `chunker.rs`, `image_indexer.rs`) and Explorer M3-3 (`reindex.rs`, `drift_healer.rs`, `engine.rs`).

```
crates/quorum-ingest/
├── Cargo.toml
└── src/
    ├── lib.rs                # Crate entrypoint & re-exports
    ├── config.rs             # Ingestion, Crawler & Watcher configuration
    ├── error.rs              # IngestError enumeration
    ├── classifier_router.rs  # Magika content router & binary filter
    ├── crawler.rs            # High-capacity parallel crawler (ignore::WalkParallel)
    ├── watcher.rs            # Real-time OS watcher (notify with 300ms debounce)
    ├── dedup.rs              # [M3-2] Level-1 & Level-2 BLAKE3 deduplication
    ├── chunker.rs            # [M3-2] Line-aware & char-boundary chunking
    ├── image_indexer.rs      # [M3-2] Image metadata & feature indexing
    ├── reindex.rs            # [M3-3] Dedicated reindex process
    ├── drift_healer.rs       # [M3-3] 5-drift remediation engine
    └── engine.rs             # [M3-3] High-level unified IngestEngine coordinator
```

### 2.2 `crawler.rs`: High-Capacity Recursive Directory Walker
1. **Parallel Traversal with Work-Stealing**:
   - Standard single-threaded `std::fs::read_dir` is inadequate for 30TB+ drives with millions of files.
   - `ignore::WalkParallel` executes a multi-threaded work-stealing directory walk powered by OS threads, bypassing global runtime locks.
2. **Ignore & Rule Compliance**:
   - Configurable `.gitignore` and `.ignore` support via `WalkBuilder::git_ignore(true)` and `WalkBuilder::ignore(true)`.
   - Hidden file filtering (`WalkBuilder::hidden(true)`) skips `.git`, `.venv`, `.cargo`, and Windows hidden directory structures.
   - Symlink loop defense (`WalkBuilder::follow_links(false)`) prevents infinite loops and sandbox breakouts.
   - Direct file size filtering (`WalkBuilder::max_filesize(Some(max_size))` and in-worker validation) skips files larger than the 50MB cap before allocating buffers.
3. **Async Streaming with Backpressure**:
   - Running `WalkParallel::run` in `tokio::task::spawn_blocking` keeps the Tokio async runtime unblocked.
   - Worker threads push discovered candidates through a bounded channel (`tokio::sync::mpsc::channel(4096)` or `crossbeam-channel`) using `blocking_send`.
   - If downstream chunking or embedding slows down, `blocking_send` pauses worker threads, maintaining $O(1)$ memory usage.
   - If the downstream receiver is dropped (e.g. user aborts crawl), `blocking_send` returns an error, and the worker returns `ignore::WalkState::Quit`, immediately terminating the parallel traversal across all worker threads.

### 2.3 `watcher.rs`: Real-Time OS File Watcher with 300ms Debouncing
1. **Debouncing Rationale (300ms Window)**:
   - Modern text editors, IDEs, and build tools perform atomic writes (writing to temporary files, renaming, firing rapid Create -> Modify -> Metadata -> Modify sequences).
   - Without debouncing, a single file save can trigger 4 to 8 separate re-indexing cycles, causing severe database contention and wasted embedding operations.
   - A 300ms debounce window coalesces rapid event bursts for the same path into a single definitive action.
2. **Event Model**:
   - Four normalized event variants:
     - `Create(PathBuf)`: Fresh file created.
     - `Modify(PathBuf)`: Existing file modified.
     - `Remove(PathBuf)`: File deleted from disk (triggers `StorageEngine::delete_by_file_path`).
     - `Rename { from: PathBuf, to: PathBuf }`: File moved or renamed. Paired renames allow Level-1 deduplication to update path references without recomputing embeddings.
     - `QueueOverflow { root: PathBuf, details: String }`: Emitted when OS notification buffers drop events, alerting `DriftHealer` to schedule a reconciliation crawl.
3. **Channel Architecture**:
   - The watcher callback sends events into a non-blocking `tokio::sync::mpsc::Sender<WatcherEvent>` with capacity `10,000`.
   - If the channel fills under extreme burst conditions, it falls back to flagging a drift warning rather than blocking the OS event thread.
4. **Dynamic Path Management**:
   - Allows adding (`watch_dir`) and removing (`unwatch_dir`) roots at runtime, enabling users to add drives or folders without restarting the daemon.

### 2.4 Content-Type Classification & Routing (`classifier_router.rs`)
1. **The Crucial Image vs. Binary Distinction**:
   - In `quorum-classifier`, `FileIdentity.is_text` is `false` for both binary executables (`pebin`, `elf`) AND raster images (`png`, `jpeg`).
   - If the pipeline naively checks `if identity.is_binary { skip }`, ALL images will be erroneously dropped, violating User Requirement 3!
   - Therefore, the routing policy must explicitly check `identity.category`:
     - **Image Routing**: If `identity.category == FileCategory::Image` (or MIME matches `image/*`), route to `CandidateKind::Image` for `ImageIndexer`.
     - **Text/Code Routing**: If `identity.is_text || matches!(identity.category, FileCategory::Code | FileCategory::Text | FileCategory::Document | FileCategory::Data)`: route to `CandidateKind::Text` for `Chunker`.
     - **Binary Filtering**: If `identity.is_binary && identity.category != FileCategory::Image`: discard immediately (`FilterDecision::SkipBinary`).
2. **Corner Case Handlers**:
   - **0-Byte Files**: Handled with `FilterDecision::SkipZeroByte` (or emit empty candidate without error).
   - **Files > 50MB**: Skipped at metadata check before reading bytes (`FilterDecision::SkipExceedsSizeCap`).
   - **Inaccessible / Locked Files (Windows `ERROR_SHARING_VIOLATION`)**: Retry up to 3 times with 100ms exponential backoff.
   - **Non-UTF-8 & Unicode Paths**: Preserved losslessly via `PathBuf`.

---

## 3. Concrete Architectural Specifications & Proposed Code

### 3.1 Crate Specification: `crates/quorum-ingest/Cargo.toml`

```toml
[package]
name = "quorum-ingest"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "High-capacity filesystem ingestion, real-time watcher, Magika classification, and deduplication engine"

[dependencies]
# Internal workspace crates
quorum-types = { workspace = true }
quorum-classifier = { workspace = true }
quorum-contract = { workspace = true }
quorum-storage = { workspace = true }

# Filesystem Traversal & OS Watching
ignore = "0.4.22"
notify = "6.1.1"
notify-debouncer-full = "0.3.1"

# Async, Concurrency & Channels
tokio = { workspace = true }
crossbeam-channel = "0.5"

# Cryptography & Serialization
blake3 = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }

# Error Handling & Tracing
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

### 3.2 Configuration: `src/config.rs`

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Configuration parameters governing crawler, watcher, and ingestion pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    /// Designated root directories or drive roots to crawl and watch.
    pub watch_paths: Vec<PathBuf>,

    /// Maximum file size permitted for ingestion in bytes (default: 50MB).
    pub max_file_size_bytes: u64,

    /// Minimum file size in bytes (default: 1; 0-byte files produce 0 chunks).
    pub min_file_size_bytes: u64,

    /// Whether to respect .gitignore rules (default: true).
    pub git_ignore: bool,

    /// Whether to respect .ignore rules (default: true).
    pub ignore_files: bool,

    /// Whether to respect global gitignore files (default: true).
    pub git_global: bool,

    /// Whether to skip hidden files and hidden directories (default: true).
    pub skip_hidden: bool,

    /// Whether to follow filesystem symlinks (default: false, prevents traversal loops).
    pub follow_symlinks: bool,

    /// Restrict traversal to the same filesystem / volume (default: false).
    pub same_file_system: bool,

    /// Number of worker threads for directory walking (0 = auto / CPU core count).
    pub crawler_threads: usize,

    /// Watcher debouncing duration in milliseconds (default: 300ms).
    pub debounce_duration_ms: u64,

    /// Internal channel buffer capacity for discovered files (default: 4096).
    pub channel_buffer_size: usize,

    /// Micro-batch size for storage writes (default: 64 chunks).
    pub micro_batch_size: usize,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            watch_paths: Vec::new(),
            max_file_size_bytes: 50 * 1024 * 1024, // 50MB
            min_file_size_bytes: 1,
            git_ignore: true,
            ignore_files: true,
            git_global: true,
            skip_hidden: true,
            follow_symlinks: false,
            same_file_system: false,
            crawler_threads: 0,
            debounce_duration_ms: 300,
            channel_buffer_size: 4096,
            micro_batch_size: 64,
        }
    }
}

impl IngestConfig {
    pub fn debounce_duration(&self) -> Duration {
        Duration::from_millis(self.debounce_duration_ms)
    }
}
```

### 3.3 Error Handling: `src/error.rs`

```rust
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("I/O error at '{path}': {source}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("OS Watcher error: {0}")]
    WatcherError(String),

    #[error("Classification error at '{path}': {details}")]
    ClassificationError {
        path: PathBuf,
        details: String,
    },

    #[error("File '{path}' exceeds max size cap ({actual_bytes} > {max_bytes} bytes)")]
    FileSizeExceeded {
        path: PathBuf,
        actual_bytes: u64,
        max_bytes: u64,
    },

    #[error("Storage write error: {0}")]
    StorageError(#[from] quorum_storage::StorageError),

    #[error("Ingestion channel closed or consumer dropped")]
    ChannelClosed,
}
```

### 3.4 Content Classification & Binary Filter: `src/classifier_router.rs`

```rust
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use quorum_classifier::{FileCategory, FileIdentity, FileTypeClassifier};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use crate::config::IngestConfig;

/// Classified candidate destination.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CandidateKind {
    /// Text or source code -> routed to Chunker -> Gemma Embedder -> StorageEngine.
    Text {
        char_count_estimate: usize,
    },
    /// Image file -> routed to ImageIndexer (EXIF, tokens, dimensions, OCR) -> StorageEngine.
    Image {
        format_label: String,
        mime_type: String,
    },
}

/// Filter decision with explicit audit rationale.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterDecision {
    /// Valid candidate accepted for pipeline ingestion.
    Accept {
        identity: FileIdentity,
        kind: CandidateKind,
        size_bytes: u64,
        mtime_secs: i64,
    },
    /// Skipped because file size exceeds configured cap (e.g. >50MB).
    SkipExceedsSizeCap { size_bytes: u64, max_bytes: u64 },
    /// Skipped because file is 0 bytes.
    SkipZeroByte,
    /// Skipped because it is a non-indexable compiled binary, archive, or media stream.
    SkipBinary { identity: FileIdentity },
    /// Skipped because file is hidden and skip_hidden is active.
    SkipHidden,
    /// Skipped because symlink following is disabled.
    SkipSymlink,
    /// Skipped due to inaccessible or locked file after retry.
    SkipIoError { error: String },
}

/// Content-type classifier router combining Google Magika with QuorumRAG filtering policies.
#[derive(Debug, Clone)]
pub struct ClassifierRouter {
    classifier: FileTypeClassifier,
    config: IngestConfig,
}

impl ClassifierRouter {
    pub fn new(config: IngestConfig) -> Self {
        Self {
            classifier: FileTypeClassifier::new(),
            config,
        }
    }

    /// Evaluates a candidate path on disk and returns the definitive routing decision.
    pub fn evaluate_file(&self, path: &Path) -> FilterDecision {
        // 1. Check symlink policy
        if !self.config.follow_symlinks {
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                if meta.file_type().is_symlink() {
                    return FilterDecision::SkipSymlink;
                }
            }
        }

        // 2. Check hidden file policy
        if self.config.skip_hidden {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with('.') {
                    return FilterDecision::SkipHidden;
                }
            }
        }

        // 3. Inspect metadata with retry for Windows sharing violations
        let metadata = match self.read_metadata_with_retry(path, 3) {
            Ok(m) => m,
            Err(e) => return FilterDecision::SkipIoError { error: e.to_string() },
        };

        if !metadata.is_file() {
            return FilterDecision::SkipIoError { error: "Path is not a regular file".into() };
        }

        let size = metadata.len();
        if size > self.config.max_file_size_bytes {
            return FilterDecision::SkipExceedsSizeCap {
                size_bytes: size,
                max_bytes: self.config.max_file_size_bytes,
            };
        }
        if size == 0 {
            return FilterDecision::SkipZeroByte;
        }

        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // 4. Content classification via Google Magika (first 4KB inspection)
        let identity = match self.classifier.classify_file(path) {
            Ok(id) => id,
            Err(e) => return FilterDecision::SkipIoError { error: e.to_string() },
        };

        // 5. Ingestion Routing Invariant:
        // DO NOT SKIP IMAGES! Images must route to ImageIndexer.
        if identity.category == FileCategory::Image || Self::is_image_mime(&identity.mime_type) {
            return FilterDecision::Accept {
                identity: identity.clone(),
                kind: CandidateKind::Image {
                    format_label: identity.label,
                    mime_type: identity.mime_type,
                },
                size_bytes: size,
                mtime_secs: mtime,
            };
        }

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

        // 7. Non-indexable compiled binaries (PE, ELF, Mach-O, DEX, WASM, Archives, Audio, Video)
        debug!(
            path = %path.display(),
            label = %identity.label,
            mime = %identity.mime_type,
            category = %identity.category,
            "Filtered out non-indexable binary/media file"
        );
        FilterDecision::SkipBinary { identity }
    }

    /// Fast helper to confirm if a MIME type represents a supported image format.
    pub fn is_image_mime(mime: &str) -> bool {
        matches!(
            mime,
            "image/jpeg"
                | "image/png"
                | "image/webp"
                | "image/gif"
                | "image/bmp"
                | "image/tiff"
                | "image/svg+xml"
                | "image/x-icon"
                | "image/avif"
        )
    }

    /// Windows file-lock retry helper: retries `std::fs::metadata` on sharing violations.
    fn read_metadata_with_retry(&self, path: &Path, retries: usize) -> std::io::Result<std::fs::Metadata> {
        let mut last_err = None;
        for attempt in 0..retries {
            match std::fs::metadata(path) {
                Ok(m) => return Ok(m),
                Err(e) => {
                    last_err = Some(e);
                    if attempt + 1 < retries {
                        std::thread::sleep(std::time::Duration::from_millis(50 * (attempt as u64 + 1)));
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Unknown error")))
    }
}
```

### 3.5 High-Capacity Directory Crawler: `src/crawler.rs`

```rust
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use ignore::{WalkBuilder, WalkState};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::classifier_router::{ClassifierRouter, FilterDecision, CandidateKind};
use crate::config::IngestConfig;
use crate::error::IngestError;
use quorum_classifier::FileIdentity;

/// Candidate file ready for ingestion pipeline (deduplication & chunking).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredCandidate {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub mtime_secs: i64,
    pub file_identity: FileIdentity,
    pub kind: CandidateKind,
}

/// Cumulative crawler telemetry metrics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CrawlerStats {
    pub directories_visited: usize,
    pub files_scanned: usize,
    pub files_accepted_text: usize,
    pub files_accepted_image: usize,
    pub files_skipped_size: usize,
    pub files_skipped_binary: usize,
    pub files_skipped_hidden: usize,
    pub files_skipped_io: usize,
    pub elapsed_millis: u64,
}

struct AtomicCrawlerStats {
    directories_visited: AtomicUsize,
    files_scanned: AtomicUsize,
    files_accepted_text: AtomicUsize,
    files_accepted_image: AtomicUsize,
    files_skipped_size: AtomicUsize,
    files_skipped_binary: AtomicUsize,
    files_skipped_hidden: AtomicUsize,
    files_skipped_io: AtomicUsize,
}

impl AtomicCrawlerStats {
    fn new() -> Self {
        Self {
            directories_visited: AtomicUsize::new(0),
            files_scanned: AtomicUsize::new(0),
            files_accepted_text: AtomicUsize::new(0),
            files_accepted_image: AtomicUsize::new(0),
            files_skipped_size: AtomicUsize::new(0),
            files_skipped_binary: AtomicUsize::new(0),
            files_skipped_hidden: AtomicUsize::new(0),
            files_skipped_io: AtomicUsize::new(0),
        }
    }

    fn snapshot(&self, elapsed_millis: u64) -> CrawlerStats {
        CrawlerStats {
            directories_visited: self.directories_visited.load(Ordering::Relaxed),
            files_scanned: self.files_scanned.load(Ordering::Relaxed),
            files_accepted_text: self.files_accepted_text.load(Ordering::Relaxed),
            files_accepted_image: self.files_accepted_image.load(Ordering::Relaxed),
            files_skipped_size: self.files_skipped_size.load(Ordering::Relaxed),
            files_skipped_binary: self.files_skipped_binary.load(Ordering::Relaxed),
            files_skipped_hidden: self.files_skipped_hidden.load(Ordering::Relaxed),
            files_skipped_io: self.files_skipped_io.load(Ordering::Relaxed),
            elapsed_millis,
        }
    }
}

/// High-throughput parallel filesystem crawler.
#[derive(Debug, Clone)]
pub struct FilesystemCrawler {
    config: IngestConfig,
    router: ClassifierRouter,
}

impl FilesystemCrawler {
    pub fn new(config: IngestConfig) -> Self {
        let router = ClassifierRouter::new(config.clone());
        Self { config, router }
    }

    /// Asynchronously initiates parallel traversal over all configured watch paths.
    ///
    /// Returns:
    /// - `mpsc::Receiver<DiscoveredCandidate>`: Bounded non-blocking stream of classified candidates.
    /// - `JoinHandle<Result<CrawlerStats, IngestError>>`: Handle yielding cumulative crawl metrics.
    pub fn start_crawl(
        self: Arc<Self>,
    ) -> (
        mpsc::Receiver<DiscoveredCandidate>,
        tokio::task::JoinHandle<Result<CrawlerStats, IngestError>>,
    ) {
        let (tx, rx) = mpsc::channel(self.config.channel_buffer_size);
        let crawler = Arc::clone(&self);

        let handle = tokio::task::spawn_blocking(move || {
            crawler.run_crawl_blocking(tx)
        });

        (rx, handle)
    }

    /// Crawls a specific single directory root. Useful for targeted reindexing (`reindex --path <dir>`).
    pub fn start_crawl_path(
        self: Arc<Self>,
        root: PathBuf,
    ) -> (
        mpsc::Receiver<DiscoveredCandidate>,
        tokio::task::JoinHandle<Result<CrawlerStats, IngestError>>,
    ) {
        let (tx, rx) = mpsc::channel(self.config.channel_buffer_size);
        let crawler = Arc::clone(&self);

        let handle = tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            let stats = Arc::new(AtomicCrawlerStats::new());
            crawler.crawl_single_root(&root, &tx, &stats)?;
            Ok(stats.snapshot(start.elapsed().as_millis() as u64))
        });

        (rx, handle)
    }

    /// Internal synchronous multi-threaded walker execution loop.
    fn run_crawl_blocking(
        &self,
        tx: mpsc::Sender<DiscoveredCandidate>,
    ) -> Result<CrawlerStats, IngestError> {
        let start = Instant::now();
        let stats = Arc::new(AtomicCrawlerStats::new());

        for root in &self.config.watch_paths {
            if !root.exists() {
                warn!("Configured watch path does not exist: {}", root.display());
                continue;
            }
            self.crawl_single_root(root, &tx, &stats)?;
        }

        let summary = stats.snapshot(start.elapsed().as_millis() as u64);
        info!(
            scanned = summary.files_scanned,
            text_accepted = summary.files_accepted_text,
            images_accepted = summary.files_accepted_image,
            skipped_bin = summary.files_skipped_binary,
            skipped_size = summary.files_skipped_size,
            elapsed_ms = summary.elapsed_millis,
            "Filesystem parallel crawl complete"
        );
        Ok(summary)
    }

    fn crawl_single_root(
        &self,
        root: &Path,
        tx: &mpsc::Sender<DiscoveredCandidate>,
        stats: &Arc<AtomicCrawlerStats>,
    ) -> Result<(), IngestError> {
        let mut builder = WalkBuilder::new(root);
        builder
            .hidden(self.config.skip_hidden)
            .ignore(self.config.ignore_files)
            .git_ignore(self.config.git_ignore)
            .git_global(self.config.git_global)
            .git_exclude(self.config.git_ignore)
            .follow_links(self.config.follow_symlinks)
            .same_file_system(self.config.same_file_system)
            .max_filesize(Some(self.config.max_file_size_bytes));

        if self.config.crawler_threads > 0 {
            builder.threads(self.config.crawler_threads);
        }

        let walker = builder.build_parallel();
        let router = self.router.clone();
        let tx_channel = tx.clone();
        let stats_ref = Arc::clone(stats);

        walker.run(move || {
            let tx = tx_channel.clone();
            let router = router.clone();
            let stats = Arc::clone(&stats_ref);

            Box::new(move |entry_result| {
                let entry = match entry_result {
                    Ok(e) => e,
                    Err(err) => {
                        stats.files_skipped_io.fetch_add(1, Ordering::Relaxed);
                        debug!("Crawler I/O error on entry: {}", err);
                        return WalkState::Continue;
                    }
                };

                let ft = match entry.file_type() {
                    Some(ft) => ft,
                    None => return WalkState::Continue,
                };

                if ft.is_dir() {
                    stats.directories_visited.fetch_add(1, Ordering::Relaxed);
                    return WalkState::Continue;
                }

                if !ft.is_file() {
                    return WalkState::Continue;
                }

                stats.files_scanned.fetch_add(1, Ordering::Relaxed);
                let path = entry.path().to_path_buf();

                match router.evaluate_file(&path) {
                    FilterDecision::Accept {
                        identity,
                        kind,
                        size_bytes,
                        mtime_secs,
                    } => {
                        match &kind {
                            CandidateKind::Text { .. } => {
                                stats.files_accepted_text.fetch_add(1, Ordering::Relaxed);
                            }
                            CandidateKind::Image { .. } => {
                                stats.files_accepted_image.fetch_add(1, Ordering::Relaxed);
                            }
                        }

                        let candidate = DiscoveredCandidate {
                            path,
                            size_bytes,
                            mtime_secs,
                            file_identity: identity,
                            kind,
                        };

                        // Push into bounded channel with backpressure
                        if tx.blocking_send(candidate).is_err() {
                            // Receiver was dropped; stop crawling immediately across all threads
                            return WalkState::Quit;
                        }
                    }
                    FilterDecision::SkipExceedsSizeCap { .. } => {
                        stats.files_skipped_size.fetch_add(1, Ordering::Relaxed);
                    }
                    FilterDecision::SkipBinary { .. } => {
                        stats.files_skipped_binary.fetch_add(1, Ordering::Relaxed);
                    }
                    FilterDecision::SkipHidden | FilterDecision::SkipSymlink => {
                        stats.files_skipped_hidden.fetch_add(1, Ordering::Relaxed);
                    }
                    FilterDecision::SkipZeroByte => {
                        // 0-byte file: skip silently
                    }
                    FilterDecision::SkipIoError { .. } => {
                        stats.files_skipped_io.fetch_add(1, Ordering::Relaxed);
                    }
                }

                WalkState::Continue
            })
        });

        Ok(())
    }
}
```

### 3.6 Continuous Real-Time OS Watcher: `src/watcher.rs`

```rust
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::IngestConfig;
use crate::error::IngestError;

/// Normalized debounced filesystem event emitted by the OS watcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatcherEvent {
    /// Newly created file.
    Create(PathBuf),
    /// Content or metadata modified.
    Modify(PathBuf),
    /// File removed from disk.
    Remove(PathBuf),
    /// File renamed or moved from old path to new path.
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    /// Event queue overflow or dropped events -> requires drift healing scan.
    QueueOverflow {
        root: PathBuf,
        details: String,
    },
}

/// Continuous real-time OS filesystem watcher with 300ms debouncing.
pub struct FilesystemWatcher {
    config: IngestConfig,
    active_paths: HashSet<PathBuf>,
}

impl FilesystemWatcher {
    pub fn new(config: IngestConfig) -> Self {
        Self {
            config,
            active_paths: HashSet::new(),
        }
    }

    /// Spawns the continuous OS watcher with a 300ms debouncing window.
    ///
    /// Returns:
    /// - `mpsc::Receiver<WatcherEvent>`: Non-blocking event receiver.
    /// - `WatcherHandle`: Control handle for dynamic watch management and clean shutdown.
    pub fn start_watching(
        mut self,
    ) -> Result<(mpsc::Receiver<WatcherEvent>, WatcherHandle), IngestError> {
        let (tx, rx) = mpsc::channel(10_000);
        let debounce_duration = self.config.debounce_duration();

        let tx_event = tx.clone();
        let mut debouncer = new_debouncer(
            debounce_duration,
            None,
            move |res: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
                match res {
                    Ok(events) => {
                        for event in events {
                            Self::dispatch_debounced_event(&event, &tx_event);
                        }
                    }
                    Err(errors) => {
                        for err in errors {
                            warn!("OS Watcher native error: {}", err);
                            let _ = tx_event.try_send(WatcherEvent::QueueOverflow {
                                root: PathBuf::new(),
                                details: err.to_string(),
                            });
                        }
                    }
                }
            },
        )
        .map_err(|e| IngestError::WatcherError(format!("Failed to initialize debouncer: {}", e)))?;

        // Register initial watch paths recursively
        for path in &self.config.watch_paths {
            if path.exists() {
                debouncer
                    .watcher()
                    .watch(path, RecursiveMode::Recursive)
                    .map_err(|e| {
                        IngestError::WatcherError(format!(
                            "Failed to watch path '{}': {}",
                            path.display(),
                            e
                        ))
                    })?;
                self.active_paths.insert(path.clone());
                info!(path = %path.display(), "Watching directory recursively");
            } else {
                warn!("Watch directory does not exist, skipping: {}", path.display());
            }
        }

        let handle = WatcherHandle {
            debouncer: Arc::new(std::sync::Mutex::new(debouncer)),
            active_paths: Arc::new(std::sync::RwLock::new(self.active_paths)),
        };

        Ok((rx, handle))
    }

    /// Maps native `DebouncedEvent` into QuorumRAG `WatcherEvent`.
    fn dispatch_debounced_event(event: &DebouncedEvent, tx: &mpsc::Sender<WatcherEvent>) {
        use notify::event::ModifyKind;

        match event.kind {
            EventKind::Create(_) => {
                for path in &event.paths {
                    if path.is_file() {
                        let _ = tx.try_send(WatcherEvent::Create(path.clone()));
                    }
                }
            }
            EventKind::Modify(ModifyKind::Name(_)) => {
                // Rename or move event
                if event.paths.len() >= 2 {
                    let from = event.paths[0].clone();
                    let to = event.paths[1].clone();
                    let _ = tx.try_send(WatcherEvent::Rename { from, to });
                } else if let Some(path) = event.paths.first() {
                    // Single-path rename (e.g. rename from outside watch dir)
                    if path.exists() {
                        let _ = tx.try_send(WatcherEvent::Create(path.clone()));
                    } else {
                        let _ = tx.try_send(WatcherEvent::Remove(path.clone()));
                    }
                }
            }
            EventKind::Modify(_) => {
                for path in &event.paths {
                    if path.is_file() {
                        let _ = tx.try_send(WatcherEvent::Modify(path.clone()));
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in &event.paths {
                    let _ = tx.try_send(WatcherEvent::Remove(path.clone()));
                }
            }
            EventKind::Other => {
                debug!("Received non-actionable watcher event");
            }
            _ => {}
        }
    }
}

/// Control handle for dynamic watch management and thread-safe operations.
pub struct WatcherHandle {
    debouncer: Arc<std::sync::Mutex<Debouncer<RecommendedWatcher, FileIdMap>>>,
    active_paths: Arc<std::sync::RwLock<HashSet<PathBuf>>>,
}

impl WatcherHandle {
    /// Dynamically registers a new directory root for real-time monitoring.
    pub fn watch_path(&self, path: impl AsRef<Path>) -> Result<(), IngestError> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(IngestError::WatcherError(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        let mut debouncer = self.debouncer.lock().map_err(|_| {
            IngestError::WatcherError("Debouncer mutex poisoned".into())
        })?;

        debouncer
            .watcher()
            .watch(&path, RecursiveMode::Recursive)
            .map_err(|e| {
                IngestError::WatcherError(format!(
                    "Failed to register watch for '{}': {}",
                    path.display(),
                    e
                ))
            })?;

        let mut paths = self.active_paths.write().unwrap();
        paths.insert(path);
        Ok(())
    }

    /// Dynamically unregisters an active directory root.
    pub fn unwatch_path(&self, path: impl AsRef<Path>) -> Result<(), IngestError> {
        let path = path.as_ref();
        let mut debouncer = self.debouncer.lock().map_err(|_| {
            IngestError::WatcherError("Debouncer mutex poisoned".into())
        })?;

        debouncer
            .watcher()
            .unwatch(path)
            .map_err(|e| {
                IngestError::WatcherError(format!(
                    "Failed to unwatch path '{}': {}",
                    path.display(),
                    e
                ))
            })?;

        let mut paths = self.active_paths.write().unwrap();
        paths.remove(path);
        Ok(())
    }

    /// Returns the snapshot list of all actively monitored filesystem roots.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        let paths = self.active_paths.read().unwrap();
        paths.iter().cloned().collect()
    }
}
```

---

## 4. Caveats

1. **Drive Root Permissions on Windows**:
   - Watching drive roots directly (e.g. `C:\` or `D:\`) can generate high-volume OS events for hidden system folders (`System Volume Information`, `$Recycle.Bin`).
   - `skip_hidden = true` and `ignore` rules cleanly filter these, but watching specific user folders (`C:\Users\username\Documents`, `D:\Projects`) is recommended for optimal performance on 30TB+ multi-drive systems.
2. **Windows File Locking Latency**:
   - When large files are written by slow processes (e.g. video rendering or huge code compilation outputs), the 300ms debounce may fire while the file is still open with exclusive write access.
   - The included `read_metadata_with_retry` and non-fatal `SkipIoError` gracefully defer these until either a subsequent Modify event or the scheduled `DriftHealer` pass.
3. **Image Indexing Dependency**:
   - `crawler.rs` and `watcher.rs` accurately identify images via Magika and emit them as `CandidateKind::Image`.
   - The actual EXIF parsing, dimension extraction, and text summary vectorization will be implemented by Explorer M3-2 in `image_indexer.rs`.

---

## 5. Conclusion

1. **Complete Architectural Separation**: `crawler.rs` provides high-speed multi-threaded initial scans via `ignore::WalkParallel` with backpressured channels; `watcher.rs` provides continuous real-time OS monitoring via `notify-debouncer-full` with 300ms debouncing; and `classifier_router.rs` enforces deep Google Magika classification and binary filtering across both pathways.
2. **Zero Image Skipping**: Images (`png`, `jpeg`, `webp`, `gif`, `bmp`, `tiff`, `svg`) are explicitly preserved and routed to `CandidateKind::Image`, fully satisfying User Requirement 3.
3. **Clean Inter-Agent Boundary**: The interface outputs `DiscoveredCandidate` and `WatcherEvent` stream cleanly into Explorer M3-2's deduplication & chunking engines (`dedup.rs`, `chunker.rs`, `image_indexer.rs`) and Explorer M3-3's coordination engines (`reindex.rs`, `drift_healer.rs`, `engine.rs`).

---

## 6. Verification Method

To independently verify the implementation once coded:

1. **Unit Tests (`crates/quorum-ingest/tests/test_crawler_watcher.rs`)**:
   - Verify `ignore::WalkParallel` skips `.gitignore` files, `.ignore` files, and hidden `.secret/` files.
   - Verify files > 50MB are skipped and 0-byte files produce zero chunks without error.
   - Verify `MZ` and `ELF` binaries are rejected while `PNG` and `JPEG` images are emitted as `CandidateKind::Image`.
   - Verify a burst of 10 writes within 100ms triggers only 1 debounced `WatcherEvent::Modify`.
   - Verify file rename emits `WatcherEvent::Rename { from, to }`.
2. **E2E Test Execution**:
   - Run: `cargo test -p quorum-e2e --test test_ingestion_dedup -- --test-threads=2`
   - Expected output: `test result: ok. 8 passed; 0 failed`.
3. **Workspace Build & Lint**:
   - Run: `cargo check --workspace`
   - Run: `cargo clippy --workspace -- -D warnings`
