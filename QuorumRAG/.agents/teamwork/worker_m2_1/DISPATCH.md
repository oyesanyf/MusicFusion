# DISPATCH — worker_m2_1

## Task Assignment
You are Worker: Milestone 2 Implementation Worker (Embedded LanceDB Storage & Tantivy FTS with Arrow Isolation).
Working directory: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_1/`

Read:
- `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_2\handoff.md` (detailed storage architecture)
- `d:\harfile\QuorumRAG\tests\e2e\test_storage_isolation.rs` (E2E test suite for storage)

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Write Ownership
You exclusively own and may create/modify:
- `d:\harfile\QuorumRAG\Cargo.toml` (add `crates/quorum-storage` to workspace members)
- `d:\harfile\QuorumRAG\crates\quorum-storage\`

## Implementation Scope
1. **`crates/quorum-storage/Cargo.toml`**:
   - Dependencies: `quorum-types`, `lancedb = "0.41.0"`, `tantivy = "0.27.0"`, `arrow-array = "60.1.0"`, `arrow-schema = "60.1.0"`, `tokio`, `futures`, `serde`, `serde_json`, `thiserror`, `tracing`, `async-trait`.
2. **Absolute Arrow Isolation Invariant**:
   - All `lancedb`, `arrow_array`, `arrow_schema`, and `RecordBatch` types must be strictly private inside `pub(crate)` modules (e.g. `arrow_interop.rs`).
   - ZERO Arrow types may appear in the public APIs, traits, structs, or error types of `crates/quorum-storage`.
   - Public API uses domain types exclusively (`DocumentChunk`, `ScoredChunk`, `AccessTier`, `StorageError`).
3. **`arrow_interop.rs`**:
   - Schema:
     - `chunk_id`: `Utf8` (mandatory)
     - `document_id`: `Utf8` (mandatory)
     - `file_path`: `Utf8` (mandatory)
     - `line_start`: `UInt32` (mandatory)
     - `line_end`: `UInt32` (mandatory)
     - `blake3_hash`: `Utf8` (mandatory)
     - `access_tier`: `UInt8` (mandatory)
     - `vector`: `FixedSizeList(Field::new("item", DataType::Float32, true), 768)`
     - `text_content`: `Utf8` (mandatory)
     - `metadata_json`: `Utf8` (mandatory)
   - `pub(crate) fn chunks_to_record_batch(chunks: &[DocumentChunk]) -> Result<RecordBatch, StorageError>`
   - `pub(crate) fn record_batch_to_chunks(batch: &RecordBatch) -> Result<Vec<DocumentChunk>, StorageError>`
4. **LanceDB Vector Storage**:
   - `StorageEngine::open(config: StorageConfig) -> Result<Self, StorageError>`
   - `insert_chunks(&self, chunks: &[DocumentChunk]) -> Result<usize, StorageError>`
   - Vector search: `table.query().nearest_to(query_vector)?.only_if(format!("access_tier <= {}", max_tier))?.limit(top_k).execute().await?`
   - IVF_PQ vector index creation: `create_index(&["vector"])` with Cosine distance.
5. **Co-located Tantivy BM25 Full-Text Search**:
   - Stored in `config.storage_path.join(".tantivy")`.
   - BM25 schema tokenizing `text_content`, storing metadata and `access_tier` for fast numeric filtering.
   - Text search executing BM25 with `access_tier <= max_tier`.
6. **Storage-Level Reciprocal Rank Fusion (RRF)**:
   - `search_hybrid(&self, text: &str, vector: &[f32], max_tier: u8, top_k: usize) -> Result<Vec<ScoredChunk>, StorageError>` combining vector and text hits with $RRF(d) = \sum \frac{1}{60 + r(d)}$.
7. **Compaction & Lifecycle**:
   - `compact(&self) -> Result<CompactionStats, StorageError>` merging small parquet fragments, cleaning up versions older than 1 day, and merging Tantivy segments.
   - `has_chunk_hash(&self, blake3_hash: &str) -> Result<bool, StorageError>`
   - `delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError>`
8. **Automated Verification**:
   - Run `cargo test -p quorum-storage`
   - Run `cargo test --manifest-path tests/e2e/Cargo.toml --test test_storage_isolation`
   - Run `cargo clippy --workspace --all-targets -- -D warnings`
   - Verify 0 warnings and 0 errors. Document findings in `handoff.md`.


## 2026-10-31T16:07:22Z
You are Worker: Milestone 2 Implementation Worker (Embedded LanceDB Storage & Tantivy FTS with Arrow Isolation).
Working directory: d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_1\

Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md, d:\harfile\QuorumRAG\PROJECT.md, d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_2\handoff.md, d:\harfile\QuorumRAG\tests\e2e\test_storage_isolation.rs, and d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_1\DISPATCH.md.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Your write ownership is:
- d:\harfile\QuorumRAG\Cargo.toml (add crates/quorum-storage to members)
- d:\harfile\QuorumRAG\crates\quorum-storage\

Implement genuine, production-grade LanceDB vector storage with private Arrow RecordBatch conversion, SQL pre-filtering (only_if), co-located Tantivy BM25 full-text index, storage-level RRF hybrid search, and compaction routines. Verify with cargo test -p quorum-storage and cargo test --manifest-path tests/e2e/Cargo.toml --test test_storage_isolation. Write your handoff report to d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_1\handoff.md and send a message when done.


## 2026-10-31T16:13:54Z
**Context**: Milestone 2 Implementation (Storage Engine)
**Content**: User requirement update: Please ensure `quorum-storage` provides explicit enum and query execution for all search modes:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    Keyword, // Tantivy BM25 lexical / keyword search
    Vector,  // LanceDB dense vector similarity search
    Hybrid,  // Combined BM25 + Vector + Reciprocal Rank Fusion
}
```
Provide both individual methods (`search_vector`, `search_text`, `search_hybrid`) and a unified `search(&self, query: &StorageSearchQuery)` that switches on `SearchMode`.
**Action**: Incorporate this into `crates/quorum-storage/src/lib.rs` and verify in unit and E2E tests.
