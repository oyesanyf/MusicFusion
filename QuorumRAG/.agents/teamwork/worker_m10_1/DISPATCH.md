# Dispatch — Worker M10 (Implementation of Features 55 & 56: 7-Metric Benchmarks, Byzantine Regressions & Solo PR Shipping)

## Assignment
You are Worker M10. You are tasked with implementing Milestone M10:
1. **Feature 55: 7-Metric Deterministic Benchmark Suite (`tests/benchmarks_r4.rs`)**
2. **Feature 56: Byzantine Peer & Fault Regression Test Suite (`crates/quorum-coordinator/tests/byzantine_fault_regression.rs`)**
3. **Solo PR Shipping Pipeline (`scripts/solo_pr_ship.ps1` & `scripts/resolve_open_prs.py`) & `.gitignore` update**
4. **End-to-End Verification of all benchmark metrics and regression tests**

## Mandatory Reading
Before writing any code, you MUST view and read:
- `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m10_1\handoff.md` (exact design for 7-metric benchmark harness)
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m10_2\handoff.md` (exact design for byzantine fault regression tests)
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m10_3\handoff.md` (exact design for solo PR ship script & git resolution)

## MANDATORY INTEGRITY WARNING
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Detailed Tasks

### 1. Update `.gitignore`
Add `.quorumrag-rag-index/` to `d:\harfile\QuorumRAG\.gitignore` so the live index directory is not tracked.

### 2. Implement `tests/benchmarks_r4.rs`
Implement the 7-metric deterministic benchmark harness per `explorer_m10_1/handoff.md`:
- Data model `BenchmarkMetricResult` and `BenchmarkSuite` that prints a structured Markdown table of all 7 metrics.
- Benchmark 1: Ingestion Throughput (>= 40.0 MB/s sustained)
- Benchmark 2: Query Latency P95 (<= 450.0 ms over N >= 30 queries)
- Benchmark 3: Dedup Hydration Speed (<= 120.0 ms)
- Benchmark 4: P2P Handshake Latency (<= 80.0 ms)
- Benchmark 5: Byzantine Tolerance (>= 33.0% faulty peers tolerated without corruption)
- Benchmark 6: Memory Footprint (<= 180.0 MB Base Daemon RSS using Windows `K32GetProcessMemoryInfo` with `sysinfo` fallback)
- Benchmark 7: RAG Precision (100.0% relevant citations, 0 false positives on multi-term entity queries)
- Register `[[test]] name = "benchmarks_r4" path = "../../tests/benchmarks_r4.rs"` in `crates/quorum-daemon/Cargo.toml`.
- Also register in `tests/e2e/Cargo.toml` if applicable.

### 3. Implement `crates/quorum-coordinator/tests/byzantine_fault_regression.rs`
Implement all 7 unified Byzantine fault regression tests per `explorer_m10_2/handoff.md`:
- `test_byzantine_sybil_score_stuffing_under_high_load`
- `test_byzantine_merkle_root_and_chunk_hash_tampering`
- `test_byzantine_ed25519_forged_signature_and_identity_spoofing`
- `test_byzantine_network_partition_and_silent_drop_400ms_deadline`
- `test_byzantine_replay_attack_and_drift_window_rejection`
- `test_invariant_fault_regression_source_code_and_temp_file_leakage`
- `test_invariant_fault_regression_deterministic_synthesis_and_monotonic_progress`

### 4. Implement `scripts/solo_pr_ship.ps1` & `scripts/resolve_open_prs.py`
Implement the complete solo PR shipping pipeline in `d:\harfile\QuorumRAG\scripts\solo_pr_ship.ps1` per `explorer_m10_3/handoff.md`:
- Process-scoped PATH prepending `C:\Program Files\Git\cmd;C:\Program Files\Git\bin;C:\Program Files\GitHub CLI;` (ZERO-TOUCH PATH LAW: NEVER modify registry).
- Parse repo slug `oyesanyf/MusicFusion` dynamically.
- Support `-Title` and `-Body` parameters, with default PR body containing the full 7-metric before/after table.
- Branch triage (`pr/solo-quorumrag-<timestamp>`), stage, commit, push, PR creation via `gh.exe` / Python REST API.
- Automated squash-merge, checkout `main`, pull `origin/main`, clean up branches.
- Verify `git status -s` is clean.
- Implement `scripts/resolve_open_prs.py` companion script.

### 5. Verification Commands to Execute
Run and document results in `handoff.md`:
1. `cargo check --workspace`
2. `cargo test -p quorum-daemon --test benchmarks_r4 -- --test-threads=2 --nocapture`
3. `cargo test -p quorum-coordinator --test byzantine_fault_regression -- --test-threads=2 --nocapture`
4. `cargo test -p quorum-coordinator --test m8_adversarial_p2p_tests -- --test-threads=2`
5. `cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2`
6. `cargo test -p quorum-ingest --test m8_5_exclusion_tests -- --nocapture`
7. `cargo test -p quorum-ingest --test m9_monotonic_progress_test -- --nocapture`

## Output
Write your comprehensive handoff report to:
`d:\harfile\QuorumRAG\.agents\teamwork\worker_m10_1\handoff.md`
Then call `send_message` to Orchestrator Gen 2 (`facfe77d-94d3-44be-95df-eec2b9dfe368`).


## 2026-11-02T07:57:17Z
You are Worker M10. Your working directory is `d:\harfile\QuorumRAG\.agents\teamwork\worker_m10_1`.

Read `d:\harfile\QuorumRAG\.agents\teamwork\worker_m10_1\DISPATCH.md` for your complete assignment, file paths, and instructions.
Read `ORIGINAL_REQUEST.md`, `PROJECT.md`, `explorer_m10_1/handoff.md`, `explorer_m10_2/handoff.md`, and `explorer_m10_3/handoff.md` before starting work.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Implement:
1. Feature 55: 7-Metric Deterministic Benchmark Suite (`tests/benchmarks_r4.rs`) & Cargo test target registrations.
2. Feature 56: Byzantine Peer & Fault Regression Test Suite (`crates/quorum-coordinator/tests/byzantine_fault_regression.rs`).
3. Solo PR shipping pipeline (`scripts/solo_pr_ship.ps1` & `scripts/resolve_open_prs.py`) and `.gitignore` update.
4. Run all verification tests with `-- --test-threads=2` and document results in `d:\harfile\QuorumRAG\.agents\teamwork\worker_m10_1\handoff.md`.

When finished, send a message to Orchestrator Gen 2 (facfe77d-94d3-44be-95df-eec2b9dfe368).


## 2026-11-02T08:18:18Z
**Context**: Milestone M10 Implementation Status
**Content**: Checking on current progress of workspace compilation and verification tests (Task 5).
**Action**: Please provide a brief status update on compilation and verification progress, and update your progress.md.
