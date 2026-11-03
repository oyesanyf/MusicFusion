# Scope: Milestone M11 - Deep Research Settings & Preferences UI

## Mission
Implement configuration fields, serialization, UI controls, and daemon preferences endpoint for Deep Research and Iterative Section RAG:
1. `crates/quorum-coordinator/src/dlp_mesh.rs`:
   - Add `pub type UiPreferencesConfig = ChatGptUiSettings;` (and re-export in `crates/quorum-coordinator/src/lib.rs`).
   - Add fields to `ChatGptUiSettings` with serde defaults and aliases:
     - `deep_research_iterative_rag: bool` (default `true`, alias `deep_research`)
     - `parent_document_retrieval: bool` (default `true`, alias `parent_doc_retrieval`)
     - `map_reduce_fact_extraction: bool` (default `true`, alias `map_reduce`)
     - Ensure `agentic_loop_enabled` has serde alias `agentic_loop_chaining`.
     - Ensure `agentic_chunk_size` has serde alias `agentic_loop_chunk_size`.
   - Update `impl Default for ChatGptUiSettings` to set all new boolean fields to `true`.
2. `crates/quorum-ui/assets/app.js`:
   - In `renderSettingsTabContent('models')`:
     - Keep the 10-option dropdown for `max_output_tokens` (256, 512, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144).
     - Add interactive toggles for:
       - `deep_research_iterative_rag` ("Deep Research & Iterative Section RAG")
       - `parent_document_retrieval` ("Parent Document Context Expansion")
       - `map_reduce_fact_extraction` ("Map-Reduce Fact Sheet Extraction")
       - `agentic_loop_enabled` / `agentic_loop_chaining` ("Recursive Agentic Loop Chaining")
     - Ensure all toggles update settings via `_saveUiPatch` and show toasts.
     - In `window._resetSettingsToDefaults()`, reset all toggles to `true`.
3. `crates/quorum-daemon/src/server.rs`:
   - Add `.route("/api/preferences", get(settings::get_settings_handler).post(settings::update_settings_handler))` as alias route for settings.

## File Boundaries (Exclusive Write Ownership)
- `crates/quorum-coordinator/src/dlp_mesh.rs`
- `crates/quorum-coordinator/src/lib.rs`
- `crates/quorum-ui/assets/app.js`
- `crates/quorum-daemon/src/server.rs`

## Acceptance Criteria
- [ ] `UiPreferencesConfig` is defined and exported.
- [ ] `ChatGptUiSettings` includes typed fields for `deep_research_iterative_rag`, `parent_document_retrieval`, and `map_reduce_fact_extraction`.
- [ ] Settings UI in `app.js` renders all 10 options for max tokens and toggles for all 4 features.
- [ ] `/api/preferences` and `/api/settings` read and write settings correctly.
- [ ] `cargo test -p quorum-coordinator --lib dlp_mesh` passes.
- [ ] Reviewer APPROVE, Challenger PASS, Forensic Auditor CLEAN.
