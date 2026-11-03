# Changes Implemented by Worker M11 (Milestone M11)

## Summary of Changes

### 1. `crates/quorum-coordinator/src/dlp_mesh.rs`
- Defined and exported `pub type UiPreferencesConfig = ChatGptUiSettings;`.
- Added helper functions `default_true() -> bool` and `default_4096() -> usize`.
- Added typed fields to `ChatGptUiSettings`:
  ```rust
  #[serde(default = "default_true", alias = "deep_research")]
  pub deep_research_iterative_rag: bool,
  #[serde(default = "default_true", alias = "parent_doc_retrieval")]
  pub parent_document_retrieval: bool,
  #[serde(default = "default_true", alias = "map_reduce")]
  pub map_reduce_fact_extraction: bool,
  ```
- Added serde alias `agentic_loop_chaining` to `agentic_loop_enabled`:
  ```rust
  #[serde(default = "default_true", alias = "agentic_loop_chaining")]
  pub agentic_loop_enabled: bool,
  ```
- Added serde alias `agentic_loop_chunk_size` to `agentic_chunk_size`:
  ```rust
  #[serde(default = "default_4096", alias = "agentic_loop_chunk_size")]
  pub agentic_chunk_size: usize,
  ```
- In `impl Default for ChatGptUiSettings`, set:
  - `deep_research_iterative_rag: true,`
  - `parent_document_retrieval: true,`
  - `map_reduce_fact_extraction: true,`
- Added three comprehensive unit tests to `tests`:
  - `test_ui_preferences_config_defaults`: Verifies default values for all new fields and existing token/loop settings.
  - `test_ui_preferences_config_serde_aliases`: Validates deserialization using aliases (`deep_research`, `parent_doc_retrieval`, `map_reduce`, `agentic_loop_chaining`, `agentic_loop_chunk_size`).
  - `test_ui_preferences_config_serde_canonical_and_defaults`: Validates deserialization from empty JSON `{}` to defaults, deserialization with canonical field names, and full serialization round-trip.

### 2. `crates/quorum-coordinator/src/lib.rs`
- Re-exported `UiPreferencesConfig` from `dlp_mesh`.

### 3. `crates/quorum-ui/assets/app.js`
- In `renderSettingsTabContent('models')`:
  - Extracted boolean values `deepResEn`, `parentDocEn`, `mapReduceEn` from UI settings with safe defaults (`!== false`).
  - Retained the complete 10-option dropdown for `#setting-max-tokens` (256 up to 262144 tokens).
  - Added interactive toggle switches with labels and descriptions:
    - `setting-deep-research`: "Deep Research & Iterative Section RAG" ("Generate structured DocumentPlan and SectionPlan multi-hop research outline.")
    - `setting-parent-doc-retrieval`: "Parent Document Context Expansion" ("Stitch adjacent/surrounding chunks from the same document for broad context.")
    - `setting-map-reduce-facts`: "Map-Reduce Fact Sheet Extraction" ("Distill retrieved chunks into dense fact sheets before synthesis.")
    - `setting-agentic-loop-enable`: "Recursive Agentic Loop Chaining" ("Auto-chain LLM turns recursively up to 256k tokens.")
  - Ensured each toggle persists immediately via `window._saveUiPatch({ field_name: this.checked })` and triggers toast notifications.
  - In `window._resetSettingsToDefaults()`: Ensured `deep_research_iterative_rag: true`, `parent_document_retrieval: true`, `map_reduce_fact_extraction: true`, and `agentic_loop_enabled: true` are reset to `true`.

### 4. `crates/quorum-daemon/src/server.rs`
- Added route alias:
  ```rust
  .route("/api/preferences", get(settings::get_settings_handler).post(settings::update_settings_handler))
  ```
  alongside `.route("/api/settings", ...)` to provide complete API parity for preference queries and updates.
