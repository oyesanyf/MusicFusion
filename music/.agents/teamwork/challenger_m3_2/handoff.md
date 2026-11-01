# Empirical Challenge Report: Milestone 3 Gate Verification (Pipeline Execution & Audio Ceiling)

**Agent**: `challenger_m3_2` (Milestone 3 Gate Challenger 2)  
**Role**: critic, specialist  
**Working Directory**: `d:\harfile\music\.agents\teamwork\challenger_m3_2\`  
**Timestamp**: 2026-11-01T03:49:00Z  
**Recipient**: `4caf3f77-b3ac-4b67-8bda-934a91faafab` (Orchestrator)  
**Verdict**: **REJECT**

---

## 1. Observation

### 1.1 Direct Source Code Observations

1. **`crates/music_pipeline/src/nodes/node1_producer_panel.rs` lines 121–134 (BPM & Musical Parameter Clobbering)**:
   ```rust
   let blueprint = Self::convert_evaluated_blueprint_to_schema(
       &judge_evaluation,
       state,
       &yue_genre_txt,
       &structured_lyrics,
   );
   state.blueprint = Some(blueprint.clone());
   if state.style_prompt.is_empty() {
       state.style_prompt = blueprint.style_of_music.clone();
   }
   state.bpm = blueprint.bpm.round() as u32;
   state.key_signature = blueprint.key_signature.clone();
   state.time_signature = blueprint.time_signature.clone();
   ```
   In pure-Rust mode (`state.is_pure_rust()`), line 100 calls `parse_judge_json("", &genre_preset, &state.style_prompt)`:
   In `crates/music_fusion/src/judge.rs` lines 198–200:
   ```rust
   EvaluatedSongBlueprint {
       title: format!("{} Anthem", genre.name),
       genre_tags: format!(
           "{}, {}, {} BPM, Key of {}, {}",
           genre.name, subgenre, genre.default_bpm, genre.default_key, genre.vocal_archetype
       ),
       bpm: genre.default_bpm as f64,
       key_signature: genre.default_key.clone(),
       time_signature: genre.default_time_signature.clone(),
   ```
   Because `parse_judge_json` hardcodes `bpm: genre.default_bpm as f64`, line 131 of `node1_producer_panel.rs` unconditionally clobbers `state.bpm` with the genre preset's default tempo, discarding any user-specified BPM!
   - Setting `state.bpm = 108` for Afrobeats is clobbered to `105` (the Afrobeats genre default).
   - Setting `state.bpm = 120` for Pop is clobbered to `122` (the Pop genre default).
   - Setting `state.bpm = 140` for Rock is clobbered to `116` (the Rock genre default).
   - User-specified `state.key_signature` (e.g. "F# Minor") and `state.time_signature` are similarly clobbered by `genre.default_key` and `genre.default_time_signature`.

2. **`crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs` lines 225–250 (Conditional Retry Routing Logic)**:
   ```rust
   pub fn judge_gate_condition(state: &MusicProductionState) -> String {
       if state.quality_passed {
           info!("Quality gate PASSED. Routing forward to analog_mastering_node.");
           ROUTE_MASTERING.to_string()
       } else if state.retry_count < 2 {
           if state.telemetry.tempo_drift_score > 0.50 {
               warn!(
                   "Severe vocal/tempo drift detected ({:.4} > 0.50). Routing back to groove_and_yue_synthesis_node (retry {}/2).",
                   state.telemetry.tempo_drift_score, state.retry_count
               );
               ROUTE_RETRY_GROOVE.to_string()
           } else {
               warn!(
                   "Quality critique failure (score {:.2} < 7.5 or telemetry alert). Routing back to virtuoso_stem_inpainting_node (retry {}/2).",
                   state.judge_score, state.retry_count
               );
               ROUTE_RETRY_STEM.to_string()
           }
       } else {
           warn!(
               "Maximum retry count reached ({}). Advancing to analog_mastering_node to ensure completion.",
               state.retry_count
           );
           ROUTE_MASTERING.to_string()
       }
   }
   ```
   - When `telemetry.tempo_drift_score > 0.50` and `retry_count < 2`: routes to `ROUTE_RETRY_GROOVE` (`"retry_groove"` -> Node 2).
   - When stems clip or headroom/score fails with drift $\le 0.50$ and `retry_count < 2`: routes to `ROUTE_RETRY_STEM` (`"retry_stem"` -> Node 4).
   - When `retry_count >= 2`: routes to `ROUTE_MASTERING` (`"mastering"` -> Node 6), terminating the retry cycle and preventing infinite Pregel loops.

3. **`crates/music_dsp/src/tape_mastering.rs` lines 9–10 & 135 & 157–158 (Mastering Brickwall True-Peak Limiter)**:
   ```rust
   pub const BRICKWALL_CEILING: f32 = 0.966; // -0.3 dBFS true peak
   ```
   ```rust
   let out_left = (delayed[0] * self.gain).clamp(-self.ceiling, self.ceiling);
   let out_right = (delayed[1] * self.gain).clamp(-self.ceiling, self.ceiling);
   (out_left, out_right)
   ```
   Every mastered sample across all 5 profiles (`StuderA800Tape1970s`, `ModernPunchyPop`, `ClubEdmSubHeavy`, `CinematicOrchestral`, `AudiophileJazz`) is clamped to $\le 0.966$ (-0.3 dBFS true peak).

4. **`crates/music_pipeline/tests/challenger_stress_tests.rs` (Challenger 1 Failure)**:
   `cargo test -p music_pipeline` currently fails on `stress_test_checkpoint_path_injection_and_traversal_attacks`:
   ```
   The following malicious components bypassed sanitize_path_component: ["COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "CON", "PRN", "AUX", "NUL", "com1", "con", "prn", "aux", "nul"]
   ```

### 1.2 Empirical Adversarial Test Execution

We authored and committed an empirical adversarial test suite in `crates/music_pipeline/tests/adversarial_pipeline_and_audio_ceiling_tests.rs` (commit `8ee758f`):

Command:
```powershell
cargo test -p music_pipeline --test adversarial_pipeline_and_audio_ceiling_tests
```

Output:
```
running 13 tests
test test_adversarial_mastering_profiles_mapping ... ok
test test_adversarial_retry_routing_stem_clipping_and_headroom_violation ... ok
test test_adversarial_retry_routing_retry_exhaustion_forces_mastering ... ok
test test_adversarial_retry_routing_severe_tempo_drift ... ok
test test_stategraph_live_retry_loop_execution_and_termination ... ok
test test_adversarial_brickwall_ceiling_extreme_overdrive_across_all_5_profiles ... ok
test test_full_pipeline_multi_genre_roots_reggae_76bpm ... ok
test test_full_pipeline_multi_genre_pop_120bpm ... ok
test test_full_pipeline_multi_genre_rock_metal_140bpm ... ok
test test_full_pipeline_multi_genre_afrobeats_108bpm ... ok
test test_full_pipeline_multi_genre_cinematic_orchestral_80bpm ... ok
test test_song_iteration_branching_and_prompt_update ... ok
test test_targeted_stem_regeneration_lineage_v1_to_v3 ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.54s
```

---

## 2. Logic Chain

1. **Audio Brickwall True-Peak Invariant Holds Empirically**:
   - `test_adversarial_brickwall_ceiling_extreme_overdrive_across_all_5_profiles` probed all 5 mastering profiles (`StuderA800Tape1970s`, `ModernPunchyPop`, `ClubEdmSubHeavy`, `CinematicOrchestral`, `AudiophileJazz`) with extreme adversarial audio: +24 dB overdrive (x16 amplitude), alternating Nyquist square waves (+5.0/-5.0), severe DC offsets (+3.0), and Dirac impulses (+100.0).
   - In 100% of cases, output samples were strictly bounded by `sample.abs() <= 0.96605` (-0.3 dBFS true peak), with zero NaNs, zero Infs, and zero digital clipping.
2. **Conditional Retry Routing and Termination Holds Empirically**:
   - Inducing severe tempo drift (> 0.50) reliably routes back to Node 2 (`ROUTE_RETRY_GROOVE`).
   - Inducing stem clipping or low judge scores with safe drift ($\le 0.50$) reliably routes to Node 4 (`ROUTE_RETRY_STEM`).
   - Inducing retry exhaustion (`retry_count >= 2`) reliably forces execution forward to Node 6 (`ROUTE_MASTERING`), preventing infinite Pregel execution loops.
   - `test_stategraph_live_retry_loop_execution_and_termination` confirmed this dynamic behavior inside a live `StateGraph` instance.
3. **Targeted Stem Regeneration & Multi-Generation Branching Holds Empirically**:
   - `regenerate_stem` successfully executed targeted single-node sub-graph runs (`virtuoso_stem_inpainting_node` -> `dsp_and_judge_gate_node` -> `analog_mastering_node`) advancing version lineage linearly: `v1 -> v2 -> v3`.
   - SQLite tables `song_versions` and `stem_records` cleanly tracked all 3 versions, and disk stems were verified intact.
   - `iterate_song` successfully branched with an updated style prompt and updated lyrics.
4. **Vulnerability 1: BPM & Musical Attribute Clobber Defect (Node 1)**:
   - When a caller invokes `PipelineRunner::execute_song` with custom BPM, key signature, or time signature (e.g. `MusicProductionState::new("adv-rock-140", "Rock", 140, "E Minor")`), Node 1 in pure-Rust mode generates a fallback blueprint using `parse_judge_json("", &genre_preset, ...)`.
   - `parse_judge_json` hardcodes `bpm: genre.default_bpm as f64` (e.g. 116 for Rock, 122 for Pop, 105 for Afrobeats).
   - Node 1 line 131 unconditionally writes `state.bpm = blueprint.bpm.round() as u32;`, silently overriding the caller's explicit 140 BPM to 116 BPM, 108 BPM to 105 BPM, and 120 BPM to 122 BPM.
   - This directly violates the Universal Multi-Genre requirement from `ORIGINAL_REQUEST.md §Follow-up`:
     > "Support any genre/style passed via CLI (--genre, --style, --bpm, --key, --create-hit), REST API (POST /api/songs/create), MCP (music_create_hit)..."
5. **Vulnerability 2: Windows Reserved Device Name Traversal Bypass**:
   - As independently discovered and verified by `challenger_m3_1`, `CheckpointManager::sanitize_path_component` allows DOS device names (`COM1`-`COM9`, `CON`, `PRN`, `AUX`, `NUL`), causing `cargo test -p music_pipeline` to fail on `stress_test_checkpoint_path_injection_and_traversal_attacks`.

---

## 3. Caveats

- In `pure_rust` mode, Node 1 and Node 5 bypass live Ollama HTTP sockets, executing deterministically.
- All 13 tests in `adversarial_pipeline_and_audio_ceiling_tests.rs` pass, and all audio mastering profiles enforce the -0.3 dBFS ceiling.
- However, the combination of (a) Node 1 clobbering user BPM/key signatures and (b) the failing Windows device name path traversal test in `challenger_stress_tests.rs` requires a REJECT verdict until remediated by `worker_m3`.

---

## 4. Conclusion & Required Remediation

### Verdict: **REJECT**

### Remediation 1: Fix Node 1 BPM, Key, and Time Signature Clobbering
In `crates/music_pipeline/src/nodes/node1_producer_panel.rs`:
Before overwriting `state.bpm`, `state.key_signature`, and `state.time_signature`, preserve user-specified values:

```rust
// In crates/music_pipeline/src/nodes/node1_producer_panel.rs lines 120-134:
let mut blueprint = Self::convert_evaluated_blueprint_to_schema(
    &judge_evaluation,
    state,
    &yue_genre_txt,
    &structured_lyrics,
);

// If user explicitly configured BPM/key/time_sig, preserve them in blueprint and state
if state.bpm > 0 {
    blueprint.bpm = state.bpm as f64;
} else {
    state.bpm = blueprint.bpm.round() as u32;
}

if !state.key_signature.is_empty() {
    blueprint.key_signature = state.key_signature.clone();
} else {
    state.key_signature = blueprint.key_signature.clone();
}

if !state.time_signature.is_empty() {
    blueprint.time_signature = state.time_signature.clone();
} else {
    state.time_signature = blueprint.time_signature.clone();
}

state.blueprint = Some(blueprint.clone());
if state.style_prompt.is_empty() {
    state.style_prompt = blueprint.style_of_music.clone();
}
```

### Remediation 2: Fix Windows Reserved Device Names in `sanitize_path_component`
In `crates/music_pipeline/src/checkpointer.rs`:
Update `sanitize_path_component` to reject DOS device names (`con`, `prn`, `aux`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`) as detailed in `challenger_m3_1/handoff.md`.

---

## 5. Verification Method

To independently verify the adversarial challenge findings:

1. **Run the adversarial audio ceiling and pipeline test suite**:
   ```powershell
   cargo test -p music_pipeline --test adversarial_pipeline_and_audio_ceiling_tests
   ```
   *Expected*: 13 tests pass in ~22 seconds, asserting brickwall ceiling $\le 0.966$ across all 5 profiles and testing retry routing, stem regeneration, and multi-genre pipeline runs.

2. **Run challenger 1's path traversal stress test**:
   ```powershell
   cargo test -p music_pipeline --test challenger_stress_tests
   ```
   *Current Result*: Fails on `stress_test_checkpoint_path_injection_and_traversal_attacks` due to unblocked Windows device names (`COM1`, `CON`, etc.).
   *Expected Post-Remediation*: All 12 tests pass cleanly.

3. **Verify clippy compliance**:
   ```powershell
   cargo clippy -p music_pipeline -- -D warnings
   ```
   *Expected*: 0 warnings, 0 errors.
