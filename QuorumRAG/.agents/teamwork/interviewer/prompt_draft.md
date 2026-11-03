Incorporate Deep Research and Iterative Section RAG settings and engine capabilities into QuorumRAG—including parent document retrieval, recursive agentic loop chaining up to 262k tokens, map-reduce fact extraction, and offline engine truncation/continuation fixes—for production use to produce exhaustive, multi-page responses grounded in data without mid-sentence cutoffs.

Working directory: d:\harfile\QuorumRAG
Integrity mode: development

## Requirements

### R1. Deep Research and Iterative Section RAG Settings UI
Add configuration fields to `UiPreferencesConfig` in `crates/quorum-coordinator/src/dlp_mesh.rs` and render interactive controls in QuorumRAG Settings (`crates/quorum-ui/assets/app.js`):
- `max_output_tokens: usize` dropdown with options:
  - `256` -> `256 tokens (Ultra-Concise Micro-Budget)`
  - `512` -> `512 tokens (Short Summary)`
  - `2048` -> `2048 tokens (Concise)`
  - `4096` -> `4096 tokens (Standard Production)`
  - `8192` -> `8192 tokens (Default Production / Generous Output)`
  - `16384` -> `16384 tokens (Full Research Report)`
  - `32768` -> `32768 tokens (Agentic Loop: 32k - Extended Architecture)`
  - `65536` -> `65536 tokens (Agentic Loop: 64k - Full Application Modules)`
  - `131072` -> `131072 tokens (Agentic Loop: 128k - Complex Multi-Tier Systems)`
  - `262144` -> `262144 tokens (Agentic Loop: 256k - Complete Full-Stack Codebase)`
- `agentic_loop_chaining: bool` (default `true`): Auto-chain LLM turns recursively up to 262k tokens.
- `agentic_loop_chunk_size: usize` (default `4096`, options: 2048 / 4096 / 8192 tokens per recursive hop).
- `deep_research_iterative_rag: bool` (default `true`): Structured multi-hop research (`SectionPlan` / `DocumentPlan`).
- `parent_document_retrieval: bool` (default `true`): Expanding matched chunks with surrounding/adjacent parent chunks.
- `map_reduce_fact_extraction: bool` (default `true`): Distilling retrieved chunks into dense fact sheets before synthesis.

### R2. Parent Document Retrieval and Expanded Breadth
Implement parent document retrieval in `crates/quorum-coordinator/src/offline_engine.rs` and `crates/quorum-coordinator/src/agentic/`:
- Embed granular chunks for accurate retrieval, but stitch surrounding/adjacent chunks from the same document/file path to provide broader context rather than isolated fragments.
- Expand retrieval breadth by supporting `top_k` from 20 up to 50 chunks, combining hybrid vector and lexical search with reranking.

### R3. Iterative Section Generation and Recursive Agentic Loop Chaining
Implement multi-hop iterative RAG and recursive turn chaining:
- When enabled via settings or when the output token budget requires multi-section expansion, generate a structured `DocumentPlan` containing `SectionPlan` subtopics and targeted search queries from initial broad retrieval.
- Iterate through each section, executing dedicated targeted vector queries and synthesizing standalone chapters using retrieved context while passing the previous section tail (`rolling_summary`, ~1500 chars) for continuity.
- Recursively chain LLM turns up to the configured `max_output_tokens` when `done_reason == "length"` or when the user prompts "continue" or "tell me more", feeding turn outputs back into context.
- Enforce a sentence-boundary completion guard so that generated text never cuts off mid-sentence.

### R4. Map-Reduce Fact Sheet Extraction
Implement a Map-Reduce / Refine extraction step over retrieved chunks prior to section writing:
- Run parallel extraction over chunk groups (e.g., 3-4 chunks) to extract relevant facts, quotes, and metrics verbatim into fact sheets.
- Provide the distilled fact sheets to the synthesis model for coherent, high-density chapter writing.

### R5. Offline Engine Fixes, Truncation Guards, and Actions Bar
Fix existing retrieval, ranking, and UI response defects:
- Fix `file_hits` in `offline_engine.rs:499` to check filename tokens (`file_l`) rather than body tokens (`matches_whole_word`).
- Remove the broken `scored.score < 0.35` RRF filter at `offline_engine.rs:505`.
- Ensure follow-up queries like "tell me more" and "continue" never fail with empty chunks.
- Format all responses to restate the question at the start, provide inline `[1]`, `[2]` citations, append `### 📚 Sources & Document Links` with `SOURCES` badges, and render the `.response-actions-bar` (`Continue`, `Copy`, `Regenerate`, `Read Aloud`, `Save File`, `Clear`, `👍`, `👎`).

### R6. Build, Service Deployment, and Live Verification
- Terminate existing daemon instances, clean/delete `target/debug/quorumrag.exe`, and rebuild `quorumrag.exe`.
- Ship verified changes using `scripts/solo_pr_ship.ps1`.
- Restart `quorumrag.exe` listening on `0.0.0.0:3031` and verify live SSE responses for `"tell me about sacred maths"` and `"tell me more"` without mid-sentence cutoffs.

## Acceptance Criteria

### Settings & Configuration
- [ ] `UiPreferencesConfig` contains fields for `max_output_tokens`, `agentic_loop_chaining`, `agentic_loop_chunk_size`, `deep_research_iterative_rag`, `parent_document_retrieval`, and `map_reduce_fact_extraction`.
- [ ] Settings UI displays the exact dropdown options from 256 tokens up to 262144 tokens, along with toggles for iterative RAG, parent document retrieval, map-reduce, and agentic loop chaining.

### Retrieval & Synthesis
- [ ] Parent document retrieval stitches adjacent chunks from the same document without returning isolated fragments.
- [ ] Iterative section generation creates a structured outline of subtopics and targeted queries, querying and writing section by section.
- [ ] Generated responses never terminate mid-sentence.
- [ ] "tell me more" and "continue" follow-up queries retrieve relevant context and chain turns successfully without empty chunks.
- [ ] Filename token search in `file_hits` checks `file_l`, and the `scored.score < 0.35` RRF filter is removed.

### UI & Presentation
- [ ] Responses open with a restatement of the question.
- [ ] Responses contain inline bracketed citations matching cited sources.
- [ ] Responses conclude with `### 📚 Sources & Document Links`, source badges, and the `.response-actions-bar` action buttons.

### Verification
- [ ] `cargo test` passes across all crates.
- [ ] Live SSE test query for `"tell me about sacred maths"` on `0.0.0.0:3031` generates an exhaustive, multi-section response without mid-sentence cutoff.
- [ ] Live SSE follow-up query for `"tell me more"` continues generating context without stalling on empty chunks.
- [ ] Changes are committed and pushed via `scripts/solo_pr_ship.ps1`.

## Verification Resources
- Existing cargo test suite (`cargo test`).
- Solo PR ship script: `scripts/solo_pr_ship.ps1`.
- Live SSE test queries on `http://127.0.0.1:3031`:
  - Initial query: `"tell me about sacred maths"`
  - Continuation query: `"tell me more"`
