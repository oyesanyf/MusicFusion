# Dispatch: explorer_m5_it3_1 (Milestone 5 Iteration 3 Daemon Fallback Routes Explorer)

## Mission
Investigate and design the exact fix for Axum 0.7 fallback routing in `crates/quorum-daemon/src/server.rs` to resolve the finding from `challenger_m5_remediation_1`.

## Mandatory Reading
- Original User Request: `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\ORIGINAL_REQUEST.md`
- Global Architecture & Scope: `d:\harfile\QuorumRAG\PROJECT.md`
- Dead Ends Log: `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\DEAD_ENDS.md`
- Auditor Full Evidence Report: `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m5_remediation_1\handoff.md`
- Reviewer 2 Full Report: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m5_remediation_2\handoff.md`
- Challenger 1 Full Report: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m5_remediation_1\handoff.md`
- Reproduction test: `crates/quorum-daemon/tests/adversarial_web_routes_challenge.rs`

## Problem Context
`challenger_m5_remediation_1` proved that `crates/quorum-daemon/src/server.rs::build_router` mounts `/assets/*path` but lacks `.fallback(quorum_ui::static_handler)`. As a result, direct requests to `/styles.css`, `/app.js`, `/langextract.html`, `/favicon.svg`, or virtual SPA routes (`/chat/123`) return HTTP 404 instead of 200 OK.
Running `cargo test -p quorum-daemon --test adversarial_web_routes_challenge -- --test-threads=2` fails 5/5 tests.

## Task
1. Inspect `crates/quorum-daemon/src/server.rs` and `crates/quorum-ui/src/assets.rs` (especially `static_handler`).
2. Design the exact code change for `crates/quorum-daemon/src/server.rs` so that `.fallback(quorum_ui::static_handler)` is wired into the Axum router, ensuring all 5 tests in `adversarial_web_routes_challenge.rs` pass.
3. Verify that Axum 0.7 fallback semantics work seamlessly with API routes (`/api/...`) and existing `/assets/*path` routes without conflicts.
4. Output your analysis and complete fix recommendations into `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_1\handoff.md`.

## 2026-11-01T03:21:42Z
Your working directory is `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_1\`.
Read your task dispatch at `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_1\DISPATCH.md`.
Read all mandatory inputs including:
- `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\DEAD_ENDS.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m5_remediation_1\handoff.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m5_remediation_2\handoff.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m5_remediation_1\handoff.md`

Your objective: Investigate and design the exact fix for Axum 0.7 fallback routing in `crates/quorum-daemon/src/server.rs` to wire `.fallback(quorum_ui::static_handler)` without breaking API routes, making all 5 tests in `crates/quorum-daemon/tests/adversarial_web_routes_challenge.rs` pass cleanly.
Do NOT modify source files. Produce your investigation report at `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_1\handoff.md`.
When done, notify me via `send_message`.
