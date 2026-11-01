# Dispatch Log

## 2026-10-31T14:56:22Z
[Message] timestamp=2026-10-31T14:56:22Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=Build QuorumRAG, a production-grade local-first peer-to-peer retrieval-augmented generation daemon and simple ChatGPT-like web interface, adapting useful code from `d:\harfile` (including `d:\harfile\fusion` and IDE interface code) with Gemma Embedding contract and embedded LanceDB hybrid storage.

Working directory: d:\harfile\QuorumRAG
Integrity mode: development

## Requirements

### R1. High-Capacity Ingestion & Isolated Storage Engine
Build a high-throughput filesystem ingestion engine and embedded LanceDB storage layer capable of scaling to 30TB+ drives. Encapsulate all `lancedb` and `arrow-array` `RecordBatch` conversions strictly inside the storage module so raw Arrow types never leak into ingestion, networking, or UI layers. Ingestion must use `notify` and `ignore` with configurable binary and extension filters, file size limits, two-level BLAKE3 chunk deduplication (skipping redundant embeddings for unchanged or moved files), micro-batched writes, scheduled IVF_PQ and Tantivy full-text index compaction, and configurable `.lance` index storage paths.

### R2. Gemma Embedding Contract & Offline Execution
Implement a deterministic mesh-wide embedding contract using Gemma Embedding (`EmbeddingGemma` / `google/embeddinggemma-300m` or local Gemma embedding runtime) operating strictly offline without external network dependencies. Compute a deterministic BLAKE3 contract hash over embedding dimensions, normalization settings, and distance metrics, rejecting any peer during network handshake whose contract hash diverges.

### R3. rust-libp2p Mesh Network & Federated Retrieval
Implement a complete, compilable `rust-libp2p` Swarm with QUIC/TCP transports, Noise encryption, Yamux multiplexing, mDNS LAN discovery, Kademlia DHT, GossipSub query broadcast, and Request-Response chunk streaming without stubbed handlers or mock loops. The query coordinator must dispatch parallel local and mesh queries with a 400ms timeout, enforce Ed25519 `access_tier` SQL pre-filtering inside LanceDB before vector scanning, aggregate candidate chunks via Reciprocal Rank Fusion (RRF), and hold remote peer chunks strictly in volatile `ZeroizeOnDrop` memory during synthesis.

### R4. ChatGPT-like Web Interface & End-to-End Wiring
Provide a clean, responsive ChatGPT-like web chat interface adapted from existing interface code in `d:\harfile\fusion` and `d:\harfile`. Wire the web UI directly to the local daemon and federated query coordinator, supporting real-time query submission, streaming responses, and structured document source citations (peer ID, file path, line numbers, and score).

### R5. Automated Verification & Test Harness
Provide comprehensive, automated test suites verifying all core components end-to-end, including Arrow type boundary isolation, BLAKE3 deduplication and ingestion filtering, Gemma embedding contract verification, multi-node libp2p Swarm discovery and federated RRF query execution under simulated network latency, and web UI API endpoints.

## Acceptance Criteria

### Storage & Ingestion
- [ ] Raw `arrow-array` and `lancedb` types are strictly confined to the storage crate and do not appear in the public APIs or signatures of the ingestion, networking, or UI crates.
- [ ] Ingestion pipeline filters excluded files, enforces file size caps, computes two-level BLAKE3 hashes, and avoids recomputing embeddings for duplicate or moved chunks.
- [ ] LanceDB table initialization, vector indexing (IVF_PQ), Tantivy FTS indexing, and compaction routines execute successfully with configurable data paths.

### Embedding Contract & Offline Verification
- [ ] Gemma Embedding integration generates deterministic dense vectors locally without outbound network access.
- [ ] Contract hash function derives a deterministic BLAKE3 digest and rejects handshakes with mismatched configurations.

### Networking & Federated Search
- [ ] `rust-libp2p` Swarm compiles cleanly with QUIC/TCP, Noise, Yamux, mDNS, Kademlia, GossipSub, and Request-Response protocols without stubbed placeholder traits.
- [ ] Multi-node integration test validates peer discovery via mDNS and federated scatter-gather query execution across two nodes within 400ms.
- [ ] LanceDB pre-filtering strictly blocks unauthorized directories based on Ed25519 peer identity and access tier rules.
- [ ] RRF combiner correctly normalizes and merges local and remote candidate chunks, and remote chunks are zeroized on drop from memory.

### Web Interface & Integration
- [ ] Local web server launches and serves the ChatGPT-like chat interface in a browser.
- [ ] Submitting a prompt in the web interface returns streaming synthesis results with clickable or structured source citations referencing retrieved chunks.
- [ ] All unit and integration tests compile and pass via `cargo test`.


## 2026-10-31T15:05:22Z
[Message] timestamp=2026-10-31T15:05:22Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=Follow-up requirement update from user — please incorporate these mandatory requirements into the QuorumRAG architecture and implementation immediately:
1. **Search Any Local File**: The user must be able to search any local file across their machine/configured drives and ask anything about their files or general questions in the ChatGPT interface.
2. **Google Magika File Identification**: Use **Google Magika** (`magika` in Rust / deep-learning content-type detection alongside MIME/content inspection) in the ingestion pipeline (`quorum-ingest` / `raghome-ingest`) to accurately identify file types by content before routing to extractors/chunkers or filtering binaries.
3. **Ask Anything**: The ChatGPT web interface and synthesis router must let the user ask anything — seamlessly answering both specific local/mesh file questions (with citations) and general queries.
4. **Single `.exe` Launcher (Process + Auto-Launch Browser)**: Running the single compiled executable (`quorumrag.exe` / main daemon binary) must start the entire daemon + web UI server in one self-contained process (with embedded static web assets) AND automatically open the user's default web browser to the local ChatGPT interface URL upon startup (with a CLI flag like `--no-browser` for headless/test runs).

This has been recorded to ORIGINAL_REQUEST.md. Please ensure this is accounted for in the project scope and milestone breakdown.


## 2026-10-31T16:04:00Z
[Message] timestamp=2026-10-31T16:04:00Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=User requirement update — please integrate this mandatory architecture pattern into QuorumRAG:
"Keep the current Rust foundation for the daemon, Magika classifier, Gemma embedding contract, LanceDB hybrid store, rust-libp2p mesh, and single .exe launcher.
Adopt the LangGraph-style Agentic State Graph pattern in Rust inside the query/synthesis layer (an explicit state machine: ClassifyIntent -> RetrieveMesh -> GradeRelevance -> RewriteQuery (loop) -> SynthesizeWithCitations) so you get LangGraph's iterative multi-hop agentic reasoning without sacrificing the speed, memory safety, or single-binary simplicity of Rust."

This has been appended to `ORIGINAL_REQUEST.md` and `PROJECT.md`. Please ensure:
1. `quorum-coordinator` / synthesis layer implements this explicit state machine enum with a bounded loop (max 3 hops).
2. The ChatGPT web interface SSE stream emits live transition state telemetry (`ClassifyIntent -> RetrieveMesh -> GradeRelevance -> SynthesizeWithCitations`).


## 2026-10-31T16:13:09Z
[Message] timestamp=2026-10-31T16:13:09Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=User requirement update — please integrate these mandatory requirements immediately into active and upcoming milestones (M2 Storage, M3 Ingestion/Watcher, M4 Coordinator, M5 Web UI, and E2E tests):

1. **Full Search Mode Support (Keyword, Vector, and Hybrid Search)**:
   - Explicitly support all search modes in `quorum-storage`, `quorum-coordinator`, and the Web UI / API:
     - **Keyword / Lexical Search** (Tantivy BM25 exact/fuzzy keyword & filename search)
     - **Semantic Vector Search** (LanceDB dense Gemma embedding similarity search)
     - **Hybrid Search** (Combined BM25 + Dense Vector + Reciprocal Rank Fusion)
   - Allow users and the Agentic State Graph (`ClassifyIntent`) to execute or toggle between Keyword, Semantic, and Hybrid searches seamlessly.

2. **Real-Time OS File Watcher with Google Magika**:
   - There must be a live, continuous OS file watcher (`notify` crate watching configured directories/drives in real time for file creation, modification, rename, and deletion events).
   - Every new or modified file detected by the OS watcher must pass through **Google Magika** (`quorum-classifier`) for content-based file identification before routing to the appropriate extractor and indexing into LanceDB + Tantivy.

3. **Image Indexing Support (Do Not Skip Images!)**:
   - Images identified by Google Magika (`jpeg`, `png`, `webp`, `gif`, `bmp`, `tiff`, `svg`, etc.) MUST be indexed rather than skipped as generic binaries!
   - Update `quorum-classifier` and `quorum-ingest` so image files are routed to an **Image Extractor / Indexer** that extracts rich searchable representations (image format/MIME via Magika, dimensions, file path/name tokens, EXIF/IPTC/XMP metadata, embedded text/OCR or visual feature summary/caption, and BLAKE3 content hash) and indexes them into LanceDB + Tantivy so users can search for and ask questions about local images in the ChatGPT interface.

This has been recorded in `ORIGINAL_REQUEST.md` and `PROJECT.md`. Ensure `worker_m2_1` and subsequent milestone workers account for these invariants.


## 2026-10-31T16:14:14Z
[Message] timestamp=2026-10-31T16:14:14Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=User requirement update — please integrate these mandatory requirements immediately into active and upcoming milestones:

"a process myst exit to reindex and another one to fix drifts all types of drifts"

Specifically, implement two dedicated, first-class processes/subsystems (accessible as background daemon workers, CLI commands, and Web UI / REST API actions):

1. **Dedicated Reindex Process (`reindex`)**:
   - A dedicated reindexing process/worker (`quorumrag reindex [--path <dir>] [--full]`, background task, and `/api/reindex` endpoint + Web UI trigger) that traverses target directories using `ignore` + Google Magika, recomputes/verifies two-level BLAKE3 hashes, and reindexes new or forced files into LanceDB + Tantivy.

2. **Dedicated Drift Remediation Process (`fix-drift`) Covering ALL Types of Drifts**:
   - A dedicated drift-healing process/worker (`quorumrag fix-drift`, scheduled background drift monitor/healer, and `/api/drift/fix` endpoint + Web UI status/trigger) that detects and repairs **all types of drifts**:
     a. **Filesystem <-> Database Drift (Merkle State Drift)**: Files created, modified, moved, or deleted on disk while the watcher was offline or missed under heavy I/O — purging orphaned chunks for deleted files and indexing missing/modified files.
     b. **Hybrid Index Drift (LanceDB Vector <-> Tantivy BM25 Desync)**: Records present in LanceDB but missing or mismatched in the Tantivy BM25 full-text index (or vice versa) — cross-checking `chunk_id`/`chunk_hash` sets and healing both indexes into 100% parity.
     c. **Embedding Contract / Vector Drift**: Stored chunks whose `contract_hash` or vector dimension/L2-norm diverges from the active Gemma Embedding contract — automatically re-embedding drifted chunks in place.
     d. **ACL / Access-Tier Policy Drift**: When directory `access_tier` rules (`private`, `household_mesh`, `allowlist`) change in config, detecting chunks whose stored `access_tier` metadata no longer matches their directory policy and updating their metadata atomically.
     e. **Storage Fragmentation & Tombstone Drift**: Detecting uncompacted LanceDB fragments, deleted row tombstones, and unmerged Tantivy segments, and executing compaction/optimization.

This has been recorded in `ORIGINAL_REQUEST.md` and `PROJECT.md`. Ensure these two subsystems are implemented, wired to CLI/API/UI, and covered by automated tests.


## 2026-10-31T16:19:44Z
[Message] timestamp=2026-10-31T16:19:44Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=User requirement update — please incorporate this mandatory requirement immediately into `PROJECT.md` and active/upcoming milestones (M4 Coordinator/Synthesis, M5 Web UI/Launcher, and E2E tests):

"fusion in the fusin folder must be adopted and GLM models be used when system resource are available like the code in the fusion folder"

Specifically:
1. **Direct Adoption of `d:\harfile\fusion` Architecture & Code**: Inspect and directly adapt the fusion orchestration, model routing, hardware resource probing, and UI/synthesis logic from `d:\harfile\fusion` into QuorumRAG (`quorum-coordinator` / synthesis layer / `quorum-daemon` / web UI).
2. **Hardware-Aware GLM Model Selection & Execution**: Replicate and adopt the hardware-resource-aware model selection from `d:\harfile\fusion` so that **GLM models** (such as `glm-4`, `glm-4-9b`, `glm-edge`, `glm-4v`, or the exact GLM model tiers defined in `d:\harfile\fusion`) are dynamically selected and used for agentic state graph reasoning and answer synthesis whenever system resources (available free RAM / free VRAM) are sufficient, with seamless hardware-aware tiering and status reporting in the UI.

This has been recorded in `ORIGINAL_REQUEST.md` and `PROJECT.md`. Please ensure the coordinator and synthesis modules adopt the fusion codebase logic and wire the GLM hardware-aware selection rules.


## 2026-10-31T16:34:59Z
[Message] timestamp=2026-10-31T16:34:59Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=User requirement update — please integrate this multi-model architecture into `PROJECT.md` and active/upcoming milestones (M4 Coordinator/Synthesis, M5 Web UI/Launcher, and E2E tests):

1. **Immutable Embedding Contract**: Keep `google/embeddinggemma-300m` strictly locked as the immutable mesh-wide embedding contract for LanceDB.
2. **Multi-Model Synthesis Roles**:
   - **GLM-4-9B (`glm-4-9b-chat` / `glm4`)**: Default synthesis & multi-step agentic reasoning model in `quorumrag.toml`, providing massive context headroom (128K–1M tokens) and multi-hop state-graph reasoning.
   - **Qwen 2.5 (`qwen2.5:32b` / `14b` / `7b` / `3b` / `1.5b` sized by runtime free RAM/VRAM)**: Specialized for code-dominated RAG queries.
   - **Gemma 2 / Gemma 3 (`gemma3:27b` / `gemma3:9b` / `gemma2:9b`)**: Unified Google stack model for fluent natural English prose.
3. **Adaptive Intent + Hardware Fusion Router in `ClassifyIntent` Node**:
   - Implement `SynthesisModelFamily::{Glm4Agentic, Qwen25Code, GemmaProse, AutoFusion}`.
   - In `AutoFusion` mode, `ClassifyIntent` evaluates both runtime free RAM/VRAM AND query/chunk types (e.g. Magika `Code` AST chunks -> Qwen 2.5; multi-hop/multi-document context -> GLM-4-9B 128K; prose/general QA -> Gemma 3 or GLM-4-9B), emitting selected model family and rationale in SSE headers.
4. **Hot-Swappable Web UI Pill Bar**: Live model selector pill bar in the ChatGPT Web UI allowing users to switch models or keep `Auto-Fusion`.

This has been recorded in `ORIGINAL_REQUEST.md` and `PROJECT.md`.


## 2026-10-31T22:52:11Z
[Message] timestamp=2026-10-31T22:52:11Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=Mandatory User Requirement Update — Experimental Protect Env Menu & Invariants
Specifically, add an Experimental → Protect Env (OS Environment & PATH Protection Guard) menu/panel in the ChatGPT Web UI and daemon REST API (/api/experimental/env-guard):
1. UI Menu (Experimental: Protect Env):
   - Add an interactive Experimental → Protect Env menu item and modal/panel in the ChatGPT web interface sidebar/header.
   - Allow user to toggle/configure experimental environment protection rules:
     - Zero-Touch PATH Guard: Monitors and verifies PATH integrity (computes a BLAKE3 snapshot of process & user environment variables in memory/local .env backup without EVER mutating HKCU\Environment or HKLM registry).
     - Secret / .env Ingestion Redaction & Quarantine: Ensures .env, .env.*, API keys, tokens, and credential files are automatically protected/redacted from mesh broadcast and protected from accidental exposure during RAG indexing.
     - Process-Scoped Environment Sandbox: Enforces process-scoped environment isolation (std::env read-only snapshot & drift alert if any external tool attempts to truncate or alter PATH or sensitive environment variables).
     - Live Env Integrity Status & Audit Log: Shows the current BLAKE3 environment fingerprint, variable count, PATH entry health check, and one-click "Snapshot / Verify Env Integrity" button in the UI.
2. Strict Safety Invariant: Must strictly obey the Universal Zero-Touch PATH & Environment Protection Protocol — never write to HKCU\Environment, HKLM, setx, or [Environment]::SetEnvironmentVariable(..., 'User'/'Machine'); all snapshots and backups are stored strictly in local QuorumRAG state/workspace files.


## 2026-10-31T22:53:01Z
[Message] timestamp=2026-10-31T22:53:01Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=Requirement Rescission Notice — Protect Env Menu
User directive: "this copte does not need protect env proceed"
Disregard and remove the protect env menu requirement from Milestone 5 plans and specifications. Proceed directly with Milestone 4 verification and standard Milestone 5 implementation without the experimental protect env menu.


## 2026-10-31T23:03:13Z
[Message] timestamp=2026-10-31T23:03:13Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=Mandatory User Requirement Update — Fusion-Powered Google LangExtract (langextract_fusion)
User directive: "can this be added but it should use fusion [google/langextract]"
Specifically, integrate Fusion-Powered Google LangExtract (langextract_fusion) into QuorumRAG:
1. Core LangExtract Data Model & Grounding Engine (quorum-types / quorum-coordinator::langextract):
   - Extraction: { extraction_class: String, extraction_text: String, char_interval: Option<(usize, usize)>, attributes: BTreeMap<String, String>, source_chunk_id: Option<String>, source_file_path: Option<String> }
   - ExampleData: { text: String, extractions: Vec<Extraction> } (few-shot with alignment verification)
   - Precise Source Grounding (char_interval verification and hallucination filtering when require_grounding = true)
   - Multi-pass & long-document chunking support (extraction_passes, max_char_buffer)
2. Powered by ModelFusion (d:\harfile\fusion Hardware-Aware Router):
   - Local Ollama structured JSON schema extraction (AutoFusion, GLM-4-9B, Qwen 2.5, Gemma 2/3) + deterministic grounded span extractor fallback when offline
   - Integration into SynthesizeWithCitations node for grounded source span highlights
3. Interactive Visualization & ChatGPT Web UI Integration (quorum-ui / quorum-daemon):
   - REST API: POST /api/langextract (AnnotatedDocument + .jsonl export) and POST /api/langextract/visualize (interactive color-coded HTML visualization)
   - ChatGPT Web UI: LangExtract action/toggle and @extract command for structured grounded extraction and visualization


## 2026-10-31T23:04:03Z
[Message] timestamp=2026-10-31T23:04:03Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=Architectural Clarification — LangExtract Ingestion-Time Preprocessor & Query-Time Decomposition Loops
1. Ingestion-Time Structured Extraction & 100% char_interval Grounding Gate:
   - LangExtract acts as structured extraction and source-grounding preprocessor during ingestion (quorum-ingest / langextract_fusion).
   - Extracts typed entities, verbatim spans, and key-value attributes (CVEs, arXiv IDs, function/struct symbols, syscalls, technical parameters).
   - Strict char_interval verification: every extraction is mapped to exact (start_char, end_char) offset; ungrounded hallucinations (char_interval = None) are strictly filtered out so 100% of stored indicators/entities in LanceDB + Tantivy are source-grounded.
2. Query-Time Decomposition & Scalar/Lexical Prefiltering (ClassifyIntent -> RetrieveMesh):
   - At query time, ClassifyIntent runs a fast LangExtract pass over user prompt to extract structured filter criteria (file paths, CVE/arXiv IDs, symbols, attributes).
   - Extracted grounded entities are passed into RetrieveMesh (quorum-storage LanceDB + Tantivy hybrid search) as structured scalar/keyword constraints alongside dense embeddinggemma-300m vector.
Both loops incorporated across quorum-ingest, quorum-storage, quorum-coordinator, and quorum-ui.


## 2026-10-31T23:07:30Z
[Message] timestamp=2026-10-31T23:07:30Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=Exact Code Reference & Architectural Pattern — LlmFusionConsensus Multi-Arm Evaluation
Reference Path: D:\harfile\OshoosiClaw\crates\osoosi-agent-anomaly\src\breakout_defense.rs (lines 3440–3697) and C:\Users\oyesanyf\.gemini\config\plugins\arxiv-langextract\skills\arxiv-langextract\SKILL.md.
Adapt LlmFusionConsensus / LlmFusionArmReport multi-arm consensus architecture directly into QuorumRAG's langextract_fusion engine:
- Arm 1: SemanticSpanExtractor ("Character-Span & Semantic Grounding Extractor"): schema-constrained extraction, locating verbatim spans with strict char_interval: Some((start_char, end_char)) byte/char offset verification (rejecting ungrounded extractions where char_interval.is_none()).
- Arm 2: SyntacticAstVerifier: verifies structural syntax, symbol identifiers, schema attribute alignment without corruption or truncation.
- Arm 3: AdversarialCounterfactualJudge: rejects hallucinated few-shot echo patterns and verifies extracted spans belong to active document.
- Arm 4: LangGraph State Calibration: evaluates composite confidence_bp (basis points) and feeds structured indicators/entities into ClassifyIntent -> RetrieveMesh scalar/lexical prefiltering and SynthesizeWithCitations grounded highlights across local multi-model setup (GLM-4-9B, Qwen 2.5, Gemma 3).
Incorporate into crates/quorum-coordinator::langextract and ensure implementations reference this design.


## 2026-10-31T23:58:53Z
[Message] timestamp=2026-10-31T23:58:53Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=Mandatory User Requirement Update — Live IDE Execution & Production Code Generation Gate
User directive: "test it run it on the ide once dont to see it it generates production code"

Specifically, once Milestone 5 (`quorumrag.exe` single binary + ChatGPT Web UI + LangExtract + Multi-Model Fusion Router) is compiled:
1. **Launch and Run `quorumrag.exe` Live in the Workspace/IDE**:
   - Start the actual compiled `quorumrag.exe` binary against the local workspace (`d:\harfile\QuorumRAG` / `d:\harfile\fusion`), indexing real Rust source files.
2. **Live Production Code Generation & Grounded RAG Test**:
   - Query the live running daemon / ChatGPT Web UI API (`/api/chat` / `/api/query` / `/api/langextract`) with real coding & architecture prompts to verify that it retrieves grounded AST/code chunks, runs the 5-node LangGraph Agentic State Graph + Multi-Model Fusion Router (`GLM-4-9B` / `Qwen 2.5` / `Gemma 3` / offline code synthesizer), and **generates real, compilable production code with exact `[Peer:File:Lines]` citations and grounded `char_interval` spans**.
   - Also launch the ChatGPT Web UI in the browser so the user can see and interact with it live in the IDE/browser!

## 2026-11-01T03:52:07Z
[Message] timestamp=2026-11-01T03:52:07Z sender=f7bd8b12-72d0-436b-aec9-b411d44685ae priority=MESSAGE_PRIORITY_HIGH content=User requirement update: "the code needs to have caching and re cache"

Specifically, implement first-class Caching and Re-Cache (`recache`) across the coordinator, ingestion/storage layer, daemon CLI/REST API, and ChatGPT Web UI:

1. **Multi-Layer BLAKE3 Caching Engine (`QueryCache` + `EmbeddingCache`)**:
   - **Query, Retrieval & Synthesis Cache**: Cache query embeddings, hybrid/keyword/vector search results, LangExtract grounded extractions, and synthesized responses keyed by a deterministic BLAKE3 digest of `(normalized_query, search_mode, model_family, access_tier, index_epoch)` with configurable TTL & LRU/capacity bounds.
   - **Cache Hit Telemetry**: Expose `cache_hit: bool`, `cache_hits: u64`, `cache_misses: u64`, `cached_entries: usize`, and `cache_epoch: u64` in `/api/chat`, `/api/query`, `/api/health`, and `GET /api/cache/stats` (while respecting zero-trust unpinned remote peer chunk `ZeroizeOnDrop` memory rules).

2. **Dedicated Re-Cache Process, API & Web UI Control (`recache`)**:
   - **CLI Command**: `quorumrag recache [--clear] [--warm]` to flush stale caches, increment cache epoch, synchronize with LanceDB + Tantivy index state, and pre-warm hot embeddings/queries.
   - **REST API Endpoints**:
     - `POST /api/cache/recache`: Invalidates stale cache entries, increments `cache_epoch`, rebuilds/warms active embedding & query cache from current index, returning `{ status, cache_epoch, cleared_entries, warmed_entries, duration_ms }`.
     - `GET /api/cache/stats`: Returns live cache metrics (`hits`, `misses`, `hit_rate`, `cached_queries`, `cached_embeddings`, `cache_epoch`).
   - **Automatic Cache Coherence**: Automatically invalidate/re-cache affected entries when `reindex`, `fix-drift`, or the live OS file watcher modifies indexed files.
   - **ChatGPT Web UI Integration**: Add a **"Re-Cache"** button and live Cache Hit / Cache Stats indicator in the ChatGPT Web UI sidebar/header so the user can inspect cache status and trigger a full re-cache with one click.
