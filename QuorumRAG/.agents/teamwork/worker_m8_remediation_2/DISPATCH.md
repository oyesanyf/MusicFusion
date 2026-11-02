# Dispatch — Worker M8 Remediation 2

## Identity & Role
- **Role**: teamwork_worker
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m8_remediation_2`
- **Parent Conversation ID**: `facfe77d-94d3-44be-95df-eec2b9dfe368`

## Write Ownership
You exclusively own and may edit:
- `crates/quorum-coordinator/src/offline_engine.rs`
- `crates/quorum-coordinator/src/agentic/retrieve.rs`

## Mandatory Reference Documents
- **`ORIGINAL_REQUEST.md`**: `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` (MANDATORY: read first)
- **`PROJECT.md`**: `d:\harfile\QuorumRAG\PROJECT.md`
- **Explorer 1 Report**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_rem2_1\handoff.md`
- **Explorer 2 Report**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_rem2_2\handoff.md`
- **Auditor Report**: `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m8_remediation_1\handoff.md`

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Implementation Tasks
Implement the 3 surgical fixes synthesized from Explorer reports:

1. **Proper Noun Normalization in `crates/quorum-coordinator/src/offline_engine.rs`**:
   In `EXACT_TYPO_MAP`, add lowercase proper nouns:
   ```rust
   ("angela", "Angela"),
   ("davis", "Davis"),
   ("femi", "Femi"),
   ("oyesanya", "Oyesanya"),
   ("cigna", "Cigna"),
   ("patientpoint", "PatientPoint"),
   ("mckinney", "McKinney"),
   ```
   Verify that `cargo test -p quorum-coordinator --lib -- --test-threads=2` passes 13/13 (including `test_spellcheck_corrects_chat_box_typos_without_rewriting`).

2. **Multi-Term Quorum Precision in `crates/quorum-coordinator/src/agentic/retrieve.rs` & `offline_engine.rs`**:
   Ensure `required_query_hits` strictly enforces:
   ```rust
   let required_query_hits = if query_terms.len() >= 2 {
       ((query_terms.len() + 1) / 2).max(2).min(query_terms.len())
   } else {
       1
   };
   ```
   Do not collapse `required_query_hits` to `required_entity_hits` when `!entity_terms.is_empty()`.
   Verify `cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2` passes 6/6 (including `test_adv_multi_term_entity_matching_precision`).

3. **Scope Supplemental Entity Search to Live Daemon in `retrieve.rs`**:
   Ensure supplemental entity text search only runs when `daemon_active && !prelim_entities.is_empty()`, so that mock tracking in `cargo test -p quorum-coordinator --test m8_p2p_consensus_latency_tests -- --test-threads=2` (`test_rewrite_hop_vector_cache_bypass`) remains at 2 and passes 6/6.

4. **Verify Full Test Suites**:
   - `cargo test -p quorum-coordinator --lib -- --test-threads=2` (13/13)
   - `cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2` (6/6)
   - `cargo test -p quorum-coordinator --test m8_p2p_consensus_latency_tests -- --test-threads=2` (6/6)
   - `cargo test -p quorum-coordinator --test m8_adversarial_p2p_tests -- --test-threads=2` (12/12)
   - `cargo check --workspace` (0 errors)

5. Write `handoff.md` and send completion message back to parent.
