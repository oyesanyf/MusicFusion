# BRIEFING — 2026-11-02T14:33:00Z

## Mission
Investigate and design complete code specifications for Feature 59 (Question-Restating LLM Orchestration, deterministic offline question-restater, and end-of-response sources & document links block with explicit relevance scores) in `crates/quorum-coordinator/src/offline_engine.rs`.

## 🔒 My Identity
- Archetype: teamwork_explorer
- Roles: explorer, investigator, analyst
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m11_3\
- Original parent: 6e43b521-b448-4762-842a-0bc8d66e2854
- Milestone: M11 (Requirement 2: Question-Restating LLM Orchestration)

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Output handoff to d:\harfile\QuorumRAG\.agents\teamwork\explorer_m11_3\handoff.md
- Ground strictly in retrieved documents (e.g. `ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_CV_2026.pdf`)
- Mandate question restating ("Who is femi" -> "Femi is ...")
- End-of-response sources and document links block with explicit relevance scores

## Current Parent
- Conversation ID: 6e43b521-b448-4762-842a-0bc8d66e2854
- Updated: 2026-11-02T14:25:01Z

## Investigation State
- **Explored paths**:
  - `ORIGINAL_REQUEST.md`, `SCOPE.md`, `explorer_survey_adapt_1/handoff.md`
  - `crates/quorum-coordinator/src/offline_engine.rs` (lines 480–935, 1–250, 1420–1507)
  - `crates/quorum-types/src/lib.rs` (`DocumentChunk`, `ScoredChunk`, `Citation`)
  - `crates/quorum-ui/assets/app.js` (lines 710–745)
  - `crates/quorum-daemon/src/server.rs`
- **Key findings**:
  - `query_local_ollama_chat` system prompt lacked question-restatement invariant and allowed conversational preambles.
  - Deterministic fallback in `synthesize_with_history` emitted raw sections without an opening affirmative restatement.
  - Both live streaming and deterministic paths lacked the standardized end-of-response sources block with relevance scores.
- **Unexplored areas**: None for M11.3 scope.

## Key Decisions Made
- Designed `DeterministicOfflineSynthesisEngine::format_question_restated_prefix` supporting interrogative forms (`who is`, `what is`, `where did`, `what did ... at ...`, `tell me about`, `explain`, `what the`) and casing preservation (`RedactX`, `VaultGemma`).
- Designed `DeterministicOfflineSynthesisEngine::format_sources_block` formatting markdown links `[📄 <file_name>](<file_path>) — **Relevance Score: {:.2}** (Lines <start>–<end>)`.
- Designed `query_local_ollama_chat` system prompt with 6 mandatory invariants and user prompt concluding directive.
- Designed streaming token delta emission for the sources block upon LLM stream completion.
- Authored 2 comprehensive unit tests (`test_format_question_restated_prefix`, `test_deterministic_synthesis_restates_question_and_appends_sources_block`).

## Artifact Index
- `DISPATCH.md` — Received dispatch instructions
- `BRIEFING.md` — Agent working memory
- `progress.md` — Liveness heartbeat and milestone checklist
- `handoff.md` — Complete 5-component report and drop-in code specifications
