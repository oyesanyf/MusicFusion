# Progress — challenger_m7_2

Last visited: 2026-11-01T17:40:00Z

## Verification Plan & Status
- [x] Vector 1: `workers/generate_song.py` and `UvBridge` analysis & execution
  - [x] Verified `generate_song.py` existence, AST syntax validity, all CLI arguments parsing.
  - [x] Empirically verified exit codes (0 = success, 2 = missing weights/dependencies, 3 = CUDA OOM).
  - [x] Discovered edge case: Windows torch `shm.dll` loading raises `OSError` (uncaught by `except ImportError:`, producing exit code 1 instead of 2).
  - [x] Verified `UvBridge` in `python_bridge.rs` enforces `CREATE_NO_WINDOW` (0x08000000) on Windows across all `Command` invocations.
  - [x] Verified `UvBridge` enforces timeouts via `tokio::time::timeout`.
  - [x] Verified graceful fallback to pure-Rust physical modeling in `node2_groove_yue.rs`.
  - [x] Verified Tokio console logging strings match exact required formats.
- [x] Vector 2: "ALL INSTRUMENTS MUST BE USED" empirical audio analysis
  - [x] Empirically analyzed 56 on-disk stems across all 8 versions (`storage/songs/`).
  - [x] Reggae songs (6 versions: `roots_reggae_anthem`, `song-one-love-shining-1977`, `song_reggae_anthem_001` across v1 & v2):
    * drums.wav: -41.26 dBFS / -41.15 dBFS (> -60 dBFS) — PASSED
    * bass.wav: -16.46 dBFS (> -60 dBFS) — PASSED
    * guitar.wav: -42.47 dBFS / -42.45 dBFS (> -60 dBFS) — PASSED
    * other.wav: -25.59 dBFS / -25.58 dBFS (> -60 dBFS) — PASSED
    * yue_vocals.wav: -21.32 dBFS / -21.31 dBFS (> -60 dBFS) — PASSED
    * raspy_tenor_vocals.wav: -21.11 dBFS / -21.09 dBFS (> -60 dBFS) — PASSED
    * final_master.wav: -16.99 dBFS / -17.25 dBFS (> -60 dBFS) — PASSED
  - [!] `song_afrobeats_002` (v1 and v2):
    * drums.wav: **-68.79 dBFS** (v1) and **-68.67 dBFS** (v2) — **FAILED INVARIANT (> -60 dBFS RMS)**!
    * `tests/audit_stems_non_silence.py` fails with: `AssertionError: SILENT STEM DETECTED: song_afrobeats_002/v1/drums.wav rms=-68.52996626651088`.
    * Root cause traced to `StemFilter::highpass(3500.0, 0.707)` in `separate_stems_pure_rust` (`node4_stem_inpainting.rs`), which filters out low/mid Afrobeats percussion.
- [x] Vector 3: Run `python tests/validate_disk_songs.py`
  - [x] 100% empirical validation passed: 88 WAV files verified (48 kHz stereo PCM16 headers, true-peak <= 0.96605, zero clipping, matching durations 75.79s @ 76 BPM and 73.85s @ 78 BPM, 7 canonical lyric sections, zero UTF-8 BOM).
- [x] Vector 4: Full workspace test suite execution
  - [x] `cargo test --workspace`: 100% passed (music_core: 27/27, music_dsp: 104/104, music_fusion: 43/43, music_pipeline: 60/60, music_cli: 39/39).
  - [x] `cargo test --manifest-path tests/Cargo.toml`: 191/191 passed.
- [x] Vector 5: Hand-off report and final verdict: REJECT due to `song_afrobeats_002` `drums.wav` RMS < -60 dBFS invariant violation.
