# Task Assignment — Worker M6-1 (Milestone 6 Phase 1: Full E2E & Live IDE Acceptance)

- **Role**: teamwork_worker
- **Milestone**: Milestone 6 Phase 1 (Final Acceptance: 100% E2E Test Suite & Live IDE Gate Execution)
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_1\`
- **Parent Conversation ID**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`

## Scope
Execute and verify the full acceptance test matrix across the entire QuorumRAG workspace:

1. **Full 4-Tier Opaque-Box E2E Test Suite**:
   Run the canonical unified E2E test runner:
   ```powershell
   powershell -ExecutionPolicy Bypass -File "d:\harfile\QuorumRAG\tests\e2e\run_all_e2e_tests.ps1"
   ```
   Verify 100% pass rate across all 67 tests (Tiers 1–4):
   - `test_storage_isolation` (10 tests)
   - `test_gemma_contract` (14 tests)
   - `test_ingestion_dedup` (10 tests)
   - `test_federated_mesh` (10 tests)
   - `test_web_endpoints` (10 tests)
   - `test_cross_feature_combinations` (7 tests)
   - `test_real_world_scenarios` (6 tests)

2. **Feature 42 Live IDE Execution Gate**:
   Run the live IDE execution and production code generation verification script:
   ```powershell
   powershell -ExecutionPolicy Bypass -File "d:\harfile\QuorumRAG\scripts\verify_live_ide_execution.ps1"
   ```
   Verify all 6 verification gates pass cleanly (exit code 0):
   - Gate 1: `quorumrag.exe` built and ready.
   - Gates 2 & 3: Live daemon starts and health probe passes on port 3031.
   - Gate 4: Real workspace files (`d:\harfile\QuorumRAG`) indexed via `/api/reindex` with 0 writer errors.
   - Gate 5: Live `/api/chat` query extracts compilable production code with exact `[Peer:File:Lines]` citations (`fusion.rs`), compilable `pub struct RrfCombiner;` verified via `cargo check` in isolated temporary harness.
   - Gate 6: LangExtract 4-arm consensus with 100% byte interval grounding and interactive HTML visualization.
   - Cleanup: `POST /api/shutdown` cooperative termination, storage compaction flush, 0 orphaned lockfiles.

3. **Workspace-Wide Compilation & Linting**:
   ```powershell
   cargo clippy --workspace --all-targets --jobs 2 -- -D warnings
   ```
   Verify 0 errors, 0 warnings.

4. **Workspace-Wide Test Matrix**:
   ```powershell
   cargo test --workspace --jobs 2 -- --test-threads=2
   ```
   Verify 100% pass across all workspace crates.

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Deliverable
Write your complete report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_1\handoff.md` with:
- Observation (detailed test runner outputs, logs, execution times)
- Logic Chain (verification of 100% E2E coverage and live IDE gate pass)
- Caveats (any observations)
- Conclusion (Explicit statement of acceptance)
- Verification Method (reproducible commands)


## 2026-11-01T05:41:05Z
You are worker_m6_1, a teamwork_worker.
Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_1\.
Read your instructions in d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_1\DISPATCH.md.
Also read ORIGINAL_REQUEST.md at d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md and TEST_READY.md at d:\harfile\QuorumRAG\TEST_READY.md.
Execute the full acceptance verification:
1. Run powershell -ExecutionPolicy Bypass -File "d:\harfile\QuorumRAG\tests\e2e\run_all_e2e_tests.ps1" (67 tests across Tiers 1-4).
2. Run powershell -ExecutionPolicy Bypass -File "d:\harfile\QuorumRAG\scripts\verify_live_ide_execution.ps1" (Live IDE execution gate, production code check, LangExtract grounding, clean shutdown).
3. Run cargo clippy --workspace --all-targets --jobs 2 -- -D warnings.
4. Run cargo test --workspace --jobs 2 -- --test-threads=2.
Write your complete report to d:\harfile\QuorumRAG\.agents\teamwork\worker_m6_1\handoff.md. Send a message to parent when done.
