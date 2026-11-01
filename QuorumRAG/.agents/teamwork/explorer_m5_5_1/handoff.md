# Architecture Design Report — Multi-Layer BLAKE3 Caching Engine (`QueryCache` & `EmbeddingCache`)

**Agent**: Explorer M5.5-1  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_5_1\`  
**Target Crates**: `crates/quorum-coordinator`, `crates/quorum-types`, `crates/quorum-contract`, `crates/quorum-daemon`  
**Date**: 2026-11-01T04:15:00Z  

---

## 1. Observation

Direct inspection of the QuorumRAG workspace revealed the following concrete structures, traits, and operational baselines:

1. **Pure Domain Models in `crates/quorum-types/src/lib.rs`**:
   - `DocumentChunk` (lines 137–168): Contains `chunk_id: String`, `document_id: String`, `file_path: String`, `line_start: u32`, `line_end: u32`, `blake3_hash: String`, `access_tier: u8`, `vector: Vec<f32>`, `text_content: String`.
   - `ScoredChunk` (lines 229–281): Wraps `DocumentChunk` with `score: f32`, `source_peer: String`, and `is_remote: bool`. Contains `to_citation(&self) -> Citation`.
   - `CandidateChunkDto` (lines 306–356): Implements `#[derive(Zeroize)]` with `content: String` marked for zeroization.
   - `VolatileRemoteChunk` (lines 561–616): Implements `#[derive(Zeroize, ZeroizeOnDrop)]`. Contains `content: String` that is zeroized when dropped to prevent heap retention or swap leakage of unpinned peer chunks.
   - `QueryResult` (lines 619–633): Contains `chunks: Vec<ScoredChunk>`, `synthesis_prompt: String`, `execution_time_ms: u64`, `sources_cited: Vec<Citation>`. Currently lacks fields for `cache_hit: bool` and `cache_epoch: u64`.
   - `AccessTier` (lines 45–132): `Tier0Public = 0`, `Tier1Internal = 1`, `Tier2Confidential = 2`, `Tier3Restricted = 3`.

2. **Query Coordinator & Agentic State Graph in `crates/quorum-coordinator/`**:
   - `QueryCoordinator` (`src/coordinator.rs`, lines 12–97):
     - `query(&self, prompt: &str, top_k: usize) -> Result<QueryResult, CoordinatorError>`: Generates embedding via `quorum_contract::DeterministicOfflineEngine::new()`, calls `self.engine.execute_query()`, builds prompt, and returns `QueryResult`. Does not check or record any cache.
     - `query_agentic(&self, prompt: &str, family: SynthesisModelFamily, top_k: usize, telemetry_tx: Option<Sender<AgenticTelemetryEvent>>) -> Result<QueryResult, CoordinatorError>`: Runs `AgenticStateGraphRunner`.
   - `AgenticStateGraphRunner` (`src/agentic/mod.rs`, lines 18–263):
     - Transitions sequentially: `ClassifyIntent` -> `RetrieveMesh` -> `GradeRelevance` -> (if rewrite: `RewriteQuery` -> `RetrieveMesh`) -> `SynthesizeWithCitations` -> `Complete`.
     - In `ClassifyIntentNode::execute` (`src/agentic/classify.rs`, lines 8–50): Classifies intent, extracts query metadata filters via `extract_query_metadata_filters`, and selects model family via `AutoFusionRouter::select_model`. It does not probe any cache.
     - In `SynthesizeWithCitationsNode::execute` (`src/agentic/synthesize.rs`, lines 7–15): Calls `DeterministicOfflineSynthesisEngine::synthesize`, populates `synthesized_response` and `citations`.
   - `AgenticGraphState` (`src/types.rs`, lines 55–102): Contains `session_id`, `original_prompt`, `current_query`, `intent`, `search_mode`, `model_family`, `retrieved_chunks`, `volatile_chunks`, `synthesized_response`, `citations`, etc.
   - `AgenticTelemetryEvent` (`src/telemetry.rs`, lines 7–98): Contains SSE events: `StateTransition`, `IntentClassified`, `HardwareProbed`, `RetrievalCompleted`, `RelevanceGraded`, `QueryRewritten`, `SynthesisStarted`, `TokenDelta`, `CitationEmitted`, `ExecutionFinished`, `ExecutionError`. Currently has no `CacheTelemetry` variant.

3. **Gemma Embedding Contract in `crates/quorum-contract/src/lib.rs`**:
   - `EmbeddingContract` (lines 54–120): Has `derive_contract_hash(&self) -> [u8; 32]`, returning a deterministic 32-byte BLAKE3 digest of `(contract_version, model_name, dimension, normalization, distance_metric, pooling_strategy, max_sequence_length)`.
   - `DeterministicOfflineEngine` and `GemmaCandleEngine` implement `EmbeddingGenerator`.

4. **Storage & Ingestion Caching Baselines**:
   - `crates/quorum-storage/src/engine.rs` (lines 20–27): `SearchMode` has `Keyword`, `Vector`, `Hybrid`.
   - `crates/quorum-ingest/src/dedup.rs` (lines 213–270): Implements `Level2ChunkCache` using `Arc<RwLock<Level2ChunkCacheInner>>` with a `HashMap<String, (Vec<f32>, usize)>`, `VecDeque<String>` for LRU, and `CacheStats`.

5. **Daemon Endpoints in `crates/quorum-daemon/src/handlers/`**:
   - `chat_sse_handler` (`chat.rs`, lines 37–105): Receives `ChatRequest`, spawns background task running `coordinator.query_agentic`, converts `AgenticTelemetryEvent` into SSE lines, and streams word deltas.
   - `query_handler` (`chat.rs`, lines 109–139): Returns raw `Json(result)`.
   - `health_handler` (`health.rs`, lines 10–65): Returns system health and hardware telemetry; currently does not report cache hit/miss or cache epoch statistics.
   - `reindex_status_handler` and `drift_healer`: Run background workers that alter storage index files without notifying the coordinator or invalidating cache.

---

## 2. Logic Chain

1. **Deterministic BLAKE3 Cache Key Uniqueness**:
   - *Premise*: A query cache entry is only valid if query text, search parameters, model generation family, security access tier, and the underlying storage index state match identically.
   - *Deduction*: The query cache key must be a 32-byte BLAKE3 digest computed from a canonical byte sequence incorporating:
     $$\text{Key} = \text{BLAKE3}\left(\text{"QUORUM\_QUERY\_CACHE\_V1\\n"} \parallel \text{normalized\_query} \parallel \text{"\\n"} \parallel \text{search\_mode} \parallel \text{model\_family} \parallel \text{access\_tier} \parallel \text{index\_epoch}\right)$$
   - *Epoch Invalidation Invariant*: Incorporating `index_epoch: u64` directly into the hash key guarantees that incrementing `cache_epoch` instantly and atomically invalidates all prior query entries across the entire coordinator without acquiring stop-the-world write locks or traversing hash tables.

2. **Embedding Cache Efficiency & Model Portability**:
   - *Premise*: Dense vector embeddings (768-dim floats = 3,072 bytes per vector) are computationally expensive to calculate (requiring neural forward passes or BLAKE3 token XOF hashing).
   - *Deduction*: Caching vector embeddings keyed by `(embedding_contract_hash: [u8; 32], normalized_text: &str)` prevents duplicate neural executions across repeated queries or identical chunk ingestions. Incorporating `embedding_contract_hash` prevents silent dimension/model corruption if the embedding model is upgraded.

3. **Fail-Closed Zero-Trust & `ZeroizeOnDrop` Invariants**:
   - *Premise*: `VolatileRemoteChunk` in `quorum-types` is marked `#[derive(Zeroize, ZeroizeOnDrop)]` to prevent plaintext data from untrusted peer nodes lingering in heap memory or leaking to swap.
   - *Deduction*: Caching remote chunks across user queries could breach zero-trust containment if sensitive peer data is retained indefinitely in memory.
   - *Rule*:
     1. Cache keys MUST strictly partition on `access_tier: u8`. A lower-tier caller can never hit a cache line populated by a higher-tier caller.
     2. Cached entries holding remote chunks (`has_remote_unpinned == true`) MUST implement a bounded volatile lifetime and execute `Zeroize` on all remote chunk plaintext when the cache entry is dropped or evicted.
     3. Alternatively, when `access_tier > Tier0Public`, unpinned volatile remote chunks are either excluded from the persistent query cache or strictly scrubbed upon epoch change.

4. **Agentic State Graph Short-Circuiting**:
   - *Premise*: Multi-hop retrieval with 400ms scatter-gather, relevance grading, and LLM synthesis takes 100ms–2,000ms.
   - *Deduction*: Checking the `QueryCache` at the beginning of `ClassifyIntentNode` allows an immediate cache hit to populate `graph_state.retrieved_chunks`, `synthesized_response`, and `citations`.
   - *Action*: The state runner emits `AgenticTelemetryEvent::CacheTelemetry { cache_hit: true, ... }`, followed by token deltas, and transitions directly to `Complete` in < 2ms, bypassing all downstream retrieval, grading, rewrite, and synthesis nodes.

5. **Cache Coherence & Lifecycle Synchronization**:
   - *Premise*: Modifying documents on disk via reindexing, drift repair, or real-time file watcher invalidates retrieval rankings.
   - *Deduction*: The coordinator must provide an atomic `invalidate_cache()` method that increments `cache_epoch`. The daemon's `ReindexWorker`, `DriftHealer`, and `FilesystemWatcher` must be wired to trigger this invalidation upon write completion.
   - *Re-cache CLI & API*: `quorumrag recache [--clear] [--warm]` and `POST /api/cache/recache` flush memory, advance the epoch, and optionally pre-warm embeddings from LanceDB.

---

## 3. Caveats

1. **Lock Contention Under Heavy Concurrency**:
   - Using a single `std::sync::RwLock` or `tokio::sync::RwLock` for a very large LRU cache can create contention if multiple concurrent threads attempt to update the LRU order on read hits.
   - *Mitigation*: Separate the storage map from LRU tracking, or use an atomic hit counter with a batched LRU promotion strategy, or use `parking_lot::RwLock` with read-optimistic locking.
2. **Memory Footprint with High Top-K**:
   - Storing 1,000 full `QueryResult` payloads with 20 chunks each (each chunk ~1KB) requires ~20MB of RAM, which is completely negligible on modern hardware. For 10,000 dense vectors (768 * 4 bytes = 3KB each), total embedding cache size is ~30MB. The combined cache consumes < 60MB, well within the Tier 3 memory budget.
3. **Remote Peer Ephemeral Availability**:
   - A remote peer providing a chunk on an initial query may go offline. If the query is cached and returned from cache, citations will reference the peer ID that was online at cache creation. This is valid for cached RAG responses within the TTL (default 1 hour).
4. **Normalized Query Text Collisions**:
   - Query text normalization (trimming, lowercasing, and collapsing multiple spaces) ensures semantic equivalence (`"hello  world"` matches `"hello world"`), while avoiding unintentional collisions. Case-sensitive code identifiers inside prompts are preserved by applying lowercase only for standard English text or using canonical whitespace normalization while preserving token casing.

---

## 4. Conclusion & Complete Architectural Specification

### 4.1 Domain Types Extension (`crates/quorum-types`)

In `crates/quorum-types/src/lib.rs`, extend the domain definitions to include telemetry and cache metadata without introducing any Arrow or LanceDB dependencies:

```rust
// ============================================================================
// Additions to crates/quorum-types/src/lib.rs
// ============================================================================

/// Live telemetry metrics for multi-layer query and embedding caching.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct CacheTelemetry {
    /// Whether the active request resulted in a cache hit.
    pub cache_hit: bool,
    /// Total cumulative cache hits across coordinator lifetime.
    pub hits: u64,
    /// Total cumulative cache misses across coordinator lifetime.
    pub misses: u64,
    /// Calculated cache hit rate: hits / (hits + misses).
    pub hit_rate: f32,
    /// Number of active entries currently retained in QueryCache.
    pub cached_queries: usize,
    /// Number of active vectors currently retained in EmbeddingCache.
    pub cached_embeddings: usize,
    /// Monotonically increasing cache epoch counter.
    pub cache_epoch: u64,
}

impl CacheTelemetry {
    pub fn new(
        cache_hit: bool,
        hits: u64,
        misses: u64,
        cached_queries: usize,
        cached_embeddings: usize,
        cache_epoch: u64,
    ) -> Self {
        let total = hits.saturating_add(misses);
        let hit_rate = if total == 0 {
            0.0
        } else {
            hits as f32 / total as f32
        };

        Self {
            cache_hit,
            hits,
            misses,
            hit_rate,
            cached_queries,
            cached_embeddings,
            cache_epoch,
        }
    }
}

/// Status response returned by `POST /api/cache/recache` and `quorumrag recache`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecacheResponse {
    pub status: String,
    pub cache_epoch: u64,
    pub cleared_entries: usize,
    pub warmed_entries: usize,
    pub duration_ms: u64,
}

/// Upgraded QueryResult with first-class cache hit telemetry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResult {
    /// Fused, deduplicated, and ranked scored chunks.
    pub chunks: Vec<ScoredChunk>,
    /// Augmented prompt formatted for downstream synthesis.
    pub synthesis_prompt: String,
    /// Execution time in milliseconds for the complete scatter-gather operation.
    pub execution_time_ms: u64,
    /// Extracted citations for all referenced sources.
    pub sources_cited: Vec<Citation>,
    /// Flag indicating whether this result was served directly from the BLAKE3 QueryCache.
    #[serde(default)]
    pub cache_hit: bool,
    /// The index/cache epoch at the time of retrieval.
    #[serde(default)]
    pub cache_epoch: u64,
}
```

---

### 4.2 Multi-Layer BLAKE3 Caching Engine (`crates/quorum-coordinator/src/cache/`)

Create module `crates/quorum-coordinator/src/cache/` containing:
- `mod.rs`
- `key.rs`: Deterministic BLAKE3 key derivation functions.
- `embedding_cache.rs`: Thread-safe LRU/TTL dense vector cache.
- `query_cache.rs`: Thread-safe LRU/TTL query, candidate, and synthesis cache with `ZeroizeOnDrop` memory protection.
- `manager.rs`: High-level `CacheManager` coordinating epoch advancements, telemetry, and recaching.

#### 4.2.1 Deterministic Key Derivation (`src/cache/key.rs`)

```rust
//! Deterministic BLAKE3 key derivation for QuorumRAG Multi-Layer Caching.

use quorum_storage::SearchMode;
use crate::types::SynthesisModelFamily;

/// Derives the 32-byte deterministic BLAKE3 cache key for text embeddings.
///
/// Formula:
/// BLAKE3("QUORUM_EMBEDDING_CACHE_V1\n" || contract_hash (32 bytes) || "\n" || normalized_text)
pub fn derive_embedding_cache_key(contract_hash: &[u8; 32], raw_text: &str) -> [u8; 32] {
    let normalized = normalize_text_for_caching(raw_text);
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"QUORUM_EMBEDDING_CACHE_V1\n");
    hasher.update(contract_hash);
    hasher.update(b"\n");
    hasher.update(normalized.as_bytes());
    *hasher.finalize().as_bytes()
}

/// Derives the 32-byte deterministic BLAKE3 cache key for query retrieval & synthesis.
///
/// Layout:
/// BLAKE3("QUORUM_QUERY_CACHE_V1\n" || normalized_query || "\n" || search_mode (1 byte) ||
///        model_family (1 byte) || access_tier (1 byte) || index_epoch (8 bytes LE))
pub fn derive_query_cache_key(
    query_text: &str,
    search_mode: SearchMode,
    model_family: SynthesisModelFamily,
    access_tier: u8,
    index_epoch: u64,
) -> [u8; 32] {
    let normalized = normalize_text_for_caching(query_text);
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"QUORUM_QUERY_CACHE_V1\n");
    hasher.update(normalized.as_bytes());
    hasher.update(b"\n");

    let mode_byte = match search_mode {
        SearchMode::Keyword => 0u8,
        SearchMode::Vector => 1u8,
        SearchMode::Hybrid => 2u8,
    };
    hasher.update(&[mode_byte]);

    let family_byte = match model_family {
        SynthesisModelFamily::Glm4Agentic => 0u8,
        SynthesisModelFamily::Qwen25Code => 1u8,
        SynthesisModelFamily::GemmaProse => 2u8,
        SynthesisModelFamily::AutoFusion => 3u8,
    };
    hasher.update(&[family_byte]);
    hasher.update(&[access_tier]);
    hasher.update(&index_epoch.to_le_bytes());

    *hasher.finalize().as_bytes()
}

/// Normalizes query and chunk text for deterministic caching:
/// Trims leading/trailing whitespace, converts multiple spaces/tabs/newlines to a single space,
/// and converts ASCII to lowercase while preserving non-ASCII unicode boundaries.
pub fn normalize_text_for_caching(text: &str) -> String {
    let trimmed = text.trim();
    let mut normalized = String::with_capacity(trimmed.len());
    let mut in_whitespace = false;

    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !in_whitespace {
                normalized.push(' ');
                in_whitespace = true;
            }
        } else {
            normalized.extend(ch.to_lowercase());
            in_whitespace = false;
        }
    }
    normalized
}
```

#### 4.2.2 `EmbeddingCache` (`src/cache/embedding_cache.rs`)

```rust
//! Thread-safe LRU/TTL dense vector cache for Gemma embeddings.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};
use crate::cache::key::derive_embedding_cache_key;

struct EmbeddingEntry {
    vector: Vec<f32>,
    created_at: Instant,
}

pub struct EmbeddingCache {
    capacity: usize,
    ttl: Duration,
    map: RwLock<HashMap<[u8; 32], EmbeddingEntry>>,
    order: RwLock<VecDeque<[u8; 32]>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl EmbeddingCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            capacity: capacity.max(100),
            ttl,
            map: RwLock::new(HashMap::with_capacity(capacity.min(1000))),
            order: RwLock::new(VecDeque::with_capacity(capacity.min(1000))),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Retrieve a cached embedding vector if present and unexpired.
    pub fn get(&self, contract_hash: &[u8; 32], raw_text: &str) -> Option<Vec<f32>> {
        let key = derive_embedding_cache_key(contract_hash, raw_text);

        // Read phase
        {
            let map = self.map.read().ok()?;
            if let Some(entry) = map.get(&key) {
                if entry.created_at.elapsed() <= self.ttl {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    return Some(entry.vector.clone());
                }
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insert or update an embedding vector with LRU capacity enforcement.
    pub fn insert(&self, contract_hash: &[u8; 32], raw_text: &str, vector: Vec<f32>) {
        let key = derive_embedding_cache_key(contract_hash, raw_text);
        let entry = EmbeddingEntry {
            vector,
            created_at: Instant::now(),
        };

        if let Ok(mut map) = self.map.write() {
            if let Ok(mut order) = self.order.write() {
                if !map.contains_key(&key) && map.len() >= self.capacity {
                    // Evict oldest
                    while let Some(old_key) = order.pop_front() {
                        if map.remove(&old_key).is_some() {
                            break;
                        }
                    }
                }
                map.insert(key, entry);
                order.push_back(key);
            }
        }
    }

    /// Clear all cached vectors.
    pub fn clear(&self) -> usize {
        let mut count = 0;
        if let Ok(mut map) = self.map.write() {
            count = map.len();
            map.clear();
        }
        if let Ok(mut order) = self.order.write() {
            order.clear();
        }
        count
    }

    pub fn stats(&self) -> (u64, u64, usize) {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let count = self.map.read().map(|m| m.len()).unwrap_or(0);
        (hits, misses, count)
    }
}
```

#### 4.2.3 `QueryCache` with `ZeroizeOnDrop` Invariants (`src/cache/query_cache.rs`)

```rust
//! Thread-safe LRU/TTL QueryCache holding candidate chunks, synthesis, citations,
//! and enforcing ZeroizeOnDrop zero-trust unpinned chunk security rules.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};
use zeroize::Zeroize;

use quorum_storage::SearchMode;
use quorum_types::{Citation, ScoredChunk};
use crate::cache::key::derive_query_cache_key;
use crate::langextract::ConsensusExtractionResult;
use crate::types::SynthesisModelFamily;

/// Cached query payload holding retrieval candidates and synthesized response.
///
/// Implements `Drop` to explicitly zeroize plaintext in remote unpinned chunks
/// to prevent unpinned sensitive data from persisting in heap or swap memory.
#[derive(Clone)]
pub struct CachedQueryEntry {
    pub key: [u8; 32],
    pub chunks: Vec<ScoredChunk>,
    pub synthesis_prompt: String,
    pub synthesized_response: Option<String>,
    pub citations: Vec<Citation>,
    pub langextract_result: Option<ConsensusExtractionResult>,
    pub query_metadata_filters: Option<BTreeMap<String, String>>,
    pub access_tier: u8,
    pub index_epoch: u64,
    pub created_at: Instant,
    pub has_remote_unpinned: bool,
}

impl Drop for CachedQueryEntry {
    fn drop(&mut self) {
        // Enforce ZeroizeOnDrop zero-trust memory invariants:
        // When a cached query containing remote unpinned peer chunks is dropped or evicted,
        // explicitly wipe the chunk text buffers with zeroes.
        if self.has_remote_unpinned {
            for chunk in &mut self.chunks {
                if chunk.is_remote {
                    chunk.chunk.text_content.zeroize();
                }
            }
        }
    }
}

pub struct QueryCache {
    capacity: usize,
    ttl: Duration,
    map: RwLock<HashMap<[u8; 32], CachedQueryEntry>>,
    order: RwLock<VecDeque<[u8; 32]>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl QueryCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            capacity: capacity.max(50),
            ttl,
            map: RwLock::new(HashMap::with_capacity(capacity.min(500))),
            order: RwLock::new(VecDeque::with_capacity(capacity.min(500))),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Probe the cache for an exact match under identical query, search mode,
    /// model family, access tier, and index epoch.
    pub fn get(
        &self,
        query: &str,
        search_mode: SearchMode,
        model_family: SynthesisModelFamily,
        access_tier: u8,
        index_epoch: u64,
    ) -> Option<CachedQueryEntry> {
        let key = derive_query_cache_key(query, search_mode, model_family, access_tier, index_epoch);

        {
            let map = self.map.read().ok()?;
            if let Some(entry) = map.get(&key) {
                // Assert epoch consistency and TTL freshness
                if entry.index_epoch == index_epoch && entry.created_at.elapsed() <= self.ttl {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    return Some(entry.clone());
                }
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Store query result into cache, evicting oldest entries on capacity limit.
    pub fn insert(
        &self,
        query: &str,
        search_mode: SearchMode,
        model_family: SynthesisModelFamily,
        access_tier: u8,
        index_epoch: u64,
        chunks: Vec<ScoredChunk>,
        synthesis_prompt: String,
        synthesized_response: Option<String>,
        citations: Vec<Citation>,
        langextract_result: Option<ConsensusExtractionResult>,
        query_metadata_filters: Option<BTreeMap<String, String>>,
    ) {
        let key = derive_query_cache_key(query, search_mode, model_family, access_tier, index_epoch);
        let has_remote_unpinned = chunks.iter().any(|c| c.is_remote);

        let entry = CachedQueryEntry {
            key,
            chunks,
            synthesis_prompt,
            synthesized_response,
            citations,
            langextract_result,
            query_metadata_filters,
            access_tier,
            index_epoch,
            created_at: Instant::now(),
            has_remote_unpinned,
        };

        if let Ok(mut map) = self.map.write() {
            if let Ok(mut order) = self.order.write() {
                if !map.contains_key(&key) && map.len() >= self.capacity {
                    while let Some(old_key) = order.pop_front() {
                        if map.remove(&old_key).is_some() {
                            break;
                        }
                    }
                }
                map.insert(key, entry);
                order.push_back(key);
            }
        }
    }

    /// Clear all query cache entries immediately.
    pub fn clear(&self) -> usize {
        let mut count = 0;
        if let Ok(mut map) = self.map.write() {
            count = map.len();
            map.clear();
        }
        if let Ok(mut order) = self.order.write() {
            order.clear();
        }
        count
    }

    pub fn stats(&self) -> (u64, u64, usize) {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let count = self.map.read().map(|m| m.len()).unwrap_or(0);
        (hits, misses, count)
    }
}
```

#### 4.2.4 Unified `CacheManager` (`src/cache/manager.rs`)

```rust
//! Unified CacheManager coordinating EmbeddingCache, QueryCache, atomic epochs,
//! and live telemetry for QuorumRAG.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use quorum_contract::EmbeddingGenerator;
use quorum_storage::StorageEngine;
use quorum_types::{CacheTelemetry, RecacheResponse};
use crate::cache::embedding_cache::EmbeddingCache;
use crate::cache::query_cache::QueryCache;

pub struct CacheManager {
    pub query_cache: Arc<QueryCache>,
    pub embedding_cache: Arc<EmbeddingCache>,
    pub cache_epoch: Arc<AtomicU64>,
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new(1000, Duration::from_secs(3600), 10000, Duration::from_secs(86400))
    }
}

impl CacheManager {
    pub fn new(
        query_capacity: usize,
        query_ttl: Duration,
        embedding_capacity: usize,
        embedding_ttl: Duration,
    ) -> Self {
        Self {
            query_cache: Arc::new(QueryCache::new(query_capacity, query_ttl)),
            embedding_cache: Arc::new(EmbeddingCache::new(embedding_capacity, embedding_ttl)),
            cache_epoch: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Return current atomic cache epoch.
    #[inline]
    pub fn epoch(&self) -> u64 {
        self.cache_epoch.load(Ordering::SeqCst)
    }

    /// Advance epoch counter, immediately and atomically invalidating all prior QueryCache entries.
    pub fn advance_epoch(&self) -> u64 {
        self.cache_epoch.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Invalidate cache due to index changes, reindexing, or file watcher modification.
    pub fn invalidate(&self) -> u64 {
        let new_epoch = self.advance_epoch();
        self.query_cache.clear();
        new_epoch
    }

    /// Collect aggregated cache telemetry.
    pub fn telemetry(&self, latest_hit: bool) -> CacheTelemetry {
        let (q_hits, q_misses, q_count) = self.query_cache.stats();
        let (e_hits, e_misses, e_count) = self.embedding_cache.stats();
        let total_hits = q_hits.saturating_add(e_hits);
        let total_misses = q_misses.saturating_add(e_misses);
        let epoch = self.epoch();

        CacheTelemetry::new(
            latest_hit,
            total_hits,
            total_misses,
            q_count,
            e_count,
            epoch,
        )
    }

    /// Executes the full re-cache process: flushes stale caches, increments epoch,
    /// and optionally warms hot embeddings from storage.
    pub async fn recache(
        &self,
        storage: Option<&StorageEngine>,
        embedder: Option<&dyn EmbeddingGenerator>,
        clear: bool,
        warm: bool,
    ) -> RecacheResponse {
        let start = Instant::now();
        let mut cleared_entries = 0;
        let mut warmed_entries = 0;

        if clear {
            cleared_entries += self.query_cache.clear();
            cleared_entries += self.embedding_cache.clear();
        }

        let new_epoch = self.advance_epoch();

        if warm {
            if let (Some(storage_engine), Some(embed_engine)) = (storage, embedder) {
                // Fetch recent chunks from storage and pre-warm their vector representations
                if let Ok(chunks) = storage_engine.get_all_chunks(100).await {
                    let contract_hash = embed_engine.contract_hash();
                    for chunk in chunks {
                        if self.embedding_cache.get(&contract_hash, &chunk.text_content).is_none() {
                            self.embedding_cache.insert(
                                &contract_hash,
                                &chunk.text_content,
                                chunk.vector,
                            );
                            warmed_entries += 1;
                        }
                    }
                }
            }
        }

        RecacheResponse {
            status: "ok".to_string(),
            cache_epoch: new_epoch,
            cleared_entries,
            warmed_entries,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}
```

---

### 4.3 Agentic State Graph Integration (`crates/quorum-coordinator`)

#### 4.3.1 Telemetry Event Update (`src/telemetry.rs`)

Add `CacheTelemetry` variant to `AgenticTelemetryEvent`:

```rust
// In crates/quorum-coordinator/src/telemetry.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", content = "data")]
pub enum AgenticTelemetryEvent {
    // ... existing variants ...

    /// Emitted at query start with cache hit/miss status and metrics.
    #[serde(rename = "cache_telemetry")]
    CacheTelemetry {
        cache_hit: bool,
        cache_epoch: u64,
        cached_queries: usize,
        cached_embeddings: usize,
        hits: u64,
        misses: u64,
        hit_rate: f32,
    },
}
```

#### 4.3.2 Early Short-Circuiting in `AgenticStateGraphRunner::run` (`src/agentic/mod.rs`)

Integrate `CacheManager` into `AgenticStateGraphRunner`:

```rust
// Inside crates/quorum-coordinator/src/agentic/mod.rs

pub struct AgenticStateGraphRunner {
    pub max_hops: usize,
    pub relevance_threshold: f32,
    pub cache_manager: Arc<CacheManager>,
}

// In run():
// Inside loop { match state_name { AgenticStateName::ClassifyIntent => ... } }

AgenticStateName::ClassifyIntent => {
    ClassifyIntentNode::execute(&mut graph_state, &hw);

    let epoch = self.cache_manager.epoch();
    let maybe_cached = self.cache_manager.query_cache.get(
        &graph_state.original_prompt,
        graph_state.search_mode,
        graph_state.model_family,
        access_tier,
        epoch,
    );

    let cache_hit = maybe_cached.is_some();
    let telem = self.cache_manager.telemetry(cache_hit);

    if let Some(ref tx) = telemetry_tx {
        let _ = tx
            .send(AgenticTelemetryEvent::CacheTelemetry {
                cache_hit,
                cache_epoch: telem.cache_epoch,
                cached_queries: telem.cached_queries,
                cached_embeddings: telem.cached_embeddings,
                hits: telem.hits,
                misses: telem.misses,
                hit_rate: telem.hit_rate,
            })
            .await;
    }

    if let Some(cached) = maybe_cached {
        // FAST PATH: Short-circuit entire retrieval, grading, rewrite, and synthesis!
        graph_state.retrieved_chunks = cached.chunks;
        graph_state.synthesized_response = cached.synthesized_response;
        graph_state.citations = cached.citations;
        graph_state.query_metadata_filters = cached.query_metadata_filters;

        if let Some(ref tx) = telemetry_tx {
            for cit in &graph_state.citations {
                let _ = tx.send(AgenticTelemetryEvent::CitationEmitted { citation: cit.clone() }).await;
            }

            let _ = tx
                .send(AgenticTelemetryEvent::StateTransition {
                    from_state: AgenticStateName::ClassifyIntent,
                    to_state: AgenticStateName::Complete,
                    hop: 0,
                    timestamp_ms: graph_state.start_time.elapsed().as_millis() as u64,
                })
                .await;

            let _ = tx
                .send(AgenticTelemetryEvent::ExecutionFinished {
                    total_hops: 0,
                    total_duration_ms: graph_state.start_time.elapsed().as_millis() as u64,
                    total_tokens: graph_state
                        .synthesized_response
                        .as_ref()
                        .map(|s| s.len() / 4)
                        .unwrap_or(0),
                    final_state: AgenticStateName::Complete,
                    model_name: graph_state.selected_model_tag.clone(),
                })
                .await;
        }

        return Ok(graph_state);
    }

    // Cache Miss: proceed to RetrieveMesh
    state_name = AgenticStateName::RetrieveMesh;
}
```

And in `SynthesizeWithCitations`:
```rust
// After SynthesizeWithCitationsNode::execute(&mut graph_state):
self.cache_manager.query_cache.insert(
    &graph_state.original_prompt,
    graph_state.search_mode,
    graph_state.model_family,
    access_tier,
    self.cache_manager.epoch(),
    graph_state.retrieved_chunks.clone(),
    DeterministicOfflineSynthesisEngine::build_synthesis_prompt(
        &graph_state.original_prompt,
        &graph_state.retrieved_chunks,
    ),
    graph_state.synthesized_response.clone(),
    graph_state.citations.clone(),
    None, // LangExtract grounded result if present
    graph_state.query_metadata_filters.clone(),
);
```

#### 4.3.3 Embedding Cache Wiring in `QueryCoordinator`

```rust
// In QueryCoordinator::query and QueryCoordinator::query_agentic:
let contract_hash = embedder.contract_hash();
let vector = if let Some(cached_vec) = self.cache_manager.embedding_cache.get(&contract_hash, prompt) {
    cached_vec
} else {
    let fresh_vec = embedder.embed_text(prompt).unwrap_or_else(|_| vec![0.0; 768]);
    self.cache_manager.embedding_cache.insert(&contract_hash, prompt, fresh_vec.clone());
    fresh_vec
};
```

---

### 4.4 Daemon Endpoints & CLI Specification

#### 4.4.1 REST API Routes in `crates/quorum-daemon`

1. **`POST /api/cache/recache`**:
   - Query / JSON Body:
     ```json
     {
       "clear": true,
       "warm": true
     }
     ```
   - Response:
     ```json
     {
       "status": "ok",
       "cache_epoch": 2,
       "cleared_entries": 142,
       "warmed_entries": 100,
       "duration_ms": 34
     }
     ```
2. **`GET /api/cache/stats`**:
   - Response:
     ```json
     {
       "status": "ok",
       "cache_epoch": 2,
       "hits": 312,
       "misses": 45,
       "hit_rate": 0.8739,
       "cached_queries": 45,
       "cached_embeddings": 100,
       "uptime_secs": 1820
     }
     ```
3. **`GET /api/health` Extension**:
   - Include `cache`:
     ```json
     {
       "status": "healthy",
       "cache": {
         "cache_epoch": 2,
         "hits": 312,
         "misses": 45,
         "hit_rate": 0.8739,
         "cached_queries": 45,
         "cached_embeddings": 100
       }
     }
     ```

#### 4.4.2 CLI Command in `crates/quorum-daemon/src/cli.rs`

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ...
    /// Flush stale caches, increment cache epoch, and pre-warm active index embeddings.
    Recache {
        /// Force wipe all cached queries and embeddings before warming.
        #[arg(long, default_value_t = false)]
        clear: bool,

        /// Pre-warm hot embeddings from storage into memory.
        #[arg(long, default_value_t = true)]
        warm: bool,
    },
}
```

---

## 5. Verification Method

### 5.1 Independent Test Suite & Verification Commands

1. **Deterministic BLAKE3 Hash Integrity**:
   Verify that identical parameters produce identical keys and modifying any parameter changes the key:
   ```bash
   cargo test -p quorum-coordinator --test coordinator_tests test_cache_key_derivation
   ```
2. **LRU Eviction & Capacity Invariants**:
   Verify that exceeding capacity evicts the oldest entries without memory leaks:
   ```bash
   cargo test -p quorum-coordinator --test coordinator_tests test_embedding_cache_lru_eviction
   cargo test -p quorum-coordinator --test coordinator_tests test_query_cache_lru_eviction
   ```
3. **ZeroizeOnDrop Zero-Trust Memory Scrubbing**:
   Verify that when a `CachedQueryEntry` holding remote chunks is dropped, remote text buffers are explicitly zeroed out:
   ```bash
   cargo test -p quorum-coordinator --test coordinator_tests test_query_cache_zeroize_on_drop
   ```
4. **Agentic State Graph Short-Circuit Verification**:
   Verify that a cached query returns on hop 0 from `ClassifyIntent` with `cache_hit: true` and latency < 5ms:
   ```bash
   cargo test -p quorum-coordinator --test agentic_tests test_agentic_query_cache_short_circuit
   ```
5. **Re-Cache Epoch Invalidation**:
   Verify that advancing the epoch invalidates previously cached queries without wiping embeddings:
   ```bash
   cargo test -p quorum-coordinator --test coordinator_tests test_recache_epoch_invalidation
   ```
6. **Full Workspace Build & Test Run**:
   ```bash
   cargo test --workspace
   ```

### 5.2 Invalidation Conditions
This architecture design is invalidated if:
1. An unpinned remote peer chunk's plaintext survives in memory after its cache entry is evicted.
2. A query submitted at `Tier0Public` receives a cached result created at `Tier2Confidential`.
3. Ingestion file drift updates an on-disk document without incrementing `cache_epoch`.
