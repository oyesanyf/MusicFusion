# Dispatch — Worker M8 Remediation 1

## Identity & Role
- **Role**: teamwork_worker
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m8_remediation_1`
- **Parent Conversation ID**: `facfe77d-94d3-44be-95df-eec2b9dfe368`

## Mandatory Reference Documents
- **`ORIGINAL_REQUEST.md`**: `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` (MANDATORY: read first)
- **`PROJECT.md`**: `d:\harfile\QuorumRAG\PROJECT.md`
- **Audit Failure Evidence**: `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m8_1\handoff.md`
- **Challenger Failure Evidence**: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m8_2\handoff.md`
- **Reviewer Changes Evidence**: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m8_2\handoff.md`

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Objective
Verify, remediate, and confirm passing status for Milestone M8:
1. Multi-term entity matching precision in `crates/quorum-coordinator/src/agentic/retrieve.rs`:
   Ensure `test_adv_multi_term_entity_matching_precision` in `adversarial_challenge_m8_2_tests.rs` passes 100%. A document containing only 1 term in isolation for a multi-word query (threshold >= 2) must be filtered out (`is_relevant = false`).
2. Strict RAG-only deterministic synthesis in `offline_engine.rs` & `agentic/mod.rs`:
   Confirm 100% deterministic synthesis from retrieved RAG chunks with real local file `Sources` at the bottom.
3. Verify test suites:
   - `cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2`
   - `cargo test -p quorum-coordinator --test m8_p2p_consensus_latency_tests -- --test-threads=2`
   - `cargo test -p quorum-coordinator --lib -- --test-threads=2`
   - `cargo check --workspace`
4. Deliver `handoff.md` with exact command outputs and send message back to parent.
