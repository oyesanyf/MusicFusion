## 2026-11-03T13:09:43Z
You are explorer_survey_p2p_1, an exploration subagent.
Working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_p2p_1\
Original request path: d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md

MANDATORY FIRST STEP:
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md thoroughly before starting.

YOUR TASK:
Investigate the authoritative source of truth across crates in d:\harfile\QuorumRAG for Requirement R1 (Cross-Peer Gaps in P2P Mesh & Configurable Max Peer Responses):

1. Persistent Ed25519 Node Identity:
   - Investigate how NetworkConfig (crates/quorum-network/src/lib.rs, config.rs), ScatterGatherEngine (crates/quorum-coordinator/src/scatter_gather.rs), and the daemon entry point (crates/quorum-daemon/src/main.rs, cli.rs, server.rs) create or load Ed25519 node identities and derived PeerIds.
   - Design the exact implementation to store a persistent 32-byte Ed25519 seed/keypair in the daemon data directory (--data-dir) so NetworkConfig (libp2p) and ScatterGatherEngine share the identical identity and derived PeerId, preventing valid peer queries from being rejected as spoofed.

2. Federated Entity & Cue-Term Supplements:
   - Investigate RetrieveMeshNode::execute in crates/quorum-coordinator/src/agentic/retrieve.rs.
   - Identify where extra entity and cue-term searches currently execute and how they are limited to local storage.
   - Design the exact extension to query remote peers across the mesh via scatter-gather rather than searching only local storage.

3. Engine-Mode Contract Differentiation:
   - Investigate EmbeddingContract and engine hash generation in crates/quorum-contract/src/lib.rs.
   - Analyze how DeterministicOfflineEngine (BLAKE3 XOF vectors) and neural models (GemmaCandleEngine / Gemma 768d) are represented.
   - Design the update to EmbeddingContract / engine hash generation to distinguish between DeterministicOfflineEngine and neural models, preventing incompatible peers from completing invalid handshakes.

4. Peer and Filename Disambiguation:
   - Investigate RrfCombiner in crates/quorum-coordinator/src/fusion.rs and how responder_peer_id is preserved or lost (ScoredChunk.source_peer).
   - Investigate crates/quorum-coordinator/src/offline_engine.rs and crates/quorum-ui/assets/app.js to see how citations and deduplication currently work.
   - Design the fix to disambiguate citations and deduplication by (source_peer, file_name) across both coordinator and WebUI.

5. Configurable max_peer_responses:
   - Backend Config, CLI & Persistence: Design adding `max_peer_responses: usize` (default e.g. 8) to CoordinatorConfig (crates/quorum-coordinator/src/config.rs), OsoosiMeshSettings (crates/quorum-coordinator/src/dlp_mesh.rs, persisted in settings.json), and CLI (--max-peer-responses in crates/quorum-daemon/src/cli.rs).
   - Runtime Scatter-Gather / Mesh Enforcement: Design enforcement in ScatterGatherEngine / SwarmMeshDispatcher::query_peers (crates/quorum-coordinator/src/scatter_gather.rs & crates/quorum-coordinator/src/mesh.rs) so once max_peer_responses valid peer responses are received, it stops waiting / caps accepted responses.
   - Interactive UI Control + Persistence: Design the `<input type="number" min="1" ...>` in Web UI Mesh / Settings configuration (crates/quorum-ui/assets/index.html & app.js) wired to settings API.
   - Unit/integration test design for max_peer_responses.

OUTPUT REQUIREMENTS:
Write your full, detailed findings, file paths, line references, and concrete implementation blueprints to:
d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_p2p_1\handoff.md

COMMUNICATION:
Send a summary message back to orchestrator (conversation ID: 939bb4ac-d018-4323-a22b-48a8594f9994) via send_message when your handoff.md is ready.
