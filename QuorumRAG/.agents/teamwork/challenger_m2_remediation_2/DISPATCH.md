# Task Assignment: Challenger M2 Remediation 2 — Deterministic RRF & Concurrency Stress

## Mission
Perform adversarial empirical verification of Milestone 2 remediation in `crates/quorum-storage/`. Execute stress tests to empirically verify:
1. Deterministic RRF ranking tie-breaking across repeated consecutive runs (e.g. 20 consecutive iterations).
2. Equal RRF scores assigned to co-scoring chunks within `1e-6` float epsilon.
3. High-concurrency simultaneous reader and compaction tasks (8 tasks x 30 queries with background compactions) with 0 errors, 0 panics, and 0 deadlocks.
4. Level-2 BLAKE3 deduplication correctly preserves distinct prefix chunks while rejecting identical text payloads.

## Mandatory Reading & Input Context
1. `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (verbatim user request)
2. `d:\harfile\QuorumRAG\PROJECT.md` (specifications)
3. `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_1\handoff.md` (Remediation report)
4. `crates/quorum-storage/tests/adversarial_storage_tests.rs`

## Verification Commands
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"

# 1. Run deterministic tie-break 20 times in a loop
1..20 | ForEach-Object { cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_rrf_identical_ranks_deterministic_tiebreak -- --nocapture }

# 2. Run concurrency stress test
cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_concurrent_reads_stress -- --nocapture

# 3. Run full adversarial storage suite
cargo test -p quorum-storage --test adversarial_storage_tests
```

## Completion Criteria
Write your complete report to `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_remediation_2\handoff.md`, update `progress.md`, and notify orchestrator via `send_message`.


## 2026-10-31T18:32:55Z
You are `challenger_m2_remediation_2`, a code-executing adversarial verifier assigned to Milestone 2 Remediation.
Your working directory is: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_remediation_2\`
Your task assignment is in: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_remediation_2\DISPATCH.md`

MANDATORY INSTRUCTIONS:
1. You MUST read `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` in full before starting.
2. Read `d:\harfile\QuorumRAG\PROJECT.md`.
3. Read the Worker's handoff report at `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_1\handoff.md`.
4. Perform empirical verification of:
   - RRF deterministic ranking tie-breaking across repeated runs (loop 20 times).
   - Equal RRF scores assigned to co-scoring chunks within `1e-6` float epsilon.
   - High-concurrency reader/compaction stress test (`test_adversarial_compaction_concurrent_reads_stress`).
5. Run using process-scoped environment variables:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
   $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   1..20 | ForEach-Object { cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_rrf_identical_ranks_deterministic_tiebreak -- --nocapture }
   cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_concurrent_reads_stress -- --nocapture
   cargo test -p quorum-storage --test adversarial_storage_tests
   ```
6. Write your complete handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_remediation_2\handoff.md` with your verdict (APPROVE or REQUEST_CHANGES), and notify orchestrator (`e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`) via `send_message`.
