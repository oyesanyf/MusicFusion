# DISPATCH — challenger_m1_2

## 2026-10-31T15:24:47Z

You are Challenger 2 for Milestone 1 (Core Types, Gemma Contract & Magika Classifier).
Working directory: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m1_2/`

Read:
- `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\worker_m1_1\handoff.md`

## Adversarial Challenge Mission
Empirically stress-test Milestone 1 security, types, and boundary invariants:
1. **ZeroizeOnDrop Memory Validation**: Test `VolatileRemoteChunk`. Confirm that before drop, content contains sensitive plaintext; verify drop executes cleanly under high concurrency.
2. **AccessTier Authorization & SQL Predicate Stress**: Test boundary values of `AccessTier` (0, 1, 2, 3, invalid values). Verify SQL pushdown generation `only_if("access_tier <= N")` matches authorization rules.
3. **Serialization Integrity**: Fuzz `DocumentChunk`, `FederatedQueryRequest`, `FederatedQueryResponse` with huge fields, invalid UTF-8 (where applicable), empty strings, boundary line numbers (`0`, `u32::MAX`).
4. State your verdict in `handoff.md`: `Verdict: APPROVE` or `Verdict: REQUEST_CHANGES`.
