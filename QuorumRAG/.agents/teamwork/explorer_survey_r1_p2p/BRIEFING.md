# BRIEFING - 2026-11-03T13:29:30Z

## Mission
Investigate codebase for Requirement R1 (Fix 4 Cross-Peer Gaps in P2P Mesh) and configurable max_peer_responses, producing a deep technical report for implementation agents.

## 🔒 My Identity
- Archetype: teamwork_explorer
- Roles: explorer, investigator, synthesizer
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_r1_p2p\
- Original parent: a004ba72-59c1-4c2c-8399-d2a932e85b03
- Milestone: Survey & Architecture Analysis for R1 & Follow-up

## 🔒 Key Constraints
- Read-only investigation - do NOT implement
- Strictly READ-ONLY. Do NOT modify source code or run build/mutation commands.
- Update progress.md periodically with "Last visited: [timestamp]"
- Output comprehensive technical report to handoff.md
- Send completion message via send_message to parent

## Current Parent
- Conversation ID: a004ba72-59c1-4c2c-8399-d2a932e85b03
- Updated: not yet

## Investigation State
- **Explored paths**:
  - `crates/quorum-network/src/lib.rs`, `config.rs`, `transport.rs`, `handshake.rs`, `actor.rs`
  - `crates/quorum-daemon/src/main.rs`, `cli.rs`, `handlers/settings.rs`, `handlers/health.rs`
  - `crates/quorum-coordinator/src/scatter_gather.rs`, `mesh.rs`, `crypto.rs`, `config.rs`, `dlp_mesh.rs`, `fusion.rs`, `offline_engine.rs`, `agentic/retrieve.rs`, `coordinator.rs`
  - `crates/quorum-contract/src/lib.rs`, `tests/adversarial_contract_tests.rs`, `tests/corner_cases_handshake_gating.rs`
  - `crates/quorum-types/src/lib.rs`
  - `crates/quorum-ui/assets/index.html`, `app.js`
- **Key findings**:
  1. Node Identity: `main.rs:180-206` sets `ed25519_seed: None` for libp2p, generating random ephemeral keys on restart, and passes `[0u8; 32]` and `"local_daemon"` to `ScatterGatherEngine`. Inbound handler verifies `derived_id == req.sender_peer_id` (`mesh.rs:223`), causing guaranteed rejection of all queries as spoofed! Solution: persist 32-byte seed in `<data_dir>/node_identity.seed`, share seed and derived `PeerId` across libp2p and `ScatterGatherEngine`.
  2. Federated Supplements: `RetrieveMeshNode::execute` (`retrieve.rs:231-290`) runs 4 supplemental text searches against `engine.local_storage` only. Solution: add `engine.search_federated_text` to query local storage and remote peers via `mesh_dispatcher` with DLP enforcement, capping peer responses to `max_peer_responses`.
  3. Engine-Mode Contract Differentiation: `EmbeddingContract` does not differentiate BLAKE3 XOF (`DeterministicOfflineEngine`) from neural Candle weights (`GemmaCandleEngine`), both generating identical contract hashes. Solution: add `engine_mode: String` to contract and hash, differentiating `canonical_deterministic_offline()` from `canonical_gemma_300m()`, causing `verify_peer_contract` to fail closed with `ContractMismatch`.
  4. Peer & Filename Disambiguation: `ScatterGatherEngine` drops `resp.responder_peer_id` before calling `RrfCombiner::fuse_rankings`, which hardcodes `"remote_peer"`. `offline_engine.rs` deduplicates solely by filename, discarding remote files with common names (e.g. `README.md`). `app.js` tracks `_seenFiles` by filename only. Solution: preserve `responder_peer_id` through `RrfCombiner::fuse_rankings_with_peers`, deduplicate by `(source_peer, file_name)` in `offline_engine.rs`, and track `(peer_id, file_name)` in `app.js`.
  5. Configurable `max_peer_responses`: Add `max_peer_responses` to `CoordinatorConfig`, `OsoosiMeshSettings`, CLI (`--max-peer-responses`), enforce in `SwarmMeshDispatcher` and `ScatterGatherEngine`, expose in Web UI Mesh Settings and wire to `/api/mesh/config`.
- **Unexplored areas**: None. All 5 areas completely analyzed with exact line numbers and code paths.

## Key Decisions Made
- Architecture design verified against all invariants and constraints. No mocks or fake data. Fail-closed P2P security. Clean backward compatibility preserved for existing tests.

## Artifact Index
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_r1_p2p\DISPATCH.md - Initial dispatch instructions
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_r1_p2p\BRIEFING.md - Working memory & identity
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_r1_p2p\progress.md - Liveness & progress heartbeat
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_r1_p2p\handoff.md - Final 5-component technical report
