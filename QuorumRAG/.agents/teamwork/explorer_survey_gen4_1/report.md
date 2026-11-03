# Exhaustive Survey & Investigation Report: Configuration Models, UI Preferences, and SSE Streaming Architecture

**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_gen4_1`  
**Investigator**: Survey Explorer 1  
**Target Milestone**: Gen4 Deep Research and Iterative Section RAG  
**Date**: 2026-11-02  

---

## Executive Summary

An exhaustive investigation was conducted across `crates/quorum-coordinator`, `crates/quorum-ui`, and `crates/quorum-daemon` regarding Deep Research, Iterative Section RAG, and Agentic Loop configuration models, serialization, UI controls, and SSE stream buffering.

Key findings:
1. **Configuration Model (`dlp_mesh.rs`)**: The configuration struct is named `ChatGptUiSettings` (no `UiPreferencesConfig` type alias exists yet). It already possesses `max_output_tokens` (default `262144`), `agentic_loop_enabled` (default `true`), `agentic_chunk_size` (default `4096`), and `agentic_strategy` (default `"auto_continuation"`). However, R1 fields `deep_research_iterative_rag: bool`, `parent_document_retrieval: bool`, and `map_reduce_fact_extraction: bool` are missing. Additionally, R1 aliases `agentic_loop_chaining` and `agentic_loop_chunk_size` need serde aliases.
2. **UI Preferences (`app.js` & `index.html`)**: The Settings modal renders under tab `'models'` with `#setting-max-tokens` containing **all 10 requested options** (256, 512, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144). Toggles for `deep_research_iterative_rag`, `parent_document_retrieval`, and `map_reduce_fact_extraction` are **completely missing** from the UI. Settings persist via `POST /api/settings` (not `/api/preferences`), and chat requests sent to `POST /api/chat` currently omit these preferences and hardcode `top_k: 12`.
3. **Daemon & Engine Pipeline (`chat.rs` & `offline_engine.rs`)**: In `chat.rs:111`, `mpsc::channel` buffer capacity is set to `16384`. However, `ChatRequest` does not forward UI preferences. In `offline_engine.rs:1154`, `let num_predict = 1600usize;` is hardcoded, completely ignoring `ui.max_output_tokens`, `ui.agentic_loop_enabled`, `ui.agentic_chunk_size`, and `ui.agentic_strategy`. Furthermore, `offline_engine.rs:1309` uses `tx.try_send(...)` which drops tokens when the channel is full, line 499 improperly tests body text instead of filename tokens for `file_hits`, and line 505 discards candidates using a raw RRF threshold `< 0.35`.

---

## 1. Observation

### 1.1 `crates/quorum-coordinator/src/dlp_mesh.rs`

#### Struct Definition & Existing Fields
Lines 186–257:
```rust
/// Personal & Application Settings persisted in `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatGptUiSettings {
    // ... General, Appearance, Notifications, Profile, Security, Parental, Contact, Voice, Personalization, Pets, Data Controls ...
    // IDE General & AI Models / Endpoints Settings
    pub enable_streaming: bool,
    pub auto_scroll: bool,
    pub sizing_strategy: String,
    pub ollama_url: String,
    pub ipc_url: String,
    pub cdp_port: u16,
    pub primary_workhorse_model: String,
    pub fusion_panel_size: usize,
    pub vision_specialist_model: String,
    pub audio_specialist_model: String,
    pub multimodal_auto_routing: bool,
    pub natural_human_voice: bool,
    pub sampling_temperature: f32,
    pub max_output_tokens: usize,
    pub agentic_loop_enabled: bool,
    pub agentic_chunk_size: usize,
    pub agentic_strategy: String,
    pub restrl_daemon_running: bool,
    pub restrl_processed_tasks: u64,
    // Dynamic key-value settings for all interactive controls (Appearance, Personalization, Family, Contacts, Voice, etc.)
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}
```

#### Default Trait Implementation
Lines 267–325:
```rust
impl Default for ChatGptUiSettings {
    fn default() -> Self {
        Self {
            // ...
            sampling_temperature: 0.20,
            max_output_tokens: 262144,
            agentic_loop_enabled: true,
            agentic_chunk_size: 4096,
            agentic_strategy: "auto_continuation".to_string(),
            restrl_daemon_running: true,
            restrl_processed_tasks: 14,
            extra: std::collections::HashMap::new(),
        }
    }
}
```

#### Persistence Controller
Lines 328–333 & 383–421:
- `PersistedDaemonSettings` wraps `pub ui: ChatGptUiSettings`, `pub mesh: OsoosiMeshSettings`, and `pub dlp: MeshDlpPolicy`.
- `OsoosiMeshDlpController::init_persistence(data_dir: &Path)` reads `<data_dir>/settings.json` into memory (`RwLock<PersistedDaemonSettings>`).
- `OsoosiMeshDlpController::save_to_disk()` serializes `PersistedDaemonSettings` as pretty JSON to `<data_dir>/settings.json`.
- Singleton accessor: `global_mesh_dlp_controller()` returns `Arc<OsoosiMeshDlpController>`.

#### Missing Fields from R1
| Field Name | Type | R1 Specification | Current Status in `ChatGptUiSettings` |
|---|---|---|---|
| `deep_research_iterative_rag` | `bool` | Default `true`: Structured multi-hop research (`SectionPlan` / `DocumentPlan`) | **Missing** (stored only as dynamic JSON in `extra`) |
| `parent_document_retrieval` | `bool` | Default `true`: Expanding matched chunks with surrounding/adjacent chunks | **Missing** (stored only as dynamic JSON in `extra`) |
| `map_reduce_fact_extraction` | `bool` | Default `true`: Distilling retrieved chunks into dense fact sheets before synthesis | **Missing** (stored only as dynamic JSON in `extra`) |
| `agentic_loop_chaining` | `bool` | Default `true`: Auto-chain LLM turns recursively up to 262k tokens | Present under name `agentic_loop_enabled` (no `agentic_loop_chaining` serde alias) |
| `agentic_loop_chunk_size` | `usize` | Default `4096`: Options 2048 / 4096 / 8192 per recursive hop | Present under name `agentic_chunk_size` (no `agentic_loop_chunk_size` serde alias) |
| `UiPreferencesConfig` | Type Alias | Alias for `ChatGptUiSettings` | **Missing** |

---

### 1.2 `crates/quorum-ui/assets/app.js` and `crates/quorum-ui/assets/index.html`

#### Settings Modal Layout & Tab Navigation
- In `crates/quorum-ui/assets/index.html:292–406`:
  - `#settings-view` is an overlay drawer with sidebar navigation list `#settings-nav-list`.
  - Tab button: `<button class="chatgpt-nav-item" data-tab="models">` opens the AI Models & Endpoints tab.
  - Footer buttons: `#btn-reset-settings` calls `window._resetSettingsToDefaults()`, `#btn-cancel-settings` calls `window._closeSettingsFromFooter()`, and `#btn-save-settings` calls `window._saveAllPreferencesFromFooter()`.

#### Settings Rendering in `app.js`
In `crates/quorum-ui/assets/app.js:2084–2405`:
```javascript
} else if (tab === 'models') {
  // ...
  const maxTok = String(ui.max_output_tokens || 262144);
  const loopEn = ui.agentic_loop_enabled !== false;
  const chunkSz = String(ui.agentic_chunk_size || 4096);
  const loopStrat = ui.agentic_strategy || 'auto_continuation';
```

Lines 2356–2371 render the `max_output_tokens` dropdown:
```html
<div class="settings-row">
  <div class="settings-row-info">
    <label for="setting-max-tokens" class="settings-row-label">Max Output Tokens</label>
    <div class="settings-row-desc">Upper bound on generated response token count per directive.</div>
  </div>
  <select id="setting-max-tokens" class="pill-select" style="min-width:295px;" onchange="window._saveUiPatch({max_output_tokens: Number(this.value)}); window._toast('Max output tokens set to ' + this.value)">
    <option value="256" ${maxTok === '256' ? 'selected' : ''}>256 tokens (Ultra-Concise Micro-Budget)</option>
    <option value="512" ${maxTok === '512' ? 'selected' : ''}>512 tokens (Short Summary)</option>
    <option value="2048" ${maxTok === '2048' ? 'selected' : ''}>2048 tokens (Concise)</option>
    <option value="4096" ${maxTok === '4096' ? 'selected' : ''}>4096 tokens (Standard Production)</option>
    <option value="8192" ${maxTok === '8192' ? 'selected' : ''}>8192 tokens (Default Production / Generous Output)</option>
    <option value="16384" ${maxTok === '16384' ? 'selected' : ''}>16384 tokens (Full Research Report)</option>
    <option value="32768" ${maxTok === '32768' ? 'selected' : ''}>32768 tokens (Agentic Loop: 32k - Extended Architecture)</option>
    <option value="65536" ${maxTok === '65536' ? 'selected' : ''}>65536 tokens (Agentic Loop: 64k - Full Application Modules)</option>
    <option value="131072" ${maxTok === '131072' ? 'selected' : ''}>131072 tokens (Agentic Loop: 128k - Complex Multi-Tier Systems)</option>
    <option value="262144" ${maxTok === '262144' ? 'selected' : ''}>262144 tokens (Agentic Loop: 256k - Complete Full-Stack Codebase)</option>
  </select>
</div>
```
- All 10 requested dropdown options are **present and exact**.

Lines 2373–2403 render other agentic controls:
```html
<div class="settings-row">
  <div class="settings-row-info">
    <label for="setting-agentic-loop-enable" class="settings-row-label">Agentic Loop Looping (Recursive Chaining)</label>
    <div class="settings-row-desc">Auto-chain LLM turns recursively up to 256k tokens. Generates multi-part outputs seamlessly by feeding turn outputs back into context.</div>
  </div>
  <input id="setting-agentic-loop-enable" type="checkbox" ${loopEn ? 'checked' : ''} style="width:16px; height:16px; accent-color:#3b82f6; cursor:pointer;" onchange="window._saveUiPatch({agentic_loop_enabled: this.checked}); window._toast('Agentic Loop recursive chaining ' + (this.checked ? 'enabled' : 'disabled'))">
</div>

<div class="settings-row">
  <div class="settings-row-info">
    <label for="setting-agentic-chunk-size" class="settings-row-label">Agentic Loop Chunk Size</label>
    <div class="settings-row-desc">Token budget generated per sequential turn before triggering continuation.</div>
  </div>
  <select id="setting-agentic-chunk-size" class="pill-select" style="min-width:260px;" onchange="window._saveUiPatch({agentic_chunk_size: Number(this.value)}); window._toast('Agentic loop chunk size set to ' + this.value)">
    <option value="4096" ${chunkSz === '4096' ? 'selected' : ''}>4096 tokens/turn (Recommended)</option>
    <option value="8192" ${chunkSz === '8192' ? 'selected' : ''}>8192 tokens/turn (Large Chunks)</option>
    <option value="2048" ${chunkSz === '2048' ? 'selected' : ''}>2048 tokens/turn (Fast Increments)</option>
  </select>
</div>

<div class="settings-row">
  <div class="settings-row-info">
    <label for="setting-agentic-strategy" class="settings-row-label">Agentic Loop Strategy</label>
    <div class="settings-row-desc">Sequencing approach for recursive context chaining and prompt triggers.</div>
  </div>
  <select id="setting-agentic-strategy" class="pill-select" style="min-width:260px;" onchange="window._saveUiPatch({agentic_strategy: this.value}); window._toast('Agentic loop strategy updated')">
    <option value="auto_continuation" ${loopStrat === 'auto_continuation' ? 'selected' : ''}>Auto-Continuation (Seamless Part-by-Part)</option>
    <option value="multi_part_assembly" ${loopStrat === 'multi_part_assembly' ? 'selected' : ''}>Multi-Part Assembly (Structured Modules)</option>
    <option value="target_budget" ${loopStrat === 'target_budget' ? 'selected' : ''}>Target Budget (Exact Token Target)</option>
  </select>
</div>
```

#### Missing UI Controls in `app.js`
1. `deep_research_iterative_rag`: No toggle in `tab === 'models'` or anywhere else in `app.js`.
2. `parent_document_retrieval`: No toggle in `tab === 'models'` or anywhere else in `app.js`.
3. `map_reduce_fact_extraction`: No toggle in `tab === 'models'` or anywhere else in `app.js`.
4. `agentic_loop_chaining`: The existing checkbox uses `agentic_loop_enabled` rather than `agentic_loop_chaining`.
5. `agentic_loop_chunk_size`: The existing select uses `agentic_chunk_size`.

#### Settings Persistence Mechanism
- **Loading Settings**: `app.js:1460–1470`:
  ```javascript
  async function loadFullSettings() {
    try {
      const resp = await fetch('/api/settings');
      if (!resp.ok) return null;
      const data = await resp.json();
      cachedSettingsData = data;
      // ...
      return data;
    } catch (_) { return null; }
  }
  ```
- **Patching Settings**: `app.js:1581–1598`:
  ```javascript
  async function saveUiSettingsPatch(patch) {
    if (!cachedSettingsData) await loadFullSettings();
    const ui = Object.assign({}, cachedSettingsData ? cachedSettingsData.ui : {}, patch);
    const resp = await fetch('/api/settings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ui })
    });
    // ...
  }
  ```
- **Footer Save**: `app.js:4274–4279`:
  ```javascript
  window._saveAllPreferencesFromFooter = async function() {
    if (cachedSettingsData && cachedSettingsData.ui) {
      await saveUiSettingsPatch(cachedSettingsData.ui);
    }
    showSettingsToast('All preferences saved to disk');
  };
  ```
- Note on `/api/preferences`: The server does **not** expose `/api/preferences`; all UI preference reads and writes go to `/api/settings`.

#### Chat Query Dispatch Payload
In `crates/quorum-ui/assets/app.js:286–303` (`streamAssistantTurn`):
```javascript
resp = await fetch('/api/chat', {
  method: 'POST',
  signal: activeAbortController ? activeAbortController.signal : undefined,
  headers: {
    'Content-Type': 'application/json',
    'Accept': 'text/event-stream',
  },
  body: JSON.stringify({
    query_id: activeQueryId,
    prompt: text,
    search_mode: currentSearchMode,
    model_family: currentModel,
    top_k: 12,
    access_tier: 0,
    bypass_cache: Boolean(bypassCache),
    history: turnHistorySnapshot
  })
});
```
- Observation: `top_k` is hardcoded to `12`.
- Observation: None of the UI preferences (`max_output_tokens`, `agentic_loop_enabled`, `deep_research_iterative_rag`, `parent_document_retrieval`, `map_reduce_fact_extraction`, etc.) are included in the request body.

---

### 1.3 `crates/quorum-daemon/`

#### Route Registration in `crates/quorum-daemon/src/server.rs`
Lines 20–27:
```rust
// Chat & Query Endpoints (SSE streaming & JSON)
.route("/api/chat", post(chat::chat_sse_handler))
.route("/api/chat/stream", get(chat_stream_compat_handler))
.route("/api/chat/cancel", post(chat::cancel_chat_handler))
.route("/api/chat/spellcheck", post(chat::spellcheck_handler))
.route("/api/query", post(chat::query_handler))

// ChatGPT Settings, OshoosiClaw P2P Mesh, Shared Folders & HIPAA PHI DLP Endpoints
.route("/api/settings", get(settings::get_settings_handler).post(settings::update_settings_handler))
```
- There is no `/api/preferences` route registered in `server.rs`.

#### Chat Request Payload and SSE Channel Buffering in `crates/quorum-daemon/src/handlers/chat.rs`
Lines 17–45:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    #[serde(alias = "query")]
    pub prompt: String,
    pub search_mode: Option<SearchMode>,
    pub model_family: Option<String>,
    pub top_k: Option<usize>,
    pub access_tier: Option<u8>,
    #[serde(default)]
    pub history: Vec<ChatTurnMessage>,
    pub query_id: Option<String>,
    #[serde(default)]
    pub bypass_cache: bool,
}
```

Lines 100–111 in `chat_sse_handler`:
```rust
let top_k = payload.top_k.unwrap_or(12).max(10);
let _access_tier = payload.access_tier.unwrap_or(0);
// ...
let (tx, rx) = tokio::sync::mpsc::channel::<AgenticTelemetryEvent>(16384);
```
- The SSE event buffer channel capacity is initialized to **16384**.

Lines 135–138 in `chat_sse_handler`:
```rust
match coordinator
    .query_agentic_with_history(&prompt, history, family, top_k, Some(task_tx.clone()))
    .await
```
- Preferences from `PersistedDaemonSettings` or `ChatRequest` are **not passed as arguments** into `query_agentic_with_history`.

---

### 1.4 Downstream Execution in `offline_engine.rs`

#### Hardcoded Output Tokens
In `crates/quorum-coordinator/src/offline_engine.rs:1153–1155`:
```rust
let temp = ui.sampling_temperature.clamp(0.05, 0.20);
let num_predict = 1600usize;
let num_ctx = 16384usize;
```
- Although `ui` is loaded from `global_mesh_dlp_controller()` at line 1110 (`let ui = controller.settings.read().map(|g| g.ui.clone()).unwrap_or_default();`), `num_predict` is hardcoded to `1600usize`, completely ignoring `ui.max_output_tokens` (which defaults to `262144`)!

#### Dropped Streaming Tokens
In `crates/quorum-coordinator/src/offline_engine.rs:1309–1311`:
```rust
let _ = tx.try_send(AgenticTelemetryEvent::TokenDelta {
    delta: tok.to_string(),
});
```
- `try_send` is used without blocking or backpressure retry. If the channel fills, tokens are silently dropped.

#### Filename Matching and RRF Score Cutoff Bugs
In `crates/quorum-coordinator/src/offline_engine.rs:499–507`:
```rust
let file_hits = query_terms
    .iter()
    .filter(|t| matches_whole_word(t.as_str()) || file_l.contains(t.as_str()))
    .count();

// Strict minimum similarity cutoff: discard candidates below calibrated similarity threshold (0.35 / 35%)
if scored.score < 0.35 && scored.score > 0.0 && file_hits == 0 {
    continue;
}
```
- `matches_whole_word(t.as_str())` evaluates against `text_l` (chunk body text), not `file_l` (filename).
- `scored.score < 0.35` rejects valid chunks because reciprocal rank fusion scores are typically ~0.016 (e.g. 1 / (60 + 2) ≈ 0.0161), well below 0.35.

---

## 2. Logic Chain

1. **User Setting Ignored (From Observation 1.1 to 1.4)**:
   - Observation: `ChatGptUiSettings::default()` sets `max_output_tokens: 262144`.
   - Observation: In `offline_engine.rs:1154`, `num_predict` is hardcoded to `1600usize`.
   - Logic: When a user selects `262144 tokens` in Settings (as shown in the user's screenshot), the request triggers Ollama inference with `"num_predict": 1600`. Ollama cuts off generation after at most 1600 tokens regardless of the UI selection.
   - Conclusion: `num_predict` must be dynamically derived from `ui.max_output_tokens` and `ui.agentic_chunk_size`.

2. **Missing Configuration Fields in Rust Struct (From Observation 1.1)**:
   - Observation: `deep_research_iterative_rag`, `parent_document_retrieval`, and `map_reduce_fact_extraction` do not exist as fields in `ChatGptUiSettings`.
   - Observation: `agentic_loop_chaining` is currently called `agentic_loop_enabled`, and `agentic_loop_chunk_size` is called `agentic_chunk_size`.
   - Logic: Without explicit fields and serde aliases on `ChatGptUiSettings`, typed engine code cannot read these flags at compile-time, and serde deserialization routes them into the untyped `extra: HashMap<String, Value>`.
   - Conclusion: Add the typed fields to `ChatGptUiSettings`, add serde aliases for the R1 alternative names, update `Default`, and define `pub type UiPreferencesConfig = ChatGptUiSettings;`.

3. **Missing UI Toggles (From Observation 1.2)**:
   - Observation: `app.js` only renders controls for `setting-max-tokens`, `setting-agentic-loop-enable`, `setting-agentic-chunk-size`, and `setting-agentic-strategy`.
   - Logic: The user cannot enable or disable Deep Research iterative RAG, parent document retrieval, or map-reduce fact extraction from the UI.
   - Conclusion: Add interactive toggle switches for `deep_research_iterative_rag`, `parent_document_retrieval`, `map_reduce_fact_extraction`, and synchronize `agentic_loop_chaining` in `app.js:renderSettingsTabContent('models')`.

4. **Query Pipeline Communication (From Observation 1.2, 1.3, and 1.4)**:
   - Observation: `app.js` sends `{ query_id, prompt, search_mode, model_family, top_k: 12, ... }` via `POST /api/chat`.
   - Observation: `offline_engine.rs:1099–1114` fetches the persisted settings directly from `global_mesh_dlp_controller()`.
   - Logic: Even if the client does not send all preferences in `ChatRequest`, the engine can read them from `global_mesh_dlp_controller().settings.read().unwrap().ui`. However, allowing optional overrides in `ChatRequest` (or having the client send them) provides per-request flexibility. Also, `top_k: 12` in `app.js` limits retrieval breadth; R2 specifies expanding `top_k` up to 20–50 chunks.
   - Conclusion: Ensure `offline_engine.rs` respects `global_mesh_dlp_controller()`'s `ui` settings, allow optional `ChatRequest` fields, and update `top_k` retrieval to support 20–50 candidates when parent document retrieval or deep research is active.

5. **SSE Channel Token Loss (From Observation 1.3 and 1.4)**:
   - Observation: In `chat.rs:111`, `mpsc::channel` is sized to `16384`. In `offline_engine.rs:1309`, `tx.try_send(...)` is used.
   - Logic: In a 262k token response, if the consumer (network socket or browser reader) pauses slightly, a burst of NDJSON tokens will fill the 16384-element channel. Any call to `try_send` will immediately discard tokens.
   - Conclusion: Use `tokio::task::block_in_place(|| tx.blocking_send(...))` or an async retry loop so that zero tokens are dropped during streaming.

---

## 3. Caveats

1. **Ollama Execution Context**: In tests or environments where Ollama is offline or not running on `127.0.0.1:11434`, the engine falls back to `DeterministicOfflineSynthesisEngine::synthesize_deterministic_with_citations`. That fallback must also support multi-paragraph structured output and respect the Q&A opening restatement without line numbers.
2. **Channel Capacity vs. Memory**: An `mpsc::channel(16384)` for `AgenticTelemetryEvent` requires ~4–8 MB of RAM at peak buffer. This is safe and well within standard limits.
3. **Route Naming Parity**: The existing backend uses `/api/settings`. To prevent any breaking changes with other tools or test suites expecting `/api/preferences`, an alias route can be added in `server.rs` pointing to the same handler.

---

## 4. Conclusion

The architectural state and necessary modifications are summarized below:

### Configuration & Models (`crates/quorum-coordinator/src/dlp_mesh.rs`)
- Add `pub type UiPreferencesConfig = ChatGptUiSettings;` and re-export in `crates/quorum-coordinator/src/lib.rs`.
- Add explicit fields to `ChatGptUiSettings`:
  ```rust
  #[serde(default = "default_true", alias = "deep_research")]
  pub deep_research_iterative_rag: bool,
  #[serde(default = "default_true", alias = "parent_doc_retrieval")]
  pub parent_document_retrieval: bool,
  #[serde(default = "default_true", alias = "map_reduce")]
  pub map_reduce_fact_extraction: bool,
  #[serde(default = "default_true", alias = "agentic_loop_chaining")]
  pub agentic_loop_enabled: bool,
  #[serde(default = "default_4096", alias = "agentic_loop_chunk_size")]
  pub agentic_chunk_size: usize,
  ```
- Initialize all new fields to `true` in `impl Default for ChatGptUiSettings`.

### UI Controls & Settings (`crates/quorum-ui/assets/app.js`)
- In `renderSettingsTabContent('models')`:
  - Retain the existing 10-option dropdown for `max_output_tokens` (256 up to 262144).
  - Add interactive toggle switches for:
    1. `deep_research_iterative_rag` ("Deep Research & Iterative Section RAG")
    2. `parent_document_retrieval` ("Parent Document Context Expansion")
    3. `map_reduce_fact_extraction` ("Map-Reduce Fact Sheet Extraction")
    4. `agentic_loop_chaining` ("Recursive Agentic Loop Chaining")
  - In `window._resetSettingsToDefaults()`, reset all new toggles to `true`.
  - In `streamAssistantTurn()`, pass `top_k: 24` (or 20–50) and include the active preference flags in the request payload.

### Daemon & Synthesis Pipeline (`quorum-daemon` & `offline_engine.rs`)
- In `crates/quorum-daemon/src/server.rs`: Add `.route("/api/preferences", get(settings::get_settings_handler).post(settings::update_settings_handler))`.
- In `crates/quorum-coordinator/src/offline_engine.rs`:
  - Replace `let num_predict = 1600usize;` with:
    ```rust
    let target_tokens = ui.max_output_tokens.max(256);
    let chunk_size = ui.agentic_chunk_size.clamp(2048, 8192);
    let num_predict = if ui.agentic_loop_enabled {
        chunk_size.min(target_tokens)
    } else {
        target_tokens.min(4096)
    };
    ```
  - In `query_local_ollama_chat`: Replace `tx.try_send(...)` with `tokio::task::block_in_place(|| tx.blocking_send(...))` so zero tokens are dropped.
  - In `offline_engine.rs:499`: Fix `file_hits` to check filename tokens (`file_l.contains(t.as_str())`), not body tokens.
  - In `offline_engine.rs:505`: Remove the `scored.score < 0.35` RRF filter.

---

## 5. Verification Method

1. **Configuration Inspection**:
   - Verify `ChatGptUiSettings` compiles with the new fields and serde aliases.
   - Run unit tests: `cargo test -p quorum-coordinator --lib dlp_mesh`.
2. **Settings Endpoint Verification**:
   - `GET http://127.0.0.1:3031/api/settings` and `GET http://127.0.0.1:3031/api/preferences` return JSON with `ui.max_output_tokens: 262144`, `ui.deep_research_iterative_rag: true`, `ui.parent_document_retrieval: true`, `ui.map_reduce_fact_extraction: true`.
3. **UI Verification**:
   - Open Settings -> AI Models & Endpoints (`http://127.0.0.1:3031`).
   - Confirm dropdown displays all 10 options from 256 to 262144.
   - Confirm all 4 toggle switches exist and persist state upon change.
4. **SSE Streaming Verification**:
   - Issue `POST /api/chat` for `"tell me about sacred maths"`.
   - Confirm tokens stream continuously without mid-sentence cutoffs and citations render at the bottom.
