# Task Assignment: Worker M11 1 (Google LangExtract & Question-Restating LLM Orchestration)

## Working Directory
d:\harfile\QuorumRAG\.agents\teamwork\worker_m11_1\

## Exclusively Owned Files
- `crates/quorum-coordinator/src/langextract/extraction.rs`
- `crates/quorum-coordinator/src/langextract/mod.rs`
- `crates/quorum-coordinator/src/types.rs`
- `crates/quorum-coordinator/src/agentic/classify.rs`
- `crates/quorum-coordinator/src/agentic/retrieve.rs`
- `crates/quorum-coordinator/src/agentic/mod.rs`
- `crates/quorum-coordinator/src/offline_engine.rs`
- `crates/quorum-coordinator/tests/coordinator_tests.rs` (or new test file `crates/quorum-coordinator/tests/m11_langextract_tests.rs`)

## Mission & Implementation Instructions
Implement Milestone M11 (Features 58 & 59) strictly following the specifications in:
- `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m11_1\handoff.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m11_2\handoff.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m11_3\handoff.md`

### 1. `crates/quorum-coordinator/src/langextract/extraction.rs`
- Implement `pub fn extract_chat_candidates(prompt: &str) -> Vec<Extraction>`:
  - Extract question spans (interrogative prefixes or full prompt).
  - Extract document references (`.pdf`, `.docx`, `.doc`, `.txt`, `.rs`, `.md`, e.g., `ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_Enhanced_Resume_v3.docx`, `Femi_Oyesanya_CV_2026.pdf`, `Resume2026.pdf`).
  - Extract named entities and technical terms (capitalized words, known entities: `Femi`, `Oyesanya`, `RedactX`, `VaultGemma`, `Presidio`, `PatientPoint`, `Cigna`, `Angela`, `Davis`).
  - Guarantee all candidate extractions match verbatim in `prompt` so they pass `ground_extraction` and 4-arm consensus in `consensus.rs`.
- Re-export `extract_chat_candidates` in `crates/quorum-coordinator/src/langextract/mod.rs`.

### 2. `crates/quorum-coordinator/src/types.rs`
- Add to `AgenticGraphState`:
  - `pub langextract_result: Option<crate::langextract::ConsensusExtractionResult>,`
  - `pub extracted_questions: Vec<String>,`
  - `pub extracted_entities: Vec<crate::langextract::Extraction>,`
- Initialize in `AgenticGraphState::new` (`langextract_result: None`, `extracted_questions: Vec::new()`, `extracted_entities: Vec::new()`).

### 3. `crates/quorum-coordinator/src/agentic/classify.rs`
- In `ClassifyIntentNode::execute`:
  - Invoke `let candidates = crate::langextract::extraction::extract_chat_candidates(&state.original_prompt);`
  - Evaluate via `crate::langextract::LlmFusionConsensus::evaluate_extractions(&state.original_prompt, candidates, Some(state));`
  - Store `state.langextract_result = Some(consensus.clone());`
  - Partition verified questions into `state.extracted_questions` and entities into `state.extracted_entities`.
  - Merge metadata filters with `consensus.metadata_filters`.

### 4. `crates/quorum-coordinator/src/agentic/retrieve.rs`
- In `RetrieveMeshNode::execute`:
  - Check `state.extracted_entities` for `document_reference`.
  - In `precompute_criteria`, match target document filenames and apply +200 bonus (`intent_bonus += 200`), force `is_relevant = true` so requested documents are never pruned.
  - Augment Tantivy BM25 keyword search with verified entity keywords.

### 5. `crates/quorum-coordinator/src/agentic/mod.rs`
- In `QueryCache::insert`, pass `graph_state.langextract_result.clone()` instead of `None`.
- On cache hit, restore `langextract_result`, `extracted_questions`, and `extracted_entities` onto `graph_state`.

### 6. `crates/quorum-coordinator/src/offline_engine.rs`
- In `query_local_ollama_chat`:
  - Update system prompt with mandatory question-restating rules ("Who is femi" becomes "Femi is ..."), grounding strictly in retrieved documents (e.g. `ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_CV_2026.pdf`), and placing URLs/doc links strictly at the end with relevance scores.
- Implement `format_question_restated_prefix(prompt: &str) -> String` and `format_sources_block(matched_chunks: &[&ScoredChunk]) -> String`.
- In `synthesize_with_history`, prepend the restated prefix to the opening sentence and append the sources block at the end.
- In streaming mode, stream the sources block upon completion.

### 7. Build and Verification
- Run:
  `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator -- --test-threads=2`
- Ensure all tests pass with zero warnings or errors.
- Stage, commit verified changes incrementally (`git add -A; git commit -m "feat(coordinator): implement LangExtract chat extraction, question-restating synthesis, and end citations"`).

### Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Write your report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m11_1\handoff.md` and report back via send_message.
