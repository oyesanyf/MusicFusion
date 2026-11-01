# BRIEFING — 2026-10-31T22:25:00Z

## Mission
Technical exploration and architectural design for the Scatter-Gather Query Coordinator and Volatile Protection layer in `crates/quorum-coordinator`.

## 🔒 My Identity
- Archetype: explorer
- Roles: Read-only investigation, architectural analysis, synthesis, structured handoff reporting
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m4_2
- Original parent: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
- Milestone: Milestone 4 (rust-libp2p Swarm, Coordinator, Fusion & GLM Selection)

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Zero Arrow/Lance type leakage into `quorum-coordinator`
- Strict 5-component handoff report (Observation, Logic Chain, Caveats, Conclusion, Verification Method)
- Volatile memory protection: remote peer chunks held strictly in `ZeroizeOnDrop` memory structs, scrubbing heap bytes immediately upon drop
- Parallel dispatch with 400ms hard timeout guard (`tokio::time::timeout`) with graceful fallback to local results when peers lag or time out
- Ed25519 digital signature signing and anti-replay timestamp verification (< 60s tolerance)
- Access-tier SQL pre-filtering in LanceDB using peer Ed25519 public key and configured access-tier rules
- Reciprocal Rank Fusion (RRF): $RRF(d) = \sum_{m \in M} \frac{1}{60 + r_m(d)}$ aggregating local and remote candidate chunks, deduplicating identical chunks by BLAKE3 hash, and deterministic tie-breaking
- Inspect existing tests in `tests/e2e/test_federated_mesh.rs` and verify exact struct/enum compatibility

## Current Parent
- Conversation ID: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
- Updated: 2026-10-31T22:25:00Z

## Investigation State
- **Explored paths**:
  - `d:\harfile\QuorumRAG\PROJECT.md` & `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
  - `tests/e2e/test_federated_mesh.rs` & `tests/e2e/src/lib.rs` & `tests/e2e/Cargo.toml`
  - `crates/quorum-types/src/lib.rs`
  - `crates/quorum-storage/src/lib.rs` & `crates/quorum-storage/src/engine.rs`
  - `crates/quorum-contract/src/lib.rs`
  - `.agents/teamwork/explorer_m4_1/` & `.agents/teamwork/explorer_m4_3/`
- **Key findings**:
  - `tests/e2e/test_federated_mesh.rs` specifies 10 concrete test scenarios covering Ed25519 signature signing/verification, 400ms timeout guard with fast vs slow peer (total <= 480ms), access-tier blocking of confidential chunks, RRF score accumulation (2/61), ZeroizeOnDrop volatile chunk memory scrubbing, expired timestamp replay rejection (>60s), forged signature rejection, 0-peer graceful local fallback (<50ms), multi-peer consensus boost (3/61), and 20 concurrent queries without races.
  - Zero Arrow/Lance leakage is preserved: `quorum-coordinator` interacts only with `StorageEngine` and pure domain types from `quorum-types` (`DocumentChunk`, `ScoredChunk`, `AccessTier`, `Citation`, `VolatileRemoteChunk`, `CandidateChunkDto`, `QueryResult`).
  - Strict RRF with $k=60.0$, BLAKE3 hash deduplication, and deterministic tie-breaking (`b.score.partial_cmp(&a.score).then_with(chunk_id).then_with(blake3_hash)`).
  - Pluggable `MeshQueryDispatcher` trait allows seamless switching between mock peers (for unit/E2E testing) and libp2p `NetworkSwarmHandle` (for production).
- **Unexplored areas**: None for M4-2 scope. Complete architectural specification synthesized.

## Key Decisions Made
- Decomposed `crates/quorum-coordinator` into 10 modular files: `lib.rs`, `config.rs`, `error.rs`, `crypto.rs`, `acl.rs`, `fusion.rs`, `volatile.rs`, `mesh.rs`, `scatter_gather.rs`, `coordinator.rs`.
- Decoupled `MeshQueryDispatcher` so coordinator compiles and tests both with mock network and real libp2p swarm without circular dependencies.
- Designed dual coordinator layers: low-level `ScatterGatherEngine` (returns chunks, volatile chunks, execution time) and high-level `QueryCoordinator` (returns `QueryResult` with formatted `synthesis_prompt` and `sources_cited`).

## Artifact Index
- DISPATCH.md — incoming instructions
- BRIEFING.md — persistent working memory
- progress.md — liveness heartbeat
- handoff.md — final 5-component handoff report
