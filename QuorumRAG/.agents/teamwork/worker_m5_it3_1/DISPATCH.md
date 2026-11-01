# Dispatch: worker_m5_it3_1 (Milestone 5 Iteration 3 Remediation Implementation Worker)

## Mission
Implement and empirically verify the 3 coordinated fixes designed by the Explorers to achieve 100% test passing, clean live IDE execution, and a flawless forensic audit:
1. Axum fallback routing in `crates/quorum-daemon/src/server.rs` (from `explorer_m5_it3_1/handoff.md`).
2. Relevance grading calibration in `crates/quorum-coordinator/src/agentic/grade.rs` (from `explorer_m5_it3_2/handoff.md`).
3. Isolated GUID temp harness and safe file deletion in `scripts/verify_live_ide_execution.ps1` (from `explorer_m5_it3_3/handoff.md`).

## Mandatory Inputs & References
- Original User Request: `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\ORIGINAL_REQUEST.md`
- Global Scope & Contracts: `d:\harfile\QuorumRAG\PROJECT.md`
- Dead Ends Log: `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\DEAD_ENDS.md`
- Auditor Full Evidence: `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m5_remediation_1\handoff.md`
- Reviewer 2 Full Report: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m5_remediation_2\handoff.md`
- Explorer 1 Report (Server Fallback): `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_1\handoff.md`
- Explorer 2 Report (Grader Calibration): `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_2\handoff.md`
- Explorer 3 Report (Script Harness): `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_3\handoff.md`

## File Ownership
You exclusively own and may edit:
- `crates/quorum-daemon/src/server.rs`
- `crates/quorum-coordinator/src/agentic/grade.rs`
- `scripts/verify_live_ide_execution.ps1`
- `DEAD_ENDS.md`

## Implementation Checklist
### Task 1: Axum Fallback Routing in `crates/quorum-daemon/src/server.rs`
- Wire `.fallback(quorum_ui::static_handler)` into `build_router` immediately after `/api/shutdown` route and before `.layer(CorsLayer::permissive())` and `.with_state(state)`.
- Verify: `cargo test -p quorum-daemon --test adversarial_web_routes_challenge -- --test-threads=2` (All 5 tests must pass).

### Task 2: Calibrated Relevance Grading in `crates/quorum-coordinator/src/agentic/grade.rs`
- Implement `GradeRelevanceNode::has_content_relevance(prompt, chunks)` and dual-regime threshold gating per `explorer_m5_it3_2/handoff.md`.
- Include the 5 unit tests in `mod tests` inside `grade.rs`.
- Verify:
  - `cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- challenge_agentic_loop_terminates_strictly_at_max_hops` (PASS)
  - `cargo test -p quorum-coordinator --test agentic_tests -- test_agentic_loop_convergence_max_hops` (PASS)
  - `cargo test -p quorum-coordinator -- --test-threads=2` (All 27 tests pass)

### Task 3: Isolated GUID Temp Directory & Pure Cargo Harness in `scripts/verify_live_ide_execution.ps1`
- Initialize `$verifyHarnessDir = $null` before `try`.
- In Step 5, allocate unique GUID directory:
  `$verifyHarnessDir = Join-Path $env:TEMP ("quorumrag_codegen_verify_" + [System.Guid]::NewGuid().ToString("N"))`
- Use `Remove-Item -Path $verifyHarnessDir -Recurse -Force -ErrorAction SilentlyContinue` pre-creation and in `finally`.
- In `$harnessCargo`, define `[dependencies]` as empty (zero dependencies) because `pub struct RrfCombiner;` is pure standard library Rust.
- Verify:
  - `powershell -ExecutionPolicy Bypass -File scripts/verify_live_ide_execution.ps1`
  - All 5 steps and all 6 gates must pass with un-doctored output.
  - Graceful shutdown `/api/shutdown` must exit cleanly with zero orphaned `.lock` files.

### Task 4: Workspace Verification & Git Commit
- `cargo clippy -p quorum-daemon -p quorum-coordinator -p quorum-ui -p quorum-storage --all-targets -- -D warnings` (0 warnings).
- `cargo test --manifest-path tests/e2e/Cargo.toml --test test_live_ide_execution -- --test-threads=2` (2 passed).
- `cargo test --manifest-path tests/e2e/Cargo.toml --test test_web_endpoints -- --test-threads=2` (10 passed).
- Update root `DEAD_ENDS.md` with Iteration 2 dead end row.
- Commit all changes with a descriptive message following the automatic git commit & push rule.

## Mandatory Rules
- NEVER create facade or dummy implementations.
- Zero mock protocol: Production code only.
- Universal zero-touch PATH protocol: Never mutate user/system PATH or registry environment.
- Use `jobs = 2` and `-- --test-threads=2` for test execution.
- Deliver genuine un-doctored terminal outputs in your handoff report.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
