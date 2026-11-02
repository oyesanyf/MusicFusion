# Task Assignment: Forensic Auditor M11 1 (Integrity & Invariant Forensics)

## Working Directory
d:\harfile\QuorumRAG\.agents\teamwork\auditor_m11_1\

## Mission & Scope
Perform comprehensive forensic integrity audit on Milestone M11 changes (commit `53af83a`):
- Read `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
- Read `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md`
- Read `d:\harfile\QuorumRAG\.agents\teamwork\worker_m11_1\handoff.md`

Audit Invariants:
1. Production Code Invariant:
   - Verify all implementations in `extraction.rs`, `types.rs`, `classify.rs`, `retrieve.rs`, `agentic/mod.rs`, and `offline_engine.rs` are 100% genuine, authentic production logic.
   - Check for hardcoded test prompt returns, dummy mock branches, or fake data fixtures bypassing actual parsing or consensus.
2. Invariant Check: Zero Line Numbers:
   - Audit `crates/quorum-coordinator/src/offline_engine.rs` to assert that line numbers (`Lines `, `(Lines`, `:start-end`) have been completely purged from synthesized response text and Sources blocks.
3. Invariant Check: Clean Sources Block:
   - Assert `format_sources_block` outputs document name, link, and accuracy percentage, deduplicated by filename.
4. Invariant Check: 4-Arm Consensus Rigor:
   - Assert `extract_chat_candidates` outputs genuine substrings of `prompt` that undergo true grounding in Arm 1 and counterfactual verification in Arm 3.
5. Invariant Check: Environment & PATH Safety:
   - Assert zero mutations to system or user environment/PATH.
6. Run:
   `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --test m11_langextract_tests -- --test-threads=2`
7. Report forensic verdict (CLEAN or CHEATING DETECTED / INTEGRITY VIOLATION) in `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m11_1\handoff.md` and send message to orchestrator.


## 2026-11-02T15:15:07Z
You are Forensic Auditor M11 1 (teamwork_auditor).
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\auditor_m11_1\
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md, d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md, d:\harfile\QuorumRAG\.agents\teamwork\worker_m11_1\handoff.md, and d:\harfile\QuorumRAG\.agents\teamwork\auditor_m11_1\DISPATCH.md immediately.

Perform exhaustive forensic integrity audit on Milestone M11 (commit `53af83a`):
1. Authenticity: Zero mocks, zero hardcoded test returns, zero dummy branches.
2. Zero Line Numbers: Assert complete absence of line numbers in response bodies and Sources blocks.
3. Clean Sources: Assert accuracy score percentage, deduplicated by filename.
4. 4-Arm Consensus Rigor: Verbatim character intervals, counterfactual verification.
5. Environment Safety: Zero mutations to system/user environment/PATH.
Run:
`$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --test m11_langextract_tests -- --test-threads=2`
Write your forensic report and binary verdict (CLEAN or CHEATING DETECTED / INTEGRITY VIOLATION) to d:\harfile\QuorumRAG\.agents\teamwork\auditor_m11_1\handoff.md and notify parent orchestrator via send_message.
