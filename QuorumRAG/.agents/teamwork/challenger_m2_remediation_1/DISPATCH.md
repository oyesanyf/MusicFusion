# Task Assignment: Challenger M2 Remediation 1 — Deletion & Bounds Empirical Stress

## Mission
Perform adversarial empirical verification of the Milestone 2 remediation in `crates/quorum-storage/`. Execute stress tests to empirically verify:
1. Tantivy exact deletion and absence of phantom documents after deletion.
2. Compaction after deletion and reinsertion across all 3 search modes (Keyword, Semantic, Hybrid).
3. Extreme `top_k` (including `0`, `1`, `usize::MAX`, `50000`) without panic.
4. Filename keyword searches matching tokens from `field_file_path_text`.

## Mandatory Reading & Input Context
1. `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (verbatim user request)
2. `d:\harfile\QuorumRAG\PROJECT.md` (specifications)
3. `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_1\handoff.md` (Remediation report)
4. `crates/quorum-storage/tests/adversarial_storage_tests.rs`
5. `crates/quorum-storage/tests/adversarial_stress_tests.rs`

## Verification Commands
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_after_deletions_and_reinsertions -- --nocapture
cargo test -p quorum-storage --test adversarial_stress_tests test_adv_top_k_overflow_in_hybrid -- --nocapture
cargo test -p quorum-storage --test adversarial_stress_tests test_adv_filename_search_in_tantivy -- --nocapture
cargo test -p quorum-storage --test adversarial_storage_tests
cargo test -p quorum-storage --test adversarial_stress_tests
```

## Completion Criteria
Write your complete report to `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_remediation_1\handoff.md`, update `progress.md`, and notify orchestrator via `send_message`.


## 2026-10-31T18:32:48Z
You are `challenger_m2_remediation_1`, a code-executing adversarial verifier assigned to Milestone 2 Remediation.
Your working directory is: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_remediation_1\`
Your task assignment is in: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_remediation_1\DISPATCH.md`

MANDATORY INSTRUCTIONS:
1. You MUST read `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` in full before starting.
2. Read `d:\harfile\QuorumRAG\PROJECT.md`.
3. Read the Worker's handoff report at `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_1\handoff.md`.
4. Perform empirical verification of:
   - Tantivy exact deletion and compaction lifecycle (`test_adversarial_compaction_after_deletions_and_reinsertions`).
   - Extreme `top_k` (`0`, `usize::MAX`) in `test_adv_top_k_overflow_in_hybrid`.
   - Filename keyword search in `test_adv_filename_search_in_tantivy`.
5. Run using process-scoped environment variables:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
   $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_after_deletions_and_reinsertions -- --nocapture
   cargo test -p quorum-storage --test adversarial_stress_tests test_adv_top_k_overflow_in_hybrid -- --nocapture
   cargo test -p quorum-storage --test adversarial_stress_tests test_adv_filename_search_in_tantivy -- --nocapture
   cargo test -p quorum-storage --test adversarial_storage_tests
   cargo test -p quorum-storage --test adversarial_stress_tests
   ```
6. Write your complete handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_remediation_1\handoff.md` with your verdict (APPROVE or REQUEST_CHANGES), and notify orchestrator (`e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`) via `send_message`.
