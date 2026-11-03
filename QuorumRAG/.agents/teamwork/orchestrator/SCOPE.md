# Scope: QuorumRAG P2P Mesh Gaps, Continuation Deduplication & Service Production Hardening

## Architecture
QuorumRAG adaptation adds Google LangExtract candidate extraction and 4-arm consensus to chat box queries, question-restating LLM synthesis over retrieved RAG chunks with citations strictly at the end, dynamic LLM spellcheck, response action buttons, clean UI, OshoosiClaw Mesh Configuration UI and TPM 2.0 hardware identity & attestation, and port 3031 live hosting.
In Milestones M15-M17, QuorumRAG closes the 4 real cross-peer P2P mesh gaps (persistent Ed25519 node identity across libp2p and ScatterGatherEngine, federated entity & cue-term supplements in RetrieveMeshNode, engine-mode contract differentiation between deterministic offline BLAKE3 and neural Gemma models, and peer/filename disambiguation across fusion, offline engine, and Web UI), implements configurable max_peer_responses across backend, CLI, and Web UI, eliminates multi-turn continuation repetition (session-level seen chunk tracking and negative filtering, pre-retrieval query rewriting node targeting unvisited aspects, MMR retrieval diversity & 0.88 cosine deduplication, and state-aware synthesis prompt with cross-citation entity consolidation), verifies end-to-end with cargo test, live 2-daemon P2P test, and live 3-turn continuation SSE test, hot-restarts the port 3031 daemon, and ships via solo PR.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 58 | Google LangExtract Chat Extraction | Extract question spans, document references, named entities & technical terms from chat box queries; verify char intervals; run 4-arm consensus | M11 | ORIGINAL_REQUEST Section 1 |
| 59 | Question-Restating LLM Orchestration & End Citations | Orchestrate answers only to questions using retrieved RAG chunks, restating questions affirmatively ("Who is femi" -> "Femi is ..."), grounding in resume/document chunks, placing URLs/doc links strictly at the end with relevance scores | M11 | ORIGINAL_REQUEST Section 2 |
| 60 | Clean WebUI & Logo Retention | Remove home screen prompt chips while keeping top page QuorumRAG logo visible across all views | M12 | ORIGINAL_REQUEST Section 3 |
| 61 | Dynamic Real-Time Chat Box Spellcheck | Debounced real-time LLM-assisted spellcheck in chat box via local Ollama with deterministic dictionary fallback, preserving cursor position | M12 | ORIGINAL_REQUEST Section 3 |
| 62 | Response Action Buttons & server.rs Fix | Browser-style response action buttons (Copy, Regenerate, Read Aloud via Web Speech API, Feedback Thumbs) below citations; fix server.rs:115 bypass_cache: false | M12 | ORIGINAL_REQUEST Section 3, 5 |
| 63 | TPM 2.0 Hardware Identity & Attestation | Adapt OshoosiClaw TPM 2.0 inspection (<1ms Windows WMI registry + Linux sysfs + simulated fallback), PCR 0/7/16 composite digest, and X.509 EK DER parsing | M13 | ORIGINAL_REQUEST Section 4 |
| 64 | Attestation-Based Access Tier Gating | Dynamically authorize verified TPM 2.0 peers for higher tiers in PeerAclManager, software fallbacks to Tier 1, unverified to Tier 0 | M13 | ORIGINAL_REQUEST Section 4 |
| 65 | OshoosiClaw Mesh Configuration UI & Daemon Endpoints | Adapt OshoosiClaw Mesh UI into Settings ("Mesh & Shared Folders"): TPM RoT badge, connected peers with attestation badges, Pending Joins HITL approval queue, Quarantined Peers panel, and WAN multiaddr/DuckDNS settings; wire daemon REST endpoints | M13 | ORIGINAL_REQUEST Section 4 |
| 66 | Final Workspace Build, Port 3031 Live Server & Solo PR Ship | Verify $env:CARGO_INCREMENTAL = "0", all tests pass, keep quorumrag.exe running on 0.0.0.0:3031 accessible at http://10.0.0.3:3031 and http://127.0.0.1:3031, solo PR ship to main | M14 | ORIGINAL_REQUEST Section 5 |
| 67 | Action Buttons Bar: Continue, Save File & Clear | Add Continue, Save File (Blob download), and Clear buttons to .response-actions-bar in app.js and styles.css | M13.5 | Follow-up 2026-11-02 |
| 68 | Q&A Opening Restatement, Typo Map & Grounding Calibration | Enforce Q&A opening topic restatement for all prompts, expand typo map & seed words, apply file_hits * 160 filename title bonus, calibrate grounding accuracy scores (78%-99%), clean chunk boundary artifacts in offline_engine.rs | M13.5 | Follow-up 2026-11-02 |
| 69 | Port 3031 Live Server Hot-Restart | Stop PID 80036 (quorumrag_live.exe), recompile, and launch live daemon on 0.0.0.0:3031 in M14 | M14 | Follow-up 2026-11-02 |
| 70 | Persistent Ed25519 Node Identity | Store persistent 32-byte Ed25519 seed in data_dir/node_identity.seed, shared across NetworkConfig (libp2p) and ScatterGatherEngine, deriving identical PeerId and preventing query spoofing rejections | M15 | R1.1 |
| 71 | Federated Entity & Cue-Term Supplements | Add ScatterGatherEngine::search_federated_text and extend extra entity/cue searches in RetrieveMeshNode::execute to query remote mesh peers | M15 | R1.2 |
| 72 | Engine-Mode Contract Differentiation | Add engine_mode ("DeterministicOffline" vs "GemmaCandle") to EmbeddingContract and derive_contract_hash, preventing incompatible peers from completing handshakes | M15 | R1.3 |
| 73 | Peer and Filename Disambiguation | Preserve responder_peer_id in ScoredChunk.source_peer via RrfCombiner::fuse_rankings_with_peers, disambiguate citations by (source_peer, file_name) in offline_engine.rs and app.js | M15 | R1.4 |
| 74 | Configurable max_peer_responses | Add max_peer_responses to CoordinatorConfig, OsoosiMeshSettings, CLI (--max-peer-responses), runtime scatter-gather enforcement, and Web UI Mesh Settings input | M15 | Follow-up 2026-11-03 |
| 75 | Session-Level Seen Chunk Tracking & Negative Filtering | Maintain seen_chunk_ids in ChatSessionRegistry, pass as exclusion filter to StorageSearchQuery and search_hybrid_with_exclusion, skipping served chunks while allowing unseen chunks from the same document | M16 | R2.1 |
| 76 | Continuation Query Rewriting Node | Add pre-retrieval query rewriting node in agentic state graph that inspects prior turn outputs and rewrites continuation queries to target remaining unexplored aspects | M16 | R2.2 |
| 77 | Retrieval Diversity & Semantic Filtering | Apply Maximal Marginal Relevance (MMR) during candidate retrieval, and perform pre-synthesis pairwise cosine deduplication (>= 0.88) to drop boilerplate clauses | M16 | R2.3 |
| 78 | Synthesis Prompt State Awareness & Entity Consolidation | Inject ### STRICT EXCLUSION CONSTRAINTS block in generator prompt, and enforce cross-citation entity consolidation instruction to merge duplicate entities | M16 | R2.4 |
| 79 | Full Verification, Service Rebuild, Live Tests & Solo PR Ship | cargo test across all crates, live 2-daemon P2P query test, live 3-turn continuation SSE test against port 3031 with gemma4:26b, port 3031 daemon hot restart, and solo PR ship | M17 | R3 |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M11 | Google LangExtract & Question-Restating LLM Orchestration | Features 58, 59 | none | DONE |
| M12 | WebUI Clean Interface, Dynamic Spellcheck & Action Buttons | Features 60, 61, 62 | none | DONE |
| M13 | OshoosiClaw Mesh Configuration UI & TPM Hardware Identity | Features 63, 64, 65 | M11, M12 | DONE |
| M13.5 | Live Testing Feedback: Action Buttons & Q&A Grounding Calibration | Features 67, 68 | M13 | DONE |
| M14 | Final Verification, Port 3031 Hot-Restart Live Daemon & Solo PR Ship | Features 66, 69 | M11, M12, M13, M13.5 | DONE |
| M15 | Cross-Peer P2P Mesh Gaps & Configurable Max Peer Responses | Features 70, 71, 72, 73, 74: Persistent Ed25519 node seed, federated entity/cue searches, engine-mode contract differentiation, (peer, file) disambiguation in fusion/offline_engine/app.js, configurable max_peer_responses end-to-end | M14 | DONE |
| M16 | Multi-Turn Continuation Deduplication & State Graph Pipeline | Features 75, 76, 77, 78: Session seen_chunk_ids tracking & negative filtering, pre-retrieval facet rewriting node, MMR retrieval diversity & 0.88 cosine dedup, state-aware synthesis prompt & entity consolidation | M15 | IN_PROGRESS |
| M17 | Verification, Service Rebuild, Live Two-Daemon & SSE Tests & Solo PR Ship | Feature 79: Full cargo test suite, live 2-daemon P2P peer query test, live 3-turn continuation SSE test against port 3031 with gemma4:26b, port 3031 daemon hot restart, solo PR ship to main | M15, M16 | PLANNED |

## Interface Contracts

### `quorum-network` ↔ `quorum-daemon`
- `load_or_generate_node_seed(data_dir: &Path) -> Result<[u8; 32], std::io::Error>`: Reads or creates `<data_dir>/node_identity.seed`.
- `NetworkConfig`:
  - `ed25519_seed: Option<[u8; 32]>`
  - `contract_hash_hex: String`
- `start_network_node(config, handler)` uses the persistent seed to generate `Keypair`, deriving the identical `PeerId` used by `ScatterGatherEngine`.

### `quorum-contract` ↔ P2P Handshake
- `EmbeddingContract`:
  - `pub engine_mode: String`: "DeterministicOffline" vs "GemmaCandle".
  - `canonical_deterministic_offline() -> Self`
  - `canonical_gemma_300m() -> Self`
  - `derive_contract_hash(&self) -> [u8; 32]`: Hashes `engine_mode` into the 32-byte BLAKE3 contract hash so disparate engine modes fail handshake with `ContractMismatch`.

### `quorum-coordinator::scatter_gather` ↔ `quorum-coordinator::fusion`
- `search_federated_text(&self, query_text: &str, access_tier: u8, top_k: usize) -> Vec<ScoredChunk>`: Federated text search dispatched across local storage and remote mesh peers.
- `RrfCombiner::fuse_rankings_with_peers(local: Vec<ScoredChunk>, remote: Vec<(String, Vec<CandidateChunkDto>)>, k: f32, top_k: usize) -> Vec<ScoredChunk>`: Preserves real `responder_peer_id` into `ScoredChunk.source_peer`.
- `deduplicated_chunks_with_scores`: Deduplicates by `(source_peer, file_name)`.
- Citations format: `[📄 file_name (Peer: <short_peer_id>)]` when `source_peer != "local"`.

### `quorum-daemon` ↔ `quorum-coordinator` ↔ `quorum-storage` (Session Seen Chunks)
- `ChatSessionRegistry`: Thread-safe session cache storing `seen_chunk_ids: HashSet<String>`.
- `ChatRequest`: Adds `session_id: Option<String>`, `seen_chunk_ids: Vec<String>`.
- `AgenticGraphState`: Adds `seen_chunk_ids: HashSet<String>`.
- `StorageSearchQuery`: Adds `exclude_chunk_ids: Vec<String>`.
- `StorageEngine::search_hybrid_with_exclusion`: Filters out chunks in `exclude_chunk_ids` while allowing unseen chunks from the same document.

### `quorum-coordinator::agentic` ↔ Pipeline
- `RewriteContinuationNode`: Pre-retrieval node that extracts covered facets from prior turn outputs and rewrites continuation queries to target unvisited aspects.
- `apply_mmr_selection`: Penalizes high cosine similarity across candidates in `RetrieveMeshNode`.
- `deduplicate_chunk_segments_against_prior`: Pairwise sentence cosine similarity check against previous turns, dropping segments with similarity >= 0.88.
- Generator Prompt: Injects `### STRICT EXCLUSION CONSTRAINTS (DO NOT REPEAT)` and Rule 6 (Cross-Citation Entity Consolidation).

### Web UI (`app.js` & `index.html`)
- Mesh Settings: Interactive `<input type="number" min="1" max="64" id="mesh-max-peer-responses">` wired to `POST /api/mesh/config`.
- Citations: Disambiguated by `(peer_id + "::" + file_name)`.
- Chat Session: Persistent `activeSessionId` and `activeSessionSeenChunkIds` sent with `/api/chat`.
