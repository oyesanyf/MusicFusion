# BRIEFING — 2026-11-01T01:08:00Z

## Mission
Plan and design the detailed, production-grade implementation of `crates/music_fusion` for Milestone 1.

## 🔒 My Identity
- Archetype: explorer
- Roles: explorer, investigator, architect
- Working directory: d:\harfile\music\.agents\teamwork\explorer_m1_2
- Original parent: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Milestone: Milestone 1

## 🔒 Key Constraints
- Read-only investigation — do NOT implement or modify workspace source files directly.
- Sizing models MUST strictly evaluate runtime free/available memory (`res.free_ram_gb`, `res.free_vram_mb`), NEVER total physical RAM.
- Ollama client must support fast timeouts (e.g. 400ms probes), zero-crash fallbacks, model detection via `/api/tags`, and interactive missing model guidance.
- Adhere to the Universal Zero-Touch PATH & Environment Protection Protocol (process-scoped only).
- Production-grade code designs: real parsing, robust error handling, no hardcoded dummy personas or fake mockup data.

## Current Parent
- Conversation ID: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Updated: not yet

## Investigation State
- **Explored paths**: `ORIGINAL_REQUEST.md`, `PROJECT.md`, `explorer_survey_modelfusion/handoff.md`, `explorer_survey_existing/handoff.md`, `crates/music_core/Cargo.toml`, `crates/music_core/src/schema.rs`, `explorer_m1_1/DISPATCH.md`, `explorer_m1_3/DISPATCH.md`.
- **Key findings**:
  - `music_fusion` is a new crate needed under `crates/music_fusion` with path dependency `music_core = { path = "../music_core" }`.
  - Architecture requires 7 modules: `lib.rs`, `memory.rs`, `ollama_client.rs`, `panel.rs`, `judge.rs`, `prompt_templates.rs`, `error.rs`.
  - Memory probing must use `CREATE_NO_WINDOW (0x08000000)` on Windows, query `nvidia-smi`, evaluate `sys.available_memory()`, and strictly enforce the Dynamic Available Memory Law (32b, 14b, 7b, 3b, 1.5b tiers; flint/glm4 resource gating).
  - Stage routing must provide `ExecutionRoute::HuggingFaceGpu`, `HuggingFaceCpu`, `PureRustDsp` based on VRAM/RAM budgets and environment overrides (`MUSIC_FORCE_DSP`, `MUSIC_FORCE_HF`).
  - Ollama client must have 400ms health probe, zero-proxy client, interactive missing model guidance (`ollama pull <model>`), and zero crash on unreachable daemon.
  - Multi-model panel coordinates Lyricist, Arranger, and Sound Engineer in parallel via `tokio::join!`, with Universal Multi-Genre adaptation and the 1977 reggae anthem preset.
  - Judge model strips `<think>` tags and markdown code blocks, extracts JSON between `{` and `}`, produces clean `genre.txt` tags and bracketed `lyrics.txt`, and updates `MusicProductionState`.
- **Unexplored areas**: None. Design is ready for report compilation.

## Key Decisions Made
- Modularized prompt engineering into `prompt_templates.rs` for clean separation of LLM personas.
- Included simulated constructor `SystemMemory::simulated(...)` to enable 100% deterministic unit testing without requiring an NVIDIA GPU on the test runner.
- Added fail-safe fallback blueprint in Judge parser ensuring zero empty states and zero pipeline aborts even if LLM produces invalid JSON.

## Artifact Index
- `DISPATCH.md` — Inbound task dispatch record
- `BRIEFING.md` — Situational awareness and working memory
- `progress.md` — Heartbeat and progress tracker
- `handoff.md` — Complete 5-component handoff report
