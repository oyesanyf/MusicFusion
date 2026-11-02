# Challenger M10 2 Dispatch

## Role & Assignment
You are Challenger M10 2 (`challenger_m10_2`).
Your assignment is to adversarially challenge and stress-test the Byzantine peer resilience and fault regression suite in `crates/quorum-coordinator/tests/byzantine_fault_regression.rs`, along with the M8, M8.2, and M8.5 regressions.
Stress-test Sybil score stuffing, signature malleability, Merkle root & BLAKE3 tampering, partition timeouts, drift window boundaries, and source code & temp file ingestion exclusion invariants.

## Key Files to Review
- `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` (MANDATORY: Read first)
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\worker_m10_1\handoff.md`
- `d:\harfile\QuorumRAG\crates\quorum-coordinator/tests/byzantine_fault_regression.rs`
- `d:\harfile\QuorumRAG\crates\quorum-ingest/tests/m8_5_exclusion_tests.rs`

## Required Verification
Execute:
```powershell
cargo test -p quorum-coordinator --test byzantine_fault_regression -- --test-threads=2 --nocapture
cargo test -p quorum-coordinator --test m8_adversarial_p2p_tests -- --test-threads=2
cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2
cargo test -p quorum-ingest --test m8_5_exclusion_tests -- --nocapture
```
Evaluate edge cases:
- Extreme Sybil floods (100+ duplicate remote chunks).
- Mismatched public keys vs peer IDs.
- Bit-flipped signatures and post-signing tampering.
- Expired timestamps (>60s) and future timestamps (>60s).
- Silent drop / non-responsive peers under 400ms scatter-gather deadline.
- Source code extensions (.rs, .py, .js, .ts, etc.) and Office lock files (~*, .tmp, .bak) leakage prevention.

## Output Requirements
Write `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m10_2\handoff.md` with:
- Stress test observations and empirical data
- Attack / challenge scenarios evaluated
- Conclusion with explicit verdict: `APPROVE` or `REQUEST_CHANGES`

## 2026-11-02T08:52:52Z
You are Challenger M10 2 (`challenger_m10_2`).
Your working directory is `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m10_2`.
Read your dispatch instructions in `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m10_2\DISPATCH.md`.
Read `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`, `d:\harfile\QuorumRAG\PROJECT.md`, and `d:\harfile\QuorumRAG\.agents\teamwork\worker_m10_1\handoff.md`.
Adversarially challenge and stress-test the Byzantine peer resilience and fault regression suite in `crates/quorum-coordinator/tests/byzantine_fault_regression.rs` and related suites.
Execute the required verification commands.
Write your challenge report to `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m10_2\handoff.md` and send your verdict (`APPROVE` or `REQUEST_CHANGES`) to your parent via send_message.
