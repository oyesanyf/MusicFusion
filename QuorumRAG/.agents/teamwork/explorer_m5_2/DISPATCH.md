# DISPATCH — 2026-11-01T00:25:00Z

## Target
`explorer_m5_2`

## Role
Milestone 5 Explorer 2 (Axum 0.7 Web Server, SSE Streaming & Single Executable Launcher)

## Objective
Investigate and design `crates/quorum-daemon`:
1. Design Axum 0.7 web server architecture:
   - Serving embedded static assets from `quorum-ui` at `/` and `/assets/*`.
   - `POST /api/chat` and `POST /api/query`: SSE streaming (`text/event-stream`) emitting live Agentic State Graph telemetry events (`ClassifyIntent -> RetrieveMesh -> GradeRelevance -> RewriteQuery -> SynthesizeWithCitations`), model family selection (`AutoFusion`, `GLM-4-9B`, `Qwen 2.5`, `Gemma 3`), streaming markdown tokens, and structured source citation cards (`chunk_id`, `peer_id`, `file_path`, `lines`, `score`, `text`).
   - `POST /api/reindex`: triggers asynchronous background `ReindexWorker` and returns task status JSON.
   - `GET /api/reindex/status`: returns live reindex progress.
   - `POST /api/drift/fix`: triggers asynchronous background `DriftHealer` across all 5 drifts and returns healed drift report.
   - `GET /api/drift/status`: returns current drift check status.
   - `POST /api/langextract`: structured JSON extraction via `LlmFusionConsensus`.
   - `POST /api/langextract/visualize`: renders color-coded HTML document.
   - `GET /api/health` and `GET /api/models`: returns hardware telemetry (RAM/VRAM), active model family, and libp2p swarm peer count.
2. Design Single Executable Launcher (`quorumrag.exe`):
   - Command-line interface with `clap`:
     - `quorumrag` (starts daemon + web UI on default port 3030 and auto-opens default browser to `http://127.0.0.1:3030`).
     - `--no-browser`: starts daemon without launching browser (for headless/test runs).
     - `--port <PORT>`: overrides default port (default: 3030).
     - `--data-dir <PATH>`: overrides storage data directory.
     - `quorumrag reindex [--path <dir>] [--full]`: runs standalone reindex CLI process.
     - `quorumrag fix-drift`: runs standalone drift healing CLI process.
3. Design Browser Auto-Launch on Startup:
   - Uses `open::that(url)` or Windows process launcher (`cmd /c start http://...` or `powershell Start-Process`) spawned on a background thread after the TCP listener binds.
4. Clean Graceful Shutdown:
   - Handles `Ctrl+C` / SIGINT / SIGTERM gracefully, flushing LanceDB and Tantivy writers and closing libp2p swarm cleanly.

## Inputs
- `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\PROJECT.md`
- `crates/quorum-coordinator`
- `crates/quorum-ingest`

## Deliverable
Write your 5-component report to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_2\handoff.md`. Message parent when complete.
