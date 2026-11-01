## 2026-11-01T02:05:44Z
You are a Worker subagent in a Teamwork hierarchy for Milestone 2.
Your working directory is: d:\harfile\music\.agents\teamwork\worker_m2\

MANDATORY FIRST STEP:
Read the authoritative user request at:
d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md
and the master project index at:
d:\harfile\music\PROJECT.md
and the E2E test infra at:
d:\harfile\music\TEST_INFRA.md

READ THE EXPLORER HANDOFF REPORTS BEFORE CODING:
1. WAV I/O, AudioBuffer, and Telemetry specifications:
   d:\harfile\music\.agents\teamwork\explorer_m2_1\handoff.md
2. Universal Drum & Rhythm Synthesizer and Karplus-Strong Guitar/Wah specifications:
   d:\harfile\music\.agents\teamwork\explorer_m2_2\handoff.md
3. Vocal Formant, Sample-Accurate Crossfader, and Tape Mastering specifications:
   d:\harfile\music\.agents\teamwork\explorer_m2_3\handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

EXCLUSIVE WRITE OWNERSHIP:
You exclusively own:
- crates/music_dsp/Cargo.toml
- crates/music_dsp/src/lib.rs
- crates/music_dsp/src/error.rs
- crates/music_dsp/src/wav.rs
- crates/music_dsp/src/telemetry_calc.rs
- crates/music_dsp/src/groove_synth.rs
- crates/music_dsp/src/guitar_wah.rs
- crates/music_dsp/src/vocal_formant.rs
- crates/music_dsp/src/crossfader.rs
- crates/music_dsp/src/tape_mastering.rs
- crates/music_dsp/tests/dsp_tests.rs

IMPLEMENTATION REQUIREMENTS:
1. Implement `crates/music_dsp/src/wav.rs`:
   - `AudioBuffer` struct (sample rate 48,000 Hz, channels, samples `Vec<f32>`, duration, frame count).
   - Methods: `mix`, `scale_gain`, `normalize`, `clamp`, `resample_linear`, `slice`, `peak_level`, `rms_level`.
   - Pure-Rust RIFF WAV reader and writer supporting 16-bit, 24-bit PCM and 32-bit float without external C libraries.
2. Implement `crates/music_dsp/src/telemetry_calc.rs`:
   - Function `calculate_telemetry(buffer: &AudioBuffer, expected_bpm: u32) -> StemTelemetry`.
   - Authentic digital signal processing algorithms: peak dBFS, RMS dBFS, dynamic range dB, clipping detection ($|s| \ge 1.0$), tempo drift score via autocorrelation, SNR dB, and Radix-2 FFT spectral flatness.
3. Implement `crates/music_dsp/src/groove_synth.rs`:
   - Universal drum pattern synthesizer generating 48 kHz stereo audio buffers across all genres and BPMs (40–220): One-Drop Reggae (76 BPM, kick + rimshot on 3, nyabinghi hand drums), Four-on-the-Floor (120–128 BPM), Boom-Bap & Trap (70–144 BPM, pitch-bent 808 sub bass), Afrobeats & Amapiano (108–114 BPM log-drums, clave), Rock & Metal (128–180 BPM double-kick backbeat), Jazz swing ride (110–160 BPM), Orchestral timpani rolls (60–100 BPM in 6/8).
4. Implement `crates/music_dsp/src/guitar_wah.rs`:
   - Extended Karplus-Strong physical string delay model with fractional delay and pick noise excitation.
   - Chamberlin state-variable Wah-Wah filter (400 Hz–2200 Hz sweep with LFO & envelope follower).
   - 16-bar soaring psychedelic blues/rock guitar solo generator in A Major / modal scales.
5. Implement `crates/music_dsp/src/vocal_formant.rs`:
   - Formant filter bank (F1, F2, F3 parallel peak filters) for Jamaican raspy tenor and universal vocal timbres.
   - Asymmetric non-linear harmonic saturation for vocal rasp without digital clipping.
   - Pitch-lock vocoder / phase alignment helper.
6. Implement `crates/music_dsp/src/crossfader.rs`:
   - Sample-accurate stem inpainting crossfader locked to BPM beat grid.
   - Equal-power (sine/cosine) and linear crossfade curves.
7. Implement `crates/music_dsp/src/tape_mastering.rs`:
   - 48 kHz analog tape saturation using $\tanh(x)$ curve.
   - 55–85 Hz sub-bass warmth resonant filter.
   - Reference spectral EQ Matchering (5 profiles: Studer A800 1970s tape, Modern Pop, Club EDM, Orchestral, Audiophile Jazz).
   - Peak brickwall limiter guaranteeing zero digital clipping ($|sample| \le 0.98$).
8. Implement `crates/music_dsp/src/lib.rs` and `crates/music_dsp/tests/dsp_tests.rs`:
   - Comprehensive unit tests verifying WAV roundtrips, telemetry math, drum pattern timings, guitar physical modeling, vocal formant shaping, crossfade smoothness, and tape mastering.
9. Verification:
   - Run `cargo check --workspace`
   - Run `cargo test -p music_dsp`
   - Run `cargo test --workspace`
   - Run `cargo test --manifest-path tests/Cargo.toml`
   - Ensure 100% tests pass with zero warnings or errors.
10. Commit:
   - Stage and commit the clean changes cleanly with git.
