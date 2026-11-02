## 2026-11-02T15:27:28Z

You are Worker M12 1 (teamwork_worker).
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\worker_m12_1\
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md, d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md, d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_2\handoff.md, and d:\harfile\QuorumRAG\.agents\teamwork\worker_m12_1\DISPATCH.md immediately.

Your exclusively owned files:
- `crates/quorum-daemon/src/server.rs`
- `crates/quorum-ui/assets/index.html`
- `crates/quorum-ui/assets/styles.css`
- `crates/quorum-ui/assets/app.js`

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Implement:
1. Fix `crates/quorum-daemon/src/server.rs:115`: Add `bypass_cache: false` in `ChatRequest` struct initializer.
2. Ensure `#welcome-banner` (`🌐 QuorumRAG`) remains permanently visible across all views in `app.js` and `styles.css`. Remove prompt chips completely (`display: none !important`).
3. Wire dynamic debounced chat box spellchecking in `app.js` (`POST /api/chat/spellcheck`), non-destructively preserving cursor position.
4. Add the 4 browser-style response action buttons (Copy, Regenerate, Read Aloud via Web Speech API, Feedback Thumbs) below citations in assistant messages.
5. In `appendCitation` in `app.js`: Remove all line numbers (`:start-end`, `Lines`), display document link and visible accuracy score percentage (`${scorePct}%`), and deduplicate citations by filename.
6. Verify builds and tests:
   `$env:CARGO_INCREMENTAL = "0"; cargo check -p quorum-daemon`
   `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-ui`
   `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-daemon --test adversarial_endpoint_tests`
7. Stage and commit verified changes (`git add -A; git commit -m "feat(ui,daemon): add dynamic spellcheck, action buttons, logo retention, and fix server ChatRequest"`).

Write your completion report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m12_1\handoff.md` and notify parent orchestrator via send_message.

## 2026-11-02T15:34:27Z

**Context**: Worker M12 1 status check
**Content**: Please report your progress on the `server.rs:115` fix, clean UI & permanent logo retention, real-time spellcheck, action buttons, and `appendCitation` zero-line-number citation badge updates.
**Action**: Update progress.md and send status.

