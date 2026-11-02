# Technical Investigation Handoff Report: WebUI Chat Box, Clean Interface, Response Action Buttons & server.rs Fix

**Author**: Survey Explorer 2 (`teamwork_explorer`)  
**Parent Orchestrator ID**: `6e43b521-b448-4762-842a-0bc8d66e2854`  
**Date**: 2026-11-02T14:19:00Z  
**Scope**: Requirements 3 & 5 (partial) from `ORIGINAL_REQUEST.md` and `DISPATCH.md`

---

## 1. Observation

### 1.1 `crates/quorum-daemon/src/server.rs:115` Compiler Failure
- Executed `$env:CARGO_INCREMENTAL="0"; cargo check -p quorum-daemon` (task `0d5a63e2-64cc-4f8f-8ba4-b6412bdfb99e/task-82`).
- Verbatim compiler diagnostic output:
```text
Checking quorum-daemon v0.1.0 (D:\harfile\QuorumRAG\crates\quorum-daemon)
error[E0063]: missing field `bypass_cache` in initializer of `ChatRequest`
   --> crates\quorum-daemon\src\server.rs:115:15
    |
115 |     let req = chat::ChatRequest {
    |               ^^^^^^^^^^^^^^^^^ missing `bypass_cache`

For more information about this error, try `rustc --explain E0063`.
error: could not compile `quorum-daemon` (lib) due to 1 previous error
```
- Direct inspection of `crates/quorum-daemon/src/handlers/chat.rs:18-44`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
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
- Direct inspection of `crates/quorum-daemon/src/server.rs:109-125`:
```rust
async fn chat_stream_compat_handler(
    State(state): State<Arc<DaemonState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let prompt = params.get("prompt").cloned().unwrap_or_default();
    let family = params.get("model_family").cloned();
    let req = chat::ChatRequest {
        prompt,
        search_mode: None,
        model_family: family,
        top_k: Some(12),
        access_tier: Some(0),
        history: Vec::new(),
        query_id: None,
    };
    chat::chat_sse_handler(State(state), axum::Json(req)).await
}
```
- Line 115 instantiates `chat::ChatRequest` without specifying `bypass_cache`. Because `ChatRequest` does not implement `Default` and requires all struct fields, this prevents `quorum-daemon` from compiling.

### 1.2 Home Screen Prompt Chips vs Top Page 🌐 QuorumRAG Logo
- In `crates/quorum-ui/assets/index.html:167-179`:
```html
      <!-- Messages Scroll Area (Centered 768px Reading Column) -->
      <div id="messages-container" class="messages-container custom-scroll">
        <div class="reading-col-768px" style="max-width: 768px; width: 100%; margin: 0 auto;">
          <!-- Top Page Logo -->
          <div id="welcome-banner" class="welcome-banner">
            <div class="welcome-logo">🌐</div>
            <h1>QuorumRAG</h1>
          </div>

          <!-- Message stream rendered here -->
          <div id="chat-stream" class="chat-stream"></div>
        </div>
      </div>
```
- In `crates/quorum-ui/assets/styles.css:459-478`:
```css
.welcome-banner {
  text-align: center;
  padding: 40px 20px 24px 20px;
}

.welcome-logo {
  font-size: 44px;
  margin-bottom: 10px;
}

.welcome-banner h1 {
  font-size: 28px;
  font-weight: 700;
  margin-bottom: 6px;
}

.quick-prompts,
.prompt-chip {
  display: none !important;
}
```
- In `crates/quorum-ui/assets/app.js:112-113`:
```javascript
  // Ensure any legacy quick-prompt chips are removed while keeping the top page logo (#welcome-banner) visible
  document.querySelectorAll('.quick-prompts, .prompt-chip').forEach(el => el.remove());
```
- In `crates/quorum-ui/assets/app.js:119-122` (New Chat):
```javascript
    if (welcomeBanner) {
      welcomeBanner.style.display = '';
      welcomeBanner.classList.remove('hidden');
    }
```
- In `crates/quorum-ui/assets/app.js:249-252` (Stream Turn):
```javascript
    if (welcomeBanner) {
      welcomeBanner.classList.remove('hidden');
      welcomeBanner.style.display = '';
    }
```
- In `crates/quorum-ui/assets/app.js:3346` (Archive Click):
```javascript
        if (welcomeBanner) welcomeBanner.classList.add('hidden');
```

### 1.3 Dynamic Real-Time Spellchecking in Chat Box
- In `crates/quorum-ui/assets/app.js:127-194`:
  - `promptInput.addEventListener('input', ...)` implements adaptive debouncing: 80ms when the last character is whitespace or punctuation (`' '`, `'?'`, `'.'`), 320ms when typing continuous characters.
  - Sends `POST /api/chat/spellcheck` with `{ text: trimmed, use_llm: true }`.
  - On response, verifies `promptInput.value.trim() === trimmed && data.changed`.
  - Non-destructively replaces text while retaining the cursor position (`selectionStart`) and trailing spaces.
  - Updates `#spellcheck-status`: `✨ Auto-corrected: "${data.corrected}"`.
  - `submitPrompt()` also awaits `spellcheckChatBox(true)` before dispatching `/api/chat` to guarantee typos are corrected before retrieval.
- In `crates/quorum-daemon/src/handlers/chat.rs:205-248`:
  - `spellcheck_handler` receives `Json<SpellcheckRequest>`.
  - Spawns blocking task: `DeterministicOfflineSynthesisEngine::correct_query_spelling(&raw, use_llm)`.
  - Returns `SpellcheckResponse { original, corrected, changed, engine }`.
- In `crates/quorum-coordinator/src/offline_engine.rs:940-1370`:
  - Implements multi-tier correction:
    1. Deterministic typo dictionary (`EXACT_TYPO_MAP` covering ~100 common typos e.g., `waht`, `teh`, `femi`, `oyesanya`, `exprience`, `documnt`).
    2. Global corpus vocabulary lookup with Levenshtein edit-distance matching (1-2 distance).
    3. Accidental Caps Lock / shouting normalization.
    4. Copula completion for 2-word queries ("WHAT REDACTX" -> "What is RedactX?").
    5. Local Ollama LLM integration via `query_local_ollama_spellcheck(&det_corrected)`:
       - Connects to `127.0.0.1:11434` (`qwen2.5:1.5b`) with a 400ms connection timeout and fast fallback.
       - System prompt: `"You are a real-time spellchecker for a chat input box. Fix ONLY spelling mistakes and accidental ALL-CAPS in the user's text. Output ONLY the corrected text with the exact same number of words. Never answer the question. Never change proper names like Femi Oyesanya, RedactX, VaultGemma, Presidio, Cigna, or Angela Davis."`
       - Strict validation (`validate_spellcheck_candidate`): ensures candidate matches word count, preserves protected domain terms and corpus words, and bounds edit distance.

### 1.4 Browser-Style Response Action Buttons
- In `crates/quorum-ui/assets/app.js:789-814` (`createAssistantMessage`):
  - Builds `<div class="message message-assistant">` containing:
    1. `<div class="message-header">`
    2. `<div class="message-body">`
    3. `<div class="citation-container">` (grounded sources)
    4. `<div class="response-actions-bar">` (action toolbar)
- In `crates/quorum-ui/assets/app.js:816-919` (`renderResponseActionsBar`):
  - **Copy**: `<button class="response-action-btn btn-copy-response">` (copies markdown to clipboard, shows `✓ Copied!` for 2s).
  - **Regenerate**: `<button class="response-action-btn btn-regenerate-response">` (resubmits prompt with `bypassCache: true`, triggering cache bypass in RAG pipeline).
  - **Read Aloud**: `<button class="response-action-btn btn-speak-response">` (uses `window.speechSynthesis` and `SpeechSynthesisUtterance`, strips bracketed citation indices, toggles between speak and stop with `onend`/`onerror` hooks).
  - **Feedback Thumbs**: `<button class="response-action-btn btn-feedback-up">` and `<button class="response-action-btn btn-feedback-down">` (mutually exclusive thumbs up / down feedback toggles).
- In `crates/quorum-ui/assets/styles.css:580-618`:
  - Full styles for `.response-actions-bar`, `.response-action-btn`, `:hover`, and `.active`.

---

## 2. Logic Chain

1. **Failure Diagnosis**:
   - `ChatRequest` gained a new `pub bypass_cache: bool` field (with `#[serde(default)]` for JSON deserialization).
   - In `crates/quorum-daemon/src/server.rs:115`, the compat GET route `/api/chat/stream` constructs `ChatRequest` directly using struct expression syntax without specifying `bypass_cache`.
   - In Rust, struct expressions require explicit initialization of all non-defaulted fields. This directly caused `error[E0063]`, breaking `cargo check -p quorum-daemon`.
   - Adding `bypass_cache: false` directly resolves this error and preserves normal caching behavior for the GET compatibility endpoint unless explicitly overridden.

2. **UI Cleanliness & Top Page Logo Retention**:
   - The sample prompt chips were historically hardcoded chips suggesting retrieval prompts ("Explain scatter-gather retrieval", "Show RrfCombiner & write unit test").
   - By omitting these chips from `index.html` and setting `.quick-prompts, .prompt-chip { display: none !important; }` in `styles.css`, the chat interface is clean, uncluttered, and ChatGPT-like.
   - The top page logo is anchored in `#welcome-banner` (`<div class="welcome-logo">🌐</div><h1>QuorumRAG</h1>`).
   - In `app.js`, line 3346 had `if (welcomeBanner) welcomeBanner.classList.add('hidden');` when unarchiving conversations from settings. Removing that line ensures the top page logo remains permanently visible across all conversation views.

3. **Real-Time Dynamic LLM Spellcheck**:
   - Fast user typing requires real-time debounce to avoid hammering the local Ollama daemon or coordinator.
   - 80ms on delimiters (`' '`, `'?'`, `'.'`) delivers immediate responsiveness upon finishing a word; 320ms on alphanumeric typing avoids querying mid-word keystrokes.
   - Because Ollama (`qwen2.5:1.5b`) is local, a 400ms connection timeout ensures offline resiliency: if Ollama is not running, the deterministic dictionary and Levenshtein corpus matching take over seamlessly without blocking the user.
   - Preserving `selectionStart` and trailing whitespace prevents cursor jumping or focus loss while typing.

4. **Action Button Placement & Grounding Separation**:
   - Appending `.citation-container` first and `.response-actions-bar` last guarantees that document sources and URL links appear strictly at the bottom of the answer text, followed by the action controls (Copy, Regenerate, Read Aloud, Feedback).
   - This satisfies both Requirement 2 ("URL and doc link must be strictly at the end of the response with relevance scores added") and Requirement 3 ("Add browser-style response action buttons at the end of each assistant response").

---

## 3. Caveats

- **No Caveats on Compiler Fix**: The compiler error in `server.rs:115` was directly reproduced and verified with `cargo check`.
- **Browser TTS Availability**: `window.speechSynthesis` is supported across modern browsers (Chromium, Firefox, Safari). If run in a headless environment without an audio subsystem, `window.speechSynthesis.speak` is a no-op without throwing unhandled exceptions.
- **Ollama Warmup**: When Ollama `qwen2.5:1.5b` is first loaded, the initial query may take ~500ms; `offline_engine.rs` uses `"keep_alive": "24h"` to keep the model warm in memory for subsequent keystrokes.

---

## 4. Conclusion & Concrete Code Proposals

### 4.1 Fix for `crates/quorum-daemon/src/server.rs:115`
In `crates/quorum-daemon/src/server.rs`:
```rust
// BEFORE:
    let req = chat::ChatRequest {
        prompt,
        search_mode: None,
        model_family: family,
        top_k: Some(12),
        access_tier: Some(0),
        history: Vec::new(),
        query_id: None,
    };

// AFTER:
    let req = chat::ChatRequest {
        prompt,
        search_mode: None,
        model_family: family,
        top_k: Some(12),
        access_tier: Some(0),
        history: Vec::new(),
        query_id: None,
        bypass_cache: false,
    };
```

### 4.2 UI Changes for Logo Retention in `crates/quorum-ui/assets/app.js`
In `crates/quorum-ui/assets/app.js:3344-3349`:
```javascript
// BEFORE:
      item.addEventListener('click', () => {
        closeSettingsView();
        if (welcomeBanner) welcomeBanner.classList.add('hidden');
        appendMessage('user', target.title);
      });

// AFTER:
      item.addEventListener('click', () => {
        closeSettingsView();
        if (welcomeBanner) {
          welcomeBanner.style.display = '';
          welcomeBanner.classList.remove('hidden');
        }
        appendMessage('user', target.title);
      });
```

### 4.3 Citation Relevance Score Display in `crates/quorum-ui/assets/app.js`
In `crates/quorum-ui/assets/app.js:720-725`:
```javascript
// BEFORE:
    badge.innerHTML = `📄 ${escapeHtml(fileName)} <span style="opacity:0.7;">:${cit.line_start}-${cit.line_end}</span>`;

// AFTER:
    const scorePct = Math.round(Number(cit.score || 0) * 100);
    badge.innerHTML = `📄 ${escapeHtml(fileName)} <span style="opacity:0.7;">:${cit.line_start}-${cit.line_end}</span> <span class="score-pill" style="opacity:0.85; font-size:10px; margin-left:4px; padding:1px 5px; border-radius:4px; background:rgba(66,133,244,0.2);">${scorePct}%</span>`;
```

---

## 5. Verification Method

### 5.1 Compilation Verification
Execute in powershell:
```powershell
$env:CARGO_INCREMENTAL = "0"
cargo check -p quorum-daemon
```
*Expected*: Zero compiler errors; `quorum-daemon` compiles cleanly.

### 5.2 Test Verification
Execute daemon endpoint tests:
```powershell
$env:CARGO_INCREMENTAL = "0"
cargo test -p quorum-daemon --test adversarial_endpoint_tests -- test_chat_stream_compat_get_endpoint
cargo test -p quorum-ui
```
*Expected*: All UI and daemon endpoint tests pass (`test result: ok`).

### 5.3 WebUI Interface Verification
1. Launch daemon:
   `target\debug\quorumrag.exe --host 127.0.0.1 --port 3031 --no-browser`
2. Open `http://127.0.0.1:3031`:
   - Inspect home screen: The `🌐 QuorumRAG` logo and header are visible; no sample prompt chips ("Explain scatter-gather retrieval", etc.) appear.
   - Type `waht is femi oyesanya edcation`: observe prompt auto-corrects to `What is Femi Oyesanya education` with `#spellcheck-status` reporting `✨ Auto-corrected`.
   - Send query: Assistant response completes with grounded `Sources` at the bottom, followed by the four action buttons: `📋 Copy`, `🔄 Regenerate`, `🔊 Read Aloud`, `👍`, `👎`.
   - Click `📋 Copy`: text is copied to clipboard with `✓ Copied!` confirmation.
   - Click `🔄 Regenerate`: query resubmits with cache bypass.
   - Click `🔊 Read Aloud`: audio starts speaking response text via Web Speech API; button changes to `⏹ Stop Reading`.
   - Click `👍`: thumbs up highlights active.

---
*Report complete and self-contained.*
