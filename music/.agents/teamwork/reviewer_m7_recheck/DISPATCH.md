# DISPATCH: reviewer_m7_recheck

## Role
Milestone 7 Recheck Reviewer

## Objective
Review and verify both Milestone 7 remediations:
1. Afrobeats `drums.wav` RMS non-silence fix in `crates/music_pipeline/src/nodes/node4_stem_inpainting.rs` (3-band percussion filter + `draft_groove` blend) and Python worker `except Exception as e:` in `workers/generate_song.py`.
2. Melodic Singing Synthesis in `crates/music_dsp/src/melodic_singing.rs`, `crates/music_pipeline/src/nodes/node2_groove_yue.rs`, `node3_vocal_timbre.rs`, `crates/music_fusion/src/tts_engine.rs`:
   - Complete elimination of flat desktop screen-reader speech (`System.Speech` / `Microsoft David Desktop`).
   - Authentic 48 kHz stereo melodic singing synthesis locked to scale notes (A Major Pentatonic: 220.0, 246.94, 277.18, 329.63, 369.99, 440.0 Hz).
   - Natural 5.5 Hz vibrato, glottal pulse harmonics, vowel formants, 3-part harmony choir backing (Root + Major 3rd + Perfect 5th), 12ms stereo doubler, and Schroeder studio reverb.
   - Top 7 open HF model catalog (`ASLP-lab/DiffRhythm`, `ACE-Step/ACE-Step-v1-3.5B`, `m-a-p/YuE-s1-7B-anneal-en-cot`, `suno/bark`, `fishaudio/fish-speech-1.5`, `Plachtaa/Seed-VC`, `facebook/musicgen-medium`) in `hf_model_discovery.rs`, `workers/generate_song.py`, and WebUI (`index.html` & `app.js`).
3. Verification of all stored sample songs on disk across v1 and v2 (`roots_reggae_anthem`, `song-one-love-shining-1977`, `song_afrobeats_002`, `song_reggae_anthem_001`):
   - All 7 instrument groups active (RMS > -60 dBFS across all stems).
   - Loud, melodic, musically sung vocals in `yue_vocals.wav`, `raspy_tenor_vocals.wav`, and `final_master.wav`.
   - Zero digital clipping (peaks <= -1.0 dBFS / 0.891).
   - Workspace tests pass (`cargo test --workspace`) and E2E tests pass (`cargo test --manifest-path tests/Cargo.toml`).

## Mandatory Reading
- `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` (MUST READ FIRST)
- `d:\harfile\music\.agents\teamwork\worker_m7_remed\handoff.md`
- `d:\harfile\music\.agents\teamwork\worker_m7_vocal\handoff.md`

## Instructions
1. Run build and test commands to verify code health.
2. Review implementation code across `crates/music_dsp/src/melodic_singing.rs`, `node2_groove_yue.rs`, `node3_vocal_timbre.rs`, `node4_stem_inpainting.rs`, `tts_engine.rs`, `hf_model_discovery.rs`, `generate_song.py`, and WebUI files.
3. Verify all 7 instruments are active on disk.
4. Issue an unambiguous verdict: **APPROVE** or **REQUEST_CHANGES**.
5. Write your report in `d:\harfile\music\.agents\teamwork\reviewer_m7_recheck\handoff.md` and notify parent.


## 2026-11-01T19:04:09Z
You are `reviewer_m7_recheck`, the Milestone 7 Recheck Reviewer.
Your working directory is `d:\harfile\music\.agents\teamwork\reviewer_m7_recheck\`.

MANDATORY FIRST STEP:
Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` and `d:\harfile\music\.agents\teamwork\reviewer_m7_recheck\DISPATCH.md`. Also read the workers' handoffs:
- `d:\harfile\music\.agents\teamwork\worker_m7_remed\handoff.md`
- `d:\harfile\music\.agents\teamwork\worker_m7_vocal\handoff.md`

TASKS:
1. Objectively and thoroughly review the implementations for:
   - Afrobeats drums.wav non-silence fix (3-band filter + draft_groove blend in `node4_stem_inpainting.rs`) and Python exception exit code 2 in `workers/generate_song.py`.
   - Melodic Singing Synthesis in `crates/music_dsp/src/melodic_singing.rs`, `node2_groove_yue.rs`, `node3_vocal_timbre.rs`, `tts_engine.rs` (removal of flat screen-reader speech, scale-locking to A Major Pentatonic, vibrato, 3-part harmony choir, Schroeder studio reverb).
   - Top 7 open HF model catalog in `hf_model_discovery.rs`, `workers/generate_song.py`, and WebUI.
   - Sample songs on disk: 8 song versions, 88 WAV files, 56 stems, verifying non-silence (> -50 dBFS RMS) and no clipping.
2. Run build and test checks:
   - `python tests/audit_stems_non_silence.py`
   - `python tests/validate_disk_songs.py`
   - `cargo test --workspace`
   - `cargo test --manifest-path tests/Cargo.toml`
3. Issue an unambiguous verdict: **APPROVE** or **REQUEST_CHANGES**.
4. Write your report in `d:\harfile\music\.agents\teamwork\reviewer_m7_recheck\handoff.md` and notify parent.
