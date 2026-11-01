# DISPATCH — explorer_survey_2

## Task Assignment
You are Explorer 2: Storage, Ingestion & Embedding Scout.
Your mission is to investigate and design the technical specifications and boundaries for R1 (Storage & Ingestion) and R2 (Gemma Embedding Contract).

### Required Exploration:
1. Examine storage architecture with embedded LanceDB and Arrow:
   - How to strictly encapsulate `lancedb` and `arrow-array` `RecordBatch` conversions inside the storage crate so zero raw Arrow types leak into public APIs/signatures of other crates.
   - Schema design for chunks: chunk_id, document_id, file_path, line_start, line_end, blake3_hash, access_tier, vector (embedding), text_content, etc.
   - Vector indexing (IVF_PQ), Tantivy full-text index creation and querying, scheduled compaction routines, configurable `.lance` storage paths, scaling to 30TB+ drive capacities.
2. Ingestion pipeline:
   - High-throughput file crawler using `notify` (file system events) and `ignore` (gitignore and custom rules).
   - Configurable filters: binary detection, extension filters, max file size caps.
   - Two-level BLAKE3 chunk deduplication: file-level hash + chunk-level hash to avoid recomputing embeddings for unchanged or moved files.
   - Micro-batched writes to storage.
3. Gemma Embedding contract (R2):
   - Offline, deterministic dense vector generation (e.g., local candle / ort / tract / rust-bert / local runtime or fast deterministic offline Gemma embedding runtime).
   - BLAKE3 contract hash computation over dimensions (e.g. 768 or configured dimension), normalization settings (L2), distance metric (Cosine/L2).
   - Handshake validation logic to reject peers with mismatched contract hashes.
4. Deliver a structured report to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_2\handoff.md`.

Read `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` thoroughly before beginning.

## 2026-10-31T14:58:03Z
From: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Content: You are Explorer 2: Storage & Ingestion Scout.
Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_2\.
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md and d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_2\DISPATCH.md.
Investigate and design technical specifications for R1 (Storage & Ingestion: LanceDB encapsulation, Arrow isolation, IVF_PQ, Tantivy FTS, scheduled compaction, notify/ignore filesystem ingestion, two-level BLAKE3 deduplication) and R2 (Gemma Embedding contract: offline deterministic vectors, BLAKE3 contract hash derivation and validation).
Write your structured findings to d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_2\handoff.md and send a message when done.
