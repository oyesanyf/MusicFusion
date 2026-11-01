# Technical Specification & Survey Report: Storage, Ingestion & Embedding Contract (R1 & R2)

**Author**: Explorer 2 (Storage, Ingestion & Embedding Scout)  
**Target Module**: QuorumRAG `quorum-storage` & `quorum-ingest` & `quorum-contract`  
**Date**: 2026-10-31  

---

## 1. Observation

### 1.1 Existing Codebase & Environment Survey
1. **Rust Toolchain**:
   - `cargo --version`: `cargo 1.99.1 (5f94df478 2026-08-27)`
   - `rustc --version`: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`
   - Direct verification confirmed modern Rust edition 2021 support with full async/await and SIMD support.

2. **Crate Ecosystem Availability**:
   - `lancedb = "0.41.0"`: Available on crates.io, serverless low-latency vector database with embedded Lance columnar format, IVF_PQ index creation, and SQL filter pushdown.
   - `tantivy = "0.27.0"`: Available on crates.io, full-text search engine with BM25 scoring, schema-based tokenization, fast numeric fields, and segment merging.
   - `arrow-array = "60.1.0"` & `arrow-schema = "60.1.0"`: Standard Apache Arrow Rust crates.
   - `ignore = "0.4.34"`: ripgrep-grade filesystem crawler supporting `.gitignore`, `.ignore`, global git ignore rules, and parallel directory walk.
   - `notify = "6.1.1"` (or "8.0" / "9.0"): Cross-platform filesystem watcher detecting file creates, writes, moves, and deletions.
   - `blake3 = "1.5"`: High-speed cryptographic tree hashing (~1-3 GB/s per core, SIMD accelerated AVX2/AVX-512) already proven and heavily utilized across `d:\harfile\OshoosiClaw` (`crates/osoosi-audit/src/pqc_primitives.rs:283`, `crates/osoosi-memory/src/lib.rs:2819`).
   - `candle-core = "0.9.1"`, `candle-nn = "0.9.1"`, `candle-transformers = "0.9.1"`, `tokenizers = "0.19"`: Available in workspace dependencies across `d:\harfile\OshoosiClaw\Cargo.toml:117-133` for offline neural execution without Python or external network runtimes.

3. **Pattern Discoveries from `d:\harfile`**:
   - `d:\harfile\OshoosiClaw\crates\osoosi-telemetry\src\file_watch.rs:15-100`: Exemplifies file skip and ignore filtering logic (ignoring ephemeral locks, databases, `.git`, `node_modules`, `target`, and noisy OS logs).
   - `d:\harfile\ModelFusion\crates\mesh`: Demonstrates local peer communication patterns, JSON/binary serialization, and cryptographic hashing.

---

## 2. Logic Chain

### 2.1 Storage Architecture & Absolute Arrow Isolation (R1)
1. **Observation**: Apache Arrow's memory layout (`RecordBatch`, `ArrayRef`, `Float32Array`, `SchemaRef`) and LanceDB internal table references (`lancedb::Table`, `lancedb::Connection`) are version-sensitive, complex internal structures. Leaking them across crate boundaries breaks modularity, prevents simple JSON/Protobuf/CBOR serialization over libp2p or web UI SSE endpoints, and couples the entire system to specific Arrow versions.
2. **Logic**:
   - Define a pure Rust domain model in `quorum-storage` (or shared `quorum-types`):
     ```rust
     #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
     pub struct DocumentChunk {
         pub chunk_id: String,           // Deterministic UUID or b3 hash: "{doc_id}:{chunk_idx}"
         pub document_id: String,        // BLAKE3 hash of source file
         pub file_path: String,          // Relative path from watch root
         pub line_start: u32,            // 1-indexed start line
         pub line_end: u32,              // 1-indexed end line
         pub blake3_hash: String,        // 64-char hex BLAKE3 of chunk text
         pub access_tier: u8,            // 0=Public, 1=Internal, 2=Confidential, 3=Restricted
         pub vector: Vec<f32>,           // 768-dim unit-normalized vector
         pub text_content: String,       // Plain text
         pub metadata_json: String,      // Extensible JSON string
     }
     ```
   - Restrict all Arrow and LanceDB conversions to private `pub(crate)` modules:
     - `arrow_interop.rs`: Converts `&[DocumentChunk]` -> `RecordBatch` and `RecordBatch` -> `Vec<DocumentChunk>`.
     - Uses `arrow_array::builder`:
       - `StringBuilder` for IDs, paths, hashes, text, and metadata.
       - `UInt32Builder` for `line_start`, `line_end`.
       - `UInt8Builder` for `access_tier`.
       - `FixedSizeListBuilder::new(Float32Builder::new(), 768)` for `vector`.
   - The public interface of `StorageEngine` exposes ONLY pure Rust types:
     ```rust
     pub struct StorageEngine { /* private LanceDB & Tantivy fields */ }
     impl StorageEngine {
         pub async fn open(config: StorageConfig) -> Result<Self, StorageError>;
         pub async fn insert_chunks(&self, chunks: &[DocumentChunk]) -> Result<usize, StorageError>;
         pub async fn search_vector(&self, query: &VectorSearchQuery) -> Result<Vec<ScoredChunk>, StorageError>;
         pub async fn search_text(&self, query: &TextSearchQuery) -> Result<Vec<ScoredChunk>, StorageError>;
         pub async fn search_hybrid(&self, query: &HybridSearchQuery) -> Result<Vec<ScoredChunk>, StorageError>;
         pub async fn compact(&self) -> Result<CompactionStats, StorageError>;
         pub async fn delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError>;
         pub async fn has_chunk_hash(&self, blake3_hash: &str) -> Result<bool, StorageError>;
     }
     ```

### 2.2 Vector Indexing (IVF_PQ) & SQL Pre-Filtering (R1)
1. **Observation**: R3 requires enforcing Ed25519 `access_tier` SQL pre-filtering inside LanceDB before vector scanning. Furthermore, datasets scaling to 30TB+ drive capacities (potentially tens of millions of chunks) cannot perform brute-force flat vector scans.
2. **Logic**:
   - LanceDB supports inverted file product quantization (`IVF_PQ`) vector indexing:
     - Partitions (`num_partitions`): 256 for < 1M vectors, 1024 for 1M-10M, 4096 for > 10M vectors.
     - Sub-vectors (`num_sub_vectors`): 16 or 96 (768 is divisible by both 16 [48 dims each] and 96 [8 dims each]).
     - Distance metric: `DistanceType::Cosine`.
     - Index creation trigger: Automatic background task when table row count crosses `auto_ivf_pq_threshold` (e.g. 5,000 chunks).
   - SQL Pre-Filtering:
     - LanceDB natively supports scalar push-down filters:
       `table.query().nearest_to(&query_vector)?.only_if(format!("access_tier <= {}", max_tier))?.limit(top_k).execute().await?`
     - This guarantees that vectors in higher security tiers are pruned before calculating dot products or vector distances, fulfilling the Ed25519 access-tier isolation invariant.

### 2.3 Tantivy Full-Text Search (FTS) & Hybrid Synthesis (R1)
1. **Observation**: Vector search alone fails on exact keyword matches, symbol identifiers (`fn derive_contract_hash`), error logs, and specific numerical codes. Tantivy provides high-performance BM25 inverted index retrieval.
2. **Logic**:
   - Co-locate Tantivy index inside `storage_dir/.tantivy`:
     - Fields:
       - `chunk_id`: `STRING | STORED`
       - `file_path`: `TEXT | STORED`
       - `text_content`: `TEXT | STORED` (indexed with English / code tokenizer)
       - `access_tier`: `FAST | STORED` (fast u64 numeric field for pre-filtering)
       - `blake3_hash`: `STRING | STORED`
   - Hybrid Search combining LanceDB + Tantivy:
     - Vector Query -> Top K candidate chunks from LanceDB (scored by Cosine Similarity).
     - Text Query -> Top K candidate chunks from Tantivy (scored by BM25).
     - Storage-level Reciprocal Rank Fusion (RRF):
       $$\text{RRF\_Score}(d) = \sum_{m \in \{\text{vector}, \text{text}\}} \frac{1}{k + r_m(d)}$$
       where $k = 60$ (default smoothing constant) and $r_m(d)$ is the 1-based rank in method $m$.
     - Returns combined `ScoredChunk` list.

### 2.4 Compaction & 30TB+ Scaling Lifecycle (R1)
1. **Observation**: High-throughput micro-batching on large filesystems produces numerous small Lance files (fragments) and Tantivy segments, causing read amplification and file descriptor exhaustion on 30TB+ drives.
2. **Logic**:
   - Implement scheduled compaction:
     - LanceDB: `table.compact_files(CompactionOptions::default()).execute().await?` merges small parquet fragments into consolidated columnar chunks.
     - Prunes obsolete versions: `table.cleanup_old_versions(Duration::from_secs(86400)).execute().await?`.
     - Tantivy: Periodic commit and segment merge via `IndexWriter`.
   - Dedicated Tokio background loop runs compaction every `compaction_interval_secs` (e.g. 3600s) or when dirty fragment count exceeds 50.

### 2.5 Ingestion Pipeline: `notify`, `ignore`, & Two-Level BLAKE3 Deduplication (R1)
1. **Observation**: Crawling 30TB+ or hundreds of thousands of source code and document files requires skipping non-text binaries, respecting `.gitignore`, avoiding duplicate embedding calculations, and absorbing rapid filesystem write bursts.
2. **Logic**:
   - **Crawler**: `ignore::WalkBuilder` traverses directories in parallel, automatically respecting `.gitignore`, `.ignore`, global git rules, and hidden file exclusions.
   - **Watcher**: `notify::recommended_watcher` listens for OS file events (`Create`, `Modify`, `Remove`, `Rename`).
   - **Debouncer**: An asynchronous sliding window (300ms) aggregates rapid events for the same file before triggering crawler checks.
   - **Configurable Filters**:
     - Binary check: Reads first 1024 bytes. If `0x00` is found or non-printable ASCII exceeds 30%, skips file immediately.
     - Extension filter: Whitelist (`["rs", "py", "ts", "js", "go", "c", "cpp", "md", "txt", "json", "toml", "yaml", "html", "css", "sql"]`).
     - Max file size: Skips files exceeding `max_file_size_bytes` (default 10MB).
   - **Two-Level BLAKE3 Deduplication**:
     - **Level 1 (File Level)**:
       - Maintain local file catalog `(file_path, mtime, size, file_blake3)`.
       - Check `(path, mtime, size)`. If unchanged: skip with 0 disk reads.
       - If changed, stream file through `blake3::Hasher`. If `file_blake3` matches catalog: update mtime, skip re-chunking and re-embedding.
       - **Moved/Renamed File Handling**: If `file_blake3` exists under an old path, update `file_path` in catalog and LanceDB metadata without recomputing any chunk embeddings!
     - **Level 2 (Chunk Level)**:
       - Chunk file content (sliding window 512 tokens / ~1500 chars with 10% overlap).
       - Compute `chunk_blake3 = blake3::hash(chunk_text.as_bytes()).to_hex().to_string()`.
       - Query storage bloom filter / cache: `storage.has_chunk_hash(&chunk_blake3)`.
       - If exists: reuse existing chunk embedding vector directly.
       - If new: queue for batch embedding.
     - **Micro-Batched Writes**:
       - Chunks collected into batches of 64-128.
       - Single batched neural embedding generation.
       - Single atomic LanceDB append + Tantivy write.

### 2.6 Gemma Embedding Contract & Offline Verification (R2)
1. **Observation**: In a P2P federated RAG mesh, cosine similarities are mathematically invalid if peers use different dimensions, different normalization, different token pooling, or different embedding models. A cryptographic contract must govern peer admission.
2. **Logic**:
   - Define canonical contract:
     ```rust
     #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
     pub struct EmbeddingContract {
         pub contract_version: u32,       // 1
         pub model_name: String,          // "google/embeddinggemma-300m"
         pub dimension: usize,            // 768
         pub normalization: String,       // "L2"
         pub distance_metric: String,     // "Cosine"
         pub pooling_strategy: String,    // "MeanPooling"
         pub max_sequence_length: usize,  // 512
     }
     ```
   - Deterministic BLAKE3 Contract Hash:
     ```rust
     pub fn derive_contract_hash(contract: &EmbeddingContract) -> [u8; 32] {
         let mut hasher = blake3::Hasher::new();
         hasher.update(b"QUORUM_RAG_EMBEDDING_CONTRACT_V1\n");
         hasher.update(&contract.contract_version.to_le_bytes());
         hasher.update(contract.model_name.as_bytes());
         hasher.update(b"\n");
         hasher.update(&(contract.dimension as u64).to_le_bytes());
         hasher.update(contract.normalization.as_bytes());
         hasher.update(b"\n");
         hasher.update(contract.distance_metric.as_bytes());
         hasher.update(b"\n");
         hasher.update(contract.pooling_strategy.as_bytes());
         hasher.update(b"\n");
         hasher.update(&(contract.max_sequence_length as u64).to_le_bytes());
         *hasher.finalize().as_bytes()
     }
     ```
   - Handshake Validation:
     - During libp2p connection handshake, peer transmits `remote_contract_hash: [u8; 32]`.
     - Validated using constant-time comparison `subtle::ConstantTimeEq::ct_eq`.
     - If mismatched: immediately abort session with `HandshakeError::ContractMismatch`.
   - Offline Deterministic Runtime:
     - Strictly offline: zero outbound HuggingFace or cloud network requests.
     - Dual-mode `EmbeddingGenerator` trait:
       1. `GemmaCandleEngine`: Production engine using local safetensors & tokenizers.
       2. `DeterministicOfflineEngine`: Fast development and CI engine producing deterministic L2-normalized 768-dim vectors derived from token BLAKE3 hashes. Enables instant local test execution and CI reproducibility without requiring 1GB model weights downloads.

---

## 3. Caveats
1. **Model Weights Size**: Full Gemma 300M safetensors require ~600MB-1.2GB disk space. The dual-mode architecture guarantees that unit/integration tests can run fully offline in seconds using `DeterministicOfflineEngine`, while production environments load real safetensors.
2. **LanceDB Crate Evolution**: LanceDB 0.41 has active updates; wrapping all Arrow interactions inside `quorum-storage` shields all other crates from any future LanceDB API refactoring.
3. **Filesystem Event Limits**: On Linux, `notify` relies on inotify which has `max_user_watches` limits for giant directory trees; on Windows, `ReadDirectoryChangesW` handles large directory trees well. A periodic full-walk reconciliation scan (e.g. every 6 hours) supplements `notify` to catch any missed OS events.

---

## 4. Conclusion & Actionable Specification

### 4.1 Recommended Workspace Crate Layout
```
QuorumRAG/
├── crates/
│   ├── quorum-types/         # Pure Rust domain types (DocumentChunk, EmbeddingContract, ACL)
│   ├── quorum-storage/       # Embedded LanceDB + Tantivy (Arrow strictly encapsulated)
│   ├── quorum-ingest/        # ignore crawler + notify watcher + 2-level BLAKE3 deduplicator
│   ├── quorum-contract/      # Gemma embedding contract, BLAKE3 derivation, offline engine
│   ├── quorum-network/       # rust-libp2p Swarm, federated scatter-gather coordinator, RRF
│   ├── quorum-ui/            # Axum web server, ChatGPT-like UI, SSE streaming
│   └── quorum-daemon/        # Binary entry point wiring all crates together
├── Cargo.toml                # Workspace Cargo.toml
└── PROJECT.md                # Project architecture and tracking document
```

### 4.2 Exact Type & Interface Contracts
- **`DocumentChunk`** (in `quorum-types`):
  `chunk_id: String`, `document_id: String`, `file_path: String`, `line_start: u32`, `line_end: u32`, `blake3_hash: String`, `access_tier: u8`, `vector: Vec<f32>`, `text_content: String`, `metadata_json: String`.
- **`StorageEngine`** (in `quorum-storage`):
  Guaranteed zero Arrow exports. Internal conversion to/from `RecordBatch`. Vector search with `access_tier <= X` SQL pushdown. Tantivy BM25 search. Storage-level RRF combiner. Scheduled IVF_PQ index creation and file compaction.
- **`IngestionEngine`** (in `quorum-ingest`):
  Crawler with `ignore::WalkBuilder`. Watcher with `notify::recommended_watcher` + 300ms debouncer. 1024-byte binary sniffing. Level-1 file BLAKE3 catalog + rename detection. Level-2 chunk BLAKE3 deduplication. Micro-batched inserts.
- **`EmbeddingContract` & `EmbeddingGenerator`** (in `quorum-contract`):
  Deterministic 32-byte BLAKE3 contract hash derivation over 768-dim L2 Cosine specification. Constant-time handshake gating. Offline deterministic engine for instant tests.

---

## 5. Verification Method

### 5.1 Automated Test Suite Commands
When implemented, verify using:
1. **Arrow Isolation Verification**:
   ```powershell
   cargo test -p quorum-storage --lib -- tests::test_arrow_type_isolation
   ```
   *Pass Condition*: Crate compiles cleanly with no Arrow types in public signatures; round-trip `DocumentChunk` <-> `RecordBatch` preserves 100% field parity.

2. **Two-Level BLAKE3 Ingestion & Deduplication**:
   ```powershell
   cargo test -p quorum-ingest --lib -- tests::test_two_level_blake3_dedup
   ```
   *Pass Condition*: Second pass on unmodified files performs 0 embeddings; moved files update paths without re-embedding; single-line edits re-embed only affected chunks.

3. **Gemma Contract & Handshake Verification**:
   ```powershell
   cargo test -p quorum-contract --lib -- tests::test_contract_hash_and_handshake
   ```
   *Pass Condition*: Expected 32-byte hash matches deterministic golden value; mismatched dimension or metric fails validation.

4. **Storage Hybrid Search & Compaction**:
   ```powershell
   cargo test -p quorum-storage --lib -- tests::test_hybrid_search_and_compaction
   ```
   *Pass Condition*: LanceDB vector search respects `access_tier` SQL filter; Tantivy BM25 search returns relevant keywords; RRF fuses scores; compaction executes without data loss.
