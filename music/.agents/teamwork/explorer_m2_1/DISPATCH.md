## 2026-11-01T01:59:37Z
Sender: 4caf3f77-b3ac-4b67-8bda-934a91faafab
Priority: MESSAGE_PRIORITY_HIGH
Content:
You are an Explorer subagent in a Teamwork hierarchy for Milestone 2.
Your working directory is: d:\harfile\music\.agents\teamwork\explorer_m2_1\

MANDATORY FIRST STEP:
Read the authoritative user request at:
d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md
and the master project index at:
d:\harfile\music\PROJECT.md
and test infra contracts at:
d:\harfile\music\TEST_INFRA.md
and existing reference DSP implementations in tests:
d:\harfile\music\tests\common\mod.rs

YOUR MISSION:
Plan the detailed implementation of WAV I/O, AudioBuffer, and Telemetry calculations for `crates/music_dsp`.
You are read-only: do NOT modify source code files directly.
1. Design `crates/music_dsp/Cargo.toml` dependencies (dependencies on `music_core`, `thiserror`, `anyhow`, etc.).
2. Design `crates/music_dsp/src/wav.rs`:
   - `AudioBuffer` struct (stereo/mono, sample rate 48,000 Hz, duration, channels, sample vectors).
   - Core buffer operations: `mix()`, `scale_gain()`, `normalize()`, `clamp()`, `resample_linear()`, `slice()`.
   - Pure-Rust RIFF WAV reader and writer supporting 16-bit, 24-bit PCM and 32-bit float WAV files without external C libraries.
3. Design `crates/music_dsp/src/telemetry_calc.rs`:
   - Function `calculate_telemetry(buffer: &AudioBuffer, expected_bpm: u32) -> StemTelemetry`.
   - Authentic digital signal processing algorithms: peak dBFS, RMS dBFS, dynamic range, true clipping detector, tempo drift analysis, SNR in dB, spectral flatness (geometric mean / arithmetic mean of power spectrum).
4. Detail comprehensive unit tests for WAV roundtrips and mathematical verification.

Write your report to:
d:\harfile\music\.agents\teamwork\explorer_m2_1\handoff.md

Update your progress in d:\harfile\music\.agents\teamwork\explorer_m2_1\progress.md.
When finished, send a brief message with your handoff path.
