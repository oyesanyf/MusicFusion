# Dispatch — Explorer M5.5-1: Coordinator Caching Architecture & BLAKE3 Keys

## Objective
Investigate and design the core in-memory / persistent caching engine for `crates/quorum-coordinator` and `crates/quorum-types`:
1. `EmbeddingCache`:
   - BLAKE3 keyed by `(embedding_contract_hash, normalized_text)` -> dense 768d vector.
   - Thread-safe (e.g. `Arc<RwLock<...>>` or `dashmap` or LRU with mutex), configurable capacity (e.g., default 10,000 entries), TTL.
2. `QueryCache`:
   - Deterministic BLAKE3 key derived from: `(normalized_query, search_mode, model_family, access_tier, index_epoch)`.
   - Caches retrieval candidates, synthesis prompt, citations, generated answer text, and LangExtract grounded extractions.
   - Enforce `ZeroizeOnDrop` zero-trust memory invariants: unpinned volatile remote chunks must either not leak plaintext indefinitely or be sanitized.
3. Cache Hit Telemetry:
   - Expose `cache_hit: bool`, `cache_hits: u64`, `cache_misses: u64`, `cached_queries: usize`, `cached_embeddings: usize`, `cache_epoch: u64`.
   - Wire telemetry into `QueryResult` and SSE stream event payloads (`event: cache_telemetry`).
4. Integration into Agentic State Graph:
   - In `ClassifyIntent`, check `QueryCache` early. If hit, short-circuit retrieval and emit cached synthesis immediately.

## Input Files to Inspect
- `crates/quorum-coordinator/src/lib.rs`
- `crates/quorum-coordinator/src/coordinator.rs`
- `crates/quorum-coordinator/src/agentic/mod.rs`
- `crates/quorum-coordinator/src/agentic/classify.rs`
- `crates/quorum-coordinator/src/agentic/synthesize.rs`
- `crates/quorum-coordinator/src/offline_engine.rs`
- `crates/quorum-types/src/lib.rs`
- `ORIGINAL_REQUEST.md`

## Output
Write your findings and design report to `.agents/teamwork/explorer_m5_5_1/handoff.md`. Include concrete Rust struct definitions, key derivation functions, and integration code snippets.


## 2026-11-01T03:59:40Z
You are Explorer M5.5-1. Read d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md and d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_1\DISPATCH.md. Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_1\. Investigate crates/quorum-coordinator and crates/quorum-types to design the Multi-Layer BLAKE3 Caching Engine (QueryCache, EmbeddingCache, TTL/LRU, ZeroizeOnDrop invariants, cache hit telemetry, and Agentic State Graph integration). Write your detailed design to d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_1\handoff.md and notify me when complete.
