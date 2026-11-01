# Dispatch: Explorer M3-3 (Pipeline Nodes 4, 5, 6: Inpainting, Judge Gate, Analog Mastering)

## Objective
Design and specify Nodes 4, 5, and 6 of the 6-stage sequential fusion pipeline in `crates/music_pipeline`:
1. **Node 4**: `virtuoso_stem_inpainting_node.rs`
   - Demucs 4-stem separation + Stable Audio / pure-Rust DSP inpainting.
   - Generates 16-bar instrumental interlude / solo using `music_dsp::guitar_wah::synthesize_lead_solo` and Nyabinghi hand-drum polyrhythms.
   - Slices and splices stems with BPM-grid alignment using `music_dsp::crossfader::splice_stem`.
2. **Node 5**: `dsp_and_judge_gate_node.rs`
   - Calculates real signal telemetry (`StemTelemetry`) across all stems via `music_dsp::telemetry_calc::calculate_telemetry`.
   - Invokes `music_fusion::judge::OllamaJudgeEvaluator`.
   - Inspects `quality_score >= 0.85` and clipping.
   - Configures the conditional retry edge: if `quality_score < 0.85` and `retry_count < 2`, increments `retry_count` and routes back to Node 4 (or Node 2); otherwise routes forward to Node 6.
3. **Node 6**: `analog_mastering_node.rs`
   - AudioSR 48 kHz upsampling + Matchering or pure-Rust Studer A800 analog tape saturation (`tape_mastering::apply_analog_mastering`).
   - 55–85 Hz sub-bass warmth resonant filter, reference EQ profile, Blumlein mid-side stereo widener, lookahead brickwall limiter (0.966 / -0.3 dBFS ceiling).
   - Saves `final_master.wav` and writes full song metadata.

## Inputs
- Read `ORIGINAL_REQUEST.md`, `PROJECT.md`, `TEST_INFRA.md`.
- Inspect `crates/music_dsp/src/guitar_wah.rs`, `crossfader.rs`, `telemetry_calc.rs`, `tape_mastering.rs`.
- Inspect `crates/music_fusion/src/judge.rs`.

## Output
Deliver `d:\harfile\music\.agents\teamwork\explorer_m3_3\handoff.md` with complete node implementations, retry edge logic, mastering profiles, and verification plans.


## 2026-11-01T02:55:08Z
[Message] timestamp=2026-11-01T02:55:08Z sender=4caf3f77-b3ac-4b67-8bda-934a91faafab priority=MESSAGE_PRIORITY_HIGH content=You are Explorer M3-3 for Milestone 3 (Native Rust StateGraph & 6 Fusion Nodes).
Your working directory is d:\harfile\music\.agents\teamwork\explorer_m3_3\.
Read d:\harfile\music\ORIGINAL_REQUEST.md, d:\harfile\music\PROJECT.md, d:\harfile\music\TEST_INFRA.md, and your DISPATCH.md at d:\harfile\music\.agents\teamwork\explorer_m3_3\DISPATCH.md.
Design Nodes 4, 5, and 6:
- Node 4: virtuoso_stem_inpainting_node (Demucs 4-stem, 16-bar wah solo and nyabinghi fill generation, crossfade splicing).
- Node 5: dsp_and_judge_gate_node (telemetry calculation, judge evaluation, quality score >= 0.85 check, conditional retry edge < 2 back to Node 4/2, else Node 6).
- Node 6: analog_mastering_node (AudioSR/Matchering or pure-Rust 48 kHz Studer A800 tape saturation, sub-bass warmth, lookahead brickwall limiter <= 0.966).
Write your design and handoff report to d:\harfile\music\.agents\teamwork\explorer_m3_3\handoff.md.
Send a message back to parent when done.
