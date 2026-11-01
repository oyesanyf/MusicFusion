# DISPATCH: worker_m7

## Objective
Implement Milestone 7:
1. **R1: Pure-Rust Physical Modeling Synthesis**:
   - In `crates/music_pipeline/src/nodes/node2_groove_yue.rs`, delete stepped frequency-sweep oscillator and hard silence gating.
   - Maintain continuous phase accumulator across buffer boundaries (`ContinuousPhaseAccumulator`: `phase += 2.0 * PI * freq / sample_rate`).
   - Lock synthesis to 76 BPM (0.7894s / beat) and strict A Major pentatonic / harmonic scale (220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz).
   - In `crates/music_dsp/src/guitar_wah.rs`, implement Karplus-Strong string algorithm using delay-line ring buffer excited by low-pass filtered pink noise burst, 0.994 feedback damping factor for reggae offbeat skank chords (beats 2 & 4) and pentatonic guitar leads.
   - In `crates/music_dsp/src/groove_synth.rs`, implement Reggae One-Drop drum & dub bass synthesizer:
     * Kick on Beat 3: rapid exponential pitch-drop sine (110 Hz down to 42 Hz over 60 ms) + band-pass white noise snare/rimshot burst (250-1800 Hz).
     * 16th-note closed hi-hats: high-pass filtered metallic square-wave ratios (>6000 Hz, 25 ms decay envelope).
     * Dub Sub-Bass: warm low-pass filtered (180 Hz cutoff) sine + triangle bassline walking on roots and fifths (A1 = 55.0 Hz, D2 = 73.42 Hz, E2 = 82.41 Hz) with 15 ms linear attack, 120 ms exponential release.
     * Hammond Organ Bubble: additive drawbar sine harmonics (1x, 2x, 3x, 4x) with 6 Hz rotary speaker tremolo LFO, ADSR.

2. **R2: Stem Headroom, Tape Saturation Mastering & WAV Serialization**:
   - In `crates/music_dsp/src/wav.rs`, sanitize `to_wav_bytes_with_format` by removing disruptive 4ms edge-fading so PCM roundtrips are bit-accurate (fixing all 5 failing unit tests). Add `hound` 16-bit PCM serialization helper.
   - Add `hound = "3.5"` to `Cargo.toml` (workspace), `crates/music_dsp/Cargo.toml`, and `crates/music_pipeline/Cargo.toml`.
   - In `crates/music_pipeline/src/nodes/node6_analog_mastering.rs` (and `node5_dsp_judge_gate.rs`):
     * Process internal stems in normalized f32 (-1.0 to 1.0).
     * Scale each individual stem by 0.20 (-14 dBFS RMS headroom) before summing stems together.
     * Apply 2nd-order Butterworth low-pass filter at 12,000 Hz on synthetic instruments.
     * Apply smooth `f32::tanh(sample * 1.2)` soft-clipping analog tape saturator.
     * Peak normalize to -1.0 dBFS (0.89125).
     * Write 16-bit PCM samples via `hound`.
     * Add `"lyrics": state.lyrics` into `manifest.json`.

3. **R3: uv Subprocess Bridge for Neural Audio with Graceful Fallback**:
   - In `crates/music_fusion/src/memory.rs`, add `NeuralModelChoice` and `select_neural_model(&self, engine_mode: &str) -> NeuralModelChoice` mapping live VRAM: `>= 12GB` -> `YuE-s1-7B`, `>= 6GB` -> `ACE-Step-v1.5`, `>= 3.5GB` -> `musicgen-medium`, `< 3.5GB` / pure_rust -> `PureRustFallback`.
   - In `crates/music_pipeline/src/python_bridge.rs`, implement `UvBridge`:
     * Locates `uv`.
     * Bootstraps `d:\harfile\music\.venv` via `uv venv`.
     * Installs dependencies via `uv pip install`: `torch`, `torchaudio`, `transformers`, `diffusers`, `accelerate`, `soundfile`. Caches via `.deps_installed` marker file.
     * Invokes `workers/generate_song.py` via `tokio::process::Command` (with `CREATE_NO_WINDOW` on Windows).
   - Create `workers/generate_song.py`:
     * Arguments: `--model`, `--prompt`, `--lyrics_file`, `--duration`, `--output_dir`, etc.
     * Generates `yue_vocals.wav` and `yue_instrumentals.wav` at 48 kHz stereo.
     * Exits 0 on success, 2 if weights unavailable/offline, 3 on CUDA OOM.
   - In `crates/music_pipeline/src/nodes/node2_groove_yue.rs`:
     * In auto/hf_hybrid mode, invoke `UvBridge`.
     * If GPU VRAM insufficient or weights unavailable, cleanly fall back to pure-Rust physical modeling synthesizer.
     * Print explicit Tokio console logs:
       - HF GPU: `🎛️  [SYNTHESIS ENGINE] Track rendered by Hugging Face GPU weights (Model: ..., Device: CUDA, VRAM: ... GB free)`
       - Fallback: `⚠️  [SYNTHESIS ENGINE FALLBACK] GPU VRAM insufficient or neural weights/dependencies unavailable: .... Automatically falling back to pure-Rust physical modeling synthesizer.`
       - Pure-Rust: `🎹  [SYNTHESIS ENGINE] Track rendered by pure-Rust physical modeling synthesizer (Karplus-Strong string algorithm, Reggae one-drop drums & dub bass)`

4. **R4: Regenerate Stored Sample Songs & Run Verification**:
   - Re-run `--create-reggae-anthem` and verify via the Rust spectral/clipping inspector that zero samples clip (> 0 dBFS) and no silence dropouts occur.
   - Regenerate all 4 stored sample songs in `storage/songs/` (both v1 and v2):
     * `roots_reggae_anthem`
     * `song-one-love-shining-1977`
     * `song_afrobeats_002`
     * `song_reggae_anthem_001`
   - Run verification tests:
     * `cargo test -p music_core`
     * `cargo test -p music_dsp`
     * `cargo test -p music_fusion`
     * `cargo test -p music_pipeline`
     * `cargo test -p music_cli`
     * `cargo test --manifest-path tests/Cargo.toml`
     * `python tests/validate_disk_songs.py`
     * Confirm 100% test pass rate across all crates.
   - Stage, commit, and push verified changes per auto-commit protocol.

## Mandatory Reading
- Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` (MUST READ FIRST)
- Read Explorer Reports:
  - `d:\harfile\music\.agents\teamwork\explorer_m7_1\handoff.md` (DSP & Headroom)
  - `d:\harfile\music\.agents\teamwork\explorer_m7_2\handoff.md` (uv Python Bridge)
  - `d:\harfile\music\.agents\teamwork\explorer_m7_3\handoff.md` (Sample Songs & Tests)

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.


## 2026-11-01T16:20:39Z
You are worker_m7 (Milestone 7 Implementation Worker).
Your working directory is: d:\harfile\music\.agents\teamwork\worker_m7\

You MUST read:
1. d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md (MUST READ FIRST)
2. d:\harfile\music\.agents\teamwork\worker_m7\DISPATCH.md
3. d:\harfile\music\.agents\teamwork\explorer_m7_1\handoff.md (DSP, Karplus-Strong, Headroom, Mastering)
4. d:\harfile\music\.agents\teamwork\explorer_m7_2\handoff.md (uv Python Bridge, VRAM Sizing, generate_song.py, Logging)
5. d:\harfile\music\.agents\teamwork\explorer_m7_3\handoff.md (Sample Song Regeneration & Full Test Verification)


## 2026-11-01T16:32:40Z
CRITICAL USER REQUIREMENT REINFORCEMENT: "all instruments must be used".
You must ensure that ALL 7 instrument groups are actively synthesized, separated/preserved across stems, and mixed into `final_master.wav` for every song generation:
1. One-Drop Kick & Band-Passed Rimshot/Snare
2. >6000 Hz 16th-Note Closed Hi-Hats & Nyabinghi Percussion
3. Low-Pass 180 Hz Dub Sub-Bass walking on roots/fifths
4. Karplus-Strong Plucked Offbeat Skank Guitar Chords on beats 2 & 4
5. Karplus-Strong Pentatonic Lead Wah Guitar Solo
6. Additive Drawbar Hammond B3 Organ Bubble with 6 Hz rotary tremolo LFO + Brass/Keys
7. Lead & Backing Choir Vocals

No instrument stem may be dropped, muted, or left silent in either the individual stems or the summed master mix! Verify each stem has audible, non-silent energy.
