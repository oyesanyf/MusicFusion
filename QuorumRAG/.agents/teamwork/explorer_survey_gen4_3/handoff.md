# Handoff Report — Explorer Survey Gen 4 (Survey 3)
**Agent**: `explorer_survey_gen4_3`  
**Parent**: `8ca8fbc4-65aa-495d-9a0f-9c489a4299dc`  
**Date**: 2026-11-02  
**Handoff Type**: Hard (Investigation & Analysis Complete)

---

## 1. Observation
1. **Hardcoded Token Truncation in `offline_engine.rs:1154`**:
   `crates/quorum-coordinator/src/offline_engine.rs:1154`:
   ```rust
   let temp = ui.sampling_temperature.clamp(0.05, 0.20);
   let num_predict = 1600usize;
   let num_ctx = 16384usize;
   ```
   `ui` is read from `global_mesh_dlp_controller().settings.read()` at line 1110, containing `ui.max_output_tokens: 262144`, `ui.agentic_loop_enabled: true`, `ui.agentic_chunk_size: 4096`, and `ui.agentic_strategy: "auto_continuation"`. Line 1154 completely ignores all of these settings.
2. **Telemetry Drop Hazard in `offline_engine.rs:1309`**:
   `offline_engine.rs:1309`:
   ```rust
   let _ = tx.try_send(AgenticTelemetryEvent::TokenDelta {
       delta: tok.to_string(),
   });
   ```
   `tx.try_send` drops tokens silently when the channel buffer is full. `SynthesizeWithCitationsNode::execute` runs synchronously inside `tokio::task::block_in_place` (`crates/quorum-coordinator/src/agentic/mod.rs:394-399`). Calling `tx.blocking_send` guarantees zero dropped tokens.
3. **Missing `done_reason` Handling in `offline_engine.rs:1314`**:
   `offline_engine.rs:1314`:
   ```rust
   if val.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
       break;
   }
   ```
   Ollama emits `done_reason: "length"` when the token limit is reached. The code immediately exits without initiating subsequent continuation hops up to `max_output_tokens`.
4. **Hardcoded Chunk Limit in `offline_engine.rs:613`**:
   `let matched_chunks: Vec<&ScoredChunk> = scored_matches.into_iter().take(8).map(|(c, _, _, _)| c).collect();`
   Limits context to 8 chunks, preventing broad research reports spanning 20–50 chunks.
5. **Retrieval Defects in `offline_engine.rs:499-507`**:
   Line 499 calls `matches_whole_word(t.as_str())` on body text inside `file_hits`. Line 505 applies `scored.score < 0.35`, which discards valid RRF candidates (RRF scores are 0.01–0.05).
6. **WebUI Response Actions Bar & Settings Verification**:
   - `crates/quorum-ui/assets/app.js:852-1029` renders `.response-actions-bar` with 8 buttons: Continue, Copy, Regenerate, Read Aloud, Save File, Clear, 👍, 👎.
   - `crates/quorum-ui/assets/app.js:2354-2404` renders `setting-max-tokens` (256 up to 262144), `setting-agentic-loop-enable`, `setting-agentic-chunk-size`, and `setting-agentic-strategy`.

---

## 2. Logic Chain
1. When a user requests a research query (e.g. "tell me about sacred maths"), `query_local_ollama_chat` in `offline_engine.rs` reads `ui`.
2. Even though `ui.max_output_tokens` is configured to `262144`, line 1154 hardcodes `num_predict = 1600usize`.
3. When Ollama streams ~1600 tokens, it reaches `num_predict` and returns `done: true` with `done_reason: "length"`.
4. Line 1314 checks only `val.get("done") == true` and exits the loop immediately.
5. Because there is no recursive chaining or sentence-boundary guard, the generation terminates mid-sentence (e.g., *"Sacred math has been"*).
6. Furthermore, if consumer backpressure occurs, `tx.try_send` drops tokens.
7. To resolve this, `offline_engine.rs` must:
   - Read `ui.max_output_tokens`, `ui.agentic_loop_enabled`, and `ui.agentic_chunk_size`.
   - Use `tx.blocking_send` inside `block_in_place`.
   - Check `done_reason == "length"` and recursively chain continuation hops up to `max_output_tokens`.
   - Enforce a sentence-boundary completion guard.
   - Expand retrieval breadth from 8 to 20–50 chunks with parent document stitching.

---

## 3. Caveats
- Ollama TCP streaming is strictly air-gapped on loopback (`127.0.0.1:11434`).
- Local RAM/VRAM capacity must support multi-hop generation. Per-hop budget of 4096 tokens ensures KV cache stability.
- No source code modifications were performed in this turn (read-only investigation).

---

## 4. Conclusion
The root cause of token truncation, mid-sentence cutoffs, and dropped streaming tokens has been identified and verified with exact line numbers and quotes. Detailed implementation specifications for:
- Recursive turn chaining up to 262k tokens,
- Zero-drop `tx.blocking_send` streaming,
- Sentence-boundary completion guards,
- Iterative section generation (`DocumentPlan` / `SectionPlan` with `rolling_summary`),
- Map-Reduce fact sheet extraction,
- Parent document chunk stitching, and
- Retrieval filter fixes (`file_hits`, removing `< 0.35` cutoff)
have been formulated and documented in `report.md`.

---

## 5. Verification Method
1. Inspect `crates/quorum-coordinator/src/offline_engine.rs` line 1154 (`let num_predict = 1600usize;`), line 1309 (`tx.try_send`), and lines 499-507 (`file_hits` and `scored.score < 0.35`).
2. Inspect `crates/quorum-ui/assets/app.js` lines 852-1029 and 2354-2404 to verify the action bar buttons and max token dropdown.
3. Review detailed findings and proposed architectures in `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_gen4_3\report.md`.
4. Run `cargo test -p quorum-coordinator -p quorum-daemon` to verify baseline integrity.
