# DISPATCH: explorer_m3_remed3 (Milestone 3 Remediation Explorer 3 — Node Logic: BPM Preservation, Think Tags & Retry Counter)

## Role & Focus
Technical investigation and fix strategy for Node Logic in `crates/music_pipeline/src/nodes/`:
1. **Node 1 User BPM, Key, and Time Signature Preservation** (`node1_producer_panel.rs` lines 121-134):
   - Currently, when `parse_judge_json` is called in pure-Rust mode (or when blueprint is returned), line 131 unconditionally writes `state.bpm = blueprint.bpm.round() as u32;`, `state.key_signature = blueprint.key_signature.clone();`, and `state.time_signature = ...`.
   - This clobbers user-specified values (e.g., 108 BPM for Afrobeats clobbered to 105; 140 BPM for Rock clobbered to 116; custom keys/time signatures clobbered).
   - Formulate exact fix: if `state.bpm > 0`, preserve `state.bpm` and set `blueprint.bpm = state.bpm as f64`. If `!state.key_signature.is_empty()`, preserve `state.key_signature` and update `blueprint.key_signature`. Same for `time_signature`.
2. **Node 1 Unclosed `<think>` Tag Sanitization Inversion** (`node1_producer_panel.rs` lines 174-181):
   - In `enforce_structural_bracket_tags`:
     ```rust
     if let Some(start) = raw.find("<think>") {
         if let Some(end) = raw.find("</think>") {
             raw = format!("{}{}", &raw[..start], &raw[end + 8..]);
         } else {
             raw = raw[start + 7..].to_string(); // Inversion bug!
         }
     }
     ```
   - When `<think>` is unclosed, `raw[start + 7..]` keeps internal thoughts and discards the prompt/lyrics!
   - Formulate exact fix: drop unclosed reasoning tag/text and preserve preceding content (`raw[..start].to_string()`).
3. **Node 5 Premature Retry Incrementing** (`node5_dsp_judge_gate.rs` lines 198-201):
   - `state.increment_retry()` runs inside `execute_node` before `judge_gate_condition` checks `state.retry_count < 2`, prematurely cutting 2 retry cycles down to 1.
   - Formulate exact fix so that exactly 2 retry attempts are permitted before routing forward to `analog_mastering_node`.
4. Provide full report in `.agents/teamwork/explorer_m3_remed3/handoff.md`.


## 2026-11-01T03:50:47Z
[Message] timestamp=2026-11-01T03:50:47Z sender=4caf3f77-b3ac-4b67-8bda-934a91faafab priority=MESSAGE_PRIORITY_HIGH content=You are explorer_m3_remed3, the Milestone 3 Remediation Explorer 3 (Node Logic: User BPM Preservation, Think Tags & Retry Counter).
Your working directory is `d:\harfile\music\.agents\teamwork\explorer_m3_remed3\`.

CRITICAL CONSTRAINTS:
- Read-only investigation: NEVER edit, modify, or create production source code files.
- You MUST read `d:\harfile\music\ORIGINAL_REQUEST.md` (mandatory).
- You MUST read the full Forensic Audit Evidence report at `d:\harfile\music\.agents\teamwork\auditor_m3\handoff.md`.
- Read reviewer and challenger reports at:
  - `d:\harfile\music\.agents\teamwork\reviewer_m3_1\handoff.md`
  - `d:\harfile\music\.agents\teamwork\reviewer_m3_2\handoff.md`
  - `d:\harfile\music\.agents\teamwork\challenger_m3_1\handoff.md`
  - `d:\harfile\music\.agents\teamwork\challenger_m3_2\handoff.md`
- Read `d:\harfile\music\.agents\teamwork\explorer_m3_remed3\DISPATCH.md`.
- Read target source files:
  - `d:\harfile\music\crates\music_pipeline\src\nodes\node1_producer_panel.rs`
  - `d:\harfile\music\crates\music_pipeline\src\nodes\node5_dsp_judge_gate.rs`
  - `d:\harfile\music\crates\music_pipeline\tests\adversarial_pipeline_and_audio_ceiling_tests.rs`

INVESTIGATION OBJECTIVES:
1. Address the Node 1 BPM, Key, and Time Signature clobber defect discovered by Challenger 2 (`challenger_m3_2`):
   - In `node1_producer_panel.rs` lines 121-134, Node 1 calls `parse_judge_json` in pure-Rust mode which returns genre default BPM/key/time_sig, and unconditionally writes `state.bpm = blueprint.bpm.round() as u32;`, clobbering user-specified BPM/key/time_sig.
   - Formulate exact fix: if `state.bpm > 0`, preserve `state.bpm` and set `blueprint.bpm = state.bpm as f64`. If `!state.key_signature.is_empty()`, preserve `state.key_signature` and set `blueprint.key_signature = state.key_signature.clone()`. Same for `time_signature`.
2. Address the unclosed `<think>` tag inversion bug identified by Reviewer 2 (`reviewer_m3_2`):
   - In `node1_producer_panel.rs` lines 174-181, if `<think>` has no closing `</think>`, it discards everything before `<think>` and keeps internal thinking.
   - Formulate exact fix: drop the unclosed reasoning tag/text and preserve preceding prompt/lyrics (`raw[..start].to_string()`).
3. Address the retry counter increment ordering bug identified by Reviewer 2:
   - In `node5_dsp_judge_gate.rs` lines 198-201, `state.increment_retry()` inside node execution increments before `judge_gate_condition` checks `state.retry_count < 2`, prematurely cutting 2 retry cycles down to 1.
   - Formulate exact fix to ensure exactly 2 retry attempts are permitted before routing forward to `analog_mastering_node`.
4. Formulate test additions in `crates/music_pipeline/tests/node_tests.rs` or `adversarial_pipeline_and_audio_ceiling_tests.rs`.
5. Write your complete analysis and recommended code diff to `d:\harfile\music\.agents\teamwork\explorer_m3_remed3\handoff.md`.
6. Send completion message to parent orchestrator.
