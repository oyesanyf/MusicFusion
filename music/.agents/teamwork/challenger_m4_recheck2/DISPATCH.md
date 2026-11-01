# DISPATCH: Milestone 4 Gate Challenger 2 (Recheck)

## Mission
Empirically verify audio waveform distinctness between v1 and v2 test songs on disk, brickwall limiter ceiling, and storage metadata consistency.

## Key Checks
1. Inspect SHA256 hashes of `storage/songs/roots_reggae_anthem/v1/final_master.wav` vs `v2/final_master.wav` and `storage/songs/song-one-love-shining-1977/v1/` vs `v2/`.
   Verify they are non-identical.
2. Verify audio peak amplitude <= 0.966 (-0.30 dBFS) across all master WAV files.
3. Verify metadata consistency in `storage/songs/roots_reggae_anthem/v1/state.json` and `v2/state.json` (song_id == "roots_reggae_anthem").
4. Run `cargo test --workspace`.

## Deliverable
Write `d:\harfile\music\.agents\teamwork\challenger_m4_recheck2\handoff.md` with explicit APPROVE or REJECT.

## 2026-11-01T05:44:21Z
You are challenger_m4_recheck2 (archetype: teamwork_challenger).
Your working directory is d:\harfile\music\.agents\teamwork\challenger_m4_recheck2\.
Read the following files before starting work:
- d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md
- d:\harfile\music\PROJECT.md
- d:\harfile\music\.agents\teamwork\challenger_m4_recheck2\DISPATCH.md
- d:\harfile\music\.agents\teamwork\worker_m4_remed\handoff.md

Empirically test on-disk song files and audio properties:
1. Verify SHA256 hashes of `storage/songs/roots_reggae_anthem/v1/final_master.wav` vs `v2/final_master.wav` and `storage/songs/song-one-love-shining-1977/v1/` vs `v2/`. Confirm they are distinct.
2. Verify audio peak amplitude <= 0.966 (-0.30 dBFS).
3. Verify metadata consistency in `storage/songs/roots_reggae_anthem/` and `storage/songs/song-one-love-shining-1977/`.
4. Run `cargo test --workspace`.

Deliver your report in d:\harfile\music\.agents\teamwork\challenger_m4_recheck2\handoff.md with explicit APPROVE or REJECT. Notify caller with send_message when done.
