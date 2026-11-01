# Dispatch: Remediation Explorer M2-1 (Crossfader & Groove Synth Edge Cases)

## Objective
Analyze the root cause and specify the precise code fix for:
1. Integer multiplication overflow in `crates/music_dsp/src/crossfader.rs:108` on `bpm == 0`.
2. Slice index out of bounds panic in `crates/music_dsp/src/groove_synth.rs:412` when `render_hits` is called on a mono `AudioBuffer`.

## Inputs
- Read `ORIGINAL_REQUEST.md`, `PROJECT.md`.
- Read `d:\harfile\music\.agents\teamwork\challenger_m2_1\handoff.md`.
- Read `crates/music_dsp/src/crossfader.rs` and `crates/music_dsp/src/groove_synth.rs`.

## Output
Deliver `d:\harfile\music\.agents\teamwork\explorer_m2_remed1\handoff.md` with exact patch lines and test update instructions for `adversarial_dsp_stress_tests.rs`.

## 2026-11-01T02:30:33Z
You are Remediation Explorer M2-1 for Milestone 2 (Pure-Rust 48 kHz DSP Engine).
Your working directory is d:\harfile\music\.agents\teamwork\explorer_m2_remed1\.
Read d:\harfile\music\ORIGINAL_REQUEST.md, d:\harfile\music\PROJECT.md, d:\harfile\music\.agents\teamwork\challenger_m2_1\handoff.md, and your DISPATCH.md at d:\harfile\music\.agents\teamwork\explorer_m2_remed1\DISPATCH.md.
Investigate the exact lines and fix strategy for:
1. crates/music_dsp/src/crossfader.rs:108 integer overflow on bpm == 0.
2. crates/music_dsp/src/groove_synth.rs:412 slice index out of bounds on mono buffer.
Write your analysis to d:\harfile\music\.agents\teamwork\explorer_m2_remed1\handoff.md.
Send a message back to parent when done.
