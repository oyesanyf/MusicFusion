# Challenge Assignment: Milestone 5 UI & Web Endpoints Challenger (`challenger_m5_1`)

## 1. Role & Identity
- **Role**: Milestone 5 UI & Web Endpoints Challenger
- **Archetype**: `teamwork_challenger`
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m5_1\`
- **Parent Conversation ID**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`

## 2. Mandatory Inputs to Read
1. `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`: Read verbatim user requirements.
2. `d:\harfile\QuorumRAG\PROJECT.md`: Read Milestone 5 scope.
3. `d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_1\handoff.md`: Worker report and test commands.
4. `crates/quorum-ui/tests/ui_tests.rs` and `tests/e2e/test_web_endpoints.rs`.

## 3. Adversarial Challenge Mandate
Empirically stress-test Web UI static serving and HTTP endpoints:
1. **Asset Stress & Malformed URI Probe**:
   - Probe non-existent routes, URI traversal attempts (`/../../etc/passwd`, `/%2e%2e/`), and verify fail-closed SPA fallback or 404 responses without crashes or directory traversal leaks.
2. **5-Theme CSS & ChatGPT Dimensions Empirical Assertion**:
   - Assert all 5 theme variables are present in `styles.css` and `index.html`.
   - Assert 260px sidebar and 768px reading column constraints in DOM markup.
3. **Endpoint Concurrency & SSE Stress**:
   - Test simultaneous requests to `/api/health`, `/api/models`, and `/api/chat` under concurrency.
   - Verify SSE formatting (`text/event-stream`), `data: ` line syntax, keep-alive intervals, and citation payload schema.
4. **Interactive LangExtract Visualizer Integrity**:
   - Verify `/api/langextract/visualize` returns valid standalone HTML with embedded styles, scrubber controls, and color-coded entity highlight spans.
5. **Execution Commands**:
   - Run `cargo test -p quorum-ui -- --test-threads=2`
   - Run `cargo test --manifest-path tests/e2e/Cargo.toml --test test_web_endpoints -- --test-threads=2`
   - If writing adversarial tests, place them in `tests/` or in your working directory.

## 4. Deliverable
Write `handoff.md` with:
- Structured verdict: **APPROVE** or **REQUEST_CHANGES**.
- Observation (empirical findings, stress test logs).
- Logic Chain.
- Caveats.
- Verification commands executed and outputs.
- Send message back to orchestrator with verdict.

## 2026-11-01T01:29:51Z
You are challenger_m5_1 (Milestone 5 UI & Web Endpoints Challenger).
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\challenger_m5_1\
Your parent orchestrator is: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e

MANDATORY FIRST STEPS:
1. Initialize your BRIEFING.md and progress.md in your working directory.
2. Read your assignment in d:\harfile\QuorumRAG\.agents\teamwork\challenger_m5_1\DISPATCH.md.
3. Read d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md.
4. Read d:\harfile\QuorumRAG\PROJECT.md.
5. Read d:\harfile\QuorumRAG\.agents\teamwork\worker_m5_1\handoff.md.

SCOPE OF CHALLENGE:
Empirically stress-test Web UI static serving and HTTP endpoints:
1. Asset Stress & Malformed URI Probe: test non-existent routes, URI traversal attempts (/../../, /%2e%2e/), verify SPA fallback or 404 responses without crashes or directory traversal leaks.
2. 5-Theme CSS & ChatGPT Dimensions Empirical Assertion: verify all 5 themes, 260px sidebar, 768px reading column.
3. Endpoint Concurrency & SSE Stress: test concurrent requests to /api/health, /api/models, /api/chat. Verify text/event-stream syntax, data: lines, keep-alive intervals, and citations.
4. LangExtract visualizer integrity: verify /api/langextract/visualize returns valid HTML with embedded styles, scrubber controls, and pastel spans.
5. Execution Commands:
   - cargo test -p quorum-ui -- --test-threads=2
   - cargo test --manifest-path tests/e2e/Cargo.toml --test test_web_endpoints -- --test-threads=2

DELIVERABLE:
Write handoff.md with structured verdict (APPROVE or REQUEST_CHANGES), observation, logic chain, caveats, and verification commands.
Message parent e13403c5-b0e6-48a4-a1ec-a1f096f9f53e with your verdict.

## 2026-11-01T01:44:06Z
**Context**: Milestone 5 UI & Web Endpoints Challenge
**Content**: Status check on your empirical stress test harness and handoff report.
**Action**: Please report your current step or deliver your handoff.md when ready.
