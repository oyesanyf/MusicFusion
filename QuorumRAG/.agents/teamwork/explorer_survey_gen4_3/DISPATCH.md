## 2026-11-02T23:55:10Z
From: 8ca8fbc4-65aa-495d-9a0f-9c489a4299dc
Task: Exhaustively investigate the Agentic Chaining, Map-Reduce, and Synthesis pipeline:
1. Examine `crates/quorum-coordinator/src/offline_engine.rs` (`synthesize_with_history`, `query_local_ollama_chat`, etc.) and `crates/quorum-coordinator/src/agentic/`:
   - Inspect line 1154 in `offline_engine.rs`: verify the hardcoded `let num_predict = 1600usize;` that completely ignores `ui.max_output_tokens`, `ui.agentic_loop_enabled`, `ui.agentic_chunk_size`, and `ui.agentic_strategy`.
   - Inspect how Ollama streaming is received and forwarded via `telemetry_tx`. Specifically check `tx.blocking_send` vs `tx.try_send` in `block_in_place` to ensure zero tokens are dropped when streaming up to 262k tokens.
   - Investigate how `done_reason == "length"` is detected and how recursive turn chaining works up to `max_output_tokens`.
   - Investigate sentence-boundary completion guards (finishing the sentence rather than cutting off mid-word or mid-sentence).
2. Examine Iterative Section Generation (`DocumentPlan` and `SectionPlan`):
   - How a structured outline with subtopics and targeted search queries can be generated from broad retrieval.
   - How each section executes targeted queries, synthesizes standalone chapters, and passes `rolling_summary` (~1500 chars) for seamless continuity.
3. Examine Map-Reduce / Refine Fact Sheet Extraction (R4):
   - How chunk groups (e.g. 3-4 chunks) can be processed to extract relevant facts, quotes, metrics into dense fact sheets before chapter writing.
4. Examine Response Formatting & Actions Bar (R5):
   - Restating question at start, inline bracketed citations `[1]`, `[2]`, `### 📚 Sources & Document Links` with `SOURCES` badges.
   - UI response actions bar (`.response-actions-bar`) in `app.js` (`Continue`, `Copy`, `Regenerate`, `Read Aloud`, `Save File`, `Clear`, `👍`, `👎`).
