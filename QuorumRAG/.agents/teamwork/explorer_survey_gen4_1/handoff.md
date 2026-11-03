# Handoff Report — explorer_survey_gen4_1

**Milestone**: Gen4 Survey & Investigation (Deep Research & Iterative Section RAG)  
**Agent**: Survey Explorer 1  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_gen4_1`  
**Handoff Type**: Hard (Task Complete)  

---

## 1. Observation

1. **`crates/quorum-coordinator/src/dlp_mesh.rs`**:
   - `ChatGptUiSettings` (lines 189–257) defines `pub max_output_tokens: usize` (line 248), `pub agentic_loop_enabled: bool` (line 249), `pub agentic_chunk_size: usize` (line 250), `pub agentic_strategy: String` (line 251), and `#[serde(flatten)] pub extra: HashMap<String, serde_json::Value>` (lines 255–256).
   - In `impl Default for ChatGptUiSettings` (lines 316–319), defaults are: `max_output_tokens: 262144`, `agentic_loop_enabled: true`, `agentic_chunk_size: 4096`, `agentic_strategy: "auto_continuation"`.
   - `UiPreferencesConfig` does not exist as a type or alias.
   - Missing explicit fields from R1: `deep_research_iterative_rag: bool`, `parent_document_retrieval: bool`, `map_reduce_fact_extraction: bool`.
   - Alternative names: R1 specifies `agentic_loop_chaining` (currently named `agentic_loop_enabled`) and `agentic_loop_chunk_size` (currently named `agentic_chunk_size`).

2. **`crates/quorum-ui/assets/app.js` and `index.html`**:
   - `index.html:293–406` defines `#settings-view` modal drawer with navigation tabs. Tab button `models` navigates to "AI Models & Endpoints".
   - `app.js:2356–2371` renders `#setting-max-tokens` dropdown with all 10 requested options:
     `256`, `512`, `2048`, `4096`, `8192`, `16384`, `32768`, `65536`, `131072`, `262144`.
   - `app.js:2373–2403` renders `#setting-agentic-loop-enable`, `#setting-agentic-chunk-size`, and `#setting-agentic-strategy`.
   - Toggles for `deep_research_iterative_rag`, `parent_document_retrieval`, and `map_reduce_fact_extraction` are completely absent.
   - Persistence: Settings are loaded via `fetch('/api/settings')` (`loadFullSettings()`, line 1461) and patched via `POST /api/settings` (`saveUiSettingsPatch()`, line 1584). No `/api/preferences` route exists on the backend.
   - Chat query dispatch: `streamAssistantTurn` (`app.js:286–303`) sends `{ query_id, prompt, search_mode, model_family, top_k: 12, access_tier: 0, bypass_cache, history }` to `POST /api/chat`. User preferences are omitted from this payload and `top_k` is hardcoded to `12`.

3. **`crates/quorum-daemon/`**:
   - `crates/quorum-daemon/src/handlers/chat.rs:18–45`: `ChatRequest` contains `prompt`, `search_mode`, `model_family`, `top_k`, `access_tier`, `history`, `query_id`, `bypass_cache`.
   - `crates/quorum-daemon/src/handlers/chat.rs:111`: `let (tx, rx) = tokio::sync::mpsc::channel::<AgenticTelemetryEvent>(16384);` (buffer capacity is 16384).
   - `crates/quorum-daemon/src/handlers/chat.rs:135–137`: Calls `coordinator.query_agentic_with_history(&prompt, history, family, top_k, Some(task_tx.clone()))`.
   - `crates/quorum-coordinator/src/offline_engine.rs:1110–1114`: Reads `ui` from `global_mesh_dlp_controller()`.
   - `crates/quorum-coordinator/src/offline_engine.rs:1154`: `let num_predict = 1600usize;` is hardcoded, completely ignoring `ui.max_output_tokens` (`262144`).
   - `crates/quorum-coordinator/src/offline_engine.rs:1309`: `tx.try_send(AgenticTelemetryEvent::TokenDelta { delta: tok.to_string() })` drops tokens when the channel is full.
   - `crates/quorum-coordinator/src/offline_engine.rs:499–502`: `file_hits` checks body text (`matches_whole_word`) instead of filename tokens.
   - `crates/quorum-coordinator/src/offline_engine.rs:505`: Drops chunks when `scored.score < 0.35` due to raw RRF scores (~0.016).

---

## 2. Logic Chain

1. In `offline_engine.rs:1154`, `num_predict = 1600usize` is hardcoded. Even when `ui.max_output_tokens` is configured to `262144`, Ollama terminates generation at 1600 tokens. This was directly reported by the user ("is already set, it must respect this the last response did not").
2. The UI in `app.js` has all 10 dropdown values for `max_output_tokens`, but lacks checkboxes for `deep_research_iterative_rag`, `parent_document_retrieval`, and `map_reduce_fact_extraction`.
3. In `dlp_mesh.rs`, `ChatGptUiSettings` does not declare `deep_research_iterative_rag`, `parent_document_retrieval`, or `map_reduce_fact_extraction`. Without these fields, Rust code cannot read them strongly-typed from `ui`.
4. In `offline_engine.rs:1309`, `tx.try_send` drops tokens under buffer contention, violating the zero-token-drop requirement.
5. In `offline_engine.rs:499`, `matches_whole_word` evaluates chunk body text instead of filename tokens, and line 505 filters out valid documents because raw RRF scores are ~0.016 (< 0.35).

---

## 3. Caveats

- In headless test suites without a running local Ollama instance (`127.0.0.1:11434`), the engine falls back to `synthesize_deterministic_with_citations`. The deterministic fallback must also produce multi-paragraph responses and respect Q&A question-restating prefixes without line numbers.
- Adding an alias `/api/preferences` -> `/api/settings` in `crates/quorum-daemon/src/server.rs` ensures compatibility for callers expecting either URL path.

---

## 4. Conclusion

The codebase is fully mapped and ready for implementation.
Key implementation tasks:
1. `dlp_mesh.rs`: Define `pub type UiPreferencesConfig = ChatGptUiSettings;`, add the 3 missing boolean fields (`deep_research_iterative_rag`, `parent_document_retrieval`, `map_reduce_fact_extraction`), add serde aliases for `agentic_loop_chaining` and `agentic_loop_chunk_size`, and update `Default`.
2. `app.js`: Add the 4 toggles to `tab === 'models'`, synchronize reset and save handlers, and expand `top_k` up to 24–50 in `streamAssistantTurn`.
3. `server.rs`: Add `/api/preferences` route alias pointing to `settings::get_settings_handler` and `settings::update_settings_handler`.
4. `offline_engine.rs`: Derive `num_predict` dynamically from `ui.max_output_tokens` and `ui.agentic_chunk_size`, replace `tx.try_send` with `blocking_send` inside `block_in_place`, fix `file_hits` to check filename tokens, and remove the `scored.score < 0.35` filter.

---

## 5. Verification Method

- Detailed analysis and line references are documented in `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_gen4_1\report.md`.
- Test commands:
  - `cargo test -p quorum-coordinator`
  - `cargo test -p quorum-daemon`
- Live inspection:
  - `GET http://127.0.0.1:3031/api/settings`
  - UI inspection at `http://127.0.0.1:3031` (Settings -> AI Models & Endpoints)
