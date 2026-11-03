# Original User Request

## Approved Task Prompt — 2026-11-03T13:04:45Z

Fix the four real cross-peer P2P gaps in QuorumRAG and eliminate multi-turn continuation repetition in the agentic state graph and retrieval/synthesis pipeline for production use.

Working directory: d:\harfile\QuorumRAG
Integrity mode: development

## Requirements

### R1. Fix 4 Cross-Peer Gaps in P2P Mesh
- **Persistent Ed25519 Node Identity**: Store a persistent 32-byte Ed25519 seed/keypair in the daemon data directory (`--data-dir`) so `NetworkConfig` (`libp2p`) and `ScatterGatherEngine` share the same identity and derived `PeerId`, preventing valid peer queries from being rejected as spoofed.
- **Federated Entity & Cue-Term Supplements**: Extend extra entity and cue-term searches in `RetrieveMeshNode::execute` (`crates/quorum-coordinator/src/agentic/retrieve.rs`) to query remote peers across the mesh rather than searching only local storage.
- **Engine-Mode Contract Differentiation**: Update `EmbeddingContract` / engine hash generation in `crates/quorum-contract/src/lib.rs` to distinguish between `DeterministicOfflineEngine` (BLAKE3 XOF vectors) and neural models (`GemmaCandleEngine`), preventing incompatible peers from completing invalid handshakes.
- **Peer and Filename Disambiguation**: Preserve `responder_peer_id` through `RrfCombiner` (`ScoredChunk.source_peer`) in `crates/quorum-coordinator/src/fusion.rs`, and disambiguate citations and deduplication by `(source_peer, file_name)` across `crates/quorum-coordinator/src/offline_engine.rs` and `crates/quorum-ui/assets/app.js`.

### R2. Multi-Turn Continuation Deduplication & State Graph Pipeline
- **Session-Level Seen Chunk Tracking & Negative Filtering**: Maintain `seen_chunk_ids` in session state across continuation turns and pass them as an exclusion list / negative filter to the retrieval query engine, ensuring previously served chunks are skipped while allowing unseen chunks from the same document to be retrieved.
- **Continuation Query Rewriting Node**: In the agentic state graph before retrieval, add or update a query rewriting node that inspects previous turn outputs, identifies already covered topics (e.g. contact info, payment methods, late fees), and rewrites continuation queries to target remaining unexplored aspects (e.g. maintenance duties, termination notices, utilities, specific rental rates).
- **Retrieval Diversity & Semantic Filtering**: Apply Maximal Marginal Relevance (MMR) during candidate retrieval to penalize high vector similarity to already selected chunks, and perform a lightweight pairwise semantic deduplication pass prior to synthesis comparing newly retrieved chunk segments against embeddings of previously generated sentences to drop or mask segments above ~0.88 cosine similarity (preventing redundant boilerplate clauses like repeated $195 eviction fees from reaching the generator).
- **Synthesis Prompt State Awareness & Entity Consolidation**: Pass prior turn responses into the generator node as an explicit exclusion constraint block commanding the model to extract only net-new facts, and enforce an entity consolidation instruction that merges overlapping facts across citations into unified statements (e.g. consolidating repeated account numbers) instead of summarizing citations sequentially.

### R3. Verification, Service Rebuild, and Shipping
- Verify with `cargo test` across all crates, a live two-daemon P2P peer query test, and live 3-turn continuation SSE queries (`"tell me about Angela Davis"` -> `"continue"` -> `"continue"`) against the local daemon on port 3031 with `gemma4:26b`.
- Rebuild `quorumrag.exe` and ensure the running daemon on port 3031 is cleanly restarted with the updated binary.
- Ship all verified changes via `powershell -ExecutionPolicy Bypass -File scripts/solo_pr_ship.ps1 -Title "..."`, scoping git operations to `d:\harfile\QuorumRAG` with `-- .` and leaving system/user environment PATH untouched.

## Acceptance Criteria

### P2P Mesh Invariants
- [ ] `libp2p` and `ScatterGatherEngine` share the persistent Ed25519 identity stored in `--data-dir`, and live cross-daemon P2P queries succeed without sender keypair or derivation spoofing rejections.
- [ ] Entity and cue-term supplement queries in `RetrieveMeshNode` federate across remote mesh peers.
- [ ] `EmbeddingContract` differentiates deterministic offline (BLAKE3) from neural engines, blocking incompatible handshakes.
- [ ] Citations and deduplication preserve `source_peer` and disambiguate same-named files from different peers.

### Multi-Turn Continuation Quality
- [ ] `seen_chunk_ids` are tracked in session state across continuation turns, preventing identical chunks from being retrieved on subsequent turns while allowing unseen chunks from the same document.
- [ ] Continuation turns rewrite queries to focus on uncovered aspects instead of repeating the initial prompt.
- [ ] Redundant boilerplate clauses (e.g. repeated eviction fee amounts across turns) are filtered before synthesis via semantic deduplication / MMR.
- [ ] The generator prompt enforces prior-turn exclusion and consolidates overlapping entities (e.g. bank account numbers across citations) into unified statements without intra-turn duplication.

### Verification & Delivery
- [ ] `cargo test` passes across all crates.
- [ ] Live two-daemon P2P query test verifies successful end-to-end query and citation resolution.
- [ ] Live 3-turn continuation SSE test (`"tell me about Angela Davis"` -> `"continue"` -> `"continue"`) completes on `http://127.0.0.1:3031` with `gemma4:26b` without duplicate chunks or repeated boilerplate clauses.
- [ ] Verified changes are committed and pushed via `scripts/solo_pr_ship.ps1` with a clean working tree.

## Verification Resources
- Cargo test suite: `cargo test`
- Solo PR shipping script: `powershell -ExecutionPolicy Bypass -File scripts/solo_pr_ship.ps1 -Title "..."`
- Live two-daemon P2P query test
- Live 3-turn continuation SSE verification:
  1. `"tell me about Angela Davis"`
  2. `"continue"`
  3. `"continue"`


## Follow-up — 2026-11-03T13:06:03Z

User requirement update (must be included in R1 / Acceptance Criteria):
"the number of peers one host should get response from must be configurable"

Please ensure the team implements configurable `max_peer_responses` end-to-end:
1. **Backend Config, CLI & Persistence**:
   - Add `max_peer_responses: usize` (with a sensible production default, e.g., `8`, where `0` or explicit cap bounds how many remote peer responses a host collects/accepts per federated query) to `CoordinatorConfig` (`crates/quorum-coordinator/src/config.rs`), `OsoosiMeshSettings` (`crates/quorum-coordinator/src/dlp_mesh.rs`, persisted in `settings.json`), and CLI (`--max-peer-responses` in `crates/quorum-daemon/src/cli.rs`).
2. **Runtime Scatter-Gather / Mesh Enforcement**:
   - Enforce `max_peer_responses` in `ScatterGatherEngine` / `SwarmMeshDispatcher::query_peers` (`crates/quorum-coordinator/src/scatter_gather.rs` & `crates/quorum-coordinator/src/mesh.rs`) so once a host receives `max_peer_responses` valid non-empty/verified peer responses (or caps concurrent peer responses per query to the configured value), it stops waiting / caps accepted peer responses deterministically instead of collecting from unbounded peers.
3. **Interactive Editable UI Control + Immediate Persistence**:
   - Expose `max_peer_responses` as an interactive `<input type="number" min="1" ...>` in the Web UI Mesh / Settings configuration (`crates/quorum-ui/assets/index.html` & `crates/quorum-ui/assets/app.js`) wired to the settings API so user edits immediately update live daemon state and persist to `settings.json`.
4. **Tests**:
   - Add unit/integration tests proving changing `max_peer_responses` (e.g., `1` vs `2` when querying 3 peers) strictly caps the number of peer responses fused per query.
