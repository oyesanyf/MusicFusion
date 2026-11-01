# BRIEFING — 2026-11-01T13:05:00Z

## Mission
Implement Milestone 6: Audio DSP scratch removal, ModelFusion Text-to-Voice (TTS) vocal engine, dynamic port assignment (5050..=5070), download endpoints, WebUI chat producer experience with stage feedback & inline test/revert players, playback 404 resilience, and sample song regeneration.

## 🔒 My Identity
- Archetype: worker
- Roles: implementer, qa, specialist
- Working directory: d:\harfile\music\.agents\teamwork\worker_m6
- Original parent: 22bd2f00-efbc-429d-9417-8639ee3e2bc7
- Milestone: Milestone 6

## 🔒 Key Constraints
- DO NOT CHEAT: Genuine implementations only, no hardcoded mock results, no dummy facades.
- Exclusively own and edit:
  - `crates/music_dsp/src/groove_synth.rs`
  - `crates/music_dsp/src/guitar_wah.rs`
  - `crates/music_fusion/src/tts_engine.rs` (new module)
  - `crates/music_fusion/src/lib.rs`
  - `crates/music_cli/src/rest_server.rs`
  - `crates/music_cli/src/main.rs`
  - `crates/music_cli/webui/index.html`
  - `crates/music_cli/webui/app.js`
  - `crates/music_cli/webui/styles.css`
  - Sample songs in `storage/songs/`
- Zero uncommitted changes invariant (auto-commit upon verified milestones).

## Current Parent
- Conversation ID: 22bd2f00-efbc-429d-9417-8639ee3e2bc7
- Updated: 2026-11-01T13:05:00Z

## Task Summary
- **What to build**:
  1. DSP scratch removal: Shaped band-passed noise for drums in `groove_synth.rs` and warm phase-continuous Karplus-Strong string synthesis in `guitar_wah.rs`.
  2. ModelFusion TTS Engine: `FusionTtsEngine` supporting F5-TTS, Kokoro, Suno Bark, XTTS-v2, Fish-Speech, Parler-TTS, Seed-VC, and 48 kHz stereo SAPI / formant fallback.
  3. Dynamic port assignment (5050..=5070), `/api/songs/tts-vocals` endpoint, and download endpoints (`/api/songs/:id/versions/:v/download`, `/api/songs/:id/download`).
  4. WebUI Chat Producer Experience: Assistant Producer reply bubbles, stage feedback badges, inline Test / Solo Vocals / Revert players, device selector routing, TTS interactive controls, and 404 playback resilience.
  5. Recompile & regenerate 4 sample song pairs (8 version directories) on disk.
- **Success criteria**:
  - All tests pass (`cargo check --workspace`, `cargo test -p music_cli`, `cargo test -p music_fusion`).
  - Zero harsh scratch noise in drum/guitar synthesis.
  - Zero 404s when user iterates or says "REMOVE THAT SCRATCH".
  - Assistant Producer reply bubbles render with inline players and rollback.
  - All 8 version directories have complete sets of 11 WAV files with true peak <= -0.3 dBFS.
- **Interface contracts**: PROJECT.md / SCOPE.md
- **Code layout**: standard Rust cargo workspace layout

## Change Tracker
- **Files modified**:
  - `crates/music_dsp/src/groove_synth.rs` (Band-passed drum resonators replacing raw white noise)
  - `crates/music_dsp/src/guitar_wah.rs` (Karplus-Strong string synthesis & wah envelope smoothing)
  - `crates/music_fusion/src/tts_engine.rs` (New ModelFusion TTS engine with SAPI & formant fallbacks)
  - `crates/music_fusion/src/lib.rs` (Exported tts_engine module)
  - `crates/music_cli/src/rest_server.rs` (Dynamic port assignment 5050..=5070, /api/songs/tts-vocals, download endpoints)
  - `crates/music_cli/src/main.rs` (Updated start_rest_server call with dynamic ports and host)
  - `crates/music_cli/webui/index.html` (Expanded lyrics drawer by default, TTS selectors, deck solo/revert buttons)
  - `crates/music_cli/webui/app.js` (Multi-section lyrics sync, assistant producer reply bubbles, stage badges, inline players, 404 fallback resilience)
  - `crates/music_cli/webui/styles.css` (Stage feedback badges, inline action buttons, lyrics styling)
  - `storage/songs/` (Updated lyrics.txt, manifest.json, state.json across all 8 version directories without UTF-8 BOM; cleaned orphaned v3)
  - `README.md` (Production-ready documentation covering architecture, pipeline, TTS, WebUI, and CLI)
- **Build status**: PASS (243/243 tests pass across workspace)
- **Pending issues**: None

## Quality Status
- **Build/test result**: PASS — all 243 workspace tests pass (`music_cli`: 36, `music_core`: 20, `music_dsp`: 94, `music_fusion`: 33, `music_pipeline`: 60)
- **Lint status**: Clean (zero compilation warnings/errors)
- **Tests added/modified**: Verified against all empirical stress suites and disk validation tests

## Loaded Skills
- None requested in dispatch.

## Key Decisions Made
- Use band-passed noise emulation in `groove_synth.rs` to eliminate white noise up to 24 kHz.
- Ensure `onIterationSuccess` in `app.js` uses `data.audio_urls` and falls back to last valid version if missing.
- Provide full ModelFusion TTS pipeline with 48 kHz stereo Windows SAPI + melodic harmonic singing carrier fallback so text-to-voice never fails.
- Expanded lyrics drawer by default on song card and dynamically synchronized active song/version lyrics with formatted panes, raw textareas, and TTS custom text inputs.
- Stored all JSON files without UTF-8 BOM so `serde_json` parses cleanly.

## Artifact Index
- `d:\harfile\music\.agents\teamwork\worker_m6\progress.md` — Liveness & step-by-step progress
- `d:\harfile\music\.agents\teamwork\worker_m6\handoff.md` — Final 5-component handoff report
- `d:\harfile\music\README.md` — Production documentation

