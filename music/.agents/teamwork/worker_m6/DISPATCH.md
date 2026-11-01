# DISPATCH: Milestone 6 Implementation Worker (worker_m6)

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Objective
Implement Milestone 6: Production Audio Fidelity, Scratch-Free Synthesis, ModelFusion Text-to-Voice (TTS) Vocal Engine, Chat Producer Experience & Playback Resilience.

## File Ownership
You exclusively own and may edit:
- `crates/music_dsp/src/groove_synth.rs`
- `crates/music_dsp/src/guitar_wah.rs`
- `crates/music_fusion/src/tts_engine.rs` (new module)
- `crates/music_fusion/src/lib.rs`
- `crates/music_cli/src/rest_server.rs`
- `crates/music_cli/src/main.rs`
- `crates/music_cli/webui/index.html`
- `crates/music_cli/webui/app.js`
- `crates/music_cli/webui/styles.css`
- Sample songs in `storage/songs/` (`roots_reggae_anthem`, `song-one-love-shining-1977`, `song_afrobeats_002`, `song_reggae_anthem_001`)

## Tasks to Complete

### 1. Audio DSP Scratch Removal & Transient Smoothing
In `crates/music_dsp/src/groove_synth.rs`:
- Replace raw broadband PRNG white noise with smooth band-passed/low-passed shaped noise across drum voices:
  - `SnareAcoustic`: 3.2 kHz wire sizzle resonance with smooth lowpass roll-off above 6.5 kHz.
  - `HiHatClosed` / `HiHatTriplet`: Band-limited metallic cluster (7 kHz - 10 kHz).
  - `HiHatOpen`: Resonant metallic ring (6.8 kHz & 8.9 kHz) with gentle high-shelf decay.
  - `Shaker`: 4.5 kHz gentle bead ring with smooth envelope, eliminating sandpaper scratch noise.
  - `CrashCymbal`: 4.2 kHz, 6.1 kHz, 8.3 kHz partial cluster with soft noise.
  - `Clap`: 1.8 kHz acoustic body with soft noise.
In `crates/music_dsp/src/guitar_wah.rs`:
- Verify phase-continuous Karplus-Strong string harmonic wave (`f1*0.64 + f2*0.24 + f3*0.12`), 5.2 kHz cabinet lowpass, and 380-1450 Hz wah filter sweep so guitar plucks are warm and scratch-free.

### 2. ModelFusion Text-to-Voice (TTS) Vocal Engine
Create `crates/music_fusion/src/tts_engine.rs`:
- Implement `FusionTtsEngine` supporting ModelFusion models (`f5-tts`, `kokoro-82m`, `suno/bark`, `coqui/xtts-v2`, `fish-speech-1.5`, `parler-tts-large`, `plachtaa/seed-vc`, `david_lead_zira_choir`).
- Include guaranteed 48 kHz 16-bit stereo Windows SAPI fallback (`Microsoft David Desktop` lead + `Microsoft Zira Desktop` choir + melodic singing pitch carrier) or pure-Rust formant synthesis, producing clear articulated English singing with zero silence.
- Export in `crates/music_fusion/src/lib.rs`.

### 3. Dynamic Port Assignment, Download Endpoints & TTS REST Endpoint
In `crates/music_cli/src/rest_server.rs` and `main.rs`:
- Dynamic port assignment: `bind_server(host: &str, start_port: u16, end_port: u16)` scans 5050..=5070 and binds the first available port (or OS ephemeral fallback), reporting the bound URL.
- Register `POST /api/songs/tts-vocals`:
  - Accepts `song_id`, `version`, `text`, `model`, `voice_style`.
  - Synthesizes vocal stems (`raspy_tenor_vocals.wav`, `yue_vocals.wav`).
  - Remixes `final_master.wav` using vocal-priority sidechain ducking (-1.8 dB ducking during syllables).
  - Returns JSON with `vocal_url`, `master_url`, and `audio_urls`.
- Register download endpoints:
  - `GET /api/songs/:id/versions/:v/download`
  - `GET /api/songs/:id/download`
- Fix audio stem resolution in `handle_stream_audio` to check both version root directory (`v{}/final_master.wav`) and `v{}/stems/` directory across fallback versions.

### 4. WebUI Chat Producer Experience, Inline Players, Device Selector & Light Theme
In `crates/music_cli/webui/index.html`, `app.js`, and `styles.css`:
- **Assistant Producer Reply Bubbles**: When the user sends a prompt or iteration, append an Assistant Producer reply bubble in `#chat-messages` right below the user's message.
  - Conversational producer reply text explaining what was changed (scratch removal, guitar solo wah, vocal synthesis).
  - Live stage-by-stage progress feedback badges: `🎤 VOCALS ADDED: Lead Tenor + Gospel Choir`, `🎸 ALL INSTRUMENTS MIXED`, `🛠️ SCRATCH REMOVED`.
  - Inline controls: `▶ Test This Version`, `🎤 Solo Vocals Only`, `⏪ Revert to Previous Version`, and `⬇ Download WAV`.
- **Audio Output Device Selection (`setSinkId`)**:
  - Automatically apply `State.activeAudioOutputId` via `setSinkId` to dynamically created audio players.
- **ModelFusion TTS Interactive Controls**:
  - In New Song Hero and Active Song Card, add ModelFusion TTS model selector (`f5-tts / kokoro-82m`, `suno/bark`, `coqui/xtts-v2`, `fish-speech-1.5`, `parler-tts-large`, `plachtaa/seed-vc`, `david_lead_zira_choir`), vocal style selector, custom lyrics/text input, and `🎤 Synthesize & Add Text-to-Voice to Song` button.
- **Resilient Playback (Zero 404s)**:
  - Implement `findLastValidAudioUrls()`.
  - In `onIterationSuccess()`, use `data.audio_urls` when returned; if offline or version not on disk, fall back to last valid existing version (`v1` or `v2`) so audio never 404s.
  - Attach error recovery listener to `DOM.masterAudio`.
- **ChatGPT Clean Light Theme**:
  - Maintain #ffffff canvas, #f9f9f9 sidebar, #0d0d0d text, ui-sans-serif, centered 'Hey, user. Ready to dive in?' pill composer, 'Chat | Work' toggle, and 4 extended prefilled examples.

### 5. Recompile & Regenerate Disk Sample Songs
- Recompile: `cargo build -p music_cli`
- Regenerate all 4 sample song pairs in `storage/songs/`:
  1. `roots_reggae_anthem` (v1 and v2)
  2. `song-one-love-shining-1977` (v1 and v2)
  3. `song_afrobeats_002` (v1 and v2)
  4. `song_reggae_anthem_001` (v1 and v2)
- Ensure all 8 version folders have complete sets of 11 WAV files + state/manifest, true peak <= 0.966 (-0.3 dBFS), loud articulated vocals, and zero guitar/drum scratch.
- Run tests (`cargo test -p music_cli`, `cargo test -p music_fusion`) to ensure zero regressions.

## Output
Write detailed handoff report to `d:\harfile\music\.agents\teamwork\worker_m6\handoff.md`.


## 2026-11-01T13:01:26Z
You are worker_m6 (Milestone 6 Implementation Worker).
Your working directory is `d:\harfile\music\.agents\teamwork\worker_m6\`.
Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` before starting work.
Read `d:\harfile\music\.agents\teamwork\worker_m6\DISPATCH.md` for your full task description.
Also read the detailed Explorer findings:
- `d:\harfile\music\.agents\teamwork\explorer_m6_1\handoff.md` (Audio DSP scratch removal, ModelFusion TTS, sample song regeneration)
- `d:\harfile\music\.agents\teamwork\explorer_m6_2\handoff.md` (WebUI chat stage feedback, producer replies, inline players, device selector, TTS controls)
- `d:\harfile\music\.agents\teamwork\explorer_m6_3\handoff.md` (Playback resilience, 404 fallback fix, dynamic port assignment, download endpoints)

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Scope & File Ownership:
You exclusively own and may edit:
- `crates/music_dsp/src/groove_synth.rs`
- `crates/music_dsp/src/guitar_wah.rs`
- `crates/music_fusion/src/tts_engine.rs` (new module)
- `crates/music_fusion/src/lib.rs`
- `crates/music_cli/src/rest_server.rs`
- `crates/music_cli/src/main.rs`
- `crates/music_cli/webui/index.html`
- `crates/music_cli/webui/app.js`
- `crates/music_cli/webui/styles.css`
- Sample songs on disk in `storage/songs/`


## 2026-11-01T13:16:00Z
**Context**: Milestone 6 Implementation Status
**Content**: Heartbeat check: Please provide a brief status update on your progress across Tasks 1–6 (Audio DSP filtering, tts_engine.rs, rest_server/main.rs endpoints, WebUI chat/device selection, and sample song disk regeneration).
**Action**: Update progress.md with your latest timestamp and current task.

## 2026-11-01T13:27:43Z
**Context**: CRITICAL USER REQUIREMENT UPDATE: SAMPLE SONGS MUST INCLUDE LYRICS
**Content**: The user has added a mandatory requirement: "THE SAMPLE SONGS MUST INCLUDE LYRICS".
You must ensure:
1. Every sample song (roots_reggae_anthem v1/v2, song_afrobeats_002 v1/v2, song-one-love-shining-1977 v1/v2, song_reggae_anthem_001 v1/v2) must have complete multi-section English lyrics ([Intro], [Verse 1], [Chorus], [Verse 2], [Guitar Solo], [Bridge], [Outro]):
   - In State.songs[songId].versions[ver].lyrics in crates/music_cli/webui/app.js
   - On disk in checkpoint.json / state.json, manifest.json ("lyrics": state.lyrics), and lyrics.txt in each version directory
   - Audible in yue_vocals.wav, raspy_tenor_vocals.wav, and final_master.wav matching displayed lyrics.
2. In WebUI (crates/music_cli/webui/index.html and app.js):
   - Lyrics drawer section (#lyrics-drawer-body) expanded/visible by default on the song card so users immediately see lyrics without hunting for collapsed controls.
   - switchVersion() dynamically renders active song & version's ver.lyrics into formatted panes, raw textareas, and TTS custom text inputs so switching songs immediately updates displayed lyrics.
**Action**: Incorporate this requirement into Task 4 and Task 5 before submitting your handoff.md.

## 2026-11-01T13:32:18Z
**Context**: Remote Git Repository & Final Ship Instruction
**Content**: The user has created the remote GitHub repository: `https://github.com/oyesanyf/MusicFusion.git`.
When completing your implementation and verification:
1. Ensure a comprehensive, production-ready `README.md` (`# MusicFusion`) is present at the workspace root covering the 5-stage sequential pipeline, ModelFusion TTS, ChatGPT-like Web Studio UI, audio device selection, and CLI usage.
2. In your verification / shipping step, configure remote origin:
   `git remote add origin https://github.com/oyesanyf/MusicFusion.git` (or `git remote set-url origin https://github.com/oyesanyf/MusicFusion.git`), ensure branch is `main`, and verify git status is clean.
**Action**: Note this requirement in your handoff.md.

## 2026-11-01T13:35:12Z
**Context**: CRITICAL NEW REQUIREMENT: HUGGING FACE MODEL DISCOVERY & OLLAMA LIFECYCLE
**Content**: The user has added a critical goal requirement:
1. Dynamic Hugging Face Music & Vocal Model Auto-Discovery & Upgrade (`crates/music_fusion/src/hf_model_discovery.rs` + REST `/api/models/discover-upgrade` + WebUI Controls):
   - Query Hugging Face Hub API across music generation, vocal/TTS synthesis, voice conversion, stem separation.
   - Sizing models against live system memory (free RAM & GPU VRAM) to upgrade active models per stage without OOM.
   - Save to `storage/hf_music_models_registry.json`, expose `GET /api/models/registry` and `POST /api/models/discover-upgrade`, add `--update-models` CLI flag, and wire WebUI button + badge.
2. Automatic Ollama Detection, Silent Installation, Daemon Startup & Dynamic Available Memory Model Selection (`crates/music_fusion/src/ollama_lifecycle.rs` + REST `/api/ollama/bootstrap` + WebUI):
   - Probe `127.0.0.1:11434` with 400ms timeout; locate or silently install via winget/OllamaSetup.exe (strictly obeying Zero-Touch PATH Law: process-scoped PATH only, no registry edits).
   - Start daemon with `CREATE_NO_WINDOW`.
   - Strictly size Ollama models based on runtime free memory (>=48GB/24GB VRAM: 32b, >=24GB/14GB VRAM: 14b, >=12GB/4.5GB VRAM: 7b, >=6GB/2GB VRAM: 3b, <6GB: 1.5b; no -coder suffix).
   - Background `ollama pull <model>` and interactive WebUI status pill card.
**Action**: Please let me know if you can incorporate these two modules and endpoints directly into your current implementation before submitting handoff.md, or if you prefer a focused follow-up iteration.



