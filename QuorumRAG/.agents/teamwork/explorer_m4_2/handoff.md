# Handoff Report: Scatter-Gather Query Coordinator & Volatile Protection Layer (`crates/quorum-coordinator`)

**Author**: `explorer_m4_2`  
**Milestone**: Milestone 4 (rust-libp2p Swarm, Coordinator, Fusion & GLM Selection)  
**Parent Conversation ID**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`  
**Date**: 2026-10-31  
**Status**: Complete Investigation & Implementation Blueprint  

---

## 1. Observation

Direct inspection of the workspace, domain types, existing tests, and storage interfaces reveals the following concrete evidence chains:

### 1.1 Existing E2E Test Suite & Contract Invariants (`tests/e2e/test_federated_mesh.rs`)
Inspection of `d:\harfile\QuorumRAG\tests\e2e\test_federated_mesh.rs` (359 lines) confirms 10 mandatory test scenarios that any coordinator implementation must satisfy:
1. **Ed25519 Signature Signing & Verification** (lines 18–31):
   ```rust
   let (pubkey, sig_bytes) = oracle_sign_query(&seed, query_id, sender_id, timestamp, query_text);
   let verify_result = oracle_verify_query(&pubkey, &sig_bytes, query_id, sender_id, timestamp, query_text, 60);
   assert!(verify_result.is_ok(), "Authentic Ed25519 signature must verify cleanly");
   ```
2. **400ms Scatter-Gather Hard Timeout** (lines 33–105):
   Fast peer (50ms latency, Tier 0) + slow peer (800ms latency, Tier 0):
   ```rust
   let (results, _volatile, duration_ms) = coordinator
       .execute_query("fast content", &vec![1.0; 768], 0, 10)
       .await;
   assert!(duration_ms <= 480, "Scatter-gather query took {}ms, which violated the 400ms hard deadline", duration_ms);
   assert!(!results.iter().any(|r| r.chunk.chunk_id == "c_remote_slow"));
   assert!(ids.contains(&"c_local_fast"));
   assert!(ids.contains(&"c_remote_fast"));
   ```
3. **Access-Tier Pre-Filtering** (lines 107–149):
   Requester has Tier 0. Peer has Tier 0 (`c_pub`) and Tier 2 (`c_conf`).
   ```rust
   assert_eq!(results_public.len(), 1);
   assert_eq!(results_public[0].chunk.chunk_id, "c_pub");
   assert!(!results_public.iter().any(|r| r.chunk.chunk_id == "c_conf"));
   ```
4. **Reciprocal Rank Fusion (RRF)** (lines 151–199):
   Identical chunk returned by local storage (Rank 1) and remote peer (Rank 1):
   ```rust
   let expected_rrf = (1.0 / 61.0) + (1.0 / 61.0); // 2/61 ~ 0.03278
   assert!((fused_score - expected_rrf).abs() < 1e-4);
   ```
5. **Volatile Memory Scrubbing via `ZeroizeOnDrop`** (lines 201–221):
   ```rust
   let mut volatile = dto.to_volatile_remote_chunk("peer_test");
   assert_eq!(volatile.content(), "EXTREMELY_SENSITIVE_TEMPORARY_REMOTE_CONTENT");
   volatile.content.zeroize();
   assert!(volatile.content().is_empty() || volatile.content().chars().all(|c| c == '\0'));
   ```
6. **Anti-Replay Timestamp Rejection** (lines 227–241):
   Stale timestamp (10 minutes old = 600s drift > 60s max drift) rejected:
   ```rust
   let result = oracle_verify_query(&pubkey, &sig_bytes, query_id, sender_id, stale_timestamp, query_text, 60);
   assert!(result.is_err(), "Stale query with timestamp drift > 60s must be rejected");
   ```
7. **Forged Signature Rejection** (lines 243–259):
   Corrupted signature byte (`sig_bytes[0] ^= 0xaa`) rejected:
   ```rust
   assert!(result.is_err(), "Forged signature must fail cryptographic verification");
   ```
8. **0-Peer Graceful Local Fallback** (lines 260–288):
   Coordinator with 0 remote peers returns local hits in `< 50ms`:
   ```rust
   assert_eq!(results.len(), 1);
   assert_eq!(results[0].chunk.chunk_id, "c_local_only");
   assert!(volatile.is_empty());
   assert!(duration_ms < 50, "Zero-peer query must resolve immediately");
   ```
9. **Multi-Peer Consensus Boost** (lines 289–322):
   3 peers return identical chunk at Rank 1:
   ```rust
   let expected = 3.0 / 61.0; // ~ 0.04918
   assert!((results[0].score - expected).abs() < 1e-4);
   ```
10. **Concurrent Queries Under 400ms** (lines 323–358):
    20 concurrent queries dispatched simultaneously via `tokio::spawn` all complete in `< 400ms` without data races or resource contention.

### 1.2 Domain Models & Zero-Leakage Guarantee (`crates/quorum-types/src/lib.rs`)
`quorum-types` already defines all transport-agnostic models:
- `DocumentChunk` (lines 137–226): pure Rust model with `chunk_id`, `file_path`, `line_start`, `line_end`, `blake3_hash`, `access_tier: u8`, `vector: Vec<f32>`, `text_content: String`. Zero Arrow types.
- `ScoredChunk` (lines 228–281): `chunk: DocumentChunk`, `score: f32`, `source_peer: String`, `is_remote: bool`, `.to_citation() -> Citation`.
- `AccessTier` (lines 45–133): `Tier0Public = 0`, `Tier1Internal = 1`, `Tier2Confidential = 2`, `Tier3Restricted = 3`. Monotonically ordered, `.sql_filter_lte()` produces `"access_tier <= N"`.
- `CandidateChunkDto` (lines 305–356): DTO with `into_volatile_remote_chunk(peer_id)` and `to_volatile_remote_chunk(peer_id)`.
- `VolatileRemoteChunk` (lines 557–616): Implements `Zeroize` and `ZeroizeOnDrop`. Plaintext `content: String` is scrubbed on drop.
- `FederatedQueryRequest` (lines 358–542): `query_id: Uuid`, `sender_peer_id: String`, `sender_pubkey: [u8; 32]`, `query_text: String`, `embedding: Vec<f32>`, `top_k: usize`, `timestamp: i64`, `signature: [u8; 64]`. Methods `.canonical_signing_bytes()`, `.canonical_digest()`, `.is_fresh(max_drift_secs)`.
- `FederatedQueryResponse` (lines 544–555): `query_id: Uuid`, `responder_peer_id: String`, `chunks: Vec<CandidateChunkDto>`.
- `QueryResult` (lines 618–632): `chunks: Vec<ScoredChunk>`, `synthesis_prompt: String`, `execution_time_ms: u64`, `sources_cited: Vec<Citation>`.

### 1.3 Storage Engine Contract (`crates/quorum-storage/src/engine.rs`)
- `StorageEngine::open(config: impl Into<StorageConfig>) -> Result<Self, StorageError>`
- `StorageEngine::search_hybrid(&self, text: &str, vector: &[f32], max_tier: u8, top_k: usize) -> Result<Vec<ScoredChunk>, StorageError>`
- `StorageEngine::search_vector(&self, vector: &[f32], max_tier: u8, top_k: usize) -> Result<Vec<ScoredChunk>, StorageError>`
- `StorageEngine::search_text(&self, query: &str, max_tier: u8, top_k: usize) -> Result<Vec<ScoredChunk>, StorageError>`
- `StorageEngine::search(&self, query: &StorageSearchQuery) -> Result<Vec<ScoredChunk>, StorageError>`
- All LanceDB and Arrow types are strictly internal to `crates/quorum-storage` (`pub(crate) mod arrow_interop;`). `quorum-coordinator` does not import `lancedb` or `arrow`.

### 1.4 Peer Exploration Scopes
- `explorer_m4_1`: Focuses on `crates/quorum-network` (rust-libp2p Swarm, QUIC/TCP/Noise/Yamux/mDNS/Kad/Gossip/Request-Response CBOR, `NetworkSwarmHandle`).
- `explorer_m4_2` (Current): Focuses on `crates/quorum-coordinator` retrieval core (Scatter-Gather, 400ms timeout guard, Ed25519 signing/verification, access-tier ACL, RRF with BLAKE3 dedup, `ZeroizeOnDrop` volatile protection).
- `explorer_m4_3`: Focuses on `crates/quorum-coordinator` high-level reasoning (LangGraph-style Agentic State Graph, telemetry streaming, multi-model architecture, and hardware-aware GLM selection).

---

## 2. Logic Chain

From the observations above, we establish the step-by-step architectural design and implementation logic for `crates/quorum-coordinator`:

### 2.1 Architectural Decoupling: Two-Tier Coordinator Design
To provide both production-grade libp2p mesh retrieval and full compatibility with existing E2E opaque-box test suites, the coordinator is layered into two complementary abstractions:
1. **Low-Level Retrieval Core (`ScatterGatherEngine` & `FederatedQueryCoordinator`)**:
   - Manages parallel dispatch to local storage and remote peer queries.
   - Enforces the 400ms hard deadline (`tokio::time::timeout`).
   - Executes Ed25519 signing, timestamp anti-replay verification (< 60s tolerance), and access-tier pre-filtering.
   - Computes Reciprocal Rank Fusion ($k=60.0$) with BLAKE3 deduplication and deterministic tie-breaking.
   - Wraps remote candidate chunks in `VolatileRemoteChunk` (`ZeroizeOnDrop`).
   - Returns `(Vec<ScoredChunk>, Vec<VolatileRemoteChunk>, u64)`.
2. **High-Level Agentic / Synthesis Coordinator (`QueryCoordinator`)**:
   - Accepts prompt strings directly from `quorum-ui` / API.
   - Generates dense Gemma 768d embeddings locally via `EmbeddingGenerator` (`quorum-contract`).
   - Delegates retrieval to `ScatterGatherEngine`.
   - Interacts with the Agentic State Graph (`ClassifyIntent -> RetrieveMesh -> GradeRelevance -> RewriteQuery -> SynthesizeWithCitations`) designed by `explorer_m4_3`.
   - Formats augmented `synthesis_prompt` with numbered source citations and returns `QueryResult`.

### 2.2 Pluggable Mesh Abstraction (`MeshQueryDispatcher`)
To prevent circular dependencies between `quorum-coordinator` and `quorum-network`, and to allow instant unit testing without spinning up physical network interfaces:
```rust
#[async_trait::async_trait]
pub trait MeshQueryDispatcher: Send + Sync {
    async fn query_peers(
        &self,
        request: &FederatedQueryRequest,
        timeout: Duration,
    ) -> Result<Vec<FederatedQueryResponse>, CoordinatorError>;

    fn peer_count(&self) -> usize;
}
```
- **`SwarmMeshDispatcher`**: Implemented when `NetworkSwarmHandle` is available from `crates/quorum-network`, broadcasting or streaming CBOR request/response queries across the libp2p swarm.
- **`MockMeshDispatcher`**: In-memory dispatcher holding `Vec<MockRemotePeer>`, reproducing latency and access tier filtering identical to `tests/e2e/test_federated_mesh.rs`.
- **`LocalOnlyDispatcher`**: Returns empty peer list immediately for single-node / offline execution.

### 2.3 Parallel Dispatch & 400ms Hard Timeout Guard
1. **Clock Initiation**: Record `start = tokio::time::Instant::now()`.
2. **Local Hybrid Search**:
   Spawn/await local search:
   `local_storage.search_hybrid(query_text, query_vec, access_tier, top_k * candidate_multiplier)`
   Local search executes in 5–25ms on local disk/memory.
3. **Concurrent Remote Mesh Dispatch**:
   Construct canonical `FederatedQueryRequest` signed with the coordinator's Ed25519 key.
   Wrap peer querying in `tokio::time::timeout(Duration::from_millis(400), mesh.query_peers(&request, timeout))`.
4. **Graceful Timeout Resolution**:
   - If peers respond within 400ms: all completed peer responses are parsed into candidate chunks.
   - If 400ms elapses: `tokio::time::timeout` expires with `Elapsed`. The coordinator cancels/drops pending remote requests, logs a debug warning, retains any partial responses received prior to deadline, and immediately proceeds.
   - If 0 peers respond or all time out: coordinator gracefully falls back to local results with zero errors returned to the user.
   - Total duration is bounded by $\approx 400$ms ($+ \text{jitter} \le 480$ms).

### 2.4 Cryptographic Wire Protocol & Anti-Replay Freshness (< 60s)
1. **Canonical Payload Formulation**:
   Both payload formulations are supported for 100% interoperability:
   - *Oracle Payload*: `"QUORUM_QUERY_V1\n"` || `query_id` || `sender_peer_id` || `\n` || `timestamp` (LE i64) || `\n` || `query_text`.
   - *Full Request Payload*: `FederatedQueryRequest::canonical_signing_bytes()`.
2. **Anti-Replay Verification**:
   ```rust
   let now = Utc::now().timestamp();
   let drift = (now - timestamp).abs();
   if drift > max_drift_secs { // default: 60s
       return Err(CoordinatorError::StaleTimestamp { server_time: now, query_time: timestamp, drift_secs: drift, limit: max_drift_secs });
   }
   ```
3. **Cryptographic Ed25519 Verification**:
   Signature verified via `verifying_key.verify(&payload_bytes, &signature)`. Any altered payload byte or corrupted signature fails closed immediately.

### 2.5 Access-Tier SQL Pre-Filtering in LanceDB
1. Peer Ed25519 public key (or `PeerId`) is mapped against `PeerAclManager`.
2. Authorized upper bound tier is resolved:
   `let authorized_tier = acl.get_authorized_tier(&peer_pubkey);`
3. Effective tier is computed: `let effective_tier = requested_tier.min(authorized_tier.as_u8());`
4. The local query pushes down SQL predicate: `AccessTier::from_u8(effective_tier).unwrap().sql_filter_lte()` -> `"access_tier <= N"`.
5. LanceDB performs the scalar pre-filter prior to ANN index scan, ensuring unauthorized chunks (such as confidential Tier 2 chunks requested by a Tier 0 peer) are never read or returned.

### 2.6 Reciprocal Rank Fusion (RRF) with BLAKE3 Deduplication & Deterministic Tie-Breaking
1. **Formula**:
   $$RRF(d) = \sum_{m \in M} \frac{1}{60.0 + r_m(d)}$$
   where $r_m(d) \in \{1, 2, \dots\}$ is the 1-based ordinal rank of document chunk $d$ in list $m$.
2. **Deduplication**:
   Local chunks and remote peer candidate chunks are keyed in an aggregation map by `chunk_id` and verified by `blake3_hash`.
   If multiple peers return the same chunk, their rank reciprocal values accumulate, naturally producing a **multi-peer consensus boost** (e.g. 3 peers at Rank 1 yield $3 \times \frac{1}{61} = \frac{3}{61}$).
3. **Deterministic Tie-Breaking**:
   Floating-point sorting without tie-breaking can cause unstable permutations across platforms. We enforce strict lexicographic tie-breaking:
   ```rust
   fused.sort_by(|a, b| {
       b.score
           .partial_cmp(&a.score)
           .unwrap_or(std::cmp::Ordering::Equal)
           .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
           .then_with(|| a.chunk.blake3_hash.cmp(&b.chunk.blake3_hash))
   });
   ```

### 2.7 Volatile Memory Protection (`ZeroizeOnDrop`)
1. All remote candidate chunks received over the wire are immediately ingested into `VolatileRemoteChunk` structs.
2. `VolatileRemoteChunk` implements `Zeroize` and `ZeroizeOnDrop`.
3. When the synthesis prompt is built and the volatile struct goes out of scope, the underlying heap memory buffer containing `content: String` is overwritten with zeroes (`\0`), preventing plaintext leakage into swap memory, core dumps, or lingering heap fragments.
4. Remote chunks are strictly volatile: they are **never** committed or written to local LanceDB or Tantivy storage.

---

## 3. Caveats

The following real-world edge cases and constraints must be taken into account:

1. **Clock Skew & Distributed NTP Drift**:
   - If a peer node's system clock drifts by more than 60 seconds relative to the local node's clock, authentic queries will be rejected with `CoordinatorError::StaleTimestamp`.
   - *Mitigation*: Ensure the daemon logs clear actionable warnings indicating clock drift when this error occurs (`"Peer timestamp drift {}s exceeds 60s limit — check NTP clock synchronization"`).
2. **Zero-Result Peers & Network Dropouts**:
   - A remote peer may return an empty list (`chunks: []`), or drop connection midway.
   - *Mitigation*: The RRF engine treats empty chunk lists as no-ops. If all peers drop, the system seamlessly returns local storage results without panicking or returning an error to the user.
3. **Deterministic Tie-Breaking Stability in Floating Point Comparisons**:
   - In rare cases where two distinct chunks have identical RRF scores (e.g., two distinct chunks each returned at Rank 2 by different peers, both scoring $1/62$), float sorting alone is unstable.
   - *Mitigation*: Secondary and tertiary comparisons against `chunk_id` and `blake3_hash` guarantee total ordering and 100% reproducible test runs.
4. **Volatile Memory Scrubbing Scope**:
   - Wrapping chunks in `VolatileRemoteChunk` scrubs their internal `content: String` buffer upon drop.
   - *Mitigation*: Downstream code (such as prompt formatting) must not clone plaintext into long-lived unmanaged heap structures. The formatted synthesis prompt should itself be held in a zeroizing container or discarded immediately following model inference.
5. **Windows OS Timer Resolution**:
   - Windows default timer resolution is $\approx 15.6$ms. A `tokio::time::timeout` set to 400ms may trigger at 400ms–416ms depending on OS scheduler slice boundaries.
   - *Mitigation*: `test_federated_mesh.rs` verifies `assert!(duration_ms <= 480)`, which accommodates this scheduling jitter. Coordinator implementations must use `tokio::time::Instant` rather than `std::time::Instant` to properly align with Tokio's timer driver.

---

## 4. Conclusion & Actionable File Blueprint

`crates/quorum-coordinator` should be implemented as a clean, modular crate with the following file layout and API signatures:

### 4.1 Crate Manifest: `crates/quorum-coordinator/Cargo.toml`
```toml
[package]
name = "quorum-coordinator"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "Scatter-gather query coordinator, 400ms timeout guard, Ed25519 ACL, RRF fusion, and volatile memory protection"

[dependencies]
# Pure domain types & contracts (ZERO Arrow/Lance leakage)
quorum-types = { workspace = true }
quorum-contract = { workspace = true }
quorum-storage = { workspace = true }

# Concurrency & Async
tokio = { workspace = true }
futures = { workspace = true }
async-trait = "0.1"

# Cryptography & Security
ed25519-dalek = { workspace = true }
blake3 = { workspace = true }
subtle = { workspace = true }
zeroize = { workspace = true }

# Serialization & Identity
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }

# Error Handling & Tracing
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

### 4.2 Module Blueprint

#### File 1: `crates/quorum-coordinator/src/lib.rs`
- Crate-level documentation explaining Arrow isolation, 400ms hard timeout guard, and volatile memory scrubbing.
- Re-exports:
  ```rust
  pub mod acl;
  pub mod config;
  pub mod coordinator;
  pub mod crypto;
  pub mod error;
  pub mod fusion;
  pub mod mesh;
  pub mod scatter_gather;
  pub mod volatile;

  pub use acl::{PeerAclManager, AccessTierPolicy};
  pub use config::CoordinatorConfig;
  pub use coordinator::QueryCoordinator;
  pub use crypto::{canonical_query_bytes, sign_query, verify_query};
  pub use error::CoordinatorError;
  pub use fusion::RrfCombiner;
  pub use mesh::{MeshQueryDispatcher, MockMeshDispatcher, MockRemotePeer};
  pub use scatter_gather::{FederatedQueryCoordinator, ScatterGatherEngine};
  pub use volatile::VolatileRemoteChunk;
  ```

#### File 2: `crates/quorum-coordinator/src/config.rs`
- Defines `CoordinatorConfig`:
  - `timeout_ms: u64` (default: 400)
  - `max_drift_secs: i64` (default: 60)
  - `rrf_k: f32` (default: 60.0)
  - `default_top_k: usize` (default: 10)
  - `candidate_multiplier: usize` (default: 2)
  - `default_peer_access_tier: u8` (default: 0)

#### File 3: `crates/quorum-coordinator/src/error.rs`
- Defines `CoordinatorError` with `thiserror`:
  - `Timeout(u64)`
  - `StaleTimestamp { server_time: i64, query_time: i64, drift_secs: i64, limit: i64 }`
  - `InvalidSignature(String)`
  - `InvalidPublicKey(String)`
  - `Storage(#[from] quorum_storage::StorageError)`
  - `Contract(#[from] quorum_contract::ContractError)`
  - `Network(String)`
  - `AccessDenied { requested: u8, authorized: u8 }`
  - `Internal(String)`

#### File 4: `crates/quorum-coordinator/src/crypto.rs`
- `canonical_query_bytes(query_id: Uuid, sender_peer_id: &str, timestamp: i64, query_text: &str) -> Vec<u8>`:
  Serializes exact format: `"QUORUM_QUERY_V1\n"` || `query_id.as_bytes()` || `sender_peer_id.as_bytes()` || `\n` || `timestamp.to_le_bytes()` || `\n` || `query_text.as_bytes()`.
- `sign_query(secret_seed: &[u8; 32], query_id: Uuid, sender_peer_id: &str, timestamp: i64, query_text: &str) -> (VerifyingKey, [u8; 64])`.
- `verify_query(verifying_key: &VerifyingKey, signature_bytes: &[u8; 64], query_id: Uuid, sender_peer_id: &str, timestamp: i64, query_text: &str, max_drift_secs: i64) -> Result<(), CoordinatorError>`.
- Also functions for `FederatedQueryRequest`: `sign_request` and `verify_request`.

#### File 5: `crates/quorum-coordinator/src/acl.rs`
- `PeerAclManager`: Thread-safe ACL mapping table:
  - `peer_pubkey_tiers: HashMap<[u8; 32], AccessTier>`
  - `peer_id_tiers: HashMap<String, AccessTier>`
  - `default_tier: AccessTier`
  - `pub fn get_authorized_tier(&self, pubkey: &[u8; 32]) -> AccessTier`
  - `pub fn resolve_effective_tier(&self, pubkey: &[u8; 32], requested_tier: u8) -> u8`: computes `requested_tier.min(authorized_tier.as_u8())`.

#### File 6: `crates/quorum-coordinator/src/fusion.rs`
- `RrfCombiner`:
  - `pub fn compute_rrf_score(ranks: &[usize], k: f32) -> f32`: computes $\sum \frac{1}{k + r}$.
  - `pub fn fuse_rankings(local_results: Vec<ScoredChunk>, remote_peer_lists: Vec<Vec<CandidateChunkDto>>, k: f32, top_k: usize) -> Vec<ScoredChunk>`:
    1. Aggregates RRF rank scores into `HashMap<String, (f32, DocumentChunk, String, bool)>` keyed by `chunk_id`.
    2. Adds local chunks ($1/(k + rank)$).
    3. Adds remote chunks per peer list ($1/(k + rank)$).
    4. Deterministic sort:
       `b.score.partial_cmp(&a.score).unwrap_or(Equal).then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id)).then_with(|| a.chunk.blake3_hash.cmp(&b.chunk.blake3_hash))`.
    5. Truncates to `top_k`.

#### File 7: `crates/quorum-coordinator/src/volatile.rs`
- Re-exports `VolatileRemoteChunk` from `quorum-types`.
- Provides utility functions:
  - `pub fn wrap_remote_candidates(peer_id: &str, dtos: Vec<CandidateChunkDto>) -> Vec<VolatileRemoteChunk>`
  - `pub fn scrub_volatile_memory(chunks: &mut [VolatileRemoteChunk])`

#### File 8: `crates/quorum-coordinator/src/mesh.rs`
- `MeshQueryDispatcher` async trait:
  ```rust
  #[async_trait::async_trait]
  pub trait MeshQueryDispatcher: Send + Sync {
      async fn query_peers(
          &self,
          request: &FederatedQueryRequest,
          timeout: Duration,
      ) -> Result<Vec<FederatedQueryResponse>, CoordinatorError>;

      fn peer_count(&self) -> usize;
  }
  ```
- `MockRemotePeer`: Exact replica of `quorum_e2e::MockRemotePeer` with `handle_query(requested_tier)`.
- `MockMeshDispatcher`: Thread-safe mock implementation for unit & integration testing.

#### File 9: `crates/quorum-coordinator/src/scatter_gather.rs`
- `ScatterGatherEngine`:
  - Contains `Arc<dyn StorageReader>`, `Arc<dyn MeshQueryDispatcher>`, `Arc<RwLock<PeerAclManager>>`, `CoordinatorConfig`, `SigningKey`, `sender_peer_id`.
  - Implements `execute_query(query_text: &str, query_vec: &[f32], access_tier: u8, top_k: usize) -> (Vec<ScoredChunk>, Vec<VolatileRemoteChunk>, u64)`:
    1. Launches local storage search.
    2. Concurrently launches remote peer queries bounded by `tokio::time::timeout(Duration::from_millis(config.timeout_ms))`.
    3. Wraps remote candidates in `VolatileRemoteChunk`.
    4. Computes RRF fusion.
    5. Returns fused chunks, volatile remote chunks, and elapsed milliseconds.
- `FederatedQueryCoordinator`: Generic or direct coordinator struct identical to `quorum_e2e::FederatedQueryCoordinator` for 100% compatibility with `tests/e2e/test_federated_mesh.rs`.

#### File 10: `crates/quorum-coordinator/src/coordinator.rs`
- `QueryCoordinator`:
  - Contains `Arc<ScatterGatherEngine>`, `Arc<dyn EmbeddingGenerator>` (`quorum-contract`), `CoordinatorConfig`.
  - `pub async fn query(&self, prompt: &str, top_k: usize) -> Result<QueryResult, CoordinatorError>`:
    1. Generates 768d dense embedding vector from `prompt` using offline Gemma embedding engine.
    2. Calls `scatter_gather.execute_query(prompt, &vector, config.default_peer_access_tier, top_k)`.
    3. Extracts `sources_cited: Vec<Citation>` from scored chunks.
    4. Formats augmented `synthesis_prompt` with numbered context sources.
    5. Returns `QueryResult { chunks, synthesis_prompt, execution_time_ms, sources_cited }`.

---

## 5. Verification Method

To verify the implementation independently, execute the following commands and check the specified assertions:

### 5.1 Verification Commands
```powershell
# 1. Check workspace compilation and profile settings
cargo check -p quorum-coordinator

# 2. Run standalone coordinator unit and integration tests
cargo test -p quorum-coordinator -- --test-threads=2

# 3. Run E2E federated mesh test suite
cargo test --test test_federated_mesh -- --test-threads=2

# 4. Run all E2E tests to verify zero regressions across the entire suite
cargo test --test test_cross_feature_combinations -- --test-threads=2
cargo test --test test_real_world_scenarios -- --test-threads=2

# 5. Verify Zero Arrow/Lance dependency leakage into quorum-coordinator
cargo tree -p quorum-coordinator -i arrow-array
cargo tree -p quorum-coordinator -i lancedb
# Both must report: package `arrow-array` / `lancedb` not found in dependencies of `quorum-coordinator`
```

### 5.2 Key Verification Assertions
1. **Hard 400ms Deadline**: When a remote peer introduces 800ms latency, `duration_ms <= 480` must pass, slow peer chunks must be omitted, and local/fast peer chunks must be retained.
2. **Ed25519 Replay Rejection**: Query timestamp older than 60 seconds must return `Err(CoordinatorError::StaleTimestamp)`.
3. **Forged Signature Rejection**: Corrupted signature byte must return `Err(CoordinatorError::InvalidSignature)`.
4. **Access-Tier Isolation**: Remote peer holding Tier 0 and Tier 2 chunks must only return Tier 0 chunk when queried by a Tier 0 peer.
5. **RRF Score Accumulation**: Identical chunk from local storage (Rank 1) and remote peer (Rank 1) must evaluate to $(1/61 + 1/61) \pm 1e-4 \approx 0.03278$.
6. **Volatile Scrubbing**: `volatile.content.zeroize()` must leave the buffer empty or containing all null bytes (`\0`).
7. **0-Peer Fallback**: Coordinator without remote peers must return local hits in `< 50ms`.

### 5.3 Invalidation Conditions
- Any appearance of `arrow`, `arrow_array`, `arrow_schema`, or `lancedb` in `crates/quorum-coordinator/Cargo.toml` or source imports invalidates the Arrow isolation invariant.
- Scatter-gather query exceeding 480ms when slow peers hang invalidates the 400ms timeout guard invariant.
- Accepting a query with timestamp drift $> 60$s invalidates the anti-replay security invariant.
- Permitting confidential Tier 2 chunks to be returned to a Tier 0 requester invalidates the access-tier SQL pre-filtering invariant.
