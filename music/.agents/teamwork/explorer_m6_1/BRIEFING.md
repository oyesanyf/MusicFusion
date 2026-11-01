# BRIEFING — 2026-11-01T13:00:00Z

## Mission
Investigate Audio DSP scratch noise removal in `guitar_wah.rs` and `groove_synth.rs`, ModelFusion TTS vocal engine integration across `crates/music_fusion` and pipeline nodes, and disk sample song regeneration steps.

## 🔒 My Identity
- Archetype: explorer
- Roles: Audio DSP & Vocal TTS Explorer
- Working directory: d:\harfile\music\.agents\teamwork\explorer_m6_1\
- Original parent: 22bd2f00-efbc-429d-9417-8639ee3e2bc7
- Milestone: Audio DSP & Vocal TTS Analysis (M6)

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Write reports, analysis, proposed code snippets and handoffs only in `.agents/teamwork/explorer_m6_1/`
- Zero-wait incremental progress updates
- No mock data or fake implementations

## Current Parent
- Conversation ID: 22bd2f00-efbc-429d-9417-8639ee3e2bc7
- Updated: 2026-11-01T13:00:00Z

## Investigation State
- **Explored paths**:
  - `crates/music_dsp/src/guitar_wah.rs` (Karplus-Strong physical string oscillator, WahWahFilter SVF sweep, GuitarAmpSimulator, SoloGenerator)
  - `crates/music_dsp/src/groove_synth.rs` (UniversalDrumSynthesizer, DspRng, DrumVoice noise synthesis)
  - `crates/music_dsp/src/crossfader.rs` (EqualPowerSine & EqualPowerSqrt splice_stem)
  - `crates/music_pipeline/src/nodes/node4_stem_inpainting.rs` (4-stem band separation, highpass drums filter, drum fill splicing)
  - `crates/music_pipeline/src/nodes/node2_groove_yue.rs` (SpeechSynthesizer SAPI vocals, harmonic carrier, tape slapback)
  - `crates/music_pipeline/src/nodes/node3_vocal_timbre.rs` (Formant filter bank, PitchLockVocoder, Haas presence, peak normalization)
  - `crates/music_pipeline/src/nodes/node6_analog_mastering.rs` (Vocal-priority sidechain ducking, tape saturation, brickwall peak limiter)
  - `crates/music_fusion/` (SystemMemory, ExecutionRoute, OllamaProducerPanel)
  - `crates/music_cli/src/rest_server.rs` (Axum router, audio streaming, iteration and master handlers)
  - `crates/music_cli/webui/app.js` & `index.html` (iteration failure root cause and audio URL fallback)
  - Disk sample songs in `storage/songs/` (`roots_reggae_anthem`, `song_afrobeats_002`, `song-one-love-shining-1977`, `song_reggae_anthem_001`)

- **Key findings**:
  1. Scratch noise root cause 1: `groove_synth.rs` injects raw, unfiltered white noise into 8 drum voices (`SnareAcoustic`, `HiHatClosed`, `HiHatOpen`, `Shaker`, `CrashCymbal`, `Clap`, `KickAcoustic`, `SnareRimshot`). Full-spectrum noise up to 24 kHz sounds like abrasive sandpaper/radio static.
  2. Scratch noise root cause 2: `node4_stem_inpainting.rs` applies a highpass filter at 3500 Hz (`drums_hp`) during stem separation, isolating and amplifying high-frequency noise hash into `drums.wav` and `inpainted_drums.wav`.
  3. Scratch noise root cause 3: In `app.js`, `onIterationSuccess` blindly overwrites `audioUrls.master` with `.../versions/${nextVer}/audio/final_master`, causing 404 broken playback when iterations fail or when v3 was incomplete.
  4. Vocal TTS: ModelFusion TTS models (`f5-tts`, `kokoro-82m`, `suno/bark`, `coqui/xtts-v2`, `fish-speech-1.5`, `parler-tts-large`, `plachtaa/seed-vc`) can be encapsulated in `crates/music_fusion/src/tts_engine.rs` with guaranteed 48 kHz 16-bit stereo SAPI fallback (`Microsoft David Desktop` lead + `Microsoft Zira Desktop` choir + harmonic carrier).
  5. Disk sample songs: Exact regeneration commands and duration invariants (75.78s / 73.84s for Reggae, 55.38s / 53.33s for Afrobeats) documented with full stem and master artifacts.

- **Unexplored areas**:
  - None within explorer scope. Ready to draft handoff report.

## Key Decisions Made
- Recommending bandpass/lowpass filtering of all noise voices in `groove_synth.rs` (shaker, hats, snare) with gentle high-frequency roll-off above 7–10 kHz.
- Designing dedicated `tts_engine.rs` in `music_fusion` and `POST /api/songs/tts-vocals` endpoint in `rest_server.rs`.
- Proposing exact fix for `app.js` to prevent 404 broken audio when iterating.

## Artifact Index
- `BRIEFING.md` — persistent memory
- `progress.md` — heartbeat and progress tracking
- `handoff.md` — comprehensive 5-component handoff report
