# Task Assignment: Explorer M2 Remediation 2 — Vector Store RRF Determinism & Tie-Breaking

## Mission
Investigate and formulate the exact remediation strategy for non-deterministic ordinal RRF ranking (BUG-M2-02) across `crates/quorum-storage/src/vector_store.rs` and `crates/quorum-storage/src/engine.rs`. Do NOT implement code directly. Produce a structured investigation report in `.agents/teamwork/explorer_m2_remediation_2/handoff.md`.

## Mandatory Reading & Input Context
You MUST read these authoritative files in full before beginning:
1. `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (verbatim user request)
2. `d:\harfile\QuorumRAG\PROJECT.md` (architecture, Milestone 2 specifications)
3. `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m2_1\handoff.md` (FULL UNFILTERED FORENSIC AUDIT EVIDENCE)
4. `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m2_2\handoff.md` (Reviewer 2 findings)
5. `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_2\handoff.md` (Challenger 2 empirical reproduction of BUG-M2-02)
6. `crates/quorum-storage/src/vector_store.rs` and `crates/quorum-storage/src/engine.rs` (source code under investigation)
7. `crates/quorum-storage/tests/adversarial_storage_tests.rs` (adversarial harness under investigation)

## Specific Investigation Focus
1. **Vector Store Sorting (`vector_store.rs:125`)**:
   - `all_scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))` lacks a secondary tie-breaker.
   - Investigate how co-scoring chunks with equal cosine similarity retain non-deterministic order depending on RecordBatch batch boundaries.
2. **Downstream RRF Ordinal Scoring (`engine.rs:183-187`)**:
   - `search_hybrid` enumerates `relevant_vector_hits` ordinally: `rank_0` assigns $1/(60+1) = 1/61$ to hit 0, $1/(60+2) = 1/62$ to hit 1.
   - When co-scoring hits have different ordinal positions, they receive distinct scores, completely bypassing the downstream tie-breaker on line 210 (`.then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))`).
3. **Adversarial Test Suite Analysis (`adversarial_storage_tests.rs:110`)**:
   - In `test_adversarial_rrf_identical_ranks_deterministic_tiebreak`, investigate whether the two test chunks have distinct text content or if Level-2 BLAKE3 deduplication skips inserting one of the chunks if they share identical text.
   - Formulate exact test assertions and fixtures to guarantee deterministic tie-breaking without triggering unintended dedup skips.
4. **Remediation Strategy**:
   - Provide exact line-by-line recommendations for the Worker.
   - Specify exact cargo test commands to verify determinism across repeated executions.

## Completion Criteria
Write your complete analysis and remediation plan to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_2\handoff.md`, update `progress.md`, and notify orchestrator via `send_message`.

## 2026-10-31T17:44:07Z
Sender: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
Priority: MESSAGE_PRIORITY_HIGH
Content:
You are `explorer_m2_remediation_2`, a read-only exploration agent assigned to Milestone 2 Remediation.
Your working directory is: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_2\`
Your task description is located at: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_2\DISPATCH.md`

MANDATORY INSTRUCTIONS:
1. You MUST read `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` in full before starting.
2. Read `d:\harfile\QuorumRAG\PROJECT.md`.
3. Read the FULL UNFILTERED FORENSIC AUDIT EVIDENCE in `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m2_1\handoff.md`.
4. Read `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m2_2\handoff.md`, `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_2\handoff.md`, and `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_1\handoff.md`.
5. Focus: Vector Store RRF Determinism & Secondary Tie-Breaking across `crates/quorum-storage/src/vector_store.rs`, `crates/quorum-storage/src/engine.rs`, and `crates/quorum-storage/tests/adversarial_storage_tests.rs`. Investigate why identical cosine similarity hits diverge in RRF scoring without a secondary tie-breaker, and check `test_adversarial_rrf_identical_ranks_deterministic_tiebreak` fixtures regarding Level-2 BLAKE3 deduplication.
6. Do NOT implement code directly. Produce a structured investigation and remediation plan in `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_2\handoff.md`.
7. Once finished, send a message to orchestrator (`e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`) with your key findings and handoff path.
