# DISPATCH: challenger_m7_recheck

## 2026-11-01T19:04:11Z

## Role
Milestone 7 Recheck Challenger

## Objective
Empirically stress-test and challenge Milestone 7 remediations:
1. Verify "ALL INSTRUMENTS MUST BE USED" invariant:
   - Run `python tests/audit_stems_non_silence.py` across all 56 stems. Verify every single stem has RMS > -50.0 dBFS (including `song_afrobeats_002` v1 and v2 `drums.wav`).
2. Verify Disk Song Integrity & Audio Limits:
   - Run `python tests/validate_disk_songs.py` across all 88 WAV files.
   - Assert zero samples clip (> 0.0 dBFS) and true peaks are within -1.0 dBFS (<= 0.966 / 0.891).
3. Verify Melodic Singing Synthesis:
   - Verify `yue_vocals.wav`, `raspy_tenor_vocals.wav`, and `final_master.wav` feature genuine melodic singing locked to scale notes, sustained vowel formants, natural 5.5 Hz vibrato, and 3-part choir harmonies (never flat spoken `Microsoft David` speech).
4. Verify Workspace & E2E Suites:
   - Run `cargo test --workspace`
   - Run `cargo test --manifest-path tests/Cargo.toml`
5. Issue an unambiguous verdict: **APPROVE** or **REJECT**.
6. Write your report in `d:\harfile\music\.agents\teamwork\challenger_m7_recheck\handoff.md` and notify parent.

## Mandatory Reading
- `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` (MUST READ FIRST)
- `d:\harfile\music\.agents\teamwork\worker_m7_remed\handoff.md`
- `d:\harfile\music\.agents\teamwork\worker_m7_vocal\handoff.md`
