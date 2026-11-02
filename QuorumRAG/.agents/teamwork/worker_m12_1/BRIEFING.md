# BRIEFING — 2026-11-02T15:27:50Z

## Mission
Implement QuorumRAG UI improvements (permanent logo banner, prompt chips removal, dynamic spellcheck, assistant action buttons, citation cleanup with accuracy %) and fix server ChatRequest bypass_cache.

## 🔒 My Identity
- Archetype: teamwork_worker
- Roles: implementer, qa
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork\worker_m12_1\
- Original parent: 6e43b521-b448-4762-842a-0bc8d66e2854
- Milestone: M12

## 🔒 Key Constraints
- Owned files: crates/quorum-daemon/src/server.rs, crates/quorum-ui/assets/index.html, crates/quorum-ui/assets/styles.css, crates/quorum-ui/assets/app.js
- Genuine implementations only, no hardcoded results or mocks
- Zero-wait incremental commits upon task completion

## Current Parent
- Conversation ID: 6e43b521-b448-4762-842a-0bc8d66e2854
- Updated: not yet

## Task Summary
- **What to build**: 
  1. Fix `crates/quorum-daemon/src/server.rs:115`: Add `bypass_cache: false` in `ChatRequest` struct initializer.
  2. Ensure `#welcome-banner` (`🌐 QuorumRAG`) remains permanently visible across all views in `app.js` and `styles.css`. Remove prompt chips completely (`display: none !important`).
  3. Wire dynamic debounced chat box spellchecking in `app.js` (`POST /api/chat/spellcheck`), non-destructively preserving cursor position.
  4. Add the 4 browser-style response action buttons (Copy, Regenerate, Read Aloud via Web Speech API, Feedback Thumbs) below citations in assistant messages.
  5. In `appendCitation` in `app.js`: Remove all line numbers (`:start-end`, `Lines`), display document link and visible accuracy score percentage (`${scorePct}%`), and deduplicate citations by filename.
  6. Verify builds and tests:
     `$env:CARGO_INCREMENTAL = "0"; cargo check -p quorum-daemon`
     `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-ui`
     `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-daemon --test adversarial_endpoint_tests`
  7. Stage and commit verified changes.
- **Success criteria**: All cargo check and tests pass, genuine logic, clean commit.
- **Interface contracts**: SCOPE.md, explorer handoff.md
- **Code layout**: crates/quorum-daemon, crates/quorum-ui

## Key Decisions Made
- Initial setup completed.

## Artifact Index
- `d:\harfile\QuorumRAG\.agents\teamwork\worker_m12_1\DISPATCH.md` — Dispatch assignment
- `d:\harfile\QuorumRAG\.agents\teamwork\worker_m12_1\progress.md` — Progress tracker

## Change Tracker
- **Files modified**: None yet
- **Build status**: Not run yet
- **Pending issues**: None

## Quality Status
- **Build/test result**: Not run yet
- **Lint status**: 0
- **Tests added/modified**: None yet

## Loaded Skills
- None
