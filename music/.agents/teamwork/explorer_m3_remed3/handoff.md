# Handoff Report: Milestone 3 Remediation Explorer 3 (Node Logic: User BPM Preservation, Think Tags & Retry Counter)

**Agent**: `explorer_m3_remed3`  
**Roles**: explorer, investigator  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m3_remed3\`  
**Timestamp**: 2026-11-01T03:58:00Z  
**Recipient**: `4caf3f77-b3ac-4b67-8bda-934a91faafab` (Orchestrator)  
**Target Milestone**: Milestone 3 Remediation (`crates/music_pipeline`)  
**Verdict**: **REMEDIATION_READY**

---

## 1. Observation

### 1.1 Node 1: User BPM, Key Signature, and Time Signature Clobbering
- **Source File**: `crates/music_pipeline/src/nodes/node1_producer_panel.rs` (lines 121–134):
  ```rust
  // 6. Synchronize SongBlueprint into state
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
- **Helper Method**: `crates/music_pipeline/src/nodes/node1_producer_panel.rs` (lines 268–270):
  ```rust
  SongBlueprint {
      ...
      bpm: evaluated.bpm,
      key_signature: evaluated.key_signature.clone(),
      time_signature: evaluated.time_signature.clone(),
      ...
  }
  ```
- **Pure-Rust Fallback Evaluation**: `crates/music_fusion/src/judge.rs` (lines 198–201):
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
- **Direct Empirical Observation**:
  When a caller configures custom values (e.g. `MusicProductionState::new("adv-rock-140", "Rock", 140, "E Minor")` or Afrobeats at 108 BPM with key "F# Minor"):
  - Line 100 executes `parse_judge_json("", &genre_preset, &state.style_prompt)`.
  - `parse_judge_json` hardcodes `bpm: genre.default_bpm as f64` (116 for Rock, 105 for Afrobeats, 122 for Pop).
  - Line 131 unconditionally assigns `state.bpm = blueprint.bpm.round() as u32;`, silently overwriting 140 BPM to 116 BPM, 108 BPM to 105 BPM, and 120 BPM to 122 BPM.
  - In `crates/music_pipeline/tests/adversarial_pipeline_and_audio_ceiling_tests.rs` (lines 124–125, 155–156, 186–187), the test author had to accommodate this defect using loose assertions:
    ```rust
    // Note: Node 1 in pure_rust mode synchronizes to genre preset default (105 BPM)
    assert!(final_state.bpm == 108 || final_state.bpm == 105, "BPM was {}", final_state.bpm);
    ...
    // Note: Node 1 in pure_rust mode synchronizes to genre preset default (122 BPM)
    assert!(final_state.bpm == 120 || final_state.bpm == 122, "BPM was {}", final_state.bpm);
    ...
    // Note: Node 1 in pure_rust mode synchronizes to genre preset default (116 BPM)
    assert!(final_state.bpm == 140 || final_state.bpm == 116, "BPM was {}", final_state.bpm);
    ```

### 1.2 Node 1: Unclosed `<think>` Tag Inversion Defect
- **Source File**: `crates/music_pipeline/src/nodes/node1_producer_panel.rs` (lines 174–182):
  ```rust
  // 1. Strip reasoning blocks: <think>...</think>
  if let Some(start) = raw.find("<think>") {
      if let Some(end) = raw.find("</think>") {
          raw = format!("{}{}", &raw[..start], &raw[end + 8..]);
      } else {
          raw = raw[start + 7..].to_string();
      }
  }
  ```
- **Direct Empirical Observation**:
  - In the unclosed tag branch (`end == None`), `raw = raw[start + 7..].to_string();` discards all text before `<think>` (the actual prompt or lyrics intro) and retains the internal thinking tokens (`start + 7` onwards).
  - When the LLM stream terminates before outputting `</think>`, internal reasoning is leaked into the song lyrics while user prompt/lyrics preceding `<think>` are deleted.
  - Furthermore, using `if let Some(start)` only handles a single tag pair, and using `raw.find("</think>")` without bounding from `start` would be unsafe if an inverted tag order occurred.

### 1.3 Node 5: Premature Retry Counter Increment Truncating 2 Retries to 1
- **Source File**: `crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs` (lines 198–201):
  ```rust
  // If quality failed and retries not exhausted, increment retry count
  if !quality_passed && state.retry_count < 2 {
      state.increment_retry();
  }
  ```
- **Conditional Routing Function**: `crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs` (lines 225–250):
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
- **Direct Empirical Observation in StateGraph Execution**:
  1. Initial Run (Attempt 1):
     - Starts with `retry_count == 0`.
     - Quality check fails.
     - Node 5 executes line 199: `0 < 2` is true; `state.increment_retry()` runs, setting `state.retry_count = 1`.
     - `execute_node` completes and returns `Ok(())`.
     - StateGraph conditional edge invokes `judge_gate_condition(state)`:
       - `state.retry_count` is 1.
       - Line 229: `1 < 2` is true. Logs `(retry 1/2)`.
       - Routes back to Node 2 or Node 4 (Retry #1).
  2. First Retry (Attempt 2):
     - Upstream nodes execute Retry #1.
     - Node 5 executes again: quality check fails.
     - Node 5 executes line 199: `1 < 2` is true; `state.increment_retry()` runs, setting `state.retry_count = 2`.
     - `execute_node` completes and returns `Ok(())`.
     - StateGraph conditional edge invokes `judge_gate_condition(state)`:
       - `state.retry_count` is 2.
       - Line 229: `2 < 2` is **FALSE**.
       - Falls through to `else`: logs `Maximum retry count reached (2). Advancing to analog_mastering_node`.
       - Routes forward to `ROUTE_MASTERING`.
  3. Result:
     - The pipeline completely terminates after **only 1 retry attempt**, not 2.
     - In `crates/music_pipeline/tests/adversarial_pipeline_and_audio_ceiling_tests.rs` (lines 325–337 & 405), `test_stategraph_live_retry_loop_execution_and_termination` confirmed this premature cutoff:
       ```rust
       assert_eq!(eval_count.load(Ordering::SeqCst), 2, "Gate evaluated 2 times before exhaustion cutoff");
       ```
       Pass 3 ("Third pass: pass gate") in the test harness was never reached because the condition halted execution after Pass 2.
  4. Design Intent Evidence:
     - Notice lines 233 and 239 of `node5_dsp_judge_gate.rs`:
       `(retry {}/2)`
       The author designed the formatting string specifically so that `retry_count == 1` logs `(retry 1/2)` and `retry_count == 2` logs `(retry 2/2)`.
       Because line 229 evaluated `retry_count < 2`, when `retry_count == 2` it immediately aborted, making `(retry 2/2)` unreachable dead code!

---

## 2. Logic Chain

1. **User Musical Invariant Defense (Node 1)**:
   - `ORIGINAL_REQUEST.md §Follow-up` explicitly requires supporting any user-specified genre, BPM, and musical key passed via CLI (`--bpm`, `--key`), REST API, or MCP tool calls.
   - Initial state construction via `MusicProductionState::new(song_id, genre, bpm, key_signature)` records the caller's explicit values.
   - In pure-Rust mode or fallback execution, `parse_judge_json` constructs an `EvaluatedSongBlueprint` populated strictly with static genre defaults.
   - In `node1_producer_panel.rs`, lines 131–133 unconditionally assign `state.bpm = blueprint.bpm.round() as u32;` without checking if the user had already specified a positive BPM, key signature, or time signature.
   - **Remediation Logic**: If `state.bpm > 0`, `blueprint.bpm` must be synchronized to `state.bpm as f64` and `state.bpm` preserved. If `!state.key_signature.is_empty()`, `blueprint.key_signature` must be synchronized and `state.key_signature` preserved. If `!state.time_signature.is_empty()`, `blueprint.time_signature` must be synchronized and `state.time_signature` preserved. Both `convert_evaluated_blueprint_to_schema` and `execute_node` must enforce this invariant.

2. **Prompt & Lyric Integrity Defense (Think Tag Sanitization)**:
   - When DeepSeek-R1 or local reasoning models generate output, reasoning tokens are wrapped in `<think>...</think>`.
   - If a generation is truncated by token limits or network cutoffs before emitting `</think>`, the reasoning begins at `<think>` and continues to the end of the text.
   - Discarding `[..start]` and retaining `[start + 7..]` keeps unfinalized internal scratchpads and discards actual lyric verses.
   - **Remediation Logic**: If `</think>` is missing, slice `&raw[..start]` to discard the unclosed reasoning block and retain all prior lyrics/prompts. Using a `while let Some(start)` loop with `raw[start..].find("</think>")` ensures robust removal of multiple reasoning blocks and prevents index out-of-bounds panics. If the resulting string has no lines, step 2's canonical section generator safely populates default `[Intro]...[Outro]` structure.

3. **2-Retry Cycle Boundary Defense (Node 5 & Condition Edge)**:
   - Feature 18 in `PROJECT.md` dictates: *"conditional retry edge (< 2 retries routes back to Node 4 or Node 2, else Node 6)"*.
   - A retry cycle is an iteration attempted *after* an initial failure.
   - When Node 5 increments `retry_count` on failure:
     - Initial run failure: increments `0 -> 1`. This corresponds to Retry Cycle 1 (`(retry 1/2)`).
     - First retry failure: increments `1 -> 2`. This corresponds to Retry Cycle 2 (`(retry 2/2)`).
     - Second retry failure: increments `2 -> 3`. Retries are now exhausted (3 failures total; 2 retry cycles executed).
   - In `node5_dsp_judge_gate.rs`:
     - Line 199 checked `state.retry_count < 2`.
     - Line 229 checked `state.retry_count < 2`.
     - When `state.retry_count` reached 2, line 199 refused to increment to 3, and line 229 refused to retry!
     - As a consequence, Retry Cycle 2 was blocked from running.
   - **Remediation Logic**:
     - In `execute_node` line 199: allow incrementing while `state.retry_count <= 2` (or `< 3`).
     - In `judge_gate_condition` line 229: allow retry routing while `state.retry_count <= 2`.
     - When `state.retry_count` is 1: routes to Retry Cycle 1 (`(retry 1/2)`).
     - When `state.retry_count` is 2: routes to Retry Cycle 2 (`(retry 2/2)`).
     - When `state.retry_count` is 3: routes to `analog_mastering_node` (`Maximum retry count reached (3)`).
     - Exactly 2 retry attempts are permitted before advancing to mastering.

---

## 3. Caveats

- **No Caveats on Root Causes**: The exact file paths, line numbers, and causes for all 3 issues are verified.
- **Unit Test Synchronizations**: Updating `judge_gate_condition` to permit `retry_count <= 2` requires updating direct-condition assertions in `node_tests.rs` (line 221) and `adversarial_pipeline_and_audio_ceiling_tests.rs` (line 293) so that exhaustion is tested at `retry_count >= 3` instead of `retry_count >= 2`.

---

## 4. Conclusion & Concrete Implementation Patches

### Patch 1: Node 1 BPM, Key Signature, and Time Signature Preservation
**File**: `crates/music_pipeline/src/nodes/node1_producer_panel.rs`

```diff
--- a/crates/music_pipeline/src/nodes/node1_producer_panel.rs
+++ b/crates/music_pipeline/src/nodes/node1_producer_panel.rs
@@ -121,13 +121,29 @@ impl OllamaProducerPanelNode {
         // 6. Synchronize SongBlueprint into state
-        let blueprint = Self::convert_evaluated_blueprint_to_schema(
+        let mut blueprint = Self::convert_evaluated_blueprint_to_schema(
             &judge_evaluation,
             state,
             &yue_genre_txt,
             &structured_lyrics,
         );
+
+        // If user explicitly configured BPM, key, or time signature, preserve them in blueprint and state
+        if state.bpm > 0 {
+            blueprint.bpm = state.bpm as f64;
+        } else {
+            state.bpm = blueprint.bpm.round() as u32;
+        }
+
+        if !state.key_signature.is_empty() {
+            blueprint.key_signature = state.key_signature.clone();
+        } else {
+            state.key_signature = blueprint.key_signature.clone();
+        }
+
+        if !state.time_signature.is_empty() {
+            blueprint.time_signature = state.time_signature.clone();
+        } else {
+            state.time_signature = blueprint.time_signature.clone();
+        }
+
         state.blueprint = Some(blueprint.clone());
         if state.style_prompt.is_empty() {
             state.style_prompt = blueprint.style_of_music.clone();
         }
-        state.bpm = blueprint.bpm.round() as u32;
-        state.key_signature = blueprint.key_signature.clone();
-        state.time_signature = blueprint.time_signature.clone();
@@ -268,3 +284,3 @@ impl OllamaProducerPanelNode {
-            bpm: evaluated.bpm,
-            key_signature: evaluated.key_signature.clone(),
-            time_signature: evaluated.time_signature.clone(),
+            bpm: if state.bpm > 0 { state.bpm as f64 } else { evaluated.bpm },
+            key_signature: if !state.key_signature.is_empty() { state.key_signature.clone() } else { evaluated.key_signature.clone() },
+            time_signature: if !state.time_signature.is_empty() { state.time_signature.clone() } else { evaluated.time_signature.clone() },
```

---

### Patch 2: Node 1 Unclosed `<think>` Tag Sanitization
**File**: `crates/music_pipeline/src/nodes/node1_producer_panel.rs`

```diff
--- a/crates/music_pipeline/src/nodes/node1_producer_panel.rs
+++ b/crates/music_pipeline/src/nodes/node1_producer_panel.rs
@@ -174,8 +174,10 @@ impl OllamaProducerPanelNode {
         // 1. Strip reasoning blocks: <think>...</think>
-        if let Some(start) = raw.find("<think>") {
-            if let Some(end) = raw.find("</think>") {
-                raw = format!("{}{}", &raw[..start], &raw[end + 8..]);
+        while let Some(start) = raw.find("<think>") {
+            if let Some(end_offset) = raw[start..].find("</think>") {
+                let end = start + end_offset;
+                raw = format!("{}{}", &raw[..start], &raw[end + 8..]);
             } else {
-                raw = raw[start + 7..].to_string();
+                raw = raw[..start].to_string();
+                break;
             }
         }
```

---

### Patch 3: Node 5 Exactly 2 Retry Cycles Execution
**File**: `crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs`

```diff
--- a/crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs
+++ b/crates/music_pipeline/src/nodes/node5_dsp_judge_gate.rs
@@ -198,3 +198,3 @@ impl DspAndJudgeGateNode {
         // If quality failed and retries not exhausted, increment retry count
-        if !quality_passed && state.retry_count < 2 {
+        if !quality_passed && state.retry_count <= 2 {
             state.increment_retry();
         }
@@ -228,3 +228,3 @@ pub fn judge_gate_condition(state: &MusicProductionState) -> String {
         ROUTE_MASTERING.to_string()
-    } else if state.retry_count < 2 {
+    } else if state.retry_count <= 2 {
         if state.telemetry.tempo_drift_score > 0.50 {
```

---

### Patch 4: Unit & Adversarial Tests Addition & Harmonization

#### A. In `crates/music_pipeline/tests/node_tests.rs`:
1. Add test for user BPM, key, and time signature preservation:
   ```rust
   #[tokio::test]
   async fn test_node1_user_bpm_key_time_signature_preservation() {
       let temp_dir = tempfile::tempdir().expect("tempdir");
       let mut state = MusicProductionState::new("test-preservation", "Rock", 140, "F# Minor");
       state.time_signature = "3/4".to_string();
       state.engine_mode = "pure_rust".to_string();

       let node1 = OllamaProducerPanelNode::new().with_storage_dir(temp_dir.path());
       let res = node1.execute_node(&mut state).await;
       assert!(res.is_ok());

       // Assert user values are preserved, NOT clobbered by Rock genre defaults (116 BPM, E Minor, 4/4)
       assert_eq!(state.bpm, 140);
       assert_eq!(state.key_signature, "F# Minor");
       assert_eq!(state.time_signature, "3/4");

       let bp = state.blueprint.as_ref().unwrap();
       assert_eq!(bp.bpm, 140.0);
       assert_eq!(bp.key_signature, "F# Minor");
       assert_eq!(bp.time_signature, "3/4");
   }
   ```
2. Add tests for unclosed `<think>` tag sanitization:
   ```rust
   #[tokio::test]
   async fn test_node1_unclosed_think_tag_preserves_prompt_lyrics() {
       let raw = "[Intro]\nOne love shining bright\n<think>Generating next verse with rhyming lyrics...";
       let formatted = OllamaProducerPanelNode::enforce_structural_bracket_tags(raw, "", "Roots Reggae");
       assert!(formatted.contains("[Intro]"));
       assert!(formatted.contains("One love shining bright"));
       assert!(!formatted.contains("<think>"));
       assert!(!formatted.contains("Generating next verse"));
   }

   #[tokio::test]
   async fn test_node1_unclosed_think_tag_from_start_falls_back_to_structure() {
       let raw = "<think>Internal thinking only with no closing tag";
       let formatted = OllamaProducerPanelNode::enforce_structural_bracket_tags(raw, "", "Roots Reggae");
       assert!(!formatted.contains("<think>"));
       assert!(!formatted.contains("Internal thinking"));
       assert!(formatted.contains("[Intro]"));
       assert!(formatted.contains("[Verse 1]"));
       assert!(formatted.contains("[Chorus]"));
       assert!(formatted.contains("[Outro]"));
   }
   ```
3. Update `test_node5_conditional_edge_retry_routing` for the 2-retry threshold:
   ```rust
   // 4. Retry exhausted (retry_count > 2, i.e. >= 3) -> advances to mastering
   let mut state_exhausted = MusicProductionState::default();
   state_exhausted.quality_passed = false;
   state_exhausted.retry_count = 3;
   assert_eq!(judge_gate_condition(&state_exhausted), ROUTE_MASTERING);
   ```

#### B. In `crates/music_pipeline/tests/adversarial_pipeline_and_audio_ceiling_tests.rs`:
1. Harden BPM assertions in multi-genre pipeline tests:
   ```rust
   // Afrobeats:
   assert_eq!(final_state.bpm, 108);
   assert_eq!(final_state.key_signature, "F# Minor");

   // Pop:
   assert_eq!(final_state.bpm, 120);

   // Rock:
   assert_eq!(final_state.bpm, 140);
   ```
2. Update `test_adversarial_retry_routing_retry_exhaustion_forces_mastering`:
   ```rust
   for retries in [3, 4, 5, 255] {
       let mut state = MusicProductionState::default();
       state.quality_passed = false;
       state.retry_count = retries;
       ...
       assert_eq!(judge_gate_condition(&state), ROUTE_MASTERING);
   }
   ```
3. Update `test_stategraph_live_retry_loop_execution_and_termination`:
   - Passes 1, 2, and 3 now execute fully:
     - Pass 1 (Initial): eval count 1, `retry_count` 1 -> routes to Groove (`retry 1/2`).
     - Pass 2 (Retry 1): eval count 2, `retry_count` 2 -> routes to Stem (`retry 2/2`).
     - Pass 3 (Retry 2): eval count 3 -> quality gate passes (or if failing, `retry_count` 3 exhausts and routes to Mastering).
   - Assert:
     ```rust
     assert_eq!(eval_count.load(Ordering::SeqCst), 3, "Gate evaluated 3 times (initial + 2 retries)");
     ```

---

## 5. Verification Method

Once Worker M3 applies these diffs, independent verification can be executed via:

1. **Test Node Logic & Think Tags**:
   ```powershell
   cargo test -p music_pipeline --test node_tests
   ```
   *Expected*: All node tests pass, confirming user BPM preservation, `<think>` tag stripping, and conditional routing.

2. **Test Adversarial Routing & Multi-Genre Pipelines**:
   ```powershell
   cargo test -p music_pipeline --test adversarial_pipeline_and_audio_ceiling_tests
   ```
   *Expected*: All 13+ tests pass cleanly in ~22s, validating exact BPM preservation for 108, 120, and 140 BPM, brickwall peak ceiling $\le 0.966$, and exactly 2 retry loops before termination.

3. **Workspace Full Verification & Clippy**:
   ```powershell
   cargo clippy -p music_pipeline -- -D warnings
   cargo test --workspace
   ```
   *Expected*: Zero warnings, 100% tests passing across the workspace.
