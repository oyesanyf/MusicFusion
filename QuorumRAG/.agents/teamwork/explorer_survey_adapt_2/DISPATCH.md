# Task Assignment: Survey Explorer 2 (WebUI Chat Box, Dynamic Spellcheck, Clean UI, Action Buttons, server.rs Fix)

## Working Directory
d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_2\

## Mission & Scope
Map and investigate how to implement Requirements 3 & 5 (partial) from ORIGINAL_REQUEST.md:
3. WebUI Chat Box & Clean Interface:
   - Dynamic real-time spellchecking in the chat box using LLM.
   - Remove the sample prompt chips on the home screen ("Explain scatter-gather retrieval", "Show RrfCombiner & write unit test", etc.), but KEEP the top page 🌐 QuorumRAG logo visible.
   - Add browser-style response action buttons at the end of each assistant response: Copy, Regenerate, Read Aloud, and feedback thumbs.
5. Build and Server Health (partial):
   - In `crates/quorum-daemon/src/server.rs:115`, ensure `chat::ChatRequest` constructs with `bypass_cache: false` in tests.

Investigate:
- The Web UI implementation in `crates/quorum-ui/src/assets/` (e.g. `index.html`, `app.js`, `style.css` or embedded HTML/JS files) and `crates/quorum-ui/src/routes/`.
- Where the sample prompt chips are rendered on the home screen ("Explain scatter-gather retrieval", etc.) and how to cleanly remove them while keeping the top page 🌐 QuorumRAG logo and header fully visible and visually balanced.
- How to implement dynamic real-time spellchecking in the chat box using LLM:
  - Is there a spellcheck endpoint (`/api/spellcheck` or similar) in `quorum-ui` / `quorum-coordinator`, or can an endpoint leverage local LLM / Ollama / fast model to return corrected text / suggestions as the user types (with debouncing e.g. 400-500ms)?
  - How the frontend chat box displays spelling suggestions or auto-highlights/corrects.
- How to implement browser-style response action buttons at the end of each assistant response:
  - Copy (copies response markdown/text to clipboard with toast/check feedback)
  - Regenerate (re-submits the last user prompt)
  - Read Aloud (Web Speech API `speechSynthesis` or TTS endpoint to speak response aloud with pause/resume/stop)
  - Feedback thumbs (thumbs up / thumbs down button toggle with visual state and feedback event)
- In `crates/quorum-daemon/src/server.rs:115`: check what `chat::ChatRequest` construction looks like, why `bypass_cache: false` is required in tests, and how it affects existing tests.
- Produce a detailed technical handoff report in `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_2\handoff.md`.
