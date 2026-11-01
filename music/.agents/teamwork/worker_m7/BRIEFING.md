# BRIEFING — 2026-11-01T17:13:00Z

## Mission
Implement Milestone 7: Pure-Rust physical modeling synthesis, Karplus-Strong string algorithm, Reggae one-drop drums & dub bass, stem headroom (-14 dBFS RMS, 0.20 scaling) & tape saturation mastering (-1.0 dBFS, 12kHz Butterworth LPF, hound 16-bit PCM), uv Python neural model bridge with fallback & logging, and sample songs regeneration.

## 🔒 My Identity
- Archetype: worker
- Roles: implementer, qa, specialist
- Working directory: d:\harfile\music\.agents\teamwork\worker_m7\
- Original parent: 31352664-c681-465a-a22e-aed6e548c6f0
- Milestone: Milestone 7

## 🔒 Key Constraints
- DO NOT CHEAT. All implementations must be genuine. No dummy or facade implementations.
- Continuous phase accumulator across buffer boundaries (`phase += 2.0 * PI * freq / sample_rate`).
- Strict 76 BPM and A Major pentatonic scale frequencies (220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz).
- Karplus-Strong plucked string with 0.994 feedback damping and pink noise excitation.
- Reggae one-drop kick (110->42 Hz over 60ms) on beat 3 + 250-1800 Hz BP snare, >6000 Hz hi-hats with 25ms decay, dub sub-bass (180 Hz LP, 15ms attack, 120ms release), Hammond organ bubble (1x-4x drawbars, 6 Hz tremolo LFO, ADSR).
- Internal stems processed in normalized f32 scaled by 0.20 before summing; 12,000 Hz Butterworth LPF, tanh(sample * 1.2) tape saturation, peak normalize to -1.0 dBFS (0.89125), hound 16-bit PCM WAV.
- Add "lyrics": state.lyrics to manifest.json.
- Dynamic VRAM sizing for Hugging Face neural models: >=12GB YuE-s1-7B, >=6GB ACE-Step-v1.5, >=3.5GB musicgen-medium, <3.5GB pure_rust fallback.
- Tokio console logging for neural vs pure-Rust.
- Regenerate all 4 sample songs across v1 and v2 in storage/songs/.
- Zero samples clip (>0 dBFS), no silence dropouts. All tests pass across workspace.

## Current Parent
- Conversation ID: 31352664-c681-465a-a22e-aed6e548c6f0
- Updated: 2026-11-01T17:13:00Z

## Task Summary
- **What to build**: Physical modeling synthesis, stem headroom & analog tape mastering, uv bridge & workers/generate_song.py, sample songs regeneration.
- **Success criteria**: All 4 requirements (R1, R2, R3, R4) fully met, 100% test pass rate across all crates, sample songs regenerated and validated.
- **Interface contracts**: `PROJECT.md`, `ORIGINAL_REQUEST.md`.
- **Code layout**: Per `PROJECT.md` and cargo workspace.

## Change Tracker
- **Files modified**:
  - `crates/music_dsp/Cargo.toml`, `crates/music_pipeline/Cargo.toml`, `Cargo.toml`: Added `hound = "3.5"`.
  - `crates/music_dsp/src/error.rs`: Added `HoundError`.
  - `crates/music_dsp/src/wav.rs`: Removed edge-fade/soft-knee in `to_wav_bytes_with_format`; implemented `to_wav_file_hound_pcm16`.
  - `crates/music_dsp/src/guitar_wah.rs`: ContinuousPhaseAccumulator, PinkNoiseGenerator, Karplus-Strong 0.994 feedback damping, A Major pentatonic scale, `synthesize_reggae_skank_track`.
  - `crates/music_dsp/src/groove_synth.rs`: KickOneDrop, SnareOneDrop, HiHat16thMetallic, DubSubBass, HammondOrganBubble, `synthesize_reggae_one_drop_drums_and_dub_bass`.
  - `crates/music_dsp/src/tape_mastering.rs`: ButterworthLowpass12kHz.
  - `crates/music_fusion/src/memory.rs`: NeuralModelChoice and `select_neural_model`.
  - `crates/music_fusion/src/judge.rs`: Complete 7-section blueprint and lowercase YuE tag mapping.
  - `workers/generate_song.py`: CLI python worker with exit codes (0, 2, 3).
  - `crates/music_pipeline/src/python_bridge.rs`: UvBridge bootstrapping and execution.
  - `crates/music_pipeline/src/nodes/node2_groove_yue.rs`: Pure-Rust physical modeling wiring and Tokio logging.
  - `crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs`: 0.20 stem scaling headroom.
  - `crates/music_pipeline/src/nodes/node6_analog_mastering.rs`: 0.20 stem scaling, 12kHz Butterworth LPF, tanh tape saturation, peak normalization to -1.0 dBFS, hound 16-bit PCM, `"lyrics": state.lyrics` in manifest.json.
  - `storage/songs/`: All 4 sample songs across v1 and v2 regenerated.
- **Build status**: 100% pass across all workspace crates (core, dsp, fusion, pipeline, cli, e2e)
- **Pending issues**: None

## Quality Status
- **Build/test result**: All tests passing (100% pass rate)
- **Lint status**: 0 violations
- **Tests added/modified**: Verified against comprehensive adversarial suites and `tests/validate_disk_songs.py`

## Loaded Skills
- None

## Key Decisions Made
- Used hound 16-bit PCM WAV encoder for final_master.wav.
- Scaled internal stems by 0.20 for 14 dBFS RMS headroom before Butterworth 12 kHz low-pass filtering and tanh tape saturation.
- Maintained exact 24-bar length (2 bars intro, 4 bars verse 1, 4 bars chorus, 4 bars verse 2, 4 bars guitar solo, 4 bars bridge, 2 bars outro) matching 75.78s (76 BPM) and 73.84s (78 BPM).

## Artifact Index
- `DISPATCH.md` — Implementation assignment
- `BRIEFING.md` — Persistent situational memory
- `progress.md` — Progress tracker and heartbeat
- `handoff.md` — Final handoff report
