# BRIEFING — 2026-10-31T16:34:30Z

## Mission
Orchestrate execution of QuorumRAG: a local-first peer-to-peer retrieval-augmented generation daemon with embedded LanceDB, Gemma embedding contract, rust-libp2p networking, Google Magika file identification, single .exe launcher, ChatGPT-like web UI, LangGraph-style Rust Agentic State Graph, full Search Modes + Image Indexing, dedicated Reindex & All-Drift Healing (`fix-drift`), and Multi-Model Adaptive Intent + Hardware Fusion Router (GLM-4-9B, Qwen 2.5, Gemma 2/3).

## 🔒 My Identity
- Archetype: sentinel
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork
- Orchestrator: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
- Victory Auditor: to be spawned on victory claim

## 🔒 Key Constraints
- No technical decisions — relay only
- Victory Audit is MANDATORY before reporting completion
- Strict Arrow/LanceDB encapsulation in storage crate
- Offline Gemma embedding contract with BLAKE3 contract hash
- Compilable rust-libp2p swarm without stubs
- ChatGPT-like web interface adapted from d:\harfile
- Automated test harness passing cargo test
- Google Magika deep-learning content identification in ingestion pipeline
- Search any local file and answer general questions ("Ask Anything") in ChatGPT web interface
- Single .exe launcher packaging daemon + embedded UI with automatic browser launch (--no-browser flag)
- Rust Agentic State Graph (`ClassifyIntent -> RetrieveMesh -> GradeRelevance -> RewriteQuery (loop) -> SynthesizeWithCitations`) in the query/synthesis layer with live UI streaming telemetry
- Full search mode support: Keyword (Tantivy BM25), Semantic (Gemma Vector), and Hybrid (RRF)
- Continuous OS File Watcher (`notify`) running all detected events through Google Magika before indexing
- Image indexing support (images identified by Magika are indexed with rich metadata, dimensions, path tokens, and visual summaries rather than skipped)
- Dedicated Reindex Process (`quorumrag reindex [--path <dir>] [--full]`, background worker, `/api/reindex` + UI trigger)
- Dedicated Drift Remediation Process (`fix-drift`, background healer, `/api/drift/fix` + UI trigger) covering all 5 drift types (Merkle FS/DB, LanceDB/Tantivy desync, Embedding contract/vector drift, ACL/tier policy drift, and fragmentation/tombstone drift)
- Direct adoption of `d:\harfile\fusion` architecture & code for orchestration, model routing, and synthesis
- Hardware-resource-aware GLM model selection (evaluating runtime free RAM/VRAM to route to `glm-4`, `glm-4-9b`, or `glm-edge` when resources permit)
- Immutable `google/embeddinggemma-300m` mesh embedding contract
- Multi-model synthesis support: default `glm-4-9b-chat` (agentic multi-hop & 128K context), `qwen2.5` (code-dominated AST queries), `gemma3` (prose summaries), with hot-swappable UI pill bar
- Adaptive Intent + Hardware Fusion Router (`SynthesisModelFamily::{Glm4Agentic, Qwen25Code, GemmaProse, AutoFusion}`) in `ClassifyIntent` state node
- Preserve `[profile.dev] debug = 0` and `[profile.test] debug = 0` in `Cargo.toml` and `.cargo/config.toml` (`jobs = 2`)
- Never run `cargo test` concurrently across multiple subagents; cap test concurrency at `-- --test-threads=2`
- Never invoke `taskkill.exe`; let cargo commands complete within sufficient timeouts
- Experimental Protect Env (OS Environment & PATH Protection Guard) menu/panel in UI and `/api/experimental/env-guard` REST endpoint obeying Universal Zero-Touch PATH Protocol (BLAKE3 snapshot, secret/.env redaction, process sandbox, live audit log; zero registry mutations)
- Experimental Protect Env menu rescinded per user follow-up ("this copte does not need protect env proceed"); proceed directly with standard Milestones 4 and 5
- Fusion-Powered Google LangExtract (`langextract_fusion`): Grounded structured entity/attribute extraction with exact `char_interval` byte spans, few-shot prompt alignment, hallucination filtering, multi-pass chunking, offline deterministic fallback, interactive HTML visualizer, `/api/langextract` REST endpoint, and ChatGPT Web UI `@extract` action powered by ModelFusion (GLM-4-9B, Qwen 2.5, Gemma 2/3)
- LangExtract Two-Loop Architecture: (1) Ingestion-Time Preprocessor with 100% `char_interval` grounding gate (filtering ungrounded hallucinations where `char_interval` is None) storing grounded entities/attributes in LanceDB + Tantivy, and (2) Query-Time Decomposition in `ClassifyIntent` -> `RetrieveMesh` scalar/lexical prefiltering alongside `embeddinggemma-300m` dense vectors
- `LlmFusionConsensus` Multi-Arm Architecture (adapted from `breakout_defense.rs` lines 3440–3697): Arm 1 SemanticSpanExtractor, Arm 2 SyntacticAstVerifier, Arm 3 CounterfactualJudge, and Arm 4 LangGraph calibration evaluating `confidence_bp` across local GLM-4-9B, Qwen 2.5, Gemma 3 setup
- Live IDE Execution & Production Code Generation Gate: Once compiled in Milestone 5, start `quorumrag.exe` live in IDE against workspace, execute live coding query generating compilable production code with exact `[Peer:File:Lines]` citations & grounded `char_interval` spans, and auto-open ChatGPT Web UI in browser

## User Context
- **Last user request**: Run compiled `quorumrag.exe` live in IDE, verify it generates production code with citations & grounded spans, and auto-open ChatGPT Web UI in browser.
- **Pending clarifications**: none
- **Delivered results**: none

## Project Status
- **Phase**: in progress (Milestone 4 DONE; Milestone 5 Remediation: scripts/verify_live_ide_execution.ps1 executed live: 34 files & 528 chunks indexed with 0 Tantivy errors, fusion.rs:1-12 cited, RrfCombiner code block extracted; worker_m5_remediation_1 concluding verification)
- **Route**: General path (`teamwork_orchestrator`)
- **Active Orchestrator**: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e

## Victory Audit Status
- **Triggered**: no
- **Verdict**: pending
- **Retry count**: 0

## Artifact Index
- .agents/teamwork/ORIGINAL_REQUEST.md — Approved task prompt and follow-up requirements
- PROJECT.md — Architecture, features, agentic state graph, search modes, image indexing, reindex & drift healing, multi-model fusion router
