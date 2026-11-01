# DISPATCH: worker_m7_remed

## Objective
Remediate the exact findings from `challenger_m7_2` for Milestone 7 Iteration 2:

1. **Fix `drums.wav` Inaudibility on `song_afrobeats_002` in `crates/music_pipeline/src/nodes/node4_stem_inpainting.rs`**:
   - In `separate_stems_pure_rust` (lines 153–166):
     Currently, `drums.wav` is filtered via `StemFilter::highpass(3500.0, 0.707)`. For Afrobeats (and general genres where percussion energy is concentrated in shakers at 260 Hz, claves at 330 Hz, and log drums at 52–175 Hz), this 3500 Hz highpass filter strips away the percussion energy, leaving `drums.wav` at -68.79 dBFS on v1 and -68.67 dBFS on v2 (< -60 dBFS threshold), violating the "ALL INSTRUMENTS MUST BE USED" invariant and failing `tests/audit_stems_non_silence.py`.
   - **Fix**: In `separate_stems_pure_rust`, ensure the drum stem captures the full percussion frequency range:
     Use a wide-band percussion filter (e.g. highpass at 50 Hz to remove DC/rumble, lowpass at 14,000 Hz, or blend the active groove buffer `draft_groove` scaled appropriately, e.g. 0.85 * draft_groove) so that `drums.wav` has robust RMS energy (typically -20 to -35 dBFS, strictly > -50 dBFS) across all genres including Afrobeats.

2. **Fix Exception Handling in `workers/generate_song.py`**:
   - On line 38, change `except ImportError as e:` to:
     ```python
     except Exception as e:
         sys.stderr.write(f"Missing or broken neural dependencies: {e}\n")
         sys.exit(2)
     ```
     This ensures that Windows DLL loading issues (`OSError: [WinError 126] ... shm.dll`) cleanly exit with code 2 (missing/broken dependencies) instead of uncaught crash (code 1).

3. **Regenerate `storage/songs/song_afrobeats_002` (v1 and v2)**:
   - Run the CLI generator to update `song_afrobeats_002` v1 and v2:
     ```powershell
     cargo run -p music_cli -- --create-hit --genre afrobeats --song-id song_afrobeats_002 --bpm 104 --key "C Major" --pure-rust --no-browser
     cargo run -p music_cli -- --iterate "Boost Amapiano log-drum sub-bass +3.5dB, add saxophone horn counter-melody, and accelerate groove to 108 BPM" --song-id song_afrobeats_002 --version 1 --bpm 108 --pure-rust --no-browser
     ```
   - Verify that `drums.wav` in `storage/songs/song_afrobeats_002/v1/stems/drums.wav` and `v2/stems/drums.wav` has RMS > -50 dBFS.

4. **Run Verification**:
   - `python tests/audit_stems_non_silence.py`: MUST PASS with 0 assertion errors!
   - `python tests/validate_disk_songs.py`: MUST PASS 100%!
   - `cargo test --workspace`: MUST PASS 100%!
   - `cargo test --manifest-path tests/Cargo.toml`: MUST PASS 100%!
   - Commit and push verified changes per auto-commit protocol.

## Mandatory Reading
- Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` (MUST READ FIRST)
- Read `d:\harfile\music\.agents\teamwork\challenger_m7_2\handoff.md`

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## 2026-11-01T17:41:27Z
Received message from parent (31352664-c681-465a-a22e-aed6e548c6f0):
You are worker_m7_remed (Milestone 7 Remediation Worker).
Your working directory is: d:\harfile\music\.agents\teamwork\worker_m7_remed\
Tasks:
1. In `crates/music_pipeline/src/nodes/node4_stem_inpainting.rs`, fix `separate_stems_pure_rust` (lines 153-166): do not filter drums with a harsh 3500 Hz highpass that kills low/mid Afrobeats percussion. Use a wide-band percussion filter or blend the active groove buffer `draft_groove` (e.g. 0.85 * draft_groove or wideband 50Hz-14000Hz filter) so that `drums.wav` has audible, robust RMS energy (> -45 dBFS, well above -60 dBFS) across all genres.
2. In `workers/generate_song.py`, change line 38 `except ImportError as e:` to `except Exception as e:` so Windows DLL/`OSError` import issues cleanly exit with code 2.
3. Regenerate `storage/songs/song_afrobeats_002` (v1 and v2) via the CLI.
4. Run verification:
   - `python tests/audit_stems_non_silence.py` (MUST PASS with 0 assertion errors!)
   - `python tests/validate_disk_songs.py` (MUST PASS 100%!)
   - `cargo test --workspace` (MUST PASS 100%!)
   - `cargo test --manifest-path tests/Cargo.toml` (MUST PASS 100%!)
5. Deliver your handoff report in `d:\harfile\music\.agents\teamwork\worker_m7_remed\handoff.md`. Send a completion message to the orchestrator when finished.
