# Exploration Report: Milestone M7 — Feature 44 (Deduplication Startup Hydration)

## 1. Observation

Direct observations from source inspection across `crates/quorum-ingest`, `crates/quorum-storage`, `crates/quorum-daemon`, and `crates/quorum-types`:

### 1.1 `TwoLevelDedupEngine` and Cold Cache Startup
- **Cold Instantiation** (`crates/quorum-ingest/src/dedup.rs:327-333`):
  ```rust
  impl TwoLevelDedupEngine {
      pub fn new() -> Self {
          Self {
              catalog: Arc::new(RwLock::new(Level1DedupCatalog::new())),
              cache: Arc::new(Level2ChunkCache::new(50_000)),
          }
      }
  ```
  `TwoLevelDedupEngine::new()` initializes both `Level1DedupCatalog` (`entries` and `path_to_hash` maps) and `Level2ChunkCache` (`map` and `order` queues) completely empty with 0 elements.

- **Daemon Startup Wiring** (`crates/quorum-daemon/src/main.rs:69-75`):
  ```rust
  let storage_config = quorum_storage::StorageConfig::new(cli.data_dir.clone());

  let storage = Arc::new(quorum_storage::StorageEngine::open(storage_config).await?);
  let classifier = Arc::new(quorum_classifier::FileTypeClassifier::new());
  let embedder: Arc<dyn quorum_contract::EmbeddingGenerator> = Arc::new(
      quorum_contract::DeterministicOfflineEngine::new()
  );
  let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());
  ```
  In `main.rs`, `dedup` is initialized immediately after `storage`, but no hydration call is made.

- **Continuous Background Crawl Timing** (`crates/quorum-daemon/src/main.rs:196-203`):
  ```rust
  // Continuously run background Magika crawl & incremental ingestion across all watched physical drives & shared folders
  if !ingest_config.watch_paths.is_empty() {
      let bg_worker = Arc::clone(&reindex_worker);
      let bg_paths = ingest_config.watch_paths.clone();
      let bg_shutdown = Arc::clone(&shutdown_notify);
      tokio::spawn(async move {
          tokio::time::sleep(std::time::Duration::from_millis(250)).await;
          loop { ... }
  ```
  Within 250 milliseconds of daemon launch, `bg_worker.start_reindex(...)` begins scanning the filesystem. Because `dedup` is empty, every file on disk misses the Level-1 cache and forces re-chunking and neural re-embeddings.

- **CLI Workers Also Instantiate Cold Dedup** (`crates/quorum-daemon/src/main.rs:362, 429, 534`):
  - Standalone reindex (`main.rs:362`): `let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());`
  - Standalone drift healing (`main.rs:429`): `let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());`
  - Standalone recache (`main.rs:534`): `let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());`

### 1.2 Storage Chunk Extraction Capability
- **Storage Scan API** (`crates/quorum-storage/src/engine.rs:434-437`):
  ```rust
  /// Returns all chunks stored in LanceDB.
  pub async fn scan_all_chunks(&self) -> Result<Vec<DocumentChunk>, StorageError> {
      self.vector_store.scan_all_chunks().await
  }
  ```
  `StorageEngine::scan_all_chunks(&self)` delegates directly to LanceDB's `VectorStore::scan_all_chunks(&self)`.

- **Domain Model Available Fields** (`crates/quorum-types/src/lib.rs:137-168`):
  ```rust
  pub struct DocumentChunk {
      pub chunk_id: String,       // e.g. "{doc_id}:{chunk_index}"
      pub document_id: String,    // Document-level BLAKE3 content digest
      pub file_path: String,      // Canonical or relative file path on disk
      pub line_start: u32,        // 1-indexed start line
      pub line_end: u32,          // 1-indexed end line
      pub blake3_hash: String,    // Hex-encoded BLAKE3 digest of chunk text
      pub access_tier: u8,        // Access tier
      pub vector: Vec<f32>,       // 768-dimensional Gemma dense embedding
      pub text_content: String,   // Plain text slice
      pub metadata_json: String,  // Metadata JSON
  }
  ```
  Every record retrieved by `scan_all_chunks()` already contains the document content hash (`document_id`), the chunk content hash (`blake3_hash`), the source file path (`file_path`), the chunk position (`line_start`), and the dense embedding vector (`vector`).

### 1.3 Two-Level Evaluation Logic
- **Level-1 Evaluation Algorithm** (`crates/quorum-ingest/src/dedup.rs:117-176`):
  - **Step 1 (Fast Path)**: Checks `self.path_to_hash.get(path)`. If found and `entry.mtime_secs == current_mtime && entry.size_bytes == current_size`, returns `DedupAction::Unchanged` with **zero disk reads, zero hashing, zero chunking, and zero embeddings**.
  - **Step 2 (Content Hash & Rename/Move Check)**: Calls `Level1DedupCatalog::hash_file_stream(path)`. If the hash exists under the same path with updated timestamp, it updates `mtime` and `size` and returns `DedupAction::Unchanged`. If the hash exists under a different path, it updates the secondary index and returns `DedupAction::MovedOrRenamed`.
  - **Step 3 (New or Modified)**: Returns `DedupAction::ModifiedOrNew`.
- **Level-2 Chunk Cache** (`crates/quorum-ingest/src/dedup.rs:268-300`):
  - Maps `chunk_blake3 -> (Vec<f32>, usize)`.
  - In `engine.rs:158-167` and `reindex.rs:373-388`, every candidate chunk is checked against `self.dedup.cache().get(&chunk_b3)`. If found, the existing embedding vector is reused, completely bypassing neural embedding calls (`embedder.embed_text`).

---

## 2. Logic Chain

1. **Root Cause of High Startup Overhead**:
   - Because `TwoLevelDedupEngine` is instantiated with empty hash maps on daemon startup (Observation 1.1), `dedup.evaluate_file` fails on all existing files on disk.
   - Even though the underlying chunks and embeddings are safely stored in LanceDB, the crawler cannot know this without querying LanceDB or re-embedding each chunk.
   - Consequently, the crawler executes expensive neural embeddings across all documents whenever the daemon starts or restarts, causing excessive CPU, disk I/O, and memory contention (violating Requirement R1 and Performance Acceptance Criteria).

2. **Feasibility of Direct Startup Hydration**:
   - LanceDB already holds the complete corpus in `VectorStore` accessible via `storage.scan_all_chunks().await?` (Observation 1.2).
   - Each `DocumentChunk` contains:
     - `blake3_hash` and `vector` -> exactly what `Level2ChunkCache` needs to avoid re-embeddings.
     - `document_id` (BLAKE3 hash of parent file), `file_path`, and `line_start` -> exactly what `Level1DedupCatalog` needs to reconstruct `FileCatalogEntry`.

3. **Reconstructing `Level1DedupCatalog` from Chunks**:
   - Chunks can be grouped by `document_id` using a `HashMap<String, (PathBuf, Vec<(u32, String)>)>`.
   - Chunks within each group are sorted by `(line_start, chunk_index)` to preserve deterministic sequential order in `FileCatalogEntry::chunk_hashes`.
   - For `mtime_secs` and `size_bytes`:
     - **Opportunistic Metadata Probe**: If the file exists on the local disk at `file_path`, a lightweight `std::fs::metadata(&file_path)` probe retrieves `(meta.mtime, meta.len())`. This enables the crawler to hit the Level-1 Fast Path (Observation 1.3 Step 1) on the very first crawl cycle, resulting in **zero file reads and zero hashing**.
     - **Safe Fallback**: If the file cannot be accessed (e.g. offline drive, unmounted volume, deleted file, permission restriction), fallback to `(0, 0)` ensures the entry is still recorded. If the file is subsequently hashed, it hits Step 2 of `evaluate_file` and updates the catalog without re-chunking or re-embedding.

4. **Reconstructing `Level2ChunkCache`**:
   - Adding `ensure_capacity(chunk_count)` prevents premature LRU eviction if stored chunk count exceeds the default 50,000 threshold.
   - Adding `insert_batch` or inserting all `(chunk.blake3_hash, chunk.vector)` pairs primes the Level-2 cache. If any chunk is evaluated during reindex or drift healing, `cache.get()` returns a cache hit immediately.

5. **Insertion Point in `quorum-daemon`**:
   - Lines 74-75 in `crates/quorum-daemon/src/main.rs` is the exact synchronization boundary where `storage` has been opened and `dedup` has been initialized, but before `reindex_worker`, `drift_healer`, HTTP router, and the 250ms background indexer loop are started.
   - Calling `dedup.hydrate_from_storage(&storage).await?` guarantees that before any background task or client request executes, the in-memory deduplication state is 100% synchronized with on-disk storage.

---

## 3. Implementation Specifications

### 3.1 Enhancements to `crates/quorum-ingest/src/dedup.rs`

#### 3.1.1 Additions to `Level2ChunkCache`
Add capacity reservation and batch insertion methods:

```rust
impl Level2ChunkCache {
    /// Expands the cache capacity if `min_capacity` exceeds the current capacity.
    pub fn ensure_capacity(&self, min_capacity: usize) {
        if let Ok(mut inner) = self.inner.write() {
            if min_capacity > inner.capacity {
                inner.capacity = min_capacity;
            }
        }
    }

    /// Inserts a batch of chunk hash and embedding vector pairs under a single write lock.
    pub fn insert_batch(&self, items: impl IntoIterator<Item = (String, Vec<f32>)>) {
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        for (chunk_blake3, vector) in items {
            if inner.map.contains_key(&chunk_blake3) {
                continue;
            }

            // Evict if at capacity
            while inner.map.len() >= inner.capacity && !inner.order.is_empty() {
                if let Some(oldest) = inner.order.pop_front() {
                    inner.map.remove(&oldest);
                }
            }

            let order_len = inner.order.len();
            inner.order.push_back(chunk_blake3.clone());
            inner.map.insert(chunk_blake3, (vector, order_len));
        }
    }
}
```

#### 3.1.2 Implementation of `hydrate_from_storage` on `TwoLevelDedupEngine`
Add the following method to `impl TwoLevelDedupEngine` in `crates/quorum-ingest/src/dedup.rs`:

```rust
impl TwoLevelDedupEngine {
    /// Hydrates deduplication state (both Level-1 File Catalog and Level-2 Chunk Cache)
    /// directly from stored LanceDB chunks on startup.
    ///
    /// # Behavior
    /// 1. Scans all stored `DocumentChunk` records from LanceDB via `storage.scan_all_chunks().await?`.
    /// 2. Expands `Level2ChunkCache` capacity to accommodate all stored chunks without eviction.
    /// 3. Populates `Level2ChunkCache` with `(blake3_hash, vector)` under a single batch operation.
    /// 4. Groups chunks by parent `document_id` (the BLAKE3 digest of the document).
    /// 5. Sorts chunk hashes for each document deterministically by `(line_start, chunk_index)`.
    /// 6. Probes local filesystem metadata for each unique file to prime `mtime_secs` and `size_bytes`
    ///    for immediate Level-1 Fast Path hits, safely falling back to `(0, 0)` for offline drives.
    /// 7. Populates `Level1DedupCatalog` under a single write lock.
    ///
    /// Returns the total number of hydrated chunks.
    pub async fn hydrate_from_storage(
        &self,
        storage: &Arc<quorum_storage::StorageEngine>,
    ) -> Result<usize, crate::error::IngestError> {
        let chunks = storage.scan_all_chunks().await?;
        if chunks.is_empty() {
            return Ok(0);
        }

        let chunk_count = chunks.len();

        // Step 1: Ensure Level-2 cache capacity and batch-insert chunk vectors
        self.cache.ensure_capacity(chunk_count);
        let cache_items = chunks
            .iter()
            .filter(|c| !c.vector.is_empty())
            .map(|c| (c.blake3_hash.clone(), c.vector.clone()));
        self.cache.insert_batch(cache_items);

        // Step 2: Group chunks by document_id:
        // document_id -> (file_path, Vec<(line_start, chunk_index, blake3_hash)>)
        let mut doc_groups: HashMap<String, (PathBuf, Vec<(u32, usize, String)>)> = HashMap::new();

        for chunk in chunks {
            // Parse chunk index from chunk_id (e.g. "{doc_id}:{chunk_index}")
            let chunk_index: usize = chunk
                .chunk_id
                .rsplit_once(':')
                .and_then(|(_, idx)| idx.parse().ok())
                .unwrap_or(0);

            let group = doc_groups
                .entry(chunk.document_id)
                .or_insert_with(|| (PathBuf::from(&chunk.file_path), Vec::new()));
            group.1.push((chunk.line_start, chunk_index, chunk.blake3_hash));
        }

        // Step 3: Populate Level-1 File Catalog under single lock
        let mut catalog = self.catalog.write().map_err(|_| {
            std::io::Error::other("Level 1 dedup catalog lock poisoned")
        })?;

        for (doc_id, (file_path, mut chunk_list)) in doc_groups {
            // Sort chunks by line_start and chunk_index to maintain deterministic order
            chunk_list.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            let chunk_hashes: Vec<String> = chunk_list.into_iter().map(|(_, _, hash)| hash).collect();

            // Opportunistically probe disk metadata for fast-path evaluation; fallback to (0, 0)
            let (mtime_secs, size_bytes) = match std::fs::metadata(&file_path) {
                Ok(meta) => {
                    let mtime = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    (mtime, meta.len())
                }
                Err(_) => (0, 0),
            };

            catalog.register_entry(FileCatalogEntry {
                file_path,
                file_blake3: doc_id,
                mtime_secs,
                size_bytes,
                chunk_hashes,
            });
        }

        Ok(chunk_count)
    }
}
```

### 3.2 Modifications to `crates/quorum-daemon/src/main.rs`

#### 3.2.1 Daemon Startup Hook (`main.rs:74-75`)
Insert deduplication hydration immediately after `dedup` instantiation:

```rust
    // Existing lines 69-74:
    let storage = Arc::new(quorum_storage::StorageEngine::open(storage_config).await?);
    let classifier = Arc::new(quorum_classifier::FileTypeClassifier::new());
    let embedder: Arc<dyn quorum_contract::EmbeddingGenerator> = Arc::new(
        quorum_contract::DeterministicOfflineEngine::new()
    );
    let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());

    // NEW Feature 44: Hydrate deduplication engine from LanceDB storage
    let hydrated_chunks = dedup.hydrate_from_storage(&storage).await?;
    info!(
        chunks = hydrated_chunks,
        tracked_files = dedup.catalog().read().map(|c| c.len()).unwrap_or(0),
        "Startup deduplication hydration completed from LanceDB storage"
    );

    // Existing lines 76-95:
    // 2. Ingest Config & Workers...
```

#### 3.2.2 Optional CLI Worker Wiring (`main.rs:362` and `main.rs:429`)
In `run_standalone_reindex` and `run_standalone_fix_drift`, call `dedup.hydrate_from_storage(&storage).await?;` right after `let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());` so standalone CLI operations also benefit from zero-overhead dedup cache priming.

---

## 4. Caveats

1. **LanceDB Vector Dimension Consistency**: All chunks stored in LanceDB have a fixed vector dimension (e.g. 768 for Gemma). Chunks with empty vectors are filtered out before insertion into `Level2ChunkCache`.
2. **Offline Drive / Disconnected Media**: When a physical drive or external volume was previously indexed but is offline at daemon startup, `std::fs::metadata` fails cleanly and records `mtime_secs = 0, size_bytes = 0`. The Level-2 chunk embeddings and document BLAKE3 mapping remain fully intact in memory. When the volume is remounted, `evaluate_file` recognizes the content hash and immediately detects rename/move without re-embedding.
3. **Chunk Ordering on Multi-Segment Documents**: Because LanceDB queries do not guarantee insertion row order, sorting chunks by `(line_start, chunk_index)` ensures that the reconstructed `FileCatalogEntry::chunk_hashes` matches the original document layout.

---

## 5. Conclusion

- Feature 44 directly resolves the cold-start re-embedding bottleneck by hydrating `TwoLevelDedupEngine` (both Level-1 catalog and Level-2 chunk cache) directly from LanceDB storage chunks via `storage.scan_all_chunks().await?`.
- Reconstructing `Level1DedupCatalog` with opportunistic `metadata` probing allows unchanged files to immediately hit the Level-1 Fast Path with **zero file reads, zero hashing, and zero embeddings**.
- Wiring `dedup.hydrate_from_storage(&storage).await?` at `crates/quorum-daemon/src/main.rs:75` primes the engine before background indexing starts at 250ms, fulfilling Requirement R1 and the Startup Deduplication Acceptance Criteria.

---

## 6. Verification Method

### 6.1 Unit & Integration Test Strategy
Add dedicated tests in `crates/quorum-ingest/tests/ingest_tests.rs`:

1. **`test_dedup_startup_hydration_from_storage`**:
   - Create a temporary storage directory via `tempfile::tempdir()`.
   - Initialize `StorageEngine::open(StorageConfig::new(dir.path()))`.
   - Insert 4 synthetic `DocumentChunk` records across 2 distinct documents (`doc_a` with 3 chunks, `doc_b` with 1 chunk).
   - Instantiate a fresh `TwoLevelDedupEngine::new()`.
   - Assert `dedup.catalog().read().unwrap().len() == 0`.
   - Call `dedup.hydrate_from_storage(&storage).await.unwrap()`.
   - Assert returned count equals 4.
   - Assert `dedup.catalog().read().unwrap().len() == 2`.
   - Assert `dedup.cache().has_chunk(&chunk_hash) == true` for all 4 chunk hashes.
   - Assert `dedup.cache().get(&chunk_hash)` returns the exact vector.
   - Assert document `chunk_hashes` vector has length 3 for `doc_a` and length 1 for `doc_b` in sequential order.

2. **`test_dedup_startup_hydration_empty_storage`**:
   - Initialize an empty `StorageEngine`.
   - Instantiate `TwoLevelDedupEngine::new()`.
   - Call `dedup.hydrate_from_storage(&storage).await.unwrap()`.
   - Assert returned count is 0, catalog is empty, and cache is empty without errors.

3. **`test_dedup_startup_hydration_offline_drive_fallback`**:
   - Insert a chunk with a non-existent drive path `Z:\nonexistent\doc.txt`.
   - Hydrate from storage.
   - Verify `entry.mtime_secs == 0` and `entry.size_bytes == 0`.
   - Verify chunk vector is present in `Level2ChunkCache`.

### 6.2 Execution Command
```powershell
cargo test -p quorum-ingest --test ingest_tests test_dedup_startup_hydration -- --nocapture
```

### 6.3 Invalidation Conditions
This design would be invalidated if:
1. `StorageEngine::scan_all_chunks()` failed to return `document_id` or `vector` (refuted by `crates/quorum-storage/src/vector_store.rs:206-221` and `crates/quorum-types/src/lib.rs:137-168`).
2. `TwoLevelDedupEngine` required external state beyond `document_id`, `file_path`, and `chunk_hashes` to evaluate files (refuted by `crates/quorum-ingest/src/dedup.rs:117-176`).
