# BRIEFING — 2026-11-01T01:01:20Z

## Mission
Investigate reference implementations in d:\harfile\ModelFusion and determine how to port its core patterns cleanly into standalone Rust modules inside d:\harfile\music without any runtime dependency on ModelFusion, supporting universal multi-genre production.

## 🔒 My Identity
- Archetype: explorer
- Roles: investigator, synthesizer
- Working directory: d:\harfile\music\.agents\teamwork\explorer_survey_modelfusion\
- Original parent: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Milestone: survey_modelfusion_and_design_ports

## 🔒 Key Constraints
- Read-only investigation — do NOT implement / do NOT edit project source files
- No runtime dependency on ModelFusion
- Complete findings and code blueprints in handoff.md
- Report progress via progress.md and message back via send_message

## Current Parent
- Conversation ID: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Updated: 2026-11-01T00:59:18Z (Critical multi-genre requirement update)

## Investigation State
- **Explored paths**:
  - `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` (Initial + Multi-Genre follow-up)
  - `d:\harfile\ModelFusion\crates\model_selection\src\memory.rs` (RAM/VRAM detection, CREATE_NO_WINDOW, dynamic sizing, Ollama model tiers)
  - `d:\harfile\ModelFusion\crates\core\src\fusion_engine\{mod.rs, fusion.rs, judge.rs, schema.rs, models.rs}` (panel execution, prompt formatting, judge scoring, clean JSON extraction)
  - `d:\harfile\ModelFusion\crates\core\src\providers.rs` (Ollama `/api/tags`, `/api/chat`, fallback selection, network error handling)
  - `d:\harfile\music\Cargo.toml` and `crates\music_core\src\schema.rs` (`SongBlueprint`, `SongSection`, `PipelineModelStack`, preset anthems)
- **Key findings**:
  - Hardware probing: sysinfo 0.30 handles RAM/CPU; nvidia-smi with CREATE_NO_WINDOW (0x08000000) handles VRAM with fallback to candidate paths.
  - Sizing & routing: VRAM thresholds (6GB for Stage 1, 8GB for Stage 2 & 4, 4GB for Stage 3 & 5) deterministically route between heavy HF models and pure-Rust DSP.
  - Multi-LLM panel: parallel Ollama `/api/chat` requests with specialized personas (Lyricist, Genre Music Theorist/Arranger, Sound Engineer) and a Judge model outputting strictly structured JSON.
  - Multi-Genre engine: dynamic taxonomy supports any genre (Afrobeats, Highlife, Amapiano, Reggae, Soul, Gospel, Hip-Hop, Rock, Metal, Jazz, Classical, EDM, Latin) with genre-matched BPM, modes, drum patterns, and instruments, keeping Reggae Anthem as flagship preset.
- **Unexplored areas**: None; full porting blueprint ready for handoff report.

## Key Decisions Made
- Standalone architecture: Place modules in `crates/music_fusion/src/{memory.rs, fusion_panel.rs}` (or `src/`) with zero dependency on ModelFusion.
- Built-in fail-safe: In case Ollama is offline or uninstalled, pipeline falls back to embedded genre presets with zero empty states.
- Clean formatting: Strict YuE bracket tags (`[intro]`, `[verse]`, `[chorus]`, `[instrumental]`, `[bridge]`, `[guitar solo]`, `[outro]`) in `lyrics.txt` and descriptive tags in `genre.txt`.

## Artifact Index
- d:\harfile\music\.agents\teamwork\explorer_survey_modelfusion\DISPATCH.md — Incoming parent dispatches
- d:\harfile\music\.agents\teamwork\explorer_survey_modelfusion\BRIEFING.md — Persistent working memory
- d:\harfile\music\.agents\teamwork\explorer_survey_modelfusion\progress.md — Heartbeat and progress tracker
- d:\harfile\music\.agents\teamwork\explorer_survey_modelfusion\handoff.md — Final handoff report
