# Original User Request

## Initial Request — 2026-10-31T14:55:11Z

Build QuorumRAG, a production-grade local-first peer-to-peer retrieval-augmented generation daemon and simple ChatGPT-like web interface, adapting useful code from `d:\harfile` (including `d:\harfile\fusion` and IDE interface code) with Gemma Embedding contract and embedded LanceDB hybrid storage.

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


## Follow-up — 2026-10-31T15:04:53Z

User requirement update — please incorporate these mandatory requirements into the QuorumRAG architecture and implementation immediately:
1. **Search Any Local File**: The user must be able to search any local file across their machine/configured drives and ask anything about their files or general questions in the ChatGPT interface.
2. **Google Magika File Identification**: Use **Google Magika** (`magika` in Rust / deep-learning content-type detection alongside MIME/content inspection) in the ingestion pipeline (`quorum-ingest` / `raghome-ingest`) to accurately identify file types by content before routing to extractors/chunkers or filtering binaries.
3. **Ask Anything**: The ChatGPT web interface and synthesis router must let the user ask anything — seamlessly answering both specific local/mesh file questions (with citations) and general queries.
4. **Single `.exe` Launcher (Process + Auto-Launch Browser)**: Running the single compiled executable (`quorumrag.exe` / main daemon binary) must start the entire daemon + web UI server in one self-contained process (with embedded static web assets) AND automatically open the user's default web browser to the local ChatGPT interface URL upon startup (with a CLI flag like `--no-browser` for headless/test runs).


## Follow-up — 2026-10-31T16:03:17Z

User update to add to the team:
"Keep the current Rust foundation for the daemon, Magika classifier, Gemma embedding contract, LanceDB hybrid store, rust-libp2p mesh, and single .exe launcher.
Adopt the LangGraph-style Agentic State Graph pattern in Rust inside the query/synthesis layer (an explicit state machine: ClassifyIntent -> RetrieveMesh -> GradeRelevance -> RewriteQuery (loop) -> SynthesizeWithCitations) so you get LangGraph's iterative multi-hop agentic reasoning without sacrificing the speed, memory safety, or single-binary simplicity of Rust."

Please append this to `ORIGINAL_REQUEST.md` and `PROJECT.md`, and integrate this explicit Rust Agentic State Graph (`ClassifyIntent -> RetrieveMesh -> GradeRelevance -> RewriteQuery (loop) -> SynthesizeWithCitations`) into the query/synthesis layer and web UI streaming/telemetry!


## Follow-up — 2026-10-31T16:12:21Z

User requirement update — please incorporate these mandatory requirements immediately into `ORIGINAL_REQUEST.md`, `PROJECT.md`, and active milestones (M2 Storage, M3 Ingestion/Watcher, M4 Coordinator, M5 Web UI, and E2E tests):

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


## Follow-up — 2026-10-31T16:13:27Z

User requirement update — please incorporate these mandatory requirements immediately into `ORIGINAL_REQUEST.md`, `PROJECT.md`, and active milestones:

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


## Follow-up — 2026-10-31T16:19:03Z

User requirement update — please incorporate this mandatory requirement immediately into `ORIGINAL_REQUEST.md`, `PROJECT.md`, `BRIEFING.md`, and the active milestones:

"fusion in the fusin folder must be adopted and GLM models be used when system resource are available like the code in the fusion folder"

Specifically:
1. **Direct Adoption of `d:\harfile\fusion` Architecture & Code**: Inspect and directly adapt the fusion orchestration, model routing, hardware resource probing, and UI/synthesis logic from `d:\harfile\fusion` into QuorumRAG (`quorum-coordinator` / synthesis layer / `quorum-daemon` / web UI).
2. **Hardware-Aware GLM Model Selection & Execution**: Replicate and adopt the hardware-resource-aware model selection from `d:\harfile\fusion` so that **GLM models** (such as `glm-4`, `glm-4-9b`, `glm-edge`, `glm-4v`, or the exact GLM model tiers defined in `d:\harfile\fusion`) are dynamically selected and used for agentic state graph reasoning and answer synthesis whenever system resources (available free RAM / free VRAM) are sufficient, with seamless hardware-aware tiering and status reporting in the UI.


## Follow-up — 2026-10-31T16:34:03Z

User requirement update — please adapt and improve this multi-model architecture into `ORIGINAL_REQUEST.md`, `PROJECT.md`, `BRIEFING.md`, and Milestone 4/5 (`quorum-coordinator`, `quorum-daemon`, `quorum-ui`):

"adapt and improve this idea:
1. **For Embeddings (`embeddinggemma-300m` vs. GLM)**: Keep `google/embeddinggemma-300m` locked as the immutable mesh-wide embedding contract for LanceDB so vector spaces remain mathematically identical across every node.
2. **For Local Chat & RAG Synthesis (GLM-4-9B vs. Qwen 2.5 vs. Gemma 2 / 3)**:
   - **GLM-4-9B (`glm-4-9b-chat` / `glm4`)**: Default synthesis & multi-step agentic reasoning model when hardware resources allow, providing massive context headroom (128K–1M tokens) and strong multi-hop state-graph reasoning.
   - **Qwen 2.5 (`qwen2.5:32b` / `14b` / `7b` / `3b` / `1.5b` sized dynamically by runtime free RAM / free VRAM)**: Optimal when RAG queries are heavily dominated by Rust, Python, C++, and TypeScript source code AST chunks.
   - **Gemma 2 / Gemma 3 (`gemma3:27b` / `gemma3:9b` / `gemma2:9b`)**: Optimal for a unified Google Gemma stack alongside `embeddinggemma-300m` with crisp, natural English prose.
3. **Architectural Improvement — Intent + Hardware Fusion Router & Hot-Swappable Config/UI**:
   - Because the synthesis layer talks to a local OpenAI-compatible endpoint (`127.0.0.1:11434/v1` or `llama-server`) with offline deterministic synthesis fallback when no external server is running:
     a. Lock `google/embeddinggemma-300m` as the immutable mesh embedding contract.
     b. Set `glm-4-9b-chat` (`glm4`) as the default synthesis model in `quorumrag.toml`, with full support for swapping or auto-routing across **GLM-4**, **Qwen 2.5**, and **Gemma 2/3** via `quorumrag.toml` AND via a live model selector pill bar in the ChatGPT Web UI.
     c. **Improve the idea in the Agentic State Graph (`ClassifyIntent` node)**: Add an **Adaptive Intent + Hardware Fusion Router** (`SynthesisModelFamily::{Glm4Agentic, Qwen25Code, GemmaProse, AutoFusion}`). In `AutoFusion` mode, `ClassifyIntent` inspects both runtime free RAM/VRAM AND the query/retrieved chunk types (e.g., Magika `Code` AST chunks -> routes to Qwen 2.5; massive multi-document/multi-hop context -> routes to GLM-4-9B 128K; prose/notes/general QA -> routes to Gemma 3 or GLM-4-9B), reporting the selected model family and reasoning reason in the UI citation/telemetry header!"


## Follow-up — 2026-10-31T18:50:33Z

URGENT SYSTEM RESOURCE & SERVER RESTART DIRECTIVE:
1. Preserve `[profile.dev] debug = 0` and `[profile.test] debug = 0` in `Cargo.toml` and `.cargo/config.toml` (`jobs = 2`).
2. Never run `cargo test` concurrently across 5 subagents at the same time — have 1 subagent run the test suite with `-- --test-threads=2` and let reviewers inspect the code/logs, or stagger test execution sequentially.
3. Never invoke `taskkill.exe`; let `cargo` commands finish within a sufficient timeout (`WaitMsBeforeAsync: 10000`).
4. Revive Project Orchestrator, re-schedule Sentinel progress/liveness crons, and continue execution from Milestone 2 Re-Gating into Milestones 3, 4, and 5.
## Follow-up — 2026-10-31T22:51:16Z

User requirement update — please incorporate this mandatory requirement immediately into `ORIGINAL_REQUEST.md`, `PROJECT.md`, `BRIEFING.md`, and Milestone 5 (`quorum-ui` / `quorum-daemon`):

"add experimental to protect env meu in the ui"

Specifically, add an **Experimental → Protect Env (OS Environment & PATH Protection Guard)** menu/panel in the ChatGPT Web UI and daemon REST API (`/api/experimental/env-guard`):
1. **UI Menu (`Experimental: Protect Env`)**:
   - Add an interactive **Experimental → Protect Env** menu item and modal/panel in the ChatGPT web interface sidebar/header.
   - Allow the user to toggle/configure experimental environment protection rules:
     - **Zero-Touch PATH Guard**: Monitors and verifies `PATH` integrity (computes a BLAKE3 snapshot of process & user environment variables in memory/local `.env` backup without EVER mutating `HKCU\Environment` or `HKLM` registry).
     - **Secret / `.env` Ingestion Redaction & Quarantine**: Ensures `.env`, `.env.*`, API keys, tokens, and credential files are automatically protected/redacted from mesh broadcast and protected from accidental exposure during RAG indexing.
     - **Process-Scoped Environment Sandbox**: Enforces process-scoped environment isolation (`std::env` read-only snapshot & drift alert if any external tool attempts to truncate or alter `PATH` or sensitive environment variables).
     - **Live Env Integrity Status & Audit Log**: Shows the current BLAKE3 environment fingerprint, variable count, PATH entry health check, and one-click "Snapshot / Verify Env Integrity" button in the UI.
2. **Strict Safety Invariant**: Must strictly obey the Universal Zero-Touch PATH & Environment Protection Protocol — never write to `HKCU\Environment`, `HKLM`, `setx`, or `[Environment]::SetEnvironmentVariable(..., 'User'/'Machine')`; all snapshots and backups are stored strictly in local QuorumRAG state/workspace files.

## Follow-up — 2026-10-31T22:52:25Z

User update: "this copte does not need protect env proceed" — Please disregard/remove the `protect env` menu requirement from `ORIGINAL_REQUEST.md` / `PROJECT.md` / `BRIEFING.md`. Proceed directly with Milestone 4 and Milestone 5!

## Follow-up — 2026-10-31T23:02:19Z

User requirement update — please incorporate this mandatory capability immediately into `ORIGINAL_REQUEST.md`, `PROJECT.md`, `BRIEFING.md`, and Milestones 4 & 5 (`quorum-coordinator`, `quorum-daemon`, `quorum-ui`):

"can this be added but it should use fusion [google/langextract]"

Specifically, integrate **Fusion-Powered Google LangExtract (`langextract_fusion`)** into QuorumRAG so that structured entity/attribute extraction with **exact character-interval source grounding (`char_interval`)** and **interactive HTML/UI visual highlighting** is natively available and powered by the **Fusion Router (`d:\harfile\fusion` — GLM-4-9B, Qwen 2.5, Gemma 2/3)** instead of requiring cloud API keys:

1. **Core LangExtract Data Model & Grounding Engine (`quorum-types` / `quorum-coordinator::langextract`)**:
   - **`Extraction`**: `{ extraction_class: String, extraction_text: String, char_interval: Option<(usize, usize)>, attributes: BTreeMap<String, String>, source_chunk_id: Option<String>, source_file_path: Option<String> }`
   - **`ExampleData`**: `{ text: String, extractions: Vec<Extraction> }` (few-shot example definitions with prompt alignment validation ensuring `extraction_text` is verbatim in `text` in order of appearance).
   - **Precise Source Grounding (`char_interval` Verification)**:
     - Just like `google/langextract`, locate every extracted `extraction_text` verbatim in the source document/chunk text and bind its exact `(start_char, end_char)` byte/char span (`char_interval`).
     - Automatically detect and filter out ungrounded hallucinations (where `char_interval.is_none()`, e.g., if an LLM echoed a few-shot example rather than the input text) when `require_grounding = true`.
   - **Multi-Pass & Long-Document Chunking Support (`extraction_passes`, `max_char_buffer`)**:
     - Support multi-pass extraction over retrieved RAG chunks or full local files to maximize recall on long documents ("needle-in-a-haystack").

2. **Powered by ModelFusion (`d:\harfile\fusion` Hardware-Aware Router)**:
   - Route `lx.extract` / `LangExtractFusionEngine::extract(...)` through the QuorumRAG **Fusion Router** (`AutoFusion` / `GLM-4-9B` / `Qwen 2.5` / `Gemma 2/3` on local Ollama `http://127.0.0.1:11434` with JSON schema / structured output mode, plus deterministic grounded regex/span extraction fallback when offline so it works 100% offline with zero cloud API keys).
   - Also integrate grounded span extraction into the `SynthesizeWithCitations` node so RAG answers can highlight exact grounded spans inside retrieved source chunks!

3. **Interactive Visualization & ChatGPT Web UI Integration (`quorum-ui` / `quorum-daemon`)**:
   - **REST API Endpoints**:
     - `POST /api/langextract`: Accepts `{ text_or_file_path, query_or_prompt_description, examples, extraction_passes, model_family }`, runs Fusion-powered LangExtract (either on provided text, a local file path, or top retrieved RAG chunks), and returns grounded `AnnotatedDocument` results (plus `.jsonl` export).
     - `POST /api/langextract/visualize` (or inline HTML generator): Generates a self-contained, interactive LangExtract HTML visualization with color-coded entity classes, hover attribute tooltips, and clickable source-grounded character highlights in context!
   - **ChatGPT Web UI Mode / Action**:
     - Add a **"LangExtract (Fusion Grounded Extraction)"** toggle/action in the ChatGPT interface (and `@extract` / structured extraction command) that lets the user extract structured entities/relationships/attributes from any local file or chat query and view the interactive color-highlighted source grounding right in the chat UI or export as `.jsonl` / interactive `.html`!

## Follow-up — 2026-10-31T23:03:16Z

User architectural clarification for **Fusion-Powered LangExtract in QuorumRAG** — please incorporate both the **Ingestion-Time Grounded Preprocessor** and **Query-Time Decomposition & Prefiltering** loops into `ORIGINAL_REQUEST.md`, `PROJECT.md`, `BRIEFING.md`, and the active crates (`quorum-ingest`, `quorum-storage`, `quorum-coordinator`, `quorum-ui`):

1. **Ingestion-Time Structured Extraction & 100% `char_interval` Grounding Gate**:
   - Rather than only slicing raw documents into arbitrary chunks, `LangExtract` acts as the structured extraction and source-grounding preprocessor during ingestion (`quorum-ingest` / `langextract_fusion`).
   - Run schema-constrained `extract()` (with `ExampleData` / output schema via the Multi-Model Fusion setup + fast deterministic extractor) to extract typed entities (`extraction_class`), exact verbatim spans (`extraction_text`), and key-value `attributes` (e.g., CVEs, arXiv IDs, function/struct symbols, syscalls, technical parameters).
   - **Strict `char_interval` Verification**: Every `Extraction` maps `extraction_text` to its exact `(start_char, end_char)` offset in the original source text. Any extraction that cannot be located verbatim in the source text (e.g. hallucinated or copied from few-shot `ExampleData`) gets `char_interval = None` and is **strictly filtered out** (`extractions.into_iter().filter(|e| e.char_interval.is_some())`) so 100% of stored indicators/entities in LanceDB + Tantivy are source-grounded.

2. **Query-Time Decomposition & Scalar/Lexical Prefiltering (`ClassifyIntent` -> `RetrieveMesh`)**:
   - At query time, `ClassifyIntent` in the Rust Agentic State Graph runs a fast `LangExtract` pass over the user's prompt to extract structured filter criteria (specific file paths, CVE/arXiv IDs, code symbols, target classes, attributes).
   - Those extracted grounded entities/attributes are passed into `RetrieveMesh` (`quorum-storage` LanceDB + Tantivy hybrid search) as structured scalar/keyword constraints alongside the dense `embeddinggemma-300m` vector so exact technical primitives are never blurred by vector similarity alone!

## Follow-up — 2026-10-31T23:06:45Z

Exact reference located for the user's request ("adapt this to this code ... it already appears in the comment at line 3667 of your `breakout_defense.rs` file (`arxiv-langextract + LangGraph Arm 4`)"):
- File path: `D:\harfile\OshoosiClaw\crates\osoosi-agent-anomaly\src\breakout_defense.rs` (lines 3440–3697)
- Also see: `C:\Users\oyesanyf\.gemini\config\plugins\arxiv-langextract\skills\arxiv-langextract\SKILL.md`
- Specifically, adapt the `LlmFusionConsensus` / `LlmFusionArmReport` multi-arm fusion evaluation pattern (`Arm 1: SemanticSpanExtractor` — "Character-Span & Semantic Grounding Extractor", `Arm 2: SyntacticAstVerifier`, `Arm 3: AdversarialCounterfactualJudge`, plus `Arm 4` LangGraph state calibration) directly into QuorumRAG's `langextract_fusion` engine so that:
  1. **Ingestion Stage**: Runs schema-constrained `LangExtract` with `ExampleData` / output schema and strict `char_interval` (`Some((start_char, end_char))`) character-offset verification, filtering out any ungrounded extraction (`char_interval == None`) before indexing into LanceDB + Tantivy.
  2. **Query Stage (`ClassifyIntent` -> `RetrieveMesh` -> `GradeRelevance`)**: Decomposes unstructured prompts into structured scalar/lexical filter criteria (`target_boundary`, `extraction_class`, `file_path`, `arxiv_id` / `cve_id` / technical symbols) and evaluates multi-arm `LlmFusionConsensus` confidence (`confidence_bp`) across the local Multi-Model Fusion setup (`GLM-4-9B`, `Qwen 2.5`, `Gemma 3` via Ollama / local runtime).

## Follow-up — 2026-10-31T23:58:10Z

User requirement update — please incorporate this mandatory live verification gate into `ORIGINAL_REQUEST.md`, `PROJECT.md`, `BRIEFING.md`, Milestone 5, and the Victory Audit:

"test it run it on the ide once dont to see it it generates production code"

Specifically, once Milestone 5 (`quorumrag.exe` single binary + ChatGPT Web UI + LangExtract + Multi-Model Fusion Router) is compiled:
1. **Launch and Run `quorumrag.exe` Live in the Workspace/IDE**:
   - Start the actual compiled `quorumrag.exe` binary against the local workspace (`d:\harfile\QuorumRAG` / `d:\harfile\fusion`), indexing real Rust source files.
2. **Live Production Code Generation & Grounded RAG Test**:
   - Query the live running daemon / ChatGPT Web UI API (`/api/chat` / `/api/query` / `/api/langextract`) with real coding & architecture prompts to verify that it retrieves grounded AST/code chunks, runs the 5-node LangGraph Agentic State Graph + Multi-Model Fusion Router (`GLM-4-9B` / `Qwen 2.5` / `Gemma 3` / offline code synthesizer), and **generates real, compilable production code with exact `[Peer:File:Lines]` citations and grounded `char_interval` spans**.
   - Also launch the ChatGPT Web UI in the browser so the user can see and interact with it live in the IDE/browser!
