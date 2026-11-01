# BRIEFING — 2026-11-01T15:05:00Z

## Mission
Remediate SAPI WAV parser defect in tts_engine.rs, implement Hugging Face Music Model Discovery and Ollama Lifecycle Management, integrate WebUI/REST/CLI, verify tests, and configure git remote.

## 🔒 My Identity
- Archetype: worker
- Roles: implementer, qa, specialist
- Working directory: d:\harfile\music\.agents\teamwork\worker_m6_remed\
- Original parent: 22bd2f00-efbc-429d-9417-8639ee3e2bc7
- Milestone: Milestone 6 Iteration 2 (Remediation & Model Discovery/Ollama Lifecycle)

## 🔒 Key Constraints
- Genuine implementations only: no mocks, dummy logic, or hardcoded return values.
- Zero-touch PATH law: only process-scoped PATH modifications (`std::env::set_var("PATH", ...)`). Never touch HKCU/HKLM registry.
- Dynamic available memory law: size models using live free RAM / VRAM, never total physical RAM. No `-coder` suffix.
- Fast probe timeouts (400ms) for daemons.
- SAPI WAV parser must dynamically walk RIFF subchunks from byte 12 to handle 18-byte fmt chunks.
- Clean up temporary SAPI files immediately.
- Zero empty states / fail-closed security invariants.

## Current Parent
- Conversation ID: 22bd2f00-efbc-429d-9417-8639ee3e2bc7
- Updated: not yet

## Task Summary
- **What to build**:
  1. Fix `read_wav_pcm16_stereo()` in `crates/music_fusion/src/tts_engine.rs` to dynamically walk RIFF subchunks and delete temporary SAPI files.
  2. Implement `hf_model_discovery.rs` in `crates/music_fusion` with REST endpoints `GET /api/models/registry`, `POST /api/models/discover-upgrade`, CLI flag `--update-models`, and WebUI button.
  3. Implement `ollama_lifecycle.rs` in `crates/music_fusion` with dynamic memory sizing, silent install, background daemon spawn, auto-pull, REST `POST /api/ollama/bootstrap`, and WebUI controls.
  4. Test verification: `cargo test -p music_fusion` and `cargo test --workspace`.
  5. Git remote setup: set origin to `https://github.com/oyesanyf/MusicFusion.git` on branch `main` with complete `README.md`.
- **Success criteria**: All tests pass, genuine discovery & sizing logic, REST/CLI/WebUI wired, git remote configured.
- **Interface contracts**: `ORIGINAL_REQUEST.md`, `DISPATCH.md`, `challenger_m6_1/handoff.md`.
- **Code layout**: `crates/music_fusion`, `crates/music_cli`.

## Key Decisions Made
- Dynamically parse RIFF chunks starting from offset 12 by reading 4-byte chunk ID and 4-byte chunk size until `b"data"` is reached.
- Clean up temporary SAPI files (`_sapi_script.ps1`, `_sapi_lead.wav`, `_sapi_choir.wav`, etc.) in a `finally`/cleanup block so no files leak on disk.
- Use `SystemMemory::detect_live()` to query system memory and match the memory tier table strictly.

## Artifact Index
- `crates/music_fusion/src/tts_engine.rs` — SAPI WAV parser fix & temp file cleanup.
- `crates/music_fusion/src/hf_model_discovery.rs` — HF model discovery engine.
- `crates/music_fusion/src/ollama_lifecycle.rs` — Ollama detection, silent install, sizing, auto-pull.
- `crates/music_fusion/src/lib.rs` — Module exports.
- `crates/music_cli/src/rest_server.rs` — REST endpoints for models and ollama.
- `crates/music_cli/src/cli_args.rs` & `main.rs` — `--update-models` flag.
- `crates/music_cli/webui/index.html` & `app.js` — WebUI controls.
- `handoff.md` — 5-component completion report.

## Change Tracker
- **Files modified**: None yet
- **Build status**: Pending
- **Pending issues**: None

## Quality Status
- **Build/test result**: Not yet executed
- **Lint status**: Not yet evaluated
- **Tests added/modified**: Pending

## Loaded Skills
None
