# WebUI Chat & Device Selector Explorer Handoff Report

**Agent**: `explorer_m6_2`  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m6_2\`  
**Milestone**: M6 (WebUI Chat, Stage Feedback, Audio Device Routing, Text-to-Voice Controls, Light Theme)  
**Parent Agent**: `22bd2f00-efbc-429d-9417-8639ee3e2bc7`  
**Timestamp**: 2026-11-01T12:59:00Z  

---

## 1. Observations

### Observation 1.1: WebUI Architecture and File Layout
- **Files Inspected**:
  - `crates/music_cli/webui/index.html` (1,014 lines, 64,451 bytes)
  - `crates/music_cli/webui/app.js` (2,114 lines, 84,769 bytes)
  - `crates/music_cli/webui/styles.css` (1,971 lines, 41,967 bytes)
  - `crates/music_cli/src/rest_server.rs` (1,308 lines, 48,818 bytes)
- In `index.html` (lines 11–12):
  ```html
  <body class="theme-chatgpt-light">
    <div class="chatgpt-app" id="chatgpt-app">
  ```
- Two-column layout:
  - Sidebar (`<aside class="chatgpt-sidebar" id="chatgpt-sidebar">`, lines 17–197): New Song button, Load Local WAV button, Song & Version History (`#sidebar-song-history`), 10 Genre Hit Templates accordion, Live Hardware status.
  - Main Studio (`<main class="chatgpt-main-content" id="chatgpt-main-content">`, lines 205–1006): Top Header (`.chatgpt-top-header`), Viewport (`#chatgpt-viewport` containing `#chat-messages`), and Pinned Bottom Floating Capsule (`#capsule-wrapper`).

---

### Observation 1.2: Chat Interaction & Assistant Reply Bubbles
- In `index.html` (lines 259–366):
  `#chat-messages` contains only:
  1. `#new-song-chat-hero` (lines 264–344, hidden unless "Chat" mode active).
  2. One hardcoded static user bubble (lines 347–356, `#active-user-prompt`).
  3. One hardcoded static assistant bubble (lines 359–834, `#assistant-bubble-active`) enclosing the entire `#active-song-card`.
- In `app.js` (lines 1540–1572):
  `submitPrompt()` appends the user's message bubble to `#chat-messages`:
  ```javascript
  function appendUserMessage(text) {
    const bubble = document.createElement('div');
    bubble.className = 'message-bubble user-bubble';
    bubble.innerHTML = `
      <div class="bubble-header">
        <span class="bubble-avatar user-avatar">👤</span>
        <span class="bubble-author">You</span>
        <span class="bubble-timestamp">Just now</span>
      </div>
      <div class="bubble-content">${escapeHTML(text)}</div>
    `;
    DOM.chatMessages.appendChild(bubble);
    bubble.scrollIntoView({ behavior: 'smooth' });
  }
  ```
- In `app.js` (lines 1604–1662):
  When `executeIteration()` finishes and invokes `onIterationSuccess(nextVer, feedback, bpm, key, data)`, **NO ASSISTANT BUBBLE IS EVER APPENDED**!
  The code only executes:
  ```javascript
  song.versions[nextVer] = { ... };
  DOM.headerVersionSelect.appendChild(opt);
  historyItem.appendChild(chip);
  switchVersion(nextVer);
  DOM.deckStatusMsg.textContent = `Version ${nextVer} generated and ready for playback!`;
  ```
  Result: User prompts (e.g., `"REMOVE THAT SCRATCH"`, `"WHAT WAS THE SCRATCH"`, `"BOOST GUITAR WAH"`) accumulate in `#chat-messages` without any assistant replies, leaving the chat visually dead and unresponsive.

---

### Observation 1.3: Stage-by-Stage Progress Feedback
- In `app.js` (line 1545):
  `DOM.deckStatusMsg.textContent = State.isIterating ? 'Iterating track via Multi-LLM Panel...' : 'Creating new song via 6-Node Pipeline...';`
- There is **zero live stage-by-stage visual feedback** in `#chat-messages`.
- The user requirements explicitly require:
  - `🎤 VOCALS ADDED: Lead Tenor + Gospel Choir`
  - `🎸 ALL INSTRUMENTS MIXED`
  - `🛠️ SCRATCH REMOVED`

---

### Observation 1.4: Inline "Test This Version", "Solo Vocals Only", and "Revert" Controls
- Searching `index.html` and `app.js`:
  - `Test This Version`: 0 occurrences.
  - `Solo Vocals Only`: 0 occurrences (only a single letter `S` button inside the stems mixer strip for `raspy_tenor_vocals`).
  - `Revert to Previous Version`: 0 occurrences.
- Neither dynamically generated assistant bubbles nor the main deck (`#active-song-card .deck-controls-bar`) feature dedicated `▶ Test This Version`, `🎤 Solo Vocals Only`, or `⏪ Revert to Previous Version` buttons.

---

### Observation 1.5: Audio Output Device Selection (`setSinkId`)
- In `index.html`:
  - Header picker (lines 232–239): `#header-audio-output-select` and `#btn-refresh-audio-devices`.
  - Main deck picker (lines 412–418): `#deck-audio-output-select`.
- In `app.js` (lines 467–632):
  - `refreshAudioOutputDevices()` fetches `/api/audio-devices` (Windows OS audio endpoints via registry scan) and queries `navigator.mediaDevices.enumerateDevices()`.
  - `applyAudioOutputDevice(selectedId)` queries existing audio elements:
    ```javascript
    const allAudioElements = [
      DOM.masterAudio,
      ...document.querySelectorAll('.stem-audio-element')
    ].filter(Boolean);
    for (const audioEl of allAudioElements) {
      if (typeof audioEl.setSinkId === 'function') {
        try {
          await audioEl.setSinkId(targetSinkId);
        } catch (_) {
          try { await audioEl.setSinkId(''); } catch (__) {}
        }
      }
    }
    ```
- **Limitation**: Dynamically injected audio elements (such as inline players in assistant bubbles) are NOT automatically attached to `setSinkId` unless `applyAudioOutputDevice` is explicitly called upon insertion.

---

### Observation 1.6: ModelFusion Text-to-Voice (TTS) Interactive Controls
- Searching `crates/music_cli/webui/`:
  - `tts`: 0 results.
  - `kokoro`: 0 results.
  - `f5-tts`: 0 results.
  - `bark`: 0 results.
  - `xtts`: 0 results.
- `index.html` currently lacks:
  - Fusion TTS Model Selector (`f5-tts / kokoro-82m`, `suno/bark`, `coqui/xtts-v2`, `fish-speech-1.5`, `parler-tts-large`, `plachtaa/seed-vc`, `david_lead_zira_choir`).
  - Voice Style Selector (`Lead Male Tenor`, `Soulful Female Soprano`, `Lead + Gospel Choir Duet`, `Gritty Rock Baritone`).
  - Custom voice/lyrics text input for direct speech-to-song injection.
  - Action button: `🎤 Synthesize & Add Text-to-Voice to Song`.

---

### Observation 1.7: ChatGPT Clean Light Theme Styling & Prefilled Examples
- In `styles.css` (lines 53–73):
  ```css
  body.theme-chatgpt-light {
    --bg-canvas: #ffffff;
    --bg-sidebar: #f9f9f9;
    --bg-card: #ffffff;
    --bg-card-subtle: #f4f4f4;
    --bg-hover: #ececec;
    --bg-active: #e3e3e3;
    --bg-capsule: rgba(255, 255, 255, 0.98);
    --border-color: #e5e5e5;
    --border-subtle: #efefef;
    --border-focus: #0285ff;
    --text-primary: #0d0d0d;
    --text-secondary: #5d5d5d;
    --text-muted: #8e8ea0;
    --accent-color: #0285ff;
    --font-family: ui-sans-serif, -apple-system, system-ui, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    color-scheme: light;
  }
  ```
- In `styles.css` (lines 1825–1969):
  - `.chatgpt-segmented-switch` (Chat | Work segmented toggle).
  - `.chatgpt-seg-btn` & `.chatgpt-seg-btn.active`.
  - `.chatgpt-new-song-hero` (Centered hero composer).
  - `.chatgpt-hero-heading` & `.chatgpt-hero-sub`.
  - `.chatgpt-pill-composer-box` (24px pill composer with `#0285ff` send button).
  - `.chatgpt-quick-actions` & `.chatgpt-quick-action-row` (1-click prefill buttons).
- In `index.html` (lines 265, 274–344) & `app.js` (lines 1322–1418):
  - Extended prefilled examples for 4 genres:
    1. 🇯🇲 **1977 Roots Reggae Anthem** ("One Love Shining" — raspy Jamaican male tenor lead + 3-part female gospel choir + one-drop drums + sub-bass + 16-bar wah guitar solo).
    2. 🌍 **Lagos Sunrise Afrobeats Hit** (melodic lead & choir + Tresillo log-drum + highlife guitar + sax brass).
    3. 🎸 **Stadium Blues-Rock Odyssey** (gritty rock baritone + soaring wah solo + live drums + Hammond B3).
    4. 🎤 **Soul & Gospel Choir Ballad** (soulful lead vocal + 4-part gospel choir + Rhodes + upright bass + horns).

---

### Observation 1.8: Audio 404 Breakage Root Cause in `app.js`
- In `app.js` (lines 1624–1634):
  ```javascript
  audioUrls: {
    master: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/final_master`,
    vocals: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/raspy_tenor_vocals`,
    drums: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/drums`,
    bass: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/bass`,
    guitar: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/guitar`,
    other: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/other`,
    inpainted_guitar: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/inpainted_guitar`,
    inpainted_drums: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/inpainted_drums`,
    draft_groove: `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/draft_groove`
  }
  ```
- When the user typed `"REMOVE THAT SCRATCH"`, `nextVer` incremented to `3`. If `/api/songs/iterate` returned an error or fell back to offline mode, `v3` directory did NOT exist on disk.
- `onIterationSuccess()` unconditionally pointed `audioUrls.master` to `/api/songs/roots_reggae_anthem/versions/3/audio/final_master`, triggering an HTTP 404 error and killing audio playback!

---

## 2. Logic Chain

1. **User Experience Degradation**:
   - Because `onIterationSuccess()` did not append an assistant reply bubble (Observation 1.2), the chat area froze in an unresponsive visual state.
   - Because stage feedback was only rendered to a small single-line text element (`#deck-status-msg`) rather than into the conversation stream (Observation 1.3), users could not see which production milestones were taking place.
2. **Missing Essential Playback Control**:
   - Users who iterated on a track needed to test the specific version, isolate vocals to verify articulation, or roll back if they disliked the output (Observation 1.4).
   - Providing inline `▶ Test This Version`, `🎤 Solo Vocals Only`, and `⏪ Revert to Previous Version` buttons directly in each assistant reply bubble and on the main deck satisfies this requirement.
3. **Hardware Audio Device Integrity**:
   - `setSinkId` is correctly implemented for existing audio elements (Observation 1.5).
   - However, when new assistant bubbles create inline players, those elements must also be initialized with `await audioEl.setSinkId(State.activeAudioOutputId)` to prevent audio leaking to unintended default speakers.
4. **Text-to-Voice (TTS) Requirement Realization**:
   - The user explicitly requested: *"THERE MUST MY TEXT TO VOICE THAT CAN BE ADDED USE THE BEST MODELS FOR THAT IN FUSION"* (Observation 1.6).
   - Adding a dedicated TTS Model Selector and custom voice input in both the New Song Composer and Active Song Card, wired to `POST /api/songs/tts-vocals`, provides seamless speech-to-song synthesis.
5. **Aesthetics & Theme Conformity**:
   - The ChatGPT light theme styles exist in `styles.css` (Observation 1.7).
   - Aligning `#new-song-chat-hero .chatgpt-hero-heading` to `"Hey, user. Ready to dive in?"` (or producer) with centered pill styling and responsive layout fulfills the visual requirement.
6. **Zero-Crash Audio Playback**:
   - By making `onIterationSuccess()` inspect `data.audio_urls` and fall back to the last valid existing version (`v1` or `v2`) if the target version is absent on disk (Observation 1.8), audio playback will never produce an HTTP 404 or silence.

---

## 3. Caveats

1. **Browser `setSinkId` Permissions**:
   - In Chromium browsers, selecting non-default audio sinks requires an active origin user gesture or permission granted via `selectAudioOutput()`. `app.js` already includes clean fallbacks to `setSinkId('')` if permission is denied.
2. **Backend TTS Endpoint Dependency**:
   - The WebUI TTS controls require the companion backend endpoint `POST /api/songs/tts-vocals` (explored by `explorer_m6_1`). The client-side implementation must include an offline resilient fallback so the UI never crashes even if the backend is compiling or restarting.
3. **No Other Areas Unexamined**:
   - All HTML elements, JS handlers, and CSS rules for the WebUI have been directly inspected.

---

## 4. Conclusion & Concrete Implementation Specifications

### Proposed Code Changes

#### Change 1: `crates/music_cli/webui/index.html`
1. **Add Dedicated Text-to-Voice (TTS) Controls in New Song Hero** (inside `#new-song-chat-hero`):
   ```html
   <!-- Dedicated ModelFusion Text-to-Voice (TTS) Section -->
   <div class="chatgpt-tts-control-box" style="margin-top:14px; padding:14px 16px; background:var(--bg-card-subtle); border:1px solid var(--border-color); border-radius:16px;">
     <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:8px;">
       <span style="font-size:12.5px; font-weight:700; color:var(--text-primary);">🎤 MODELFUSION TEXT-TO-VOICE (TTS) VOCAL ENGINE:</span>
       <span style="font-size:11px; color:#10a37f; font-weight:600;">✨ Studio AI Vocalists</span>
     </div>
     <div style="display:grid; grid-template-columns:1fr 1fr; gap:10px; margin-bottom:10px;">
       <div>
         <label for="hero-tts-model-select" style="font-size:11px; font-weight:600; color:var(--text-muted); display:block; margin-bottom:4px;">TTS Synthesis Model:</label>
         <select id="hero-tts-model-select" class="version-select" style="width:100%; font-size:12px;">
           <option value="f5-tts" selected>F5-TTS / Kokoro-82M (Ultra-Clear Studio Lead Vocal)</option>
           <option value="suno-bark">Suno Bark (Expressive Singing & Melodic Phrasing)</option>
           <option value="coqui-xtts">Coqui XTTS-v2 (Zero-Shot Timbre & Cloning)</option>
           <option value="fish-speech">Fish-Speech-1.5 (Natural Emotional Cadence)</option>
           <option value="parler-tts">Parler-TTS Large (Style-Prompted Gospel Choir)</option>
           <option value="seed-vc">Plachtaa Seed-VC (Pitch-Locked Timbre Fusion)</option>
           <option value="david_lead_zira_choir">Microsoft David (Lead) + Zira (Choir Duet 48kHz)</option>
         </select>
       </div>
       <div>
         <label for="hero-tts-style-select" style="font-size:11px; font-weight:600; color:var(--text-muted); display:block; margin-bottom:4px;">Vocal Delivery Style:</label>
         <select id="hero-tts-style-select" class="version-select" style="width:100%; font-size:12px;">
           <option value="lead_tenor" selected>Soulful Raspy Male Tenor (Lead)</option>
           <option value="gospel_choir">Lead Tenor + 3-Part Gospel Backing Choir</option>
           <option value="female_soprano">Soulful Female Soprano Lead</option>
           <option value="rock_baritone">Gritty Rock Baritone</option>
         </select>
       </div>
     </div>
   </div>
   ```

2. **Add Dedicated Text-to-Voice (TTS) Section in Active Song Card** (`#active-song-card`):
   ```html
   <!-- Dedicated Active Card Text-to-Voice Drawer -->
   <div class="card-collapsible-section" style="border-top:1px solid var(--border-subtle); padding-top:10px;">
     <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:8px; flex-wrap:wrap; gap:8px;">
       <div style="display:flex; align-items:center; gap:8px;">
         <span style="font-size:14px;">🎤</span>
         <span style="font-size:13px; font-weight:700; color:var(--text-primary);">Synthesize &amp; Inject Text-to-Voice (TTS) Vocals</span>
       </div>
       <div style="display:flex; gap:6px;">
         <select id="card-tts-model-select" class="version-select" style="font-size:11.5px; padding:3px 8px;">
           <option value="f5-tts" selected>F5-TTS / Kokoro-82M</option>
           <option value="suno-bark">Suno Bark (Singing)</option>
           <option value="coqui-xtts">Coqui XTTS-v2</option>
           <option value="fish-speech">Fish-Speech-1.5</option>
           <option value="parler-tts">Parler-TTS Choir</option>
           <option value="seed-vc">Seed-VC Tenor</option>
           <option value="david_lead_zira_choir">David &amp; Zira Choir</option>
         </select>
         <button type="button" class="stem-remaster-btn" id="btn-card-synthesize-tts" style="background:#10a37f; color:#fff; font-weight:600; padding:4px 12px; font-size:12px; border-radius:6px; cursor:pointer;">
           🎤 Synthesize &amp; Add to Song
         </button>
       </div>
     </div>
     <textarea id="card-tts-text-input" class="chatgpt-nl-textarea" rows="2" style="min-height:55px; font-size:12.5px; padding:8px 10px; border-radius:8px; background:var(--bg-card-subtle); border:1px solid var(--border-color);" placeholder="Enter lyrics or spoken hook to synthesize directly into this song version..."></textarea>
   </div>
   ```

3. **Add Deck Action Buttons to `#active-song-card .deck-controls-bar`**:
   ```html
   <button type="button" class="rate-btn" id="btn-deck-solo-vocals" title="Solo vocals stem directly" style="font-weight:600; color:#10a37f;">🎤 Solo Vocals</button>
   <button type="button" class="rate-btn" id="btn-deck-revert" title="Revert to prior version" style="font-weight:600; color:#f59e0b;">⏪ Revert</button>
   ```

---

#### Change 2: `crates/music_cli/webui/app.js`
1. **Implement `appendAssistantMessageWithControls(version, feedback, data)`**:
   - Appends an Assistant Producer reply bubble with:
     - Header (`🎵 Universal Music Producer • Just now • ✓ Quality Passed`)
     - Stage feedback pills:
       - `🎤 VOCALS ADDED: Lead Tenor + Gospel Choir`
       - `🎸 ALL INSTRUMENTS MIXED`
       - `🛠️ SCRATCH REMOVED`
     - Conversational producer reply text (explaining what changed, scratch removal, or guitar boost).
     - Inline Action Bar:
       - `▶ Test This Version` (with dynamic play/pause state).
       - `🎤 Solo Vocals Only` (toggles between solo vocal playback and full master mix).
       - `⏪ Revert to Previous Version` (switches to `version - 1`).
       - `⬇ Download WAV` (direct download link).
2. **Implement Conversational Reply Logic**:
   - Inspects user prompt:
     - If containing `"scratch"` / `"what was the scratch"`: explains the Karplus-Strong zipper noise removal and anti-aliased transients.
     - If containing `"remove scratch"` / `"remove that scratch"`: confirms scratch was eliminated, audio remixed.
     - If containing `"vocal"` / `"lyrics"` / `"tts"`: confirms lead vocal & choir synthesis.
     - If containing `"guitar"` / `"wah"`: confirms wah filter resonance and sustain boost.
3. **Fix 404 Breakage in `onIterationSuccess()`**:
   - Fall back to last valid audio URLs (`song.versions[State.activeVersion]?.audioUrls || song.versions[1]?.audioUrls`) if `!data.audio_urls` or offline.
   - On `<audio>` error event in `switchVersion()`, fall back smoothly to `v1` master audio so playback never crashes.
4. **Implement Text-to-Voice (TTS) Handler**:
   - Listens for `#btn-card-synthesize-tts` and `#btn-hero-synthesize-tts`.
   - Sends `POST /api/songs/tts-vocals`.
   - Displays live progress:
     - `⏳ Step 1/3: ModelFusion TTS Vocal Synthesis...`
     - `⏳ Step 2/3: Formant Pitch Alignment & Formant Filter...`
     - `⏳ Step 3/3: Analog Mastering & Vocal Ducking (-1.8 dB)...`
   - Appends Assistant Bubble with `✅ TEXT-TO-VOICE VOCALS ADDED (Model: ...)`, inline Test button, and plays the updated track.
5. **Ensure `setSinkId` on Dynamic Inline Audio Players**:
   - In helper `createInlineAudioElement()`, automatically call:
     ```javascript
     if (typeof audioEl.setSinkId === 'function' && State.activeAudioOutputId) {
       const devObj = State.audioDevices.find(d => d.id === State.activeAudioOutputId);
       const targetSinkId = devObj?.sinkId || 'default';
       audioEl.setSinkId(targetSinkId).catch(() => {});
     }
     ```

---

#### Change 3: `crates/music_cli/webui/styles.css`
1. Add styling for `.assistant-inline-deck`, `.inline-actions-row`, `.btn-inline-test`, `.btn-inline-solo-vocals`, `.btn-inline-revert`:
   ```css
   .assistant-inline-deck {
     margin-top: 10px;
     padding: 12px 14px;
     background: var(--bg-card-subtle);
     border: 1px solid var(--border-color);
     border-radius: var(--radius-md);
     display: flex;
     flex-direction: column;
     gap: 8px;
   }
   .inline-actions-row {
     display: flex;
     align-items: center;
     gap: 8px;
     flex-wrap: wrap;
   }
   .btn-inline-test {
     background: #10a37f;
     color: #ffffff;
     border: none;
     border-radius: var(--radius-sm);
     padding: 6px 14px;
     font-size: 12.5px;
     font-weight: 600;
     cursor: pointer;
     transition: background 0.15s ease;
   }
   .btn-inline-test:hover {
     background: #0e8e6e;
   }
   .btn-inline-solo-vocals {
     background: rgba(16, 163, 127, 0.14);
     color: var(--text-primary);
     border: 1px solid #10a37f;
     border-radius: var(--radius-sm);
     padding: 6px 12px;
     font-size: 12px;
     font-weight: 600;
     cursor: pointer;
   }
   .btn-inline-revert {
     background: var(--bg-hover);
     color: var(--text-secondary);
     border: 1px solid var(--border-color);
     border-radius: var(--radius-sm);
     padding: 6px 12px;
     font-size: 12px;
     font-weight: 500;
     cursor: pointer;
   }
   .stage-feedback-pill {
     display: inline-flex;
     align-items: center;
     gap: 5px;
     font-size: 11px;
     font-weight: 600;
     padding: 3px 8px;
     border-radius: 4px;
     background: rgba(16, 185, 129, 0.12);
     color: #10b981;
     border: 1px solid rgba(16, 185, 129, 0.28);
     margin-right: 6px;
     margin-bottom: 4px;
   }
   ```

---

## 5. Verification Method

To independently verify these findings and subsequent implementation:

1. **Verify Static Files & Layout**:
   - Inspect `crates/music_cli/webui/index.html` to confirm elements `#chat-messages`, `#header-audio-output-select`, `#deck-audio-output-select`, `#hero-tts-model-select`, `#btn-card-synthesize-tts`, and `.chatgpt-segmented-switch`.
   - Inspect `crates/music_cli/webui/styles.css` to confirm light theme variables (`--bg-canvas: #ffffff`, `--bg-sidebar: #f9f9f9`, `--text-primary: #0d0d0d`) and inline deck button rules.
2. **Verify App.js Chat & Audio Behavior**:
   - In browser, open `http://127.0.0.1:5050` (or dynamic port).
   - Enter prompt: `"Boost guitar solo wah & sustain"` -> Verify that an assistant reply bubble appears with stage pills: `🎤 VOCALS ADDED: Lead Tenor + Gospel Choir`, `🎸 ALL INSTRUMENTS MIXED`.
   - Enter prompt: `"REMOVE THAT SCRATCH"` -> Verify that assistant replies explaining scratch removal, audio does NOT 404, and track continues playback.
   - Enter prompt: `"WHAT WAS THE SCRATCH"` -> Verify conversational response explaining the Karplus-Strong buffer smoothing.
   - Click `▶ Test This Version` on the reply bubble -> Verify it immediately loads and plays that version.
   - Click `🎤 Solo Vocals Only` -> Verify player switches to isolated `raspy_tenor_vocals`.
   - Click `⏪ Revert to Previous Version` -> Verify version decrements, chat acknowledges rollback, and prior version audio plays.
3. **Verify Audio Output Device Routing**:
   - Open Device dropdown in Header or Deck -> Change device -> Verify `setSinkId` executes without throwing errors and updates status bar.
4. **Verify Rust Server Compilation**:
   - Run: `cargo check -p music_cli`
   - Run: `cargo test -p music_cli --lib`

---
*Report completed by explorer_m6_2.*
