# DISPATCH: challenger_m7_2

## Objective
Adversarial Verification of Milestone 7 Subprocess Bridge, Sample Songs & Instrument Completeness:
- Test `UvBridge` and `workers/generate_song.py`:
  * Verify `workers/generate_song.py` exists, is valid Python, accepts the specified CLI args, and handles exits (0, 2, 3) cleanly.
  * Verify `UvBridge` in `crates/music_pipeline/src/python_bridge.rs` uses `CREATE_NO_WINDOW` on Windows and enforces timeouts.
  * Verify graceful fallback to pure-Rust physical modeling synthesizer when GPU/models/uv are unavailable.
  * Verify Tokio console logs match the required formats for HF GPU vs. pure-Rust fallback.
- Test "ALL INSTRUMENTS MUST BE USED" Invariant:
  * Empirically analyze on-disk stems in `storage/songs/` across all 8 versions:
    1. `drums.wav` (One-Drop Kick & Rimshot + Hi-Hats + Nyabinghi)
    2. `bass.wav` (Dub Sub-Bass)
    3. `guitar.wav` (Skank chords on beats 2 & 4 + Lead wah solo)
    4. `other.wav` (Hammond Organ Bubble + Brass/Keys)
    5. `yue_vocals.wav` / `raspy_tenor_vocals.wav` (Lead & Choir Vocals)
    6. `final_master.wav` (Complete Master)
  * Verify every single stem has non-trivial RMS energy (> -60 dBFS) and non-zero samples (no muted, dropped, or silent stems).
- Test on-disk sample song integrity:
  * Run `python tests/validate_disk_songs.py` across all 8 versions.
  * Assert all 88 WAV files exist, have valid 48 kHz stereo PCM16 headers, true-peak <= 0.96605, zero clipping, matching durations (75.79s for 76 BPM, 73.85s for 78 BPM), and matching 7 canonical lyric sections across `lyrics.txt`, `manifest.json`, and `state.json`.
- Run full workspace tests: `cargo test --workspace`.

## Expected Output
Deliver your adversarial report and explicit verdict (**APPROVE** or **REJECT**) in:
`d:\harfile\music\.agents\teamwork\challenger_m7_2\handoff.md`.
