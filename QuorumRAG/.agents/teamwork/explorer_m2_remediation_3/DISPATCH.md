# Task Assignment: Explorer M2 Remediation 3 — Arithmetic Bounds, Compaction Lifecycle & Windows Lock Resilience

## Mission
Investigate and formulate the exact remediation strategy for arithmetic overflow bounds in `crates/quorum-storage/src/engine.rs` and verify the full compaction lifecycle and Windows file lock resilience across `crates/quorum-storage/`. Do NOT implement code directly. Produce a structured investigation report in `.agents/teamwork/explorer_m2_remediation_3/handoff.md`.

## Mandatory Reading & Input Context
You MUST read these authoritative files in full before beginning:
1. `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (verbatim user request)
2. `d:\harfile\QuorumRAG\PROJECT.md` (architecture, Milestone 2 specifications)
3. `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m2_1\handoff.md` (FULL UNFILTERED FORENSIC AUDIT EVIDENCE)
4. `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m2_2\handoff.md` (Reviewer 2 findings on overflow)
5. `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_1\handoff.md` (Challenger 1 empirical findings on `top_k` overflow)
6. `crates/quorum-storage/src/engine.rs` (source code under investigation)
7. `crates/quorum-storage/tests/adversarial_storage_tests.rs` (adversarial harness under investigation)

## Specific Investigation Focus
1. **Integer Arithmetic Overflow in Candidate Limit (`engine.rs:171`)**:
   - `let candidate_limit = top_k.max(5) * 2;` panics on `top_k > usize::MAX / 2`.
   - Investigate `.saturating_mul(2)` and safe upper bounds.
2. **Compaction Lifecycle with Deletions & Reinsertions**:
   - Investigate `test_adversarial_compaction_after_deletions_and_reinsertions` in `adversarial_storage_tests.rs`:
     - Insertion of file_a, file_b, file_c
     - Deletion of file_b
     - Full compaction cycle (`engine.compact()`)
     - Reinsertion of updated file_b
     - Search verification across all 3 search modes (Keyword, Semantic Vector, Hybrid RRF).
   - Verify that LanceDB Parquet fragment compaction and Tantivy segment merging maintain data consistency and prevent any tombstone leaks.
3. **Windows File Lock Resilience**:
   - Check `ReloadPolicy::Manual`, `commit_with_retry` backoff logic, and temporary file directory handling under parallel tests on Windows NTFS.
4. **Remediation Strategy**:
   - Provide exact line-by-line recommendations for the Worker.
   - Specify exact cargo test commands to verify all 11 adversarial tests and 10 E2E storage isolation tests pass with zero warnings.

## Completion Criteria
Write your complete analysis and remediation plan to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_3\handoff.md`, update `progress.md`, and notify orchestrator via `send_message`.


## 2026-10-31T17:44:18Z
[Message] timestamp=2026-10-31T17:44:18Z sender=e13403c5-b0e6-48a4-a1ec-a1f096f9f53e priority=MESSAGE_PRIORITY_HIGH content=You are `explorer_m2_remediation_3`, a read-only exploration agent assigned to Milestone 2 Remediation.
Your working directory is: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_3\`
Your task description is located at: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_3\DISPATCH.md`
