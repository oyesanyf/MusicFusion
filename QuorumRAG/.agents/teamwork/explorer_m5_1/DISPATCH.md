# DISPATCH — 2026-11-01T00:25:00Z

## Target
`explorer_m5_1`

## Role
Milestone 5 Explorer 1 (ChatGPT Web UI & LangExtract Visualizer Architecture)

## Objective
Investigate and design `crates/quorum-ui`:
1. Inspect `d:\harfile\ModelFusion\hugos_chatgpt_perfect_ui.html` and `d:\harfile\ModelFusion\hugos_chatgpt_browser_ui.html`. Extract useful layout, dark-mode CSS, message list, chat input, streaming cursor, markdown/code rendering, and source citation cards.
2. Design static asset bundling (HTML, CSS, JS) via `rust-embed` or `include_str!` so all UI assets compile directly into the binary.
3. Design the search mode toggle in the UI:
   - Keyword Search (BM25)
   - Semantic Vector Search (Dense Gemma)
   - Hybrid Search (RRF)
4. Design the Model Selector Pill Bar:
   - `Auto-Fusion` (default dynamic router)
   - `GLM-4-9B` (`glm-4-9b-chat` / `glm4` for multi-hop agentic synthesis)
   - `Qwen 2.5` (`qwen2.5` sized by free RAM/VRAM for code)
   - `Gemma 3` (`gemma3` for natural English prose)
5. Design the Interactive Google LangExtract HTML Visualizer:
   - Endpoint: `POST /api/langextract/visualize` (renders color-coded entity/span highlights over source document text with hoverable attribute cards).
   - Endpoint: `POST /api/langextract` (returns JSON with `AnnotatedDocument`).
   - Web UI integration: `@extract` command in chat input or dedicated Extract tab/modal.
6. Design Reindex and Drift Remediation UI Controls:
   - Status indicators for active file watcher and storage health.
   - Trigger buttons: "Reindex Files" (`/api/reindex`) and "Fix Drifts" (`/api/drift/fix`).

## Inputs
- `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\ModelFusion\hugos_chatgpt_perfect_ui.html`
- `d:\harfile\ModelFusion\hugos_chatgpt_browser_ui.html`

## Deliverable
Write your 5-component report to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_1\handoff.md`. Message parent when complete.
