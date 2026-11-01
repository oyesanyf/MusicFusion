# Task Assignment: Worker M2 Remediation 1 — Storage Engine Remediation

## Mission
Implement the remediation for Milestone 2 (`crates/quorum-storage/`) addressing all issues found during Iteration 1 gating:
1. BUG-M2-01: Tantivy silent deletion failure (replace `TEXT | STORED` with `STRING | STORED`, add `field_file_path_text: TEXT`, wire `QueryParser` to both, return true deleted count).
2. BUG-M2-02: Non-deterministic RRF ranking (add `chunk_id` tie-breaker to `vector_store.rs`, `tantivy_fts.rs`, and competition ranking in `engine.rs`).
3. Arithmetic overflow on `candidate_limit` (use `.saturating_mul(2).max(10)` and `top_k == 0` check).
4. Run all unit, adversarial, E2E, and clippy checks.

## Mandatory Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Mandatory Reading & Input Context
Read these authoritative reports before starting:
1. `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (verbatim user request)
2. `d:\harfile\QuorumRAG\PROJECT.md` (architecture, specifications)
3. `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_1\handoff.md` (Tantivy schema & deletion diffs)
4. `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_2\handoff.md` (RRF determinism diffs)
5. `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_3\handoff.md` (Bounds & lifecycle diffs)
6. `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m2_1\handoff.md` (Audit findings)

## File Ownership
You exclusively own and may modify:
- `crates/quorum-storage/src/tantivy_fts.rs`
- `crates/quorum-storage/src/vector_store.rs`
- `crates/quorum-storage/src/engine.rs`
- `crates/quorum-storage/tests/adversarial_storage_tests.rs`

## Verification Commands
Execute using process-scoped environment variables:
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"

# 1. Storage unit tests
cargo test -p quorum-storage --test storage_tests

# 2. Adversarial storage tests (must pass 11/11, including deletion and tiebreak)
cargo test -p quorum-storage --test adversarial_storage_tests

# 3. Adversarial stress tests (must pass all tests, including top_k overflow)
cargo test -p quorum-storage --test adversarial_stress_tests

# 4. E2E storage isolation contract
cargo test --manifest-path tests/e2e/Cargo.toml --test test_storage_isolation

# 5. Full workspace tests
cargo test --workspace

# 6. Clippy with zero warnings
cargo clippy --workspace --all-targets -- -D warnings
```

## Completion Criteria
Write your complete handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_1\handoff.md`, update `progress.md`, and notify orchestrator via `send_message`.


## 2026-10-31T17:54:57Z
Received dispatch from e13403c5-b0e6-48a4-a1ec-a1f096f9f53e.
Beginning Milestone 2 Storage Engine remediation.

## 2026-10-31T18:14:35Z
From e13403c5-b0e6-48a4-a1ec-a1f096f9f53e:
Milestone 2 Remediation Verification Status Check.
Updating progress.md and completing test matrix.
