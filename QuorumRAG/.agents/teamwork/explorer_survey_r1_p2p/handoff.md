# Technical Investigation Report: Requirement R1 & Configurable max_peer_responses

**Agent**: `explorer_survey_r1_p2p`
**Date**: 2026-11-03
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_r1_p2p\`
**Mission**: Comprehensive survey and architectural roadmap for Requirement R1 (Fix 4 Cross-Peer Gaps in P2P Mesh) and the Follow-up requirement (Configurable `max_peer_responses`).

---

## 1. Observation

Direct observations from examining the QuorumRAG codebase:

### 1.1 Persistent Ed25519 Node Identity
- **`crates/quorum-network/src/config.rs:7`**:
  ```rust
  pub struct NetworkConfig {
      /// Ed25519 seed for persistent identity (if None, random generated)
      pub ed25519_seed: Option<[u8; 32]>,
      ...
  }
  ```
  `NetworkConfig` supports an optional 32-byte seed. Default is `ed25519_seed: None`.
- **`crates/quorum-network/src/lib.rs:54-64`**:
  ```rust
  let keypair = match config.ed25519_seed {
      Some(seed) => {
          let secret = libp2p::identity::ed25519::SecretKey::try_from_bytes(seed)
              .map_err(|e| NetworkError::KeypairError(e.to_string()))?;
          let ed_kp = libp2p::identity::ed25519::Keypair::from(secret);
          libp2p::identity::Keypair::from(ed_kp)
      }
      None => libp2p::identity::Keypair::generate_ed25519(),
  };
  let local_peer_id = keypair.public().to_peer_id();
  ```
- **`crates/quorum-daemon/src/main.rs:180-206`**:
  ```rust
  let net_config = quorum_network::NetworkConfig {
      tcp_listen_addrs: vec![format!("/ip4/{}/tcp/{}", cli.host, chosen_p2p_port).parse()?],
      quic_listen_addrs: vec![format!("/ip4/{}/udp/{}/quic-v1", cli.host, chosen_p2p_port).parse()?],
      ..Default::default() // <-- ed25519_seed is None! Generates random keypair on every launch
  };
  ...
  let coordinator_engine = Arc::new(quorum_coordinator::ScatterGatherEngine::new(
      Arc::clone(&storage) as Arc<dyn quorum_coordinator::StorageSearcher>,
      mesh_dispatcher,
      Arc::new(tokio::sync::RwLock::new(quorum_coordinator::PeerAclManager::default())),
      quorum_coordinator::CoordinatorConfig::default(),
      [0u8; 32],                    // <-- Hardcoded dummy seed [0u8; 32]!
      "local_daemon".to_string(),   // <-- Hardcoded dummy peer ID "local_daemon"!
  ));
  ```
- **`crates/quorum-coordinator/src/mesh.rs:170-175, 220-235`**:
  In `OsoosiMeshInboundHandler::handle_query`:
  ```rust
  // 2. Validate sender_pubkey derives to claimed sender_peer_id
  match quorum_network::derive_peer_id_from_pubkey(&req.sender_pubkey) {
      Ok(derived_id) => {
          if derived_id.to_string() != req.sender_peer_id {
              tracing::warn!(
                  "Rejecting spoofed sender peer id: claimed={}, derived={}",
                  req.sender_peer_id,
                  derived_id
              );
              return FederatedQueryResponse {
                  query_id: req.query_id,
                  responder_peer_id: self.local_peer_id.clone(),
                  chunks: Vec::new(),
              };
          }
      }
      Err(e) => { ... }
  }
  ```
  `ScatterGatherEngine` signs outbound queries with secret seed `[0u8; 32]`, putting the public key for `[0u8; 32]` into `req.sender_pubkey`. But it sets `req.sender_peer_id = "local_daemon"`.
  When a remote peer receives the query, `derive_peer_id_from_pubkey(&req.sender_pubkey)` yields a valid libp2p `PeerId` (e.g. `12D3KooW...`), which never equals `"local_daemon"`. Every outbound query from a live node is immediately rejected as spoofed!
  Furthermore, `libp2p` generated a random keypair on startup, so the node's network identity changes on every restart.

### 1.2 Federated Entity & Cue-Term Supplements
- **`crates/quorum-coordinator/src/agentic/retrieve.rs:231-290`**:
  ```rust
  if (daemon_active || !state.extracted_entities.is_empty()) && !prelim_entities.is_empty() {
      let mut seen_ids: std::collections::HashSet<String> =
          chunks.iter().map(|c| c.chunk.chunk_id.clone()).collect();

      let entity_kw_query = prelim_entities.join(" ");
      let entity_hits = engine
          .local_storage
          .search_text(&entity_kw_query, access_tier, 96)
          .await;
      for hit in entity_hits {
          if seen_ids.insert(hit.chunk.chunk_id.clone()) {
              chunks.push(hit);
          }
      }

      if !prompt_only_entities.is_empty() && prompt_only_entities != prelim_entities {
          let prompt_kw_query = prompt_only_entities.join(" ");
          let prompt_hits = engine
              .local_storage
              .search_text(&prompt_kw_query, access_tier, 64)
              .await;
          ...
      }

      let cue_terms = crate::offline_engine::DeterministicOfflineSynthesisEngine::answer_type_cue_terms(&state.original_prompt);
      if !cue_terms.is_empty() && !prompt_only_entities.is_empty() {
          let cue_query = format!("{} {}", prompt_only_entities.join(" "), cue_terms.join(" "));
          let cue_hits = engine.local_storage.search_text(&cue_query, access_tier, 48).await;
          ...
      }

      if q_low.contains("femi") || q_low.contains("oyesanya") {
          let resume_hits = engine
              .local_storage
              .search_text("Femi Oyesanya CISSP PROFESSIONAL SUMMARY Resume CV", access_tier, 64)
              .await;
          ...
      }
  }
  ```
  All 4 supplemental text searches (`entity_kw_query`, `prompt_kw_query`, `cue_query`, and resume keywords) invoke `engine.local_storage.search_text(...)` directly.
  They do NOT query remote mesh peers via `mesh_dispatcher`. If an entity document or cue-term excerpt resides on a remote peer, it is missed if the initial hybrid search candidate pool did not contain it.

### 1.3 Engine-Mode Contract Differentiation
- **`crates/quorum-contract/src/lib.rs:55-118, 190-205, 303-317`**:
  ```rust
  pub struct EmbeddingContract {
      pub contract_version: u32,
      pub model_name: String,
      pub dimension: usize,
      pub normalization: String,
      pub distance_metric: String,
      pub pooling_strategy: String,
      pub max_sequence_length: usize,
  }

  impl EmbeddingContract {
      pub fn canonical_gemma_300m() -> Self {
          Self {
              contract_version: 1,
              model_name: "google/embeddinggemma-300m".to_string(),
              dimension: 768,
              normalization: "L2".to_string(),
              distance_metric: "Cosine".to_string(),
              pooling_strategy: "MeanPooling".to_string(),
              max_sequence_length: 512,
          }
      }
      pub fn derive_contract_hash(&self) -> [u8; 32] { ... }
  }
  ```
  `DeterministicOfflineEngine::new()` uses `EmbeddingContract::canonical_gemma_300m()`.
  `GemmaCandleEngine::new()` uses `EmbeddingContract::canonical_gemma_300m()`.
  `EmbeddingContract` has NO field indicating engine mode or implementation type.
  Both engines produce the EXACT same 32-byte BLAKE3 contract hash (`derive_contract_hash()`), even though `DeterministicOfflineEngine` derives pseudo-vectors from token BLAKE3 XOF hashes while `GemmaCandleEngine` runs neural safetensors weights.
  Cross-peer vector comparisons between these two engines are mathematically meaningless, yet `verify_peer_contract` (`crates/quorum-network/src/handshake.rs:34`) succeeds and accepts the connection because the hashes are identical!

### 1.4 Peer and Filename Disambiguation
- **`crates/quorum-coordinator/src/scatter_gather.rs:262-303`**:
  ```rust
  let mut volatile_chunks = Vec::new();
  let mut remote_lists = Vec::new();
  for resp in remote_responses {
      let mut clean_peer_chunks = Vec::new();
      for mut chunk in resp.chunks {
          ...
          clean_peer_chunks.push(chunk);
      }
      if !clean_peer_chunks.is_empty() {
          remote_lists.push(clean_peer_chunks); // resp.responder_peer_id is discarded!
      }
  }
  let fused = RrfCombiner::fuse_rankings(
      local_results,
      remote_lists,
      self.config.rrf_k,
      top_k,
  );
  ```
- **`crates/quorum-coordinator/src/fusion.rs:74`**:
  ```rust
  let scored = ScoredChunk::new(doc_chunk, candidate.score, "remote_peer", true);
  ```
  `RrfCombiner::fuse_rankings` hardcodes `"remote_peer"` as `source_peer` on all remote chunks.
- **`crates/quorum-coordinator/src/offline_engine.rs:1413-1422`**:
  ```rust
  let mut seen_filenames = std::collections::HashSet::new();
  let mut out: Vec<(&'a ScoredChunk, u32)> = Vec::new();
  for &c in matched_chunks {
      let file_name = std::path::Path::new(&c.chunk.file_path)
          .file_name()
          .and_then(|n| n.to_str())
          .unwrap_or(&c.chunk.file_path);
      if !seen_filenames.insert(file_name.to_ascii_lowercase()) {
          continue;
      }
      ...
  ```
  `deduplicated_chunks_with_scores` deduplicates strictly by `file_name.to_ascii_lowercase()`.
  If Peer A and Peer B (or local and remote) both share files named `README.md` or `contract.pdf`, the second peer's document is dropped silently.
- **`crates/quorum-ui/assets/app.js:880-884`**:
  ```javascript
  if (!cleanFileName || citationsEl._seenFiles.has(cleanFileName.toLowerCase())) {
    return;
  }
  citationsEl._seenFiles.add(cleanFileName.toLowerCase());
  ```
  Web UI tracks deduplication using `cleanFileName.toLowerCase()` only, discarding remote peer citations with identical filenames.

### 1.5 Configurable `max_peer_responses`
- **`crates/quorum-coordinator/src/config.rs:1-24`**:
  `CoordinatorConfig` lacks `max_peer_responses` (has `timeout_ms`, `max_drift_secs`, `rrf_k`, `default_top_k`, `candidate_multiplier`, `default_peer_access_tier`, `exclude_source_code`).
- **`crates/quorum-coordinator/src/dlp_mesh.rs:142-184`**:
  `OsoosiMeshSettings` lacks `max_peer_responses`.
- **`crates/quorum-daemon/src/cli.rs:15-45`**:
  `Cli` lacks `--max-peer-responses`.
- **`crates/quorum-coordinator/src/mesh.rs:117-155`**:
  `SwarmMeshDispatcher::query_peers` loops through all connected peers without an early-stopping cap.
- **`crates/quorum-coordinator/src/scatter_gather.rs:263-295`**:
  `ScatterGatherEngine::execute_query_with_cached_vector` accepts responses from all peers without bounding the number of peer candidate lists fused.
- **`crates/quorum-ui/assets/app.js:3083-3117`**:
  The Mesh Settings card has inputs for Node Alias and Bootstrap Peers, but has no input for `max_peer_responses`.

---

## 2. Logic Chain

### 2.1 Persistent Ed25519 Node Identity
1. From Observation 1.1, `libp2p` in `start_network_node` creates a new random Ed25519 keypair on every daemon startup because `net_config.ed25519_seed` is `None`.
2. From Observation 1.1, `ScatterGatherEngine::new` is initialized in `crates/quorum-daemon/src/main.rs:199-206` with seed `[0u8; 32]` and sender peer ID `"local_daemon"`.
3. When `ScatterGatherEngine` broadcasts an outbound `FederatedQueryRequest`, it signs using `[0u8; 32]`, deriving the verifying key for `[0u8; 32]` and setting `req.sender_pubkey`. It sets `req.sender_peer_id = "local_daemon"`.
4. When a receiving node runs `OsoosiMeshInboundHandler::handle_query` (`crates/quorum-coordinator/src/mesh.rs:221-228`), it derives the expected libp2p `PeerId` from `req.sender_pubkey` via `derive_peer_id_from_pubkey`. The derived `PeerId` (e.g., `12D3KooW...`) does NOT equal `"local_daemon"`.
5. Therefore, the receiving node rejects the query as a spoofed peer ID and drops it.
6. **Solution**:
   - In `<data_dir>/node_identity.seed` (32 bytes), check if the seed file exists on startup.
   - If present, read the 32 bytes. If absent, generate 32 secure random bytes (via `getrandom` or OS RNG) and atomically write to `<data_dir>/node_identity.seed`.
   - Use this persistent seed for BOTH:
     a) `net_config.ed25519_seed = Some(seed)` so `libp2p` generates the swarm identity from it.
     b) Derive `persistent_peer_id = keypair.public().to_peer_id().to_string()`.
     c) Pass `seed` and `persistent_peer_id` into `ScatterGatherEngine::new(..., seed, persistent_peer_id)`.
     d) Pass `persistent_peer_id` into `OsoosiMeshInboundHandler::new(storage, persistent_peer_id)`.
   - Now `req.sender_peer_id` matches `derive_peer_id_from_pubkey(&req.sender_pubkey)` identically, and the identity survives daemon restarts without breaking ACLs or spoofing checks.

### 2.2 Federated Entity & Cue-Term Supplements
1. From Observation 1.2, `RetrieveMeshNode::execute` detects proper nouns, extracted entities, and answer cue-terms and executes up to 4 supplemental searches.
2. Currently, all 4 calls invoke `engine.local_storage.search_text(...)` directly, bypassing the P2P mesh entirely.
3. If relevant entity or cue-term evidence resides on remote peers (such as shared lease agreements, resumes, or documents on other machines), they will never be found by the supplemental searches.
4. **Solution**:
   - Add `pub async fn search_federated_text(&self, query_text: &str, access_tier: u8, top_k: usize) -> Vec<ScoredChunk>` to `ScatterGatherEngine`.
   - `search_federated_text` performs:
     a) Local text search via `self.local_storage.search_text(query_text, access_tier, top_k).await`.
     b) Checks if P2P is enabled in `global_mesh_dlp_controller()`.
     c) Generates a 768-dim embedding using `DeterministicOfflineEngine` for wire compatibility.
     d) Sanitizes the outbound query through the outbound DLP gate.
     e) Constructs a signed `FederatedQueryRequest` and dispatches it via `self.mesh_dispatcher.query_peers(&req, timeout)`.
     f) Binds accepted peer responses to `max_peer_responses`.
     g) Enforces inbound DLP, wraps accepted candidate chunks into `ScoredChunk` preserving `resp.responder_peer_id`.
     h) Combines local and remote chunks and returns them.
   - In `RetrieveMeshNode::execute`, replace the 4 local search calls with `engine.search_federated_text(...)`.

### 2.3 Engine-Mode Contract Differentiation
1. From Observation 1.3, `EmbeddingContract::canonical_gemma_300m()` defines the contract for both `DeterministicOfflineEngine` and `GemmaCandleEngine`.
2. `derive_contract_hash` hashes `contract_version`, `model_name`, `dimension`, `normalization`, `distance_metric`, `pooling_strategy`, and `max_sequence_length`.
3. It does NOT hash the engine implementation mode. Thus, a node running BLAKE3 XOF vectors and a node running neural Candle vectors generate identical contract hashes.
4. During peer connection, `identify` exchanges `contract_hash_hex` (`crates/quorum-network/src/transport.rs:18-19, 81-85`).
5. Because the hashes match, `verify_peer_contract` succeeds, allowing incompatible nodes to form a cluster and compare mathematically incomparable vectors.
6. **Solution**:
   - Add `pub engine_mode: String` to `EmbeddingContract`.
   - Provide two canonical constructors:
     a) `canonical_deterministic_offline()` -> `model_name: "google/embeddinggemma-300m"`, `engine_mode: "DeterministicOffline"`.
     b) `canonical_gemma_300m()` -> `model_name: "google/embeddinggemma-300m"`, `engine_mode: "GemmaCandle"`.
   - Include `self.engine_mode` in `derive_contract_hash()`:
     ```rust
     hasher.update(self.engine_mode.as_bytes());
     hasher.update(b"\n");
     ```
   - In `DeterministicOfflineEngine::new()`: set `contract = EmbeddingContract::canonical_deterministic_offline()`.
   - In `GemmaCandleEngine::new()`: set `contract = EmbeddingContract::canonical_gemma_300m()`.
   - In `quorum-daemon`: initialize `net_config.contract_hash_hex` from the active embedder's contract hash (`embedder.contract_hash()`).
   - If a deterministic node encounters a neural node, the contract hashes will differ, `verify_peer_contract` will fail closed with `ContractMismatch`, and `actor.rs` will disconnect the incompatible peer immediately.

### 2.4 Peer and Filename Disambiguation
1. From Observation 1.4, `ScatterGatherEngine` aggregates peer responses into `remote_lists: Vec<Vec<CandidateChunkDto>>`, dropping `resp.responder_peer_id`.
2. `RrfCombiner::fuse_rankings` hardcodes `"remote_peer"` as `source_peer` for all remote chunks.
3. `offline_engine.rs` deduplicates strictly by filename string in `seen_filenames: HashSet<String>`.
4. `app.js` deduplicates strictly by filename string in `citationsEl._seenFiles`.
5. This leads to citation collisions and silent drops when two different peers provide files with identical names (e.g. `contract.pdf`, `report.docx`, `README.md`).
6. **Solution**:
   - In `crates/quorum-coordinator/src/fusion.rs`:
     Add `pub fn fuse_rankings_with_peers(local_results: Vec<ScoredChunk>, remote_peer_lists: Vec<(String, Vec<CandidateChunkDto>)>, k: f32, top_k: usize) -> Vec<ScoredChunk>`.
     For each `(peer_id, list)`, assign `source_peer: peer_id` on the generated `ScoredChunk`.
     Keep `fuse_rankings(local, remote_peer_lists, k, top_k)` as a delegating wrapper for backward compatibility with existing tests.
   - In `crates/quorum-coordinator/src/scatter_gather.rs`:
     Collect `remote_lists` as `Vec<(String, Vec<CandidateChunkDto>)>` paired with `resp.responder_peer_id`, and call `RrfCombiner::fuse_rankings_with_peers`.
   - In `crates/quorum-coordinator/src/offline_engine.rs`:
     In `deduplicated_chunks_with_scores`, track seen items using `(c.source_peer.to_ascii_lowercase(), file_name.to_ascii_lowercase())`.
     In `format_sources_block`, if `c.source_peer != "local"` and not empty, include peer attribution (e.g. `[📄 file_name (Peer: peer_id)]`).
   - In `crates/quorum-ui/assets/app.js`:
     In `appendCitation`, track `_seenFiles` using `(cit.peer_id || 'local').toLowerCase() + '::' + cleanFileName.toLowerCase()`.
     Display the peer indicator on the badge and in the inspector card.

### 2.5 Configurable `max_peer_responses`
1. From Observation 1.5, there is currently no configurable limit bounding the number of peer responses accepted or fused per query.
2. **Solution**:
   - `CoordinatorConfig`: add `pub max_peer_responses: usize` (default: `8`).
   - `OsoosiMeshSettings`: add `#[serde(default = "default_8")] pub max_peer_responses: usize` (default: `8`), persisted in `settings.json`.
   - `Cli`: add `--max-peer-responses <USIZE>` (default: `8`).
   - Wire in `main.rs`: initialize `settings.mesh.max_peer_responses = cli.max_peer_responses`.
   - Update `UpdateMeshConfigPayload` and `update_mesh_config_handler` in `settings.rs` to allow updating `max_peer_responses` via `POST /api/mesh/config`.
   - In `SwarmMeshDispatcher::query_peers` (and `MockMeshDispatcher::query_peers`): break out of the response collection loop as soon as `responses.len() >= max_peer_responses`.
   - In `ScatterGatherEngine::execute_query_with_cached_vector`: cap accepted `remote_lists` to `max_peer_responses`.
   - In `crates/quorum-ui/assets/app.js`: render `<input type="number" min="1" max="64" ...>` in the Mesh Settings tab wired to `window.quorumUpdateMeshConfig({ max_peer_responses: ... })`.

---

## 3. Caveats

1. **Air-Gapped / Local-Only Mode**: When running with `--local-only`, the node identity seed must still be persisted in `<data_dir>/node_identity.seed`. The persistent `PeerId` remains valid for local cryptographic commitments and audit logs.
2. **Backward Compatibility in Tests**: Over 30 existing test files instantiate `RrfCombiner::fuse_rankings(local, vec![...], 60.0, 10)` or `EmbeddingContract::canonical_gemma_300m()`. To avoid test regressions, `fuse_rankings` must remain as a convenience function delegating to `fuse_rankings_with_peers`, and `canonical_gemma_300m()` must remain valid alongside `canonical_deterministic_offline()`.
3. **No Mocks Rule**: All implementations must use actual persistent disk files, real Ed25519 key derivation, real constant-time verification, and genuine HTTP/JSON endpoints. No mock identifiers or placeholder tokens.

---

## 4. Conclusion & Concrete Architectural Specifications

Below are the exact code designs and target locations for implementation agents.

### 4.1 Node Identity Key Persistence (`crates/quorum-network` and `crates/quorum-daemon`)

#### File: `crates/quorum-network/src/lib.rs`
Add helper function to load or generate a persistent 32-byte Ed25519 seed:
```rust
/// Loads a 32-byte Ed25519 seed from `<data_dir>/node_identity.seed`, or creates it if not present.
pub fn load_or_generate_node_seed(data_dir: &std::path::Path) -> Result<[u8; 32], std::io::Error> {
    let seed_path = data_dir.join("node_identity.seed");
    if seed_path.is_file() {
        let bytes = std::fs::read(&seed_path)?;
        if bytes.len() == 32 {
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&bytes);
            return Ok(seed);
        }
    }
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    if let Some(parent) = seed_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&seed_path, seed)?;
    Ok(seed)
}
```

#### File: `crates/quorum-daemon/src/main.rs`
In `run_daemon_and_ui`:
```rust
    // Load or generate persistent 32-byte Ed25519 node seed
    let node_seed = quorum_network::load_or_generate_node_seed(&cli.data_dir)?;
    let secret = quorum_network::libp2p::identity::ed25519::SecretKey::try_from_bytes(node_seed)
        .map_err(|e| anyhow::anyhow!("Invalid node seed: {e}"))?;
    let ed_kp = quorum_network::libp2p::identity::ed25519::Keypair::from(secret);
    let persistent_peer_id = quorum_network::libp2p::identity::Keypair::from(ed_kp).public().to_peer_id();
    let persistent_peer_id_str = persistent_peer_id.to_string();

    let inbound_handler: Arc<dyn quorum_network::InboundQueryHandler> = Arc::new(
        quorum_coordinator::OsoosiMeshInboundHandler::new(
            Arc::clone(&storage) as Arc<dyn quorum_coordinator::StorageSearcher>,
            persistent_peer_id_str.clone(),
        ),
    );

    let (mesh_dispatcher, swarm_handle): (Arc<dyn quorum_coordinator::MeshQueryDispatcher>, Option<quorum_network::NetworkSwarmHandle>) = if cli.local_only {
        info!("Running in local-only mode (P2P swarm networking disabled).");
        (Arc::new(quorum_coordinator::LocalOnlyDispatcher), None)
    } else {
        let net_config = quorum_network::NetworkConfig {
            ed25519_seed: Some(node_seed),
            tcp_listen_addrs: vec![format!("/ip4/{}/tcp/{}", cli.host, chosen_p2p_port).parse()?],
            quic_listen_addrs: vec![format!("/ip4/{}/udp/{}/quic-v1", cli.host, chosen_p2p_port).parse()?],
            contract_hash_hex: embedder.contract_hash().iter().map(|b| format!("{:02x}", b)).collect(),
            ..Default::default()
        };
        match quorum_network::start_network_node(net_config, Some(inbound_handler)).await {
            Ok((_join, handle)) => {
                let disp = Arc::new(quorum_coordinator::SwarmMeshDispatcher {
                    handle: handle.clone(),
                });
                (disp, Some(handle))
            }
            Err(e) => {
                error!("Failed to start libp2p network node: {}. Falling back to local-only.", e);
                (Arc::new(quorum_coordinator::LocalOnlyDispatcher), None)
            }
        }
    };

    let mut coord_cfg = quorum_coordinator::CoordinatorConfig::default();
    coord_cfg.max_peer_responses = cli.max_peer_responses;

    let coordinator_engine = Arc::new(quorum_coordinator::ScatterGatherEngine::new(
        Arc::clone(&storage) as Arc<dyn quorum_coordinator::StorageSearcher>,
        mesh_dispatcher,
        Arc::new(tokio::sync::RwLock::new(quorum_coordinator::PeerAclManager::default())),
        coord_cfg.clone(),
        node_seed,
        persistent_peer_id_str,
    ));
```

---

### 4.2 Federated Entity & Cue-Term Supplements

#### File: `crates/quorum-coordinator/src/scatter_gather.rs`
Add `search_federated_text`:
```rust
    /// Dispatches federated text queries across local storage and remote mesh peers.
    pub async fn search_federated_text(
        &self,
        query_text: &str,
        access_tier: u8,
        top_k: usize,
    ) -> Vec<ScoredChunk> {
        let mut results = self.local_storage.search_text(query_text, access_tier, top_k).await;

        let controller = crate::dlp_mesh::global_mesh_dlp_controller();
        let (p2p_enabled, max_peer_responses) = {
            let s = controller.settings.read().unwrap_or_else(|e| e.into_inner());
            let max_peers = if s.mesh.max_peer_responses > 0 {
                s.mesh.max_peer_responses
            } else {
                self.config.max_peer_responses.max(1)
            };
            (s.mesh.p2p_enabled, max_peers)
        };

        if !p2p_enabled {
            return results;
        }

        if let Some(sanitized_query) = controller.sanitize_outbound_query(query_text) {
            let embedder = quorum_contract::DeterministicOfflineEngine::new();
            let query_vec = embedder.embed_text(query_text).unwrap_or_else(|_| vec![0.0; 768]);

            let mut req = FederatedQueryRequest {
                query_id: Uuid::new_v4(),
                sender_peer_id: self.sender_peer_id.clone(),
                sender_pubkey: [0u8; 32],
                query_text: sanitized_query,
                embedding: query_vec,
                top_k,
                timestamp: Utc::now().timestamp(),
                signature: [0u8; 64],
            };
            let tier = AccessTier::from_u8(access_tier).unwrap_or(AccessTier::Tier0Public);
            sign_request(&self.secret_seed, &mut req, tier);

            let timeout_dur = Duration::from_millis(self.config.timeout_ms.min(300));
            if let Ok(responses) = self.mesh_dispatcher.query_peers(&req, timeout_dur).await {
                let mut peer_count = 0;
                for resp in responses {
                    if peer_count >= max_peer_responses {
                        break;
                    }
                    let mut peer_added = false;
                    for mut chunk in resp.chunks {
                        if chunk.line_start > chunk.line_end {
                            continue;
                        }
                        chunk.access_tier = chunk.access_tier.min(access_tier);
                        let dlp_res = controller.inspect_and_enforce_text(
                            &chunk.content,
                            &chunk.file_path,
                            &resp.responder_peer_id,
                            "InboundMesh",
                            None,
                            false,
                        );
                        if dlp_res.allowed_to_share {
                            let doc_chunk = quorum_types::DocumentChunk {
                                chunk_id: chunk.chunk_id,
                                document_id: "remote_doc".to_string(),
                                file_path: chunk.file_path,
                                line_start: chunk.line_start,
                                line_end: chunk.line_end,
                                blake3_hash: quorum_types::DocumentChunk::compute_blake3_hash(&dlp_res.scrubbed_text),
                                access_tier: chunk.access_tier,
                                vector: vec![0.0; 768],
                                text_content: dlp_res.scrubbed_text,
                                metadata_json: "{}".to_string(),
                            };
                            results.push(ScoredChunk::new(doc_chunk, chunk.score, &resp.responder_peer_id, true));
                            peer_added = true;
                        }
                    }
                    if peer_added {
                        peer_count += 1;
                    }
                }
            }
        }
        results
    }
```

#### File: `crates/quorum-coordinator/src/agentic/retrieve.rs`
Replace the 4 local text searches in `RetrieveMeshNode::execute`:
- Line 237: `engine.search_federated_text(&entity_kw_query, access_tier, 96).await;`
- Line 251: `engine.search_federated_text(&prompt_kw_query, access_tier, 64).await;`
- Line 266: `engine.search_federated_text(&cue_query, access_tier, 48).await;`
- Line 277: `engine.search_federated_text("Femi Oyesanya CISSP PROFESSIONAL SUMMARY Resume CV", access_tier, 64).await;`

---

### 4.3 Engine-Mode Contract Differentiation

#### File: `crates/quorum-contract/src/lib.rs`
1. Update `EmbeddingContract`:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingContract {
    pub contract_version: u32,
    pub model_name: String,
    pub engine_mode: String,
    pub dimension: usize,
    pub normalization: String,
    pub distance_metric: String,
    pub pooling_strategy: String,
    pub max_sequence_length: usize,
}

impl EmbeddingContract {
    /// Canonical specification for neural Candle Gemma models.
    pub fn canonical_gemma_300m() -> Self {
        Self {
            contract_version: 1,
            model_name: "google/embeddinggemma-300m".to_string(),
            engine_mode: "GemmaCandle".to_string(),
            dimension: 768,
            normalization: "L2".to_string(),
            distance_metric: "Cosine".to_string(),
            pooling_strategy: "MeanPooling".to_string(),
            max_sequence_length: 512,
        }
    }

    /// Canonical specification for Deterministic Offline BLAKE3 XOF vectors.
    pub fn canonical_deterministic_offline() -> Self {
        Self {
            contract_version: 1,
            model_name: "google/embeddinggemma-300m".to_string(),
            engine_mode: "DeterministicOffline".to_string(),
            dimension: 768,
            normalization: "L2".to_string(),
            distance_metric: "Cosine".to_string(),
            pooling_strategy: "MeanPooling".to_string(),
            max_sequence_length: 512,
        }
    }

    pub fn derive_contract_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"QUORUM_RAG_EMBEDDING_CONTRACT_V1\n");
        hasher.update(&self.contract_version.to_le_bytes());
        hasher.update(self.model_name.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.engine_mode.as_bytes());
        hasher.update(b"\n");
        hasher.update(&(self.dimension as u64).to_le_bytes());
        hasher.update(self.normalization.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.distance_metric.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.pooling_strategy.as_bytes());
        hasher.update(b"\n");
        hasher.update(&(self.max_sequence_length as u64).to_le_bytes());
        *hasher.finalize().as_bytes()
    }
}
```
2. In `DeterministicOfflineEngine`:
```rust
impl DeterministicOfflineEngine {
    pub fn new() -> Self {
        Self {
            contract: EmbeddingContract::canonical_deterministic_offline(),
        }
    }
}
```

---

### 4.4 Peer & Filename Disambiguation

#### File: `crates/quorum-coordinator/src/fusion.rs`
Add `fuse_rankings_with_peers`:
```rust
impl RrfCombiner {
    pub fn fuse_rankings(
        local_results: Vec<ScoredChunk>,
        remote_peer_lists: Vec<Vec<CandidateChunkDto>>,
        k: f32,
        top_k: usize,
    ) -> Vec<ScoredChunk> {
        let paired = remote_peer_lists
            .into_iter()
            .enumerate()
            .map(|(i, list)| (format!("remote_peer_{}", i + 1), list))
            .collect();
        Self::fuse_rankings_with_peers(local_results, paired, k, top_k)
    }

    pub fn fuse_rankings_with_peers(
        local_results: Vec<ScoredChunk>,
        remote_peer_lists: Vec<(String, Vec<CandidateChunkDto>)>,
        k: f32,
        top_k: usize,
    ) -> Vec<ScoredChunk> {
        let mut rrf_map: HashMap<String, (f32, ScoredChunk, HashSet<usize>)> = HashMap::new();
        const LOCAL_PEER_ID: usize = 0;

        let mut seen_local_hashes: HashSet<String> = HashSet::new();
        for (rank_0, scored) in local_results.into_iter().enumerate() {
            let hash = if scored.chunk.blake3_hash.is_empty() {
                DocumentChunk::compute_blake3_hash(&scored.chunk.text_content)
            } else {
                scored.chunk.blake3_hash.clone()
            };

            if !seen_local_hashes.insert(hash.clone()) {
                continue;
            }

            let score = 1.0 / (k + (rank_0 + 1) as f32);
            let entry = rrf_map
                .entry(hash)
                .or_insert_with(|| (0.0, scored, HashSet::new()));
            entry.0 += score;
            entry.2.insert(LOCAL_PEER_ID);
        }

        for (remote_idx, (peer_id, list)) in remote_peer_lists.into_iter().enumerate() {
            let peer_numeric_id = remote_idx + 1;
            let mut seen_peer_hashes: HashSet<String> = HashSet::new();

            for (rank_0, candidate) in list.into_iter().enumerate() {
                let content_hash = DocumentChunk::compute_blake3_hash(&candidate.content);
                if !seen_peer_hashes.insert(content_hash.clone()) {
                    continue;
                }

                let score = 1.0 / (k + (rank_0 + 1) as f32);
                let entry = rrf_map.entry(content_hash.clone()).or_insert_with(|| {
                    let doc_chunk = DocumentChunk {
                        chunk_id: candidate.chunk_id.clone(),
                        document_id: "remote_doc".to_string(),
                        file_path: candidate.file_path,
                        line_start: candidate.line_start,
                        line_end: candidate.line_end,
                        blake3_hash: content_hash,
                        access_tier: candidate.access_tier,
                        vector: vec![0.0; 768],
                        text_content: candidate.content,
                        metadata_json: "{}".to_string(),
                    };
                    let scored = ScoredChunk::new(doc_chunk, candidate.score, peer_id.clone(), true);
                    (0.0, scored, HashSet::new())
                });
                entry.0 += score;
                entry.2.insert(peer_numeric_id);
            }
        }

        let mut fused: Vec<ScoredChunk> = rrf_map
            .into_iter()
            .map(|(_hash, (base_score, mut chunk, peers))| {
                let final_score = if peers.len() >= 2 {
                    base_score * 1.25
                } else {
                    base_score
                };
                chunk.score = final_score;
                chunk
            })
            .collect();

        fused.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
                .then_with(|| a.chunk.blake3_hash.cmp(&b.chunk.blake3_hash))
        });

        fused.truncate(top_k);
        fused
    }
}
```

#### File: `crates/quorum-coordinator/src/scatter_gather.rs`
Update lines 262-303:
```rust
        let mut volatile_chunks = Vec::new();
        let mut remote_lists = Vec::new();
        for resp in remote_responses {
            let mut clean_peer_chunks = Vec::new();
            for mut chunk in resp.chunks {
                if chunk.line_start > chunk.line_end {
                    continue;
                }
                chunk.access_tier = chunk.access_tier.min(access_tier);
                let dlp_res = controller.inspect_and_enforce_text(
                    &chunk.content,
                    &chunk.file_path,
                    &resp.responder_peer_id,
                    "InboundMesh",
                    None,
                    false,
                );
                if dlp_res.allowed_to_share {
                    chunk.content = dlp_res.scrubbed_text;
                    volatile_chunks.push(chunk.to_volatile_remote_chunk(&resp.responder_peer_id));
                    clean_peer_chunks.push(chunk);
                }
            }
            if !clean_peer_chunks.is_empty() {
                remote_lists.push((resp.responder_peer_id.clone(), clean_peer_chunks));
            }
        }

        let fused = RrfCombiner::fuse_rankings_with_peers(
            local_results,
            remote_lists,
            self.config.rrf_k,
            top_k,
        );
```

#### File: `crates/quorum-coordinator/src/offline_engine.rs`
1. In `deduplicated_chunks_with_scores` (lines 1408-1435):
```rust
        let mut seen_keys = std::collections::HashSet::new();
        let mut out: Vec<(&'a ScoredChunk, u32)> = Vec::new();
        for &c in matched_chunks {
            let file_name = std::path::Path::new(&c.chunk.file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&c.chunk.file_path);
            let dedup_key = (c.source_peer.to_ascii_lowercase(), file_name.to_ascii_lowercase());
            if !seen_keys.insert(dedup_key) {
                continue;
            }
            ...
```
2. In `format_sources_block` (lines 1460-1487):
```rust
        for (c, score_pct) in deduped {
            let file_name = std::path::Path::new(&c.chunk.file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&c.chunk.file_path);

            let peer_suffix = if !c.source_peer.is_empty() && c.source_peer != "local" {
                let short_peer = if c.source_peer.len() > 8 {
                    &c.source_peer[..8]
                } else {
                    &c.source_peer
                };
                format!(" (Peer: {})", short_peer)
            } else {
                String::new()
            };

            ...
            block.push_str(&format!(
                "- [📄 {}{}]({}) -- **{}% accuracy**\n",
                file_name,
                peer_suffix,
                link_target,
                score_pct
            ));
        }
```

#### File: `crates/quorum-ui/assets/app.js`
In `appendCitation` (lines 880-895):
```javascript
    const peerId = (cit.peer_id || 'local').trim();
    const dedupKey = peerId.toLowerCase() + '::' + cleanFileName.toLowerCase();
    if (!cleanFileName || citationsEl._seenFiles.has(dedupKey)) {
      return;
    }
    citationsEl._seenFiles.add(dedupKey);
    const citeNum = (citationsEl._nextIndex = (citationsEl._nextIndex || 0) + 1);

    const peerLabel = peerId !== 'local' && peerId.length > 0
      ? ` [Peer: ${peerId.length > 8 ? peerId.slice(0, 8) + '...' : peerId}]`
      : '';

    badge.innerHTML = `<span class="cite-index" style="font-weight:700;opacity:0.9;">[${citeNum}]</span> <span>📄 ${escapeHtml(cleanFileName + peerLabel)}</span> <span class="score-pill" style="opacity:0.85; font-size:11px; margin-left:3px; padding:1px 6px; border-radius:4px; background:rgba(66,133,244,0.2); font-weight:600;">· ${scorePct}%</span>`;
```

---

### 4.5 Configurable `max_peer_responses`

#### File: `crates/quorum-coordinator/src/config.rs`
```rust
pub struct CoordinatorConfig {
    pub timeout_ms: u64,
    pub max_drift_secs: i64,
    pub rrf_k: f32,
    pub default_top_k: usize,
    pub candidate_multiplier: usize,
    pub default_peer_access_tier: u8,
    pub exclude_source_code: bool,
    pub max_peer_responses: usize,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 400,
            max_drift_secs: 60,
            rrf_k: 60.0,
            default_top_k: 10,
            candidate_multiplier: 2,
            default_peer_access_tier: 0,
            exclude_source_code: true,
            max_peer_responses: 8,
        }
    }
}
```

#### File: `crates/quorum-coordinator/src/dlp_mesh.rs`
```rust
fn default_8() -> usize { 8 }

pub struct OsoosiMeshSettings {
    pub p2p_enabled: bool,
    ...
    #[serde(default = "default_8")]
    pub max_peer_responses: usize,
}
```

#### File: `crates/quorum-daemon/src/cli.rs`
```rust
    /// Maximum number of remote peer responses to collect/fuse per query (default: 8)
    #[arg(long, default_value_t = 8)]
    pub max_peer_responses: usize,
```

#### File: `crates/quorum-coordinator/src/mesh.rs`
In `SwarmMeshDispatcher::query_peers` (lines 145-155):
```rust
        let max_peer_responses = {
            let controller = crate::dlp_mesh::global_mesh_dlp_controller();
            let s = controller.settings.read().unwrap_or_else(|e| e.into_inner());
            if s.mesh.max_peer_responses > 0 {
                s.mesh.max_peer_responses
            } else {
                8
            }
        };

        let mut responses = Vec::new();
        let _ = tokio::time::timeout(timeout, async {
            while let Some(res) = futures.next().await {
                if let Ok(Ok(resp)) = res {
                    responses.push(resp);
                    if responses.len() >= max_peer_responses {
                        break;
                    }
                }
            }
        })
        .await;
```

#### File: `crates/quorum-daemon/src/handlers/settings.rs`
In `UpdateMeshConfigPayload`:
```rust
pub struct UpdateMeshConfigPayload {
    pub p2p_enabled: Option<bool>,
    ...
    pub max_peer_responses: Option<usize>,
}
```
In `update_mesh_config_handler`:
```rust
    if let Some(max_peers) = payload.max_peer_responses {
        guard.mesh.max_peer_responses = max_peers.max(1);
    }
```

#### File: `crates/quorum-ui/assets/app.js`
In the Mesh Settings tab template (inside `OSHOOSICLAW MESH PEERING` card):
```html
          <div class="settings-row">
            <div class="settings-row-info">
              <span class="settings-row-label">Max Peer Responses Per Query</span>
              <div class="settings-row-desc">Deterministic cap on remote peer candidate sets collected and fused per federated search.</div>
            </div>
            <div style="display:flex; gap:8px; align-items:center;">
              <input type="number" id="mesh-max-peer-responses" class="settings-input" style="width:90px;" min="1" max="64" value="${escapeHtml(mesh.max_peer_responses || 8)}" onchange="window.quorumUpdateMeshConfig({max_peer_responses: parseInt(this.value, 10) || 8})">
              <span style="font-size:12px; color:var(--text-muted);">peers</span>
            </div>
          </div>
```

---

## 5. Verification Method

To verify these changes after implementation:

### 5.1 Unit Tests
1. **Node Identity & Derivation Test**:
   - Create a test verifying that `load_or_generate_node_seed` persists a seed file across invocations.
   - Assert `derive_peer_id_from_pubkey(&pubkey)` exactly matches `libp2p::identity::ed25519::Keypair::from(secret).public().to_peer_id()`.
   - Verify `verify_request` succeeds without triggering spoofed peer ID warning.
2. **Contract Hash Differentiation Test**:
   - In `crates/quorum-contract/tests/adversarial_contract_tests.rs`:
     Verify `EmbeddingContract::canonical_deterministic_offline().derive_contract_hash() != EmbeddingContract::canonical_gemma_300m().derive_contract_hash()`.
     Assert `verify_contract_handshake` between deterministic and neural contracts returns `Err(ContractError::ContractMismatch { .. })`.
3. **Peer & Filename Disambiguation Test**:
   - Provide two chunks from different peers with the exact same filename (e.g. `PeerA::contract.pdf` and `PeerB::contract.pdf`).
   - Run `RrfCombiner::fuse_rankings_with_peers` and `build_deduplicated_citations`.
   - Assert both citations are preserved with distinct peer IDs and zero line numbers.
4. **Max Peer Responses Capping Test**:
   - Set up `MockMeshDispatcher` with 4 responding peers.
   - Set `max_peer_responses = 2`.
   - Execute federated query; assert exactly 2 peer responses are collected and fused.

### 5.2 Integration Tests
Run `cargo test` across all workspace crates:
```powershell
cargo test
```

### 5.3 Invalidation Conditions
- Any change that alters the byte sequence of `canonical_query_bytes` will invalidate existing test signatures.
- Any change that removes `pub fn fuse_rankings(local, remote, k, top_k)` will break callers in external integration test suites.
- Any modification to `CandidateChunkDto` without maintaining struct initialization compatibility will fail compilation in `quorum-coordinator/tests`.
