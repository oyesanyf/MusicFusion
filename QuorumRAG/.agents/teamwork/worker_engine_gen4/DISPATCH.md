## 2026-11-03T00:32:38Z
You are Worker Engine Gen4 (teamwork_worker). Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\worker_engine_gen4.
Read ORIGINAL_REQUEST.md at d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md.
Read Explorer 3's detailed report at d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_gen4_3\report.md.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

WRITE OWNERSHIP (You own exclusively):
- `crates/quorum-coordinator/src/offline_engine.rs`
- `crates/quorum-coordinator/src/agentic/`
- `crates/quorum-daemon/src/handlers/chat.rs`

TASKS:
1. Retrieval Bugfixes (`crates/quorum-coordinator/src/offline_engine.rs`):
   - In `offline_engine.rs:499`: Fix `file_hits` to check filename tokens (`file_l.contains(...)` or `file_tokens`), NOT chunk body text (`matches_whole_word`).
   - In `offline_engine.rs:505`: Remove the broken `if scored.score < 0.35 && scored.score > 0.0 && file_hits == 0 { continue; }` cutoff entirely.
   - For follow-up queries like "tell me more", "continue", "elaborate", extract context keywords from `history` so retrieval retrieves relevant chunks from prior context rather than failing with empty chunks.
2. Parent Document Retrieval & Breadth Expansion (R2):
   - Expand `top_k` up to 20..50 chunks.
   - Implement `stitch_parent_document_chunks`: when candidate chunks share `file_path` and adjacent `chunk_index` or contiguous line ranges, stitch them together so the model receives cohesive parent document sections rather than isolated fragments.
3. Map-Reduce Fact Sheet Extraction (R4):
   - When `ui.map_reduce_fact_extraction` is enabled, group retrieved chunks into clusters of 3-4 chunks and distill into dense fact sheets (quotes, dates, formulas, metrics, theorems) before chapter synthesis.
4. Dynamic Output Budget & Zero-Drop Telemetry (R1, R3, & Critical Follow-Up):
   - In `offline_engine.rs:1154`: Replace hardcoded `let num_predict = 1600usize;`!
   - Strictly read `ui.max_output_tokens`, `ui.agentic_loop_enabled`, `ui.agentic_chunk_size`, and `ui.agentic_strategy`.
   - When `ui.max_output_tokens` is large (or `agentic_loop_enabled == true`), set per-hop `num_predict` to `ui.agentic_chunk_size.clamp(2048, 8192)` (default 4096) and `num_ctx = 32768`.
   - In `offline_engine.rs:1309`: Replace `tx.try_send(...)` with `tx.blocking_send(AgenticTelemetryEvent::TokenDelta { delta })` inside `tokio::task::block_in_place` so ZERO tokens are ever dropped even on 262k token streams.
5. Recursive Agentic Loop Chaining & Sentence-Boundary Guard:
   - Handle `done_reason == "length"`: when Ollama emits `done_reason == "length"` and accumulated tokens < `ui.max_output_tokens`, chain subsequent continuation turns recursively up to `ui.max_output_tokens`, passing `rolling_summary` (last ~1500 chars of previous output) for continuity!
   - Sentence-boundary completion guard: ensure generated text ends cleanly on a sentence terminal boundary (`.`, `!`, `?`). Never leave trailing mid-sentence fragments like *"Sacred math has been"*.
6. Response Formatting & UI Events:
   - Restate the question at the start of synthesis.
   - Ensure inline bracketed citations `[1]`, `[2]` match `<document index="...">`.
   - Append `### 📚 Sources & Document Links` deduplicated by filename with accuracy percentages.
   - Emit `CitationEmitted` and `ExecutionFinished` events so the WebUI renders `.response-actions-bar` (`Continue`, `Copy`, `Regenerate`, `Read Aloud`, `Save File`, `Clear`, `👍`, `👎`).
7. Verification:
   - Run `cargo test -p quorum-coordinator`.
   - Run `cargo check --workspace`.

OUTPUT:
- Write `changes.md` and `handoff.md` to `d:\harfile\QuorumRAG\.agents\teamwork\worker_engine_gen4\`.
- Update `progress.md` in your directory.
- Send a completion message to the caller with test results.
