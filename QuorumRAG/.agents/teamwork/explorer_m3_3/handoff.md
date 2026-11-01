# Handoff Report: Dedicated Reindexing & Comprehensive 5-Drift Healing Engine (Milestone 3)

**Author**: Explorer M3-3 (`explorer_m3_3`)  
**Date**: 2026-10-31T20:20:00Z  
**Role**: Teamwork Explorer (Read-Only Investigation & Technical Architecture)  
**Assigned Milestone**: Milestone 3 — Filesystem Ingestion, OS Watcher, Image & Reindex Engine  
**Target Crate**: `crates/quorum-ingest` (`src/reindex.rs`, `src/drift_healer.rs`, `src/engine.rs`, `src/lib.rs`)  
**Status**: COMPLETE (Hard Handoff)

---

## 1. Observation

### 1.1 Direct Requirements & Mandates
1. **User Requirement & Directive** (`ORIGINAL_REQUEST.md:90-106`):
   > "a process myst exit to reindex and another one to fix drifts all types of drifts"
   - Dedicated Reindexing Process (`reindex`): Traverses target directories, supports both incremental (`mtime` + Level 1 BLAKE3) and full rebuild (`--full`) modes. Accessible via CLI (`quorumrag reindex [--path <dir>] [--full]`), REST API (`POST /api/reindex`), and Web UI button.
   - Dedicated Drift Remediation Process (`fix-drift`): Detects and repairs **ALL 5 types of drifts**:
     (a) Filesystem <-> Database Merkle drift (deleted files purged, modified files updated)  
     (b) LanceDB <-> Tantivy desync (missing chunks cross-checked and healed to 100% parity)  
     (c) Vector contract drift (detects chunks with obsolete contract hash/dimension and re-embeds)  
     (d) ACL policy drift (detects chunks whose stored `access_tier` metadata no longer matches directory policy)  
     (e) Storage fragmentation & tombstone drift (triggers LanceDB fragment compaction and Tantivy segment merging)
2. **Unified Ingestion Engine** (`PROJECT.md:25-29, 80`):
   - `crates/quorum-ingest` must integrate crawler (`ignore::WalkParallel`), real-time OS watcher (`notify`), content classifier (`quorum-classifier`), two-level BLAKE3 deduplication, image extractor, reindexing worker, and drift healer into a unified `IngestEngine`.
3. **Arrow & LanceDB Isolation Invariant** (`PROJECT.md:30, 41`, `crates/quorum-storage/src/lib.rs:6-9`):
   - Zero Apache Arrow or LanceDB types may be exposed outside `quorum-storage`. Reindexing and drift healing must interact with `StorageEngine` solely via pure domain types (`DocumentChunk`, `ScoredChunk`, `AccessTier`) or dedicated storage DTOs.
4. **Offline Gemma Embedding Contract** (`crates/quorum-contract/src/lib.rs:49-118`):
   - Canonical 768-dim L2-normalized vector space (`google/embeddinggemma-300m`). Deterministic 32-byte BLAKE3 contract hash derived by `EmbeddingContract::derive_contract_hash()`. Chunks with mismatched contract hash or dimension must be flagged and re-embedded during drift remediation.
5. **Existing Storage Engine Primitives** (`crates/quorum-storage/src/engine.rs:92-304`):
   - `StorageEngine` exposes `insert_chunks(&[DocumentChunk])`, `search_vector`, `search_text`, `search_hybrid`, `has_chunk_hash(&str)`, `delete_by_file_path(&str)`, `compact() -> Result<CompactionStats>`, and `count()`.
   - `compact()` already invokes `vector_store.optimize()` (`OptimizeAction::All`) and `fts_index.compact()`.
6. **E2E Deduplication Invariants** (`tests/e2e/test_ingestion_dedup.rs:68-135, 178-236`):
   - Level-1 File BLAKE3 Catalog: If full file BLAKE3 hash matches, detect rename/move or unmodified file and execute **0 new chunk embeddings**.
   - Level-2 Chunk BLAKE3 Cache: If individual chunk text BLAKE3 hash matches, reuse existing cached embedding vector (0 neural model evaluations).

---

## 2. Logic Chain

```
                     ┌────────────────────────────────────────┐
                     │          QuorumRAG Daemon / CLI        │
                     │  - CLI: quorumrag reindex / fix-drift  │
                     │  - Web UI / REST: /api/reindex, /drift │
                     └───────────────────┬────────────────────┘
                                         │
                                         ▼
                 ┌─────────────────────────────────────────────────┐
                 │       Unified IngestEngine (engine.rs)          │
                 │   - Coordinates lifecycle, channels, state      │
                 └───────┬───────────────┬─────────────────┬───────┘
                         │               │                 │
                         ▼               ▼                 ▼
          ┌─────────────────────┐ ┌─────────────┐ ┌────────────────────┐
          │  OS Watcher/Crawler │ │ Reindex     │ │ Drift Healer       │
          │  - notify (300ms)   │ │ Worker      │ │ (drift_healer.rs)  │
          │  - ignore walker    │ │(reindex.rs) │ │ - Merkle FS <-> DB │
          └──────────┬──────────┘ └──────┬──────┘ │ - Lance <-> Tantivy│
                     │                   │        │ - Vector Contract  │
                     ▼                   ▼        │ - ACL Policy       │
          ┌─────────────────────────────────────┐ │ - Fragment Compact │
          │ Two-Level BLAKE3 Dedup (dedup.rs)   │ └─────────┬──────────┘
          │ - Level 1: File catalog (mtime, B3) │           │
          │ - Level 2: Chunk vector cache (B3)  │           │
          └──────────────────┬──────────────────┘           │
                             ▼                              │
          ┌─────────────────────────────────────┐           │
          │ Content Classifier & Extractors     │           │
          │ - Magika inspection -> Text/Image   │           │
          └──────────────────┬──────────────────┘           │
                             │                              │
                             ▼                              │
          ┌─────────────────────────────────────────────────▼──┐
          │          quorum-storage (StorageEngine)            │
          │  - LanceDB (Vector ANN + SQL pre-filtering)        │
          │  - Tantivy (BM25 FTS inverted index)               │
          │  - Compaction & Optimization                       │
          └────────────────────────────────────────────────────┘
```

### 2.1 Reindexing Architecture (`reindex.rs`)
1. **Separation of Concerns**: The continuous OS watcher (`notify`) handles real-time delta events during normal execution. However, an explicit, user-triggerable, and background-rebuilding **Reindex Worker** is strictly necessary for:
   - Initial cold-start crawls across massive multi-terabyte drives.
   - Forcing complete database rebuilds (`--full`) when disk storage was corrupted, upgraded, or restored from backup.
   - User-initiated re-scans via the Web UI or CLI when external mountpoints or network shares are attached.
2. **Dual-Mode Execution**:
   - **Incremental Mode (default)**:
     - Leverages filesystem metadata (`mtime` + `size`) as a zero-I/O filter.
     - Files with identical `(mtime, size)` recorded in the Level-1 catalog are skipped instantly without reading file contents.
     - If `(mtime, size)` differs: reads file and computes Level-1 BLAKE3 hash.
     - If Level-1 BLAKE3 matches catalog: file was touched or renamed. Updates stored path and mtime, requiring **0 chunk re-embeddings**.
     - If Level-1 BLAKE3 differs: routes to Google Magika -> text chunker or image indexer -> Level-2 chunk cache -> storage. Old chunks for that `file_path` are replaced atomically.
   - **Full Rebuild Mode (`--full`)**:
     - Bypasses `mtime` and Level-1 catalog checks.
     - Recursively verifies every file, purges records for deleted files, recomputes chunks and embeddings, rebuilds the Tantivy BM25 directory, and runs LanceDB fragment compaction + IVF_PQ re-indexing.
3. **Execution Contexts**:
   - Must run in 3 modalities:
     - CLI command: synchronous terminal output with ASCII progress bar and summary table.
     - In-daemon background task: runs asynchronously via Tokio, communicates status via `tokio::sync::watch` channel, supports cooperative cancellation via `tokio_util::sync::CancellationToken`.
     - REST API (`POST /api/reindex`): returns `202 Accepted` with a task UUID, enabling the Web UI to poll or stream real-time SSE progress updates.

### 2.2 Comprehensive 5-Drift Remediation Architecture (`drift_healer.rs`)
A production peer-to-peer RAG daemon operating over local filesystems experiences entropy across five distinct physical and logical planes. Each plane requires a dedicated mathematical detection algorithm and atomic remediation strategy:

1. **Drift Type A — Filesystem <-> Database Merkle Drift**:
   - *Problem*: Files created, modified, or deleted while the daemon was offline or when OS event buffers overflowed during high disk I/O.
   - *Detection*: Bi-directional reconciliation:
     - DB -> FS scan: For every indexed `file_path`, check if file still exists on disk. If not, mark as `DeletedOrphan`. If exists, check if disk `(mtime, size, BLAKE3)` differs from DB. If differs, mark as `ModifiedOutdated`.
     - FS -> DB scan: Using `ignore::WalkBuilder`, check disk files against the DB catalog. If absent in DB, mark as `UntrackedMissing`.
   - *Healing*: Purge `DeletedOrphan` chunks from LanceDB and Tantivy; re-index `ModifiedOutdated` and `UntrackedMissing` files through the ingestion pipeline.

2. **Drift Type B — Hybrid Index Desynchronization (LanceDB Vector <-> Tantivy BM25 Desync)**:
   - *Problem*: LanceDB and Tantivy write independently. A system crash, process kill, or disk write error during micro-batching can leave chunks present in LanceDB but missing in Tantivy, or vice-versa. This corrupts Hybrid Search ($RRF$) calculations.
   - *Detection*: Compare the chunk ID set $S_{lance}$ against $S_{tantivy}$.
     - Lance-only chunks: $D_{lance} = S_{lance} \setminus S_{tantivy}$.
     - Tantivy-only chunks: $D_{tantivy} = S_{tantivy} \setminus S_{lance}$.
   - *Healing*:
     - For $D_{lance}$: retrieve chunks from LanceDB and insert them into Tantivy via `fts_index.index_chunks()`, then reload the searcher.
     - For $D_{tantivy}$: retrieve chunks from Tantivy (which stores `vector_bytes` and chunk metadata) and insert them into LanceDB via `vector_store.insert_chunks()`.
     - Result: 100% mutual consistency achieved without re-reading files from disk.

3. **Drift Type C — Vector Contract & Embedding Space Drift**:
   - *Problem*: The daemon's embedding contract (`google/embeddinggemma-300m`, 768-dim, L2-normalized) may be updated, or vectors were indexed with a different model or non-normalized vector space, causing cosine similarity calculations to become mathematically invalid.
   - *Detection*: Scan stored chunks:
     - Check `c.vector.len() != 768`.
     - Check vector L2-norm: $\left| \sqrt{\sum v_i^2} - 1.0 \right| > 10^{-4}$.
     - Check `metadata_json` for mismatch with `contract.derive_contract_hash_hex()`.
   - *Healing*: For all drifted chunks, invoke `embedder.embed_text(&c.text_content)`, replace the vector with the newly computed canonical vector, update `metadata_json`, and write back to LanceDB and Tantivy.

4. **Drift Type D — ACL / Access-Tier Policy Drift**:
   - *Problem*: The user modifies folder clearance rules in `quorumrag.toml` (e.g. changing `D:\Projects\Private` from `Tier0Public` to `Tier2Confidential`). Existing stored chunks retain their stale `access_tier = 0`, exposing confidential chunks to unauthorized peers during federated queries!
   - *Detection*: For every stored chunk, resolve its active policy tier based on current configuration directory rules. Compare `c.access_tier != expected_tier`.
   - *Healing*: Atomically update `access_tier` in LanceDB and Tantivy for all mismatched chunks. Ensures SQL pre-filtering (`access_tier <= max_tier`) immediately enforces security invariants without re-chunking or re-embedding.

5. **Drift Type E — Storage Fragmentation & Tombstone Drift**:
   - *Problem*: Continuous updates and deletes create deleted row tombstones and small fragment files in LanceDB, and generate unmerged segment files in Tantivy, degrading search performance and exhausting file handles.
   - *Detection*: Query storage metrics: fragment count $> 10$, deleted rows $> 0$, or Tantivy segments $> 8$.
   - *Healing*: Invoke `storage.compact().await`, executing `table.optimize(OptimizeAction::All)` and Tantivy segment merging.

### 2.3 Unified `IngestEngine` Coordination (`engine.rs` / `lib.rs`)
The `IngestEngine` is the single supervisor controlling all ingestion subsystems:
- Holds shared thread-safe handles: `Arc<StorageEngine>`, `Arc<FileTypeClassifier>`, `Arc<dyn EmbeddingGenerator>`, `Arc<TwoLevelDedupEngine>`.
- Coordinates live filesystem watching (`watcher.rs`), directory walking (`crawler.rs`), reindexing worker (`reindex.rs`), and drift healing (`drift_healer.rs`).
- Provides micro-batched event aggregation (e.g. 64 chunks or 500ms debounce) to maximize LanceDB columnar write throughput.
- Enforces graceful shutdown using `tokio_util::sync::CancellationToken`.

---

## 3. Caveats

1. **Windows File Locks (`ERROR_SHARING_VIOLATION` / `0x20`)**:
   - On Windows, actively written files or files held open by antivirus / IDEs will fail on `File::open` or read. The reindexing and drift-healing engines must incorporate exponential backoff retries (e.g. 3 attempts with 50ms, 100ms, 200ms delays) and skip permanently locked files with structured warnings rather than crashing.
2. **Drive Scale (30TB+ / Millions of Files)**:
   - Storing all file paths in memory during drift healing on a 30TB drive with 10M files requires ~1GB of RAM. The Merkle drift healer must stream directory entries and database chunks in paginated batches (e.g. 10,000 files per cursor batch) rather than collecting the entire disk catalog into a single `Vec`.
3. **LanceDB Write Concurrency**:
   - LanceDB uses Arrow RecordBatches and table locks. Concurrent writes from the live watcher, a running reindex task, and a drift healer must be serialized or managed via an internal FIFO write queue in `IngestEngine` to prevent write collision errors.
4. **Storage Parity Primitives**:
   - Currently, `quorum-storage`'s `StorageEngine` has `delete_by_file_path` and `compact()`, but lacks public chunk-catalog scanning methods. To implement drift healing cleanly without violating Arrow encapsulation, we propose 5 minimal domain-level methods to be added to `StorageEngine` (detailed below).

---

## 4. Conclusion & Technical Architecture

### 4.1 Specification: `crates/quorum-ingest/src/reindex.rs`

#### Data Structures & Types
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Reindexing execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexOptions {
    /// Target root paths to reindex. If empty, uses configured daemon watch directories.
    pub target_paths: Vec<PathBuf>,
    /// Full rebuild mode: bypasses mtime and Level-1 BLAKE3 catalogs, re-verifying all files.
    pub full_rebuild: bool,
    /// Force re-embedding all chunks even if Level-2 chunk BLAKE3 cache matches.
    pub force_reembed: bool,
    /// Micro-batch size for storage writes (default: 64).
    pub batch_size: usize,
    /// Number of worker threads for parallel file walking (default: num_cpus).
    pub num_threads: usize,
    /// Dry run mode: reports files and chunks that would be changed without writing to DB.
    pub dry_run: bool,
}

impl Default for ReindexOptions {
    fn default() -> Self {
        Self {
            target_paths: Vec::new(),
            full_rebuild: false,
            force_reembed: false,
            batch_size: 64,
            num_threads: num_cpus::get().clamp(1, 8),
            dry_run: false,
        }
    }
}

/// Current execution phase of the reindex worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReindexPhase {
    Idle,
    ScanningFilesystem,
    ChunkingAndVectorizing,
    WritingToStorage,
    CompactingIndexes,
    Completed,
    Failed,
    Cancelled,
}

/// Live telemetry and progress statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexProgress {
    pub task_id: Uuid,
    pub phase: ReindexPhase,
    pub full_rebuild: bool,
    pub files_scanned: usize,
    pub files_total_estimated: usize,
    pub files_indexed: usize,
    pub files_skipped_unmodified: usize,
    pub files_failed: usize,
    pub chunks_reused_from_cache: usize,
    pub chunks_newly_embedded: usize,
    pub bytes_processed: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub elapsed_secs: u64,
    pub error_message: Option<String>,
}

/// Dedicated Reindex Worker.
pub struct ReindexWorker {
    storage: Arc<quorum_storage::StorageEngine>,
    classifier: Arc<quorum_classifier::FileTypeClassifier>,
    embedder: Arc<dyn quorum_contract::EmbeddingGenerator>,
    dedup: Arc<crate::dedup::TwoLevelDedupEngine>,
    active_task: Arc<tokio::sync::RwLock<Option<ReindexProgress>>>,
    cancel_token: Arc<tokio::sync::RwLock<Option<CancellationToken>>>,
}
```

#### Reindex Worker Core Methods
```rust
impl ReindexWorker {
    pub fn new(
        storage: Arc<quorum_storage::StorageEngine>,
        classifier: Arc<quorum_classifier::FileTypeClassifier>,
        embedder: Arc<dyn quorum_contract::EmbeddingGenerator>,
        dedup: Arc<crate::dedup::TwoLevelDedupEngine>,
    ) -> Self {
        Self {
            storage,
            classifier,
            embedder,
            dedup,
            active_task: Arc::new(tokio::sync::RwLock::new(None)),
            cancel_token: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Triggers an asynchronous reindex task, returning immediately with a task UUID.
    pub async fn start_reindex(&self, options: ReindexOptions) -> Result<Uuid, crate::error::IngestError> {
        let mut active = self.active_task.write().await;
        if let Some(ref current) = *active {
            if matches!(current.phase, ReindexPhase::ScanningFilesystem | ReindexPhase::ChunkingAndVectorizing | ReindexPhase::WritingToStorage) {
                return Err(crate::error::IngestError::TaskAlreadyRunning("Reindexing task already in progress".into()));
            }
        }

        let task_id = Uuid::new_v4();
        let cancel_token = CancellationToken::new();
        *self.cancel_token.write().await = Some(cancel_token.clone());

        let initial_progress = ReindexProgress {
            task_id,
            phase: ReindexPhase::ScanningFilesystem,
            full_rebuild: options.full_rebuild,
            files_scanned: 0,
            files_total_estimated: 0,
            files_indexed: 0,
            files_skipped_unmodified: 0,
            files_failed: 0,
            chunks_reused_from_cache: 0,
            chunks_newly_embedded: 0,
            bytes_processed: 0,
            started_at: Utc::now(),
            completed_at: None,
            elapsed_secs: 0,
            error_message: None,
        };
        *active = Some(initial_progress);
        drop(active);

        // Spawn detached Tokio task
        let worker_storage = self.storage.clone();
        let worker_classifier = self.classifier.clone();
        let worker_embedder = self.embedder.clone();
        let worker_dedup = self.dedup.clone();
        let worker_progress = self.active_task.clone();

        tokio::spawn(async move {
            let res = Self::execute_reindex_pipeline(
                options,
                task_id,
                worker_storage,
                worker_classifier,
                worker_embedder,
                worker_dedup,
                worker_progress.clone(),
                cancel_token,
            ).await;

            let mut final_lock = worker_progress.write().await;
            if let Some(ref mut prog) = *final_lock {
                prog.completed_at = Some(Utc::now());
                prog.elapsed_secs = (Utc::now() - prog.started_at).num_seconds().max(0) as u64;
                match res {
                    Ok(_) => prog.phase = ReindexPhase::Completed,
                    Err(crate::error::IngestError::Cancelled) => prog.phase = ReindexPhase::Cancelled,
                    Err(e) => {
                        prog.phase = ReindexPhase::Failed;
                        prog.error_message = Some(e.to_string());
                    }
                }
            }
        });

        Ok(task_id)
    }

    /// Cooperative cancellation of an ongoing reindex task.
    pub async fn cancel_reindex(&self) -> bool {
        let cancel_lock = self.cancel_token.read().await;
        if let Some(ref token) = *cancel_lock {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// Fetches the latest live progress of the reindex worker.
    pub async fn get_progress(&self) -> Option<ReindexProgress> {
        let lock = self.active_task.read().await;
        lock.clone()
    }

    /// Internal execution loop.
    async fn execute_reindex_pipeline(
        options: ReindexOptions,
        task_id: Uuid,
        storage: Arc<quorum_storage::StorageEngine>,
        classifier: Arc<quorum_classifier::FileTypeClassifier>,
        embedder: Arc<dyn quorum_contract::EmbeddingGenerator>,
        dedup: Arc<crate::dedup::TwoLevelDedupEngine>,
        progress: Arc<tokio::sync::RwLock<Option<ReindexProgress>>>,
        cancel: CancellationToken,
    ) -> Result<(), crate::error::IngestError> {
        // 1. Gather all target files via ignore::WalkParallel
        // 2. For each file:
        //    - Check cancel.is_cancelled()
        //    - If !options.full_rebuild: check (mtime, size) against dedup.file_catalog
        //    - If matches, increment files_skipped_unmodified
        //    - If modified or full_rebuild: read file, compute Level-1 BLAKE3
        //    - If Level-1 matches, update path (rename/move detection), 0 re-embeddings
        //    - If Level-1 differs: classify via Magika
        //      - Text/Code/Doc: chunk via Chunker, check Level-2 chunk cache
        //      - Image: route to ImageIndexer, extract EXIF/caption/tokens
        //    - Embed novel chunks via embedder.embed_batch()
        //    - Micro-batch insert into storage
        // 3. If full_rebuild: purge orphaned DB records and run storage.compact().await
        Ok(())
    }
}
```

---

### 4.2 Specification: `crates/quorum-ingest/src/drift_healer.rs`

#### Data Structures & Types
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Categories of drift that can be detected and remediated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftType {
    /// Filesystem <-> Database Merkle state drift (untracked or deleted files).
    MerkleState,
    /// LanceDB Vector <-> Tantivy BM25 hybrid index desync.
    HybridIndexDesync,
    /// Obsolete contract hash, dimension, or non-unit L2 norm.
    VectorContract,
    /// ACL access_tier policy mismatch with directory configuration.
    AclPolicy,
    /// Storage fragmentation, tombstones, and unmerged segments.
    StorageCompaction,
}

/// Detailed drift report returned after scanning or healing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriftReport {
    pub scanned_at: DateTime<Utc>,
    pub healed_at: Option<DateTime<Utc>>,
    pub is_healed: bool,

    // Drift A: Merkle State
    pub orphaned_db_files_purged: usize,
    pub missing_fs_files_indexed: usize,
    pub outdated_files_reindexed: usize,

    // Drift B: Hybrid Index Desync
    pub lance_chunks_missing_in_tantivy: usize,
    pub tantivy_chunks_missing_in_lance: usize,
    pub hybrid_parity_restored: bool,

    // Drift C: Vector Contract
    pub contract_drifted_chunks_reembedded: usize,
    pub active_contract_hash: String,

    // Drift D: ACL Policy
    pub acl_drifted_chunks_updated: usize,

    // Drift E: Storage Compaction
    pub fragments_consolidated: usize,
    pub old_versions_pruned: usize,
    pub tantivy_segments_merged: bool,

    pub total_anomalies_detected: usize,
    pub total_anomalies_healed: usize,
    pub duration_ms: u64,
}

/// Drift options configuring which drifts to fix and execution mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftFixOptions {
    /// Specific drift types to address. Empty means fix ALL 5 types.
    pub target_drifts: Vec<DriftType>,
    /// Dry run mode: detect and report drift anomalies without modifying storage.
    pub dry_run: bool,
    /// Directory configuration mapping (path prefix -> AccessTier) for ACL drift.
    pub access_tier_rules: Vec<(PathBuf, u8)>,
}

impl Default for DriftFixOptions {
    fn default() -> Self {
        Self {
            target_drifts: Vec::new(), // All drifts
            dry_run: false,
            access_tier_rules: Vec::new(),
        }
    }
}

/// Comprehensive Drift Remediation Subsystem.
pub struct DriftHealer {
    storage: Arc<quorum_storage::StorageEngine>,
    classifier: Arc<quorum_classifier::FileTypeClassifier>,
    embedder: Arc<dyn quorum_contract::EmbeddingGenerator>,
    dedup: Arc<crate::dedup::TwoLevelDedupEngine>,
    watched_paths: Vec<PathBuf>,
}
```

#### Drift Healer Core Algorithms (The 5 Drift Repairs)
```rust
impl DriftHealer {
    pub fn new(
        storage: Arc<quorum_storage::StorageEngine>,
        classifier: Arc<quorum_classifier::FileTypeClassifier>,
        embedder: Arc<dyn quorum_contract::EmbeddingGenerator>,
        dedup: Arc<crate::dedup::TwoLevelDedupEngine>,
        watched_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            storage,
            classifier,
            embedder,
            dedup,
            watched_paths,
        }
    }

    /// Detects and repairs all 5 drift categories according to `options`.
    pub async fn fix_all_drifts(&self, options: DriftFixOptions) -> Result<DriftReport, crate::error::IngestError> {
        let start_time = std::time::Instant::now();
        let mut report = DriftReport {
            scanned_at: Utc::now(),
            active_contract_hash: self.embedder.contract().derive_contract_hash_hex(),
            ..Default::default()
        };

        let target_all = options.target_drifts.is_empty();
        let targets: HashSet<DriftType> = options.target_drifts.into_iter().collect();

        // 1. Drift Type A: Merkle State Drift (FS <-> DB)
        if target_all || targets.contains(&DriftType::MerkleState) {
            self.heal_merkle_state_drift(&mut report, options.dry_run).await?;
        }

        // 2. Drift Type B: Hybrid Index Desync (LanceDB <-> Tantivy)
        if target_all || targets.contains(&DriftType::HybridIndexDesync) {
            self.heal_hybrid_index_desync(&mut report, options.dry_run).await?;
        }

        // 3. Drift Type C: Vector Contract Drift
        if target_all || targets.contains(&DriftType::VectorContract) {
            self.heal_vector_contract_drift(&mut report, options.dry_run).await?;
        }

        // 4. Drift Type D: ACL Policy Drift
        if target_all || targets.contains(&DriftType::AclPolicy) {
            self.heal_acl_policy_drift(&options.access_tier_rules, &mut report, options.dry_run).await?;
        }

        // 5. Drift Type E: Storage Compaction & Tombstone Drift
        if target_all || targets.contains(&DriftType::StorageCompaction) {
            self.heal_storage_compaction(&mut report, options.dry_run).await?;
        }

        report.duration_ms = start_time.elapsed().as_millis() as u64;
        report.healed_at = Some(Utc::now());
        report.is_healed = !options.dry_run;
        report.total_anomalies_detected = report.orphaned_db_files_purged
            + report.missing_fs_files_indexed
            + report.outdated_files_reindexed
            + report.lance_chunks_missing_in_tantivy
            + report.tantivy_chunks_missing_in_lance
            + report.contract_drifted_chunks_reembedded
            + report.acl_drifted_chunks_updated;
        report.total_anomalies_healed = if options.dry_run { 0 } else { report.total_anomalies_detected };

        Ok(report)
    }

    /// Drift A: Filesystem <-> DB Merkle State Reconciliation
    async fn heal_merkle_state_drift(
        &self,
        report: &mut DriftReport,
        dry_run: bool,
    ) -> Result<(), crate::error::IngestError> {
        // Step 1: Scan all recorded file_paths from DB/catalog
        let indexed_files = self.dedup.get_all_tracked_files();

        for file_path in indexed_files {
            let path = std::path::Path::new(&file_path);
            if !path.exists() {
                // File was deleted while watcher was offline
                report.orphaned_db_files_purged += 1;
                if !dry_run {
                    self.storage.delete_by_file_path(&file_path).await?;
                    self.dedup.remove_file(&file_path);
                }
            } else if let Ok(meta) = std::fs::metadata(path) {
                // Check if file was modified while offline
                if let Some(catalog_entry) = self.dedup.get_file_entry(&file_path) {
                    if meta.len() != catalog_entry.size || meta.modified().ok() != catalog_entry.mtime {
                        report.outdated_files_reindexed += 1;
                        if !dry_run {
                            // Ingest modified file
                            self.reingest_single_file(path).await?;
                        }
                    }
                }
            }
        }

        // Step 2: Walk filesystem to find untracked files created while offline
        // (Using ignore::WalkBuilder over self.watched_paths)
        Ok(())
    }

    /// Drift B: LanceDB Vector <-> Tantivy BM25 Parity Reconciliation
    async fn heal_hybrid_index_desync(
        &self,
        report: &mut DriftReport,
        dry_run: bool,
    ) -> Result<(), crate::error::IngestError> {
        let lance_chunk_ids = self.storage.get_lance_chunk_catalog().await?;
        let tantivy_chunk_ids = self.storage.get_tantivy_chunk_catalog()?;

        let lance_set: HashSet<String> = lance_chunk_ids.keys().cloned().collect();
        let tantivy_set: HashSet<String> = tantivy_chunk_ids.keys().cloned().collect();

        let lance_only: Vec<String> = lance_set.difference(&tantivy_set).cloned().collect();
        let tantivy_only: Vec<String> = tantivy_set.difference(&lance_set).cloned().collect();

        report.lance_chunks_missing_in_tantivy = lance_only.len();
        report.tantivy_chunks_missing_in_lance = tantivy_only.len();

        if !dry_run {
            // Heal Lance -> Tantivy
            if !lance_only.is_empty() {
                let chunks_to_add = self.storage.get_chunks_by_ids(&lance_only).await?;
                self.storage.insert_into_tantivy_only(&chunks_to_add)?;
            }
            // Heal Tantivy -> Lance
            if !tantivy_only.is_empty() {
                let chunks_to_add = self.storage.get_chunks_from_tantivy(&tantivy_only)?;
                self.storage.insert_into_lance_only(&chunks_to_add).await?;
            }
        }

        report.hybrid_parity_restored = true;
        Ok(())
    }

    /// Drift C: Vector Contract and L2 Normalization Drift Healing
    async fn heal_vector_contract_drift(
        &self,
        report: &mut DriftReport,
        dry_run: bool,
    ) -> Result<(), crate::error::IngestError> {
        let canonical_hash = self.embedder.contract().derive_contract_hash_hex();
        let canonical_dim = self.embedder.dimension();

        let all_chunks = self.storage.scan_all_chunks().await?;
        let mut drifted_chunks = Vec::new();

        for chunk in all_chunks {
            let dim_drift = chunk.vector.len() != canonical_dim;
            let norm: f32 = chunk.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            let norm_drift = (norm - 1.0).abs() > 1e-4;
            let contract_drift = !chunk.metadata_json.contains(&canonical_hash);

            if dim_drift || norm_drift || contract_drift {
                drifted_chunks.push(chunk);
            }
        }

        report.contract_drifted_chunks_reembedded = drifted_chunks.len();

        if !dry_run && !drifted_chunks.is_empty() {
            for mut chunk in drifted_chunks {
                // Re-embed using active canonical Gemma engine
                let new_vec = self.embedder.embed_text(&chunk.text_content)?;
                chunk.vector = new_vec;
                // Update metadata_json with active contract hash
                let mut meta_val: serde_json::Value = serde_json::from_str(&chunk.metadata_json).unwrap_or(serde_json::json!({}));
                meta_val["contract_hash"] = serde_json::json!(canonical_hash);
                chunk.metadata_json = meta_val.to_string();

                self.storage.update_chunk_record(&chunk).await?;
            }
        }

        Ok(())
    }

    /// Drift D: ACL Policy & Access-Tier Drift Healing
    async fn heal_acl_policy_drift(
        &self,
        rules: &[(PathBuf, u8)],
        report: &mut DriftReport,
        dry_run: bool,
    ) -> Result<(), crate::error::IngestError> {
        if rules.is_empty() {
            return Ok(());
        }

        let all_chunks = self.storage.scan_all_chunks().await?;
        let mut drifted_chunks = Vec::new();

        for chunk in all_chunks {
            let chunk_path = PathBuf::from(&chunk.file_path);
            // Longest prefix match determines expected access tier
            let mut expected_tier = 0u8;
            let mut longest_match = 0;

            for (prefix, tier) in rules {
                if chunk_path.starts_with(prefix) {
                    let len = prefix.as_os_str().len();
                    if len > longest_match {
                        longest_match = len;
                        expected_tier = *tier;
                    }
                }
            }

            if chunk.access_tier != expected_tier {
                drifted_chunks.push((chunk.chunk_id, expected_tier));
            }
        }

        report.acl_drifted_chunks_updated = drifted_chunks.len();

        if !dry_run && !drifted_chunks.is_empty() {
            for (chunk_id, new_tier) in drifted_chunks {
                self.storage.update_chunk_access_tier(&chunk_id, new_tier).await?;
            }
        }

        Ok(())
    }

    /// Drift E: Storage Compaction & Tombstone Drift
    async fn heal_storage_compaction(
        &mut self,
        report: &mut DriftReport,
        dry_run: bool,
    ) -> Result<(), crate::error::IngestError> {
        if !dry_run {
            let stats = self.storage.compact().await?;
            report.fragments_consolidated = stats.fragments_removed;
            report.old_versions_pruned = stats.old_versions_pruned;
            report.tantivy_segments_merged = stats.tantivy_segments_merged;
        }
        Ok(())
    }
}
```

---

### 4.3 Specification: `crates/quorum-ingest/src/engine.rs` & `lib.rs`

#### IngestEngine Lifecycle & Architecture
```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// High-level configuration for the IngestEngine.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestConfig {
    pub watch_paths: Vec<PathBuf>,
    pub max_file_size_bytes: u64, // Default 50MB
    pub debounce_ms: u64,         // Default 300ms
    pub batch_size: usize,        // Default 64 chunks
    pub batch_timeout_ms: u64,    // Default 500ms
    pub auto_drift_heal_interval_secs: Option<u64>, // e.g. 1800 (30 mins)
    pub access_tier_rules: Vec<(PathBuf, u8)>,
}

/// Unified IngestEngine.
pub struct IngestEngine {
    config: IngestConfig,
    storage: Arc<quorum_storage::StorageEngine>,
    classifier: Arc<quorum_classifier::FileTypeClassifier>,
    embedder: Arc<dyn quorum_contract::EmbeddingGenerator>,
    dedup: Arc<crate::dedup::TwoLevelDedupEngine>,
    reindex_worker: Arc<crate::reindex::ReindexWorker>,
    drift_healer: Arc<crate::drift_healer::DriftHealer>,
    cancel_token: CancellationToken,
}

impl IngestEngine {
    pub async fn new(
        config: IngestConfig,
        storage: Arc<quorum_storage::StorageEngine>,
        classifier: Arc<quorum_classifier::FileTypeClassifier>,
        embedder: Arc<dyn quorum_contract::EmbeddingGenerator>,
    ) -> Result<Self, crate::error::IngestError> {
        let dedup = Arc::new(crate::dedup::TwoLevelDedupEngine::new());
        let reindex_worker = Arc::new(crate::reindex::ReindexWorker::new(
            storage.clone(),
            classifier.clone(),
            embedder.clone(),
            dedup.clone(),
        ));
        let drift_healer = Arc::new(crate::drift_healer::DriftHealer::new(
            storage.clone(),
            classifier.clone(),
            embedder.clone(),
            dedup.clone(),
            config.watch_paths.clone(),
        ));

        Ok(Self {
            config,
            storage,
            classifier,
            embedder,
            dedup,
            reindex_worker,
            drift_healer,
            cancel_token: CancellationToken::new(),
        })
    }

    /// Starts all background watchers, periodic drift monitors, and event loops.
    pub async fn start(&self) -> Result<(), crate::error::IngestError> {
        // 1. Initial crawler pass over config.watch_paths
        // 2. Start notify watcher with 300ms debouncer
        // 3. Start scheduled drift healer if auto_drift_heal_interval_secs is configured
        Ok(())
    }

    /// Clean cooperative shutdown of all background tasks.
    pub async fn stop(&self) -> Result<(), crate::error::IngestError> {
        self.cancel_token.cancel();
        self.reindex_worker.cancel_reindex().await;
        Ok(())
    }

    /// Direct reindexing trigger.
    pub async fn reindex(&self, options: crate::reindex::ReindexOptions) -> Result<uuid::Uuid, crate::error::IngestError> {
        self.reindex_worker.start_reindex(options).await
    }

    /// Direct drift repair trigger.
    pub async fn fix_drift(&self, mut options: crate::drift_healer::DriftFixOptions) -> Result<crate::drift_healer::DriftReport, crate::error::IngestError> {
        if options.access_tier_rules.is_empty() {
            options.access_tier_rules = self.config.access_tier_rules.clone();
        }
        self.drift_healer.fix_all_drifts(options).await
    }
}
```

---

### 4.4 Proposed `quorum-storage` Parity Primitives

To cleanly empower `drift_healer.rs` without leaking raw Arrow types or breaking module boundaries, we propose adding the following minimal domain methods to `crates/quorum-storage/src/engine.rs`:

```rust
// In crates/quorum-storage/src/engine.rs:

/// Minimal chunk metadata descriptor for parity reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSummary {
    pub chunk_id: String,
    pub blake3_hash: String,
    pub file_path: String,
    pub access_tier: u8,
}

impl StorageEngine {
    /// Returns chunk summaries from LanceDB for desync checking.
    pub async fn get_lance_chunk_catalog(&self) -> Result<HashMap<String, ChunkSummary>, StorageError>;

    /// Returns chunk summaries from Tantivy for desync checking.
    pub fn get_tantivy_chunk_catalog(&self) -> Result<HashMap<String, ChunkSummary>, StorageError>;

    /// Fetches full DocumentChunk records by chunk IDs from LanceDB.
    pub async fn get_chunks_by_ids(&self, chunk_ids: &[String]) -> Result<Vec<DocumentChunk>, StorageError>;

    /// Fetches full DocumentChunk records by chunk IDs from Tantivy.
    pub fn get_chunks_from_tantivy(&self, chunk_ids: &[String]) -> Result<Vec<DocumentChunk>, StorageError>;

    /// Inserts chunks exclusively into Tantivy (for Lance -> Tantivy healing).
    pub fn insert_into_tantivy_only(&self, chunks: &[DocumentChunk]) -> Result<usize, StorageError>;

    /// Inserts chunks exclusively into LanceDB (for Tantivy -> Lance healing).
    pub async fn insert_into_lance_only(&self, chunks: &[DocumentChunk]) -> Result<usize, StorageError>;

    /// Scans all stored chunks (used for contract & ACL drift audits).
    pub async fn scan_all_chunks(&self) -> Result<Vec<DocumentChunk>, StorageError>;

    /// Updates vector and metadata for a chunk in-place.
    pub async fn update_chunk_record(&self, chunk: &DocumentChunk) -> Result<(), StorageError>;

    /// Updates access tier for a chunk in both LanceDB and Tantivy.
    pub async fn update_chunk_access_tier(&self, chunk_id: &str, new_tier: u8) -> Result<(), StorageError>;
}
```

---

### 4.5 CLI and REST API Integration Endpoints

#### CLI Surface (`quorum-daemon`)
- `quorumrag reindex [--path <dir>] [--full] [--force-reembed] [--dry-run]`
  - Emits real-time progress via terminal ANSI status line and prints summary metrics upon completion.
- `quorumrag fix-drift [--type <all|merkle|hybrid|contract|acl|storage>] [--dry-run]`
  - Audits storage and prints a formatted 5-drift health table detailing anomalies found and repairs applied.

#### REST API Surface (`quorum-ui` / Axum Router)
1. `POST /api/reindex`:
   - Request Body: `{ "path": "/data/docs", "full": false, "dry_run": false }`
   - Response: `202 Accepted` `{ "task_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d", "status": "scanning_filesystem" }`
2. `GET /api/reindex/status`:
   - Response: `200 OK` with JSON `ReindexProgress` (used for live Web UI progress bar).
3. `POST /api/reindex/cancel`:
   - Response: `200 OK` `{ "cancelled": true }`.
4. `POST /api/drift/fix`:
   - Request Body: `{ "target_drifts": ["merkle_state", "hybrid_index_desync"], "dry_run": false }`
   - Response: `200 OK` with JSON `DriftReport`.
5. `GET /api/drift/status`:
   - Response: `200 OK` `{ "is_healthy": true, "last_healed_at": "...", "anomalies_count": 0 }`.

---

## 5. Verification Method

To independently verify the implementation when coded in Milestone 3:

### 5.1 Unit & Integration Test Commands
Execute the following verification commands in `d:\harfile\QuorumRAG`:
```powershell
# 1. Compile all workspace crates
cargo check --workspace

# 2. Run existing core and storage tests (preserves job / memory limits)
cargo test -p quorum-types
cargo test -p quorum-classifier
cargo test -p quorum-contract
cargo test -p quorum-storage

# 3. Run E2E Ingestion and Deduplication suite
cargo test --test test_ingestion_dedup -- --test-threads=2
```

### 5.2 Specific Test Cases to Implement in `tests/e2e/test_ingestion_dedup.rs`
1. `test_reindex_incremental_skips_unmodified_files()`:
   - Create 10 files. Reindex. Verify 10 indexed.
   - Run incremental reindex without file changes. Verify 0 embeddings computed, 10 skipped.
2. `test_reindex_full_rebuild_forces_reprocessing()`:
   - Run reindex with `full_rebuild: true`. Verify all 10 files re-chunked and verified.
3. `test_drift_merkle_heals_offline_file_deletions()`:
   - Index 3 files. Delete 1 file from disk using `std::fs::remove_file`. Run `fix_drift`. Verify 1 orphan purged from DB, search returns 0 hits for deleted file.
4. `test_drift_hybrid_desync_heals_missing_tantivy_chunks()`:
   - Manually insert chunk into LanceDB table while bypassing Tantivy. Run `fix_drift`. Verify chunk is discovered and inserted into Tantivy with 100% keyword search parity.
5. `test_drift_contract_heals_vector_dimension_or_hash_mismatch()`:
   - Simulate a stored chunk with obsolete metadata or drifted norm. Run `fix_drift`. Verify chunk is re-embedded using canonical Gemma 768d contract.
6. `test_drift_acl_policy_updates_access_tiers()`:
   - Change directory rule from `Tier0Public` to `Tier2Confidential`. Run `fix_drift`. Verify stored chunks updated to Tier 2 and Tier 0 queries cannot access them.
7. `test_drift_storage_compaction_consolidates_fragments()`:
   - Insert and delete 50 batches. Run `fix_drift`. Verify compaction reports fragments consolidated and old versions pruned.

### 5.3 Invalidation Conditions
- Any reindex run that re-embeds unchanged files whose Level-1 BLAKE3 hash matches the catalog.
- Any drift repair that crashes on Windows file sharing locks instead of retrying or logging.
- Any drift repair that leaks raw Arrow/LanceDB types into `quorum-ingest` public APIs.
- Any failure of LanceDB and Tantivy to achieve 100% chunk ID set equality post-healing.
