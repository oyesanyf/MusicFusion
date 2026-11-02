# Dispatch Instructions — Orchestrator Gen 3

## 2026-11-02T04:56:25Z
CRITICAL INVARIANTS APPLIED TO WORKSPACE & MILESTONE ROADMAP:

1. **Strict RAG-Only Synthesis (`"I dont think I want responses from the model just from RAG"`)**:
   - In `crates/quorum-coordinator/src/offline_engine.rs` and `crates/quorum-coordinator/src/agentic/mod.rs`:
     - Removed `default_model_citation` (`Local Neural Model` / `Local Neural Engine`) and removed ungrounded LLM synthesis (`query_local_ollama_chat`).
     - Responses are now synthesized 100% deterministically from retrieved RAG chunks (`matched_chunks`) with `### 📄 <filename>` headers and real local file `Sources` at the bottom.
     - When `chunks.is_empty()` or `matched_chunks.is_empty()`, returns `"No matching documents were found in your local RAG index for **<prompt>**."` with `Vec::new()` citations.
   - In `crates/quorum-coordinator/src/agentic/retrieve.rs`:
     - Applied the multi-term precision fix (`content_matched_terms >= required_query_hits && entity_hits >= required_entity_hits`) and daemon-active source-code/dump filtering.
2. **Source Code & Dump Exclusion (M8.5)**:
   - Added `exclude_source_code: bool` to `IngestConfig` (`default: false` for unit test compatibility, set to `true` in `quorum-daemon/src/main.rs`).
   - Updated `ClassifierRouter` and `FilesystemCrawler` to exclude `FileCategory::Code`, source/config/dump extensions, and Office temporary lock files (`~$*`, `.~lock*`, `*.tmp`, `*.bak`, `*.wbk`).
3. **UI Welcome Banner Removal (`"REMOVE THESE"`)**:
   - Removed the welcome banner and quick-prompt chips from `crates/quorum-ui/assets/index.html`, `app.js`, and `styles.css`.

Please ensure these invariants are strictly preserved across M8, M8.5, M9, and M10! Recorded to `ORIGINAL_REQUEST.md`.
