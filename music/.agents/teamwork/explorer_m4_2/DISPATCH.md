# DISPATCH — explorer_m4_2

## Assignment
Investigate and design Tokio HTTP REST API endpoints and Multi-LLM Iteration Fusion Panel:
1. Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` and `d:\harfile\music\PROJECT.md`.
2. Inspect installed Ollama models: `qwen2.5:32b`, `deepseek-r1:32b`, `glm-4.7-flash:latest`, `glm4:9b`, `flint:latest`, `gemma2:9b`, `mistral:latest`, `qwen2.5:7b`, `strix-qwen:latest`, `deepseek-r1:1.5b`, `qwen2.5:1.5b`.
3. Design Multi-LLM Iteration Fusion Panel (`crates/music_fusion/src/iteration_panel.rs` or `panel.rs`):
   - Model 1: Lyric & Hook Refiner LLM
   - Model 2: Arrangement & Groove Producer LLM
   - Model 3: Dedicated Iteration Co-Pilot & Diff Architect LLM
   - Judge / Synthesizer LLM: Fuses panel outputs into `v(N+1)` state + `iteration_summary` and `suggested_next_iterations: Vec<String>`.
4. Design Tokio HTTP REST API on port 5050 (port conflict fallback 5050..5060):
   - `POST /api/songs/create`: boots `StateGraph` and creates new song `v1`.
   - `POST /api/songs/iterate`: runs Multi-LLM Iteration Fusion Panel producing `v(N+1)` with diff summary & clickable suggestions.
   - `POST /api/songs/regenerate-stem`: re-executes targeted pipeline node for specified stem (`drums`, `guitar`, `vocals`).
   - `POST /api/songs/master`: runs Node 6 analog tape mastering on track with specified profile.
   - `GET /api/songs/:id/versions`: returns version tree and metadata.
   - `GET /api/songs/:id/versions/:v/audio/:stem`: streams 48 kHz WAV audio for playback.
   - `POST /api/songs/assist-iteration` / `GET /api/songs/:id/assist`: queries Iteration Co-Pilot LLM returning 3–5 concrete suggestions.
   - `GET /api/hardware`: returns live VRAM, RAM, and detected Ollama models.
5. Provide concrete implementation snippets, struct definitions, and routing architecture in `d:\harfile\music\.agents\teamwork\explorer_m4_2\handoff.md`.


## 2026-11-01T04:24:28Z
You are explorer_m4_2 for Milestone 4 (Tokio REST API & Multi-LLM Iteration Co-Pilot Fusion).
Your working directory is: d:\harfile\music\.agents\teamwork\explorer_m4_2\
Read d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md, d:\harfile\music\PROJECT.md, and your DISPATCH.md.
Investigate and design:
1. Multi-LLM Iteration Fusion Panel in crates/music_fusion (Lyricist, Arranger, Sound Engineer, dedicated Iteration Co-Pilot LLM, and Judge leveraging local Ollama models: deepseek-r1, glm-4.7-flash, flint, gemma2:9b, qwen2.5).
2. Tokio HTTP REST endpoints on 5050 (fallback 5050..5060): POST /api/songs/create, POST /api/songs/iterate, POST /api/songs/regenerate-stem, POST /api/songs/master, GET /api/songs/:id/versions, GET /api/songs/:id/versions/:v/audio/:stem, POST /api/songs/assist-iteration, GET /api/hardware.
Write your handoff report to d:\harfile\music\.agents\teamwork\explorer_m4_2\handoff.md. Send a completion message via send_message to your caller.
