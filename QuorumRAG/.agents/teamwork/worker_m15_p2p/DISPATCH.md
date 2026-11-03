## 2026-11-03T13:36:52Z

You are worker_m15_p2p, a software engineering worker (teamwork_worker).
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\worker_m15_p2p\
Your parent is: a004ba72-59c1-4c2c-8399-d2a932e85b03

MANDATORY FIRST STEP:
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md before doing anything else.
Also read d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md and the survey blueprint in d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_r1_p2p\handoff.md.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

EXCLUSIVE WRITE OWNERSHIP:
You own and have exclusive write permission to:
- crates/quorum-network/src/lib.rs (and config.rs)
- crates/quorum-contract/src/lib.rs
- crates/quorum-coordinator/src/config.rs
- crates/quorum-coordinator/src/dlp_mesh.rs
- crates/quorum-coordinator/src/scatter_gather.rs
- crates/quorum-coordinator/src/fusion.rs
- crates/quorum-coordinator/src/offline_engine.rs
- crates/quorum-coordinator/src/agentic/retrieve.rs
- crates/quorum-coordinator/src/mesh.rs
- crates/quorum-daemon/src/cli.rs
- crates/quorum-daemon/src/main.rs
- crates/quorum-daemon/src/handlers/settings.rs
- crates/quorum-ui/assets/index.html
- crates/quorum-ui/assets/app.js
- Unit/integration tests in crates/*/tests/

MISSION:
Implement Milestone M15 (Requirement R1: Fix 4 Cross-Peer Gaps in P2P Mesh & Follow-up: Configurable max_peer_responses) end-to-end:

1. Persistent Ed25519 Node Identity:
   - Add load_or_generate_node_seed(data_dir: &Path) -> Result<[u8; 32], std::io::Error> in crates/quorum-network/src/lib.rs.
   - In crates/quorum-daemon/src/main.rs: load/generate seed from cli.data_dir, derive keypair and persistent PeerId, pass Some(node_seed) to NetworkConfig, pass node_seed and persistent_peer_id_str to ScatterGatherEngine::new(...), and pass persistent_peer_id_str to OsoosiMeshInboundHandler::new(...).
   - Ensure the inbound handler's derive_peer_id_from_pubkey(&req.sender_pubkey) == req.sender_peer_id check succeeds for valid peer queries without spoofing rejection.

2. Federated Entity & Cue-Term Supplements:
   - Add search_federated_text(&self, query_text: &str, access_tier: u8, top_k: usize) -> Vec<ScoredChunk> to ScatterGatherEngine (crates/quorum-coordinator/src/scatter_gather.rs).
   - In RetrieveMeshNode::execute (crates/quorum-coordinator/src/agentic/retrieve.rs), update the 4 supplemental text searches (entity_kw_query, prompt_kw_query, cue_query, and resume keywords) to query remote mesh peers via engine.search_federated_text(...).

3. Engine-Mode Contract Differentiation:
   - Add engine_mode: String to EmbeddingContract in crates/quorum-contract/src/lib.rs.
   - Provide canonical_deterministic_offline() ("DeterministicOffline") and canonical_gemma_300m() ("GemmaCandle").
   - Include engine_mode in derive_contract_hash(&self) so BLAKE3 XOF vectors and neural Gemma vectors produce different contract hashes, causing incompatible peer handshakes to fail closed with ContractMismatch.
   - In DeterministicOfflineEngine::new(): use canonical_deterministic_offline().
   - In crates/quorum-daemon/src/main.rs: set net_config.contract_hash_hex from the active embedder contract hash.

4. Peer and Filename Disambiguation:
   - In crates/quorum-coordinator/src/fusion.rs: implement RrfCombiner::fuse_rankings_with_peers(local, remote: Vec<(String, Vec<CandidateChunkDto>)>, k, top_k) -> Vec<ScoredChunk> preserving real responder_peer_id in ScoredChunk.source_peer. Keep fuse_rankings as backward-compatible wrapper.
   - In ScatterGatherEngine: collect remote chunks paired with resp.responder_peer_id and invoke fuse_rankings_with_peers.
   - In crates/quorum-coordinator/src/offline_engine.rs: in deduplicated_chunks_with_scores, deduplicate by (c.source_peer.to_ascii_lowercase(), file_name.to_ascii_lowercase()). In format_sources_block, display (Peer: <short_peer>) when source_peer != "local".
   - In crates/quorum-ui/assets/app.js: in appendCitation, disambiguate _seenFiles by (cit.peer_id || 'local').toLowerCase() + '::' + cleanFileName.toLowerCase().

5. Configurable max_peer_responses:
   - Add max_peer_responses: usize (default: 8) to CoordinatorConfig (crates/quorum-coordinator/src/config.rs).
   - Add max_peer_responses: usize (default: 8, serde default) to OsoosiMeshSettings (crates/quorum-coordinator/src/dlp_mesh.rs), persisted in settings.json.
   - Add --max-peer-responses <USIZE> (default: 8) to Cli in crates/quorum-daemon/src/cli.rs.
   - Wire in main.rs and handlers/settings.rs (UpdateMeshConfigPayload and update_mesh_config_handler).
   - Enforce max_peer_responses in SwarmMeshDispatcher::query_peers and ScatterGatherEngine::execute_query_with_cached_vector and search_federated_text.
   - In crates/quorum-ui/assets/index.html & app.js: add interactive <input type="number" min="1" max="64" id="mesh-max-peer-responses"> in the Mesh Settings tab wired to window.quorumUpdateMeshConfig({ max_peer_responses: ... }).

6. Testing & Build Invariants:
   - Always run cargo commands with `$env:CARGO_INCREMENTAL = "0"` and `-- --test-threads=1`.
   - Write comprehensive unit/integration tests proving:
     a) Persistent node seed generation, disk reloading, and derived PeerId stability.
     b) Contract differentiation between DeterministicOffline and GemmaCandle producing distinct hashes and handshake rejection on mismatch.
     c) fuse_rankings_with_peers preserving responder_peer_id and citations distinguishing same-named files from different peers.
     d) max_peer_responses strictly capping accepted peer responses per query.
   - Run cargo test on affected crates:
     `cargo test -p quorum-network`
     `cargo test -p quorum-contract`
     `cargo test -p quorum-coordinator`
     `cargo test -p quorum-daemon`
   - Document all changes, test commands, and passing outputs in your handoff report: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m15_p2p\handoff.md`.
   - When finished, send a completion message via send_message to your parent.
