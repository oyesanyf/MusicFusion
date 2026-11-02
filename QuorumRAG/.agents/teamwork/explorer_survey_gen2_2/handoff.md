# Architecture & Codebase Exploration Report: Requirement R2 (Low-Latency Query Execution, Consensus, and Peer-to-Peer Security Hardening)

## 1. Observation

Direct code examination and architectural analysis of `d:\harfile\QuorumRAG` revealed the following specific mechanisms, hotspots, and vulnerabilities:

### 1.1 Multi-Hop RAG Query Execution Chains (`crates/quorum-coordinator`)
- **Agentic State Machine**:
  - Implemented in `crates/quorum-coordinator/src/agentic/mod.rs` (lines 88-384) with state cycle:
    `ClassifyIntent -> RetrieveMesh -> GradeRelevance -> (RewriteQuery -> RetrieveMesh)* -> SynthesizeWithCitations`.
  - Termination guarantee: `state.hop_count < state.max_hops` (default 3) in `grade.rs:76` and anti-cycle history check in `rewrite.rs:15-20`.
- **Hotspot 1: Repeated Unbuffered Heap Allocations in Lexical Chunk Scoring**:
  - In `crates/quorum-coordinator/src/agentic/retrieve.rs` (lines 76-177):
    - `score_chunk` closure is evaluated repeatedly: first in `chunks.iter().any(|c| score_chunk(c).0 > 0)` (line 158), then in `chunks.retain(...)` (line 160), and finally inside `chunks.sort_by(...)` (lines 164-176).
    - Inside `score_chunk`, lines 77-78 allocate new lowercase strings for every chunk:
      `let text_l = c.chunk.text_content.to_ascii_lowercase();`
      `let path_l = c.chunk.file_path.to_ascii_lowercase();`
    - In `chunks.sort_by`, for $N=48$ chunks, $O(N \log N) \approx 250$ comparisons occur; each comparison invokes `score_chunk(a)` and `score_chunk(b)`, producing hundreds of heap allocations and repeatedly lowercasing 1KB–10KB strings on every retrieval step.
- **Hotspot 2: Redundant LanceDB Vector Nearest-Neighbor Scans Across Rewrite Hops**:
  - In `crates/quorum-coordinator/src/coordinator.rs` (lines 171-180), the initial query is embedded once (`let vector = ...`).
  - In `agentic/mod.rs` (lines 208-215 and 288-316), when `RewriteQueryNode` appends `" (disambiguation hop N)"` and loops back to `RetrieveMeshNode::execute`, the exact same slice `vector: &[f32]` is passed into `engine.execute_query(&state.current_query, vector, access_tier, candidate_k)` (lines 208-214).
  - In `crates/quorum-coordinator/src/scatter_gather.rs` (lines 109-117), `local_storage.search_hybrid(query_text, query_vec, ...)` is called.
  - In `crates/quorum-storage/src/engine.rs` (lines 305-306), `self.search_vector(vector, ...).await?` executes a full LanceDB IVF_PQ nearest-neighbor scan with the identical vector on every hop, even though the embedding has not changed and only keyword terms were rewritten.
- **Hotspot 3: Token-Level Heap Allocations in `DeterministicOfflineEngine`**:
  - In `crates/quorum-contract/src/lib.rs` (lines 234-248):
    - For every token in the text, `let mut raw = vec![0u8; dim * 4];` (3,072 bytes) is heap-allocated and filled via `xof.fill(&mut raw)`. For a 100-token prompt or document, this triggers 100 separate 3KB heap allocations.
- **Hotspot 4: Spurious Tantivy Reader Reload Disk I/O on Search**:
  - In `crates/quorum-storage/src/tantivy_fts.rs` (lines 312 and 765-776):
    - `search` calls `self.reload_with_retry(SyncCheck::None)?`.
    - Lines 771-775:
      ```rust
      if target_seq > 0 && self.synced_seq.load(Ordering::Acquire) >= target_seq && matches!(check, SyncCheck::None) {
          return Ok(());
      }
      ```
    - When initialized or prior to write transactions in a session, `target_seq == 0`. The condition `target_seq > 0` evaluates to false, causing `self.index.load_metas()` and `self.reader.reload()` to hit the physical drive on every single keyword search query despite zero index mutations.
- **Defect Observation in Coordinator Agentic Synthesis Call**:
  - In `crates/quorum-coordinator/src/agentic/synthesize.rs` (lines 8-14):
    `DeterministicOfflineSynthesisEngine::synthesize_with_history` expects 6 arguments (`telemetry_tx: Option<&Sender<AgenticTelemetryEvent>>` as arg 6), but only 5 are passed:
    `error[E0061]: this function takes 6 arguments but 5 arguments were supplied --> crates\quorum-coordinator\src\agentic\synthesize.rs:8:33`.
    Passing `None` as the 6th argument resolves the signature mismatch.

### 1.2 Peer-to-Peer Mesh Routing & Handshakes (`crates/quorum-network` and `crates/quorum-coordinator`)
- **Transport & Protocols**:
  - `crates/quorum-network/src/transport.rs` (lines 21-98): Dual TCP (Noise + Yamux) and QUIC; mDNS (`enable_mdns`), Kademlia DHT (`set_mode(Some(Server))`), GossipSub (`/quorum-rag/query-broadcast/1.0.0`), Request-Response CBOR (`/quorum-rag/query/1.0.0`), and Identify protocol (`/quorum-rag/1.0.0/contract-<hex>`).
- **Connection Handshake**:
  - `crates/quorum-network/src/actor.rs` (lines 96-140): Handles `identify::Event::Received`. Extracts 64-character contract hash from `protocol_version` or `agent_version` and validates against local hash using constant-time comparison `verify_peer_contract` (`subtle::ConstantTimeEq`). If valid, peer is added to Kademlia and GossipSub; if invalid, peer is immediately disconnected and removed.
- **Point-to-Point Retrieval**:
  - `crates/quorum-coordinator/src/scatter_gather.rs` (lines 131-149): Coordinator constructs `FederatedQueryRequest`, signs it via `sign_request(&self.secret_seed, &mut req, tier)`, and dispatches it through `mesh_dispatcher.query_peers(&req, timeout_dur)`.

### 1.3 P2P Security Vulnerabilities & Inbound Validation Deficiencies
- **Vulnerability 1: Completely Missing Cryptographic Signature & Timestamp Verification in `OsoosiMeshInboundHandler`**:
  - In `crates/quorum-coordinator/src/mesh.rs` (lines 201-246):
    - `OsoosiMeshInboundHandler::handle_query` checks only `p2p_enabled` and `quarantined_peers`.
    - It **never** invokes `crypto::verify_request(&req, ...)` or `crypto::verify_query(...)`!
    - Impact: Any connected node can send unsigned, forged, or altered queries. Furthermore, `req.timestamp` is never checked for freshness, allowing captured requests to be replayed indefinitely.
- **Vulnerability 2: Sender Peer ID & Public Key Spoofing**:
  - In `FederatedQueryRequest` (`crates/quorum-types/src/lib.rs:360-385`), `sender_peer_id: String` and `sender_pubkey: [u8; 32]` are independently supplied by the requester.
  - Neither `NetworkSwarmActor` (`actor.rs:172-183`) nor `OsoosiMeshInboundHandler` validates that `sender_pubkey` derives to `sender_peer_id` (via `libp2p::PeerId::from_public_key`), nor that `sender_peer_id` matches the actual libp2p network connection `PeerId`.
  - Impact: A malicious peer can spoof the identity of an authorized peer.
- **Vulnerability 3: Unbounded Message Size & Vector Parameter Injection**:
  - Neither `actor.rs` nor `mesh.rs` checks:
    1. `req.embedding.len()`: If a peer sends a vector where length != 768 (e.g. 100,000 floats or 0 floats), or containing NaN/Infinity, it is passed directly to `local_storage.search_hybrid`, risking downstream panics or memory bloat in LanceDB.
    2. `req.query_text.len()`: An attacker could send a multi-megabyte string, causing denial-of-service in BM25 parsing and regex DLP inspection.
    3. `req.top_k`: Although clamped to `.min(16)`, `req.top_k == 0` is not rejected early.

### 1.4 Quorum Consensus Scoring & Byzantine Resilience
- **Aggregation Mechanism**:
  - Implemented in `crates/quorum-coordinator/src/fusion.rs` (lines 14-91): `RrfCombiner::fuse_rankings(local_results, remote_peer_lists, k=60.0, top_k)`.
  - Theoretical formula: $RRF(d) = \sum_{p \in Peers} \frac{1}{60 + r_p(d)}$.
  - Consensus boost: If a chunk is present across $\ge 2$ distinct peers, its score is boosted by $1.25\times$ (lines 68-72).
- **Vulnerability 4: Intra-Peer Duplicate Inflation Vulnerability**:
  - In `fusion.rs` (lines 40-62), remote peer rankings iterate over `list.into_iter().enumerate()`:
    ```rust
    for (rank_0, candidate) in list.into_iter().enumerate() {
        let score = 1.0 / (k + (rank_0 + 1) as f32);
        let entry = rrf_map.entry(chunk_id).or_insert_with(...);
        entry.0 += score;
        entry.2.insert(remote_peer_id);
    }
    ```
  - If a single rogue peer sends a response list repeating the exact same `chunk_id` 20 times (e.g., ranks 1..=20), `entry.0` accumulates $\sum_{r=1}^{20} \frac{1}{60+r} \approx 0.285$, which is over $17\times$ higher than an honest peer's rank 1 score ($1/61 \approx 0.01639$).
  - A single Byzantine peer can thus unilaterally force its chunk to the top of the consensus results without needing any other peer's vote.
- **Vulnerability 5: Chunk ID Collision & Desynchronization**:
  - In `fusion.rs` (line 42), `rrf_map` keys on `candidate.chunk_id: String` (an arbitrary identifier assigned by the peer) rather than the cryptographic content hash `blake3_hash`.
  - If two distinct honest nodes index identical content but assign different `chunk_id` strings (e.g. `"chunk_0"` vs `"c_42"`), they are not combined, forfeiting the 1.25x consensus boost and returning duplicate items.
  - If a Byzantine peer injects a chunk with the same `chunk_id` as a legitimate chunk but different content, the map merges them inappropriately.
- **Vulnerability 6: Unchecked Remote Chunk Invariants**:
  - In `fusion.rs` (lines 44-55), `DocumentChunk` is constructed directly from `CandidateChunkDto` without checking whether `candidate.line_start <= candidate.line_end` or `candidate.access_tier <= 3`, bypassing domain validation.

### 1.5 Strictly Offline Operation Verification
- **External Web & Search Endpoints**:
  - Grep searches for `http://`, `https://`, `reqwest`, `hyper`, and `ureq` confirm:
    - Zero external search engines (Google, Bing, DuckDuckGo) are referenced.
    - Zero cloud LLM endpoints (OpenAI, Anthropic, Gemini Cloud, AWS) exist.
    - All embeddings run via local deterministic BLAKE3 XOF (`DeterministicOfflineEngine`) or local safetensors (`GemmaCandleEngine`).
    - Web UI server runs locally on Axum 0.7 (`127.0.0.1` / `0.0.0.0`).
- **Potential Leak / SSRF Vector in Ollama Client**:
  - In `crates/quorum-coordinator/src/offline_engine.rs` (lines 376-387) and `crates/quorum-daemon/src/handlers/settings.rs` (lines 728-735):
    - `let addr: SocketAddr = host_port.to_socket_addrs()...`
    - Does **not** assert `addr.ip().is_loopback()`. If `ui.ollama_url` is misconfigured to a remote host (e.g., `http://example.com:11434`), it would attempt a DNS lookup and outbound TCP connection to the public web.
- **Potential Leak in Network Swarm DNS**:
  - In `crates/quorum-network/src/transport.rs` (line 30), `.with_dns()` is enabled. Swarm dialing should filter out public WAN IPs and public DNS names to preserve air-gapped operation.

### 1.6 Invariant Public Rust APIs, On-Disk Schemas, and Contracts
- **On-disk storage schemas (must remain strictly unchanged)**:
  - LanceDB columnar schema (`arrow_interop.rs:20-40`): 10 columns (`chunk_id: Utf8`, `document_id: Utf8`, `file_path: Utf8`, `line_start: UInt32`, `line_end: UInt32`, `blake3_hash: Utf8`, `access_tier: UInt8`, `vector: FixedSizeList(Float32, 768)`, `text_content: Utf8`, `metadata_json: Utf8`).
  - Tantivy FTS schema (`tantivy_fts.rs:125-138`): 11 fields (`chunk_id`, `document_id`, `file_path`, `file_path_text`, `line_start`, `line_end`, `blake3_hash`, `access_tier`, `text_content`, `metadata_json`, `vector_bytes`).
- **Domain Types & Public Signatures (must remain strictly unchanged)**:
  - `quorum_types`: `DocumentChunk`, `ScoredChunk`, `Citation`, `CandidateChunkDto`, `FederatedQueryRequest`, `FederatedQueryResponse`, `VolatileRemoteChunk`, `AccessTier`, `QueryResult`.
  - `quorum_contract`: `EmbeddingContract`, `EmbeddingGenerator`, `DeterministicOfflineEngine`, `verify_contract_handshake_hex`.
  - `quorum_storage`: `StorageEngine` methods (`open`, `insert_chunks`, `search_vector`, `search_text`, `search_hybrid`, `search`, `has_chunk_hash`, `delete_by_file_path`, `compact`, `count`, `index_epoch`, `bump_index_epoch`, `set_index_epoch`).
  - `quorum_coordinator`: `QueryCoordinator` methods (`query`, `query_agentic`, `query_agentic_with_history`, cache controls), `ScatterGatherEngine::execute_query`, `RrfCombiner::fuse_rankings`, `crypto::{sign_query, verify_query, sign_request, verify_request}`.
  - `quorum_network`: `InboundQueryHandler`, `start_network_node`, `NetworkSwarmHandle`, `NetworkEvent`.

---

## 2. Logic Chain

1. **Premise 1 (CPU & Memory Overhead in Multi-Hop Execution)**:
   - Direct observation shows `score_chunk` in `retrieve.rs:76-177` allocates new lowercase `String` instances for text and path on every invocation, and is called inside an $O(N \log N)$ comparator.
   - For 48 candidate chunks across multiple rewrite hops, this results in thousands of heap allocations and redundant text scans per query.
   - Pre-computing the scoring criteria into a static struct prior to sorting reduces string allocations from $O(N \log N)$ to $O(N)$ and eliminates redundant memory churn without modifying ranking semantics.
   - Similarly, in `DeterministicOfflineEngine::generate_vector`, replacing the per-token 3KB vector allocation (`vec![0u8; dim * 4]`) with a reused stack/slice buffer eliminates token-level allocation overhead.

2. **Premise 2 (I/O Overhead in Hybrid Retrieval)**:
   - Observation in `tantivy_fts.rs:771-775` indicates that when `target_seq == 0`, `reload_with_retry` fails the fast path check and forces a synchronous disk metadata read (`index.load_metas()`) and reader reload on every keyword search query.
   - Tracking `synced_seq` accurately from startup avoids spurious disk I/O on clean reads.
   - In `scatter_gather.rs` and `agentic/mod.rs`, during rewrite hops where the query embedding vector is unmodified, re-executing LanceDB vector search is redundant. Reusing local vector search results across rewrite hops eliminates duplicate vector I/O and nearest-neighbor calculations.

3. **Premise 3 (Inbound P2P Query Security & Spoofing)**:
   - Observation in `mesh.rs:201-246` demonstrates that `OsoosiMeshInboundHandler` does not invoke `verify_request` or check `req.timestamp`.
   - Any remote node can transmit an unsigned query, claim any `sender_peer_id`, or replay captured queries.
   - Enforcing `crypto::verify_request(&req, access_tier, max_drift_secs=60)` inside `OsoosiMeshInboundHandler` mathematically guarantees authenticity and freshness (<60s) before executing any search.
   - Furthermore, verifying that `PeerId::from_public_key(&req.sender_pubkey)` matches `req.sender_peer_id` closes the spoofing vulnerability.

4. **Premise 4 (Byzantine Consensus & Score Manipulation in RRF)**:
   - Observation in `fusion.rs:40-62` shows that multiple appearances of a chunk in a single peer's list repeatedly add $\frac{1}{60 + r}$ to `entry.0`.
   - A single rogue peer can list a chunk 20 times to achieve a score of $\approx 0.285$, overriding legitimate consensus.
   - Enforcing **per-peer uniqueness** (each peer list contributes at most once to any unique chunk score, taking the highest/first rank) mathematically bounds each peer's influence to at most $\frac{1}{61} \approx 0.01639$.
   - Switching the deduplication key from arbitrary `candidate.chunk_id` to cryptographic `candidate.content` BLAKE3 hash ensures that identical chunks across peers are correctly fused and receive the 1.25x consensus boost.

5. **Premise 5 (Strictly Offline Air-Gapped Operation)**:
   - Observation in `offline_engine.rs:383-387` and `transport.rs:30` shows that while no public search APIs are configured, `to_socket_addrs()` and `.with_dns()` could attempt public resolution if given non-loopback addresses.
   - Adding a fail-closed check (`addr.ip().is_loopback()`) in `offline_engine.rs` guarantees zero external WAN transmission.
   - Validating that dialed swarm multiaddrs resolve only to loopback or private LAN RFC1918 subnets guarantees zero external DNS or WAN traffic.

---

## 3. Caveats

1. **Ollama Integration Scope**: The local Ollama daemon (`127.0.0.1:11434`) is an external local process, not an embedded library. The codebase already includes fallback to `DeterministicOfflineSynthesisEngine` when Ollama is offline or not installed.
2. **Search Recall Baseline**: Lexical and vector ranking semantics must remain mathematically identical:
   - Local LanceDB distance metric remains Cosine similarity.
   - Tantivy BM25 parameters remain untouched.
   - Standard RRF formula ($k=60.0$) remains identical; Byzantine hardening only prevents rogue intra-peer duplicate score stuffing.
3. **Hardware Telemetry**: DirectML / CUDA telemetry in `hardware.rs` uses in-process system queries (`sysinfo`); no external network requests are made.

---

## 4. Conclusion

The technical architecture and code paths for Requirement R2 have been thoroughly explored and verified:

1. **Low-Latency & Overhead Reductions**:
   - Optimize `RetrieveMeshNode::execute` lexical scoring to compute chunk scores once upfront instead of re-allocating lowercase strings during $O(N \log N)$ sort comparisons.
   - Reuse pre-allocated stack buffers in `DeterministicOfflineEngine::generate_vector`.
   - Skip redundant LanceDB vector searches during agentic query rewrite hops.
   - Fix `tantivy_fts.rs::reload_with_retry` fast-path check so `target_seq == 0` does not trigger synchronous disk metadata reads.

2. **Peer-to-Peer Security & Byzantine Resilience Hardening**:
   - Wire `crypto::verify_request` and anti-replay timestamp validation (<60s) into `OsoosiMeshInboundHandler::handle_query`.
   - Validate that `req.sender_pubkey` matches `req.sender_peer_id` and bounds checks on `req.embedding` (exactly 768 finite floats) and `req.query_text` (<4096 chars).
   - In `RrfCombiner::fuse_rankings`, enforce per-peer deduplication (at most one vote per peer per chunk) to eliminate Byzantine score stuffing, and key aggregation on `blake3_hash`.
   - Validate remote `CandidateChunkDto` line ranges and access tiers fail-closed.

3. **Strictly Offline Enforcement**:
   - Assert loopback (`127.0.0.1` / `::1`) on `query_local_ollama_chat` target socket.
   - Restrict libp2p dial addresses to loopback and private LAN addresses.

4. **Interface & Schema Preservation**:
   - All public Rust APIs, Arrow/LanceDB schemas, Tantivy schemas, and cryptographic contracts remain strictly unchanged.

---

## 5. Verification Method

### 5.1 Local Test Commands
Execute the automated test suite across affected crates and integration suites:

```powershell
# 1. Quorum Coordinator Unit & Adversarial Tests
cargo test -p quorum-coordinator --test agentic_tests
cargo test -p quorum-coordinator --test coordinator_tests
cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests
cargo test -p quorum-coordinator --test adversarial_cache_challenge_tests

# 2. Quorum Network Tests (Swarm, Handshake, Adversarial)
cargo test -p quorum-network --test network_tests
cargo test -p quorum-network --test adversarial_network_tests

# 3. Quorum Storage & Contract Tests
cargo test -p quorum-storage
cargo test -p quorum-contract

# 4. Comprehensive E2E Test Suite
cargo test --manifest-path tests/e2e/Cargo.toml
```

### 5.2 Key Inspection Points & Files
- `crates/quorum-coordinator/src/agentic/retrieve.rs`: Inspect `score_chunk` allocation pattern.
- `crates/quorum-coordinator/src/mesh.rs`: Inspect `OsoosiMeshInboundHandler::handle_query` for cryptographic verification.
- `crates/quorum-coordinator/src/fusion.rs`: Inspect `RrfCombiner::fuse_rankings` for intra-peer duplicate score accumulation.
- `crates/quorum-coordinator/src/offline_engine.rs`: Inspect `query_local_ollama_chat` socket address validation.
- `crates/quorum-storage/src/tantivy_fts.rs`: Inspect `reload_with_retry` sequence check.

### 5.3 Invalidation Conditions
- Any change that alters the 10-column LanceDB schema or Tantivy FTS schema.
- Any change that breaks the public `QueryCoordinator::query` or `query_agentic` signatures.
- Any network request made to an IP address outside loopback (`127.0.0.0/8`, `::1`) or private LAN (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`).
- Any failure in the existing E2E test suite (`tests/e2e`).
