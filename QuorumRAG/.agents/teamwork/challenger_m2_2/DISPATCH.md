# DISPATCH — challenger_m2_2

## Task Assignment
You are Challenger 2: Milestone 2 Adversarial Verifier (LanceDB Vector Storage & Tantivy FTS).
Working directory: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_2/`

Read:
- `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\TEST_READY.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_1\handoff.md`
- Code in `d:\harfile\QuorumRAG\crates\quorum-storage\`

## Adversarial Challenge Objectives
1. Create and execute adversarial stress tests targeting:
   - Reciprocal Rank Fusion (RRF) edge cases: identical ranks, pure keyword hits vs pure vector hits, opposing vector cosine similarity vs text matching.
   - Tantivy BM25 tokenization edge cases: non-ASCII UTF-8, punctuation, regex-like tokens, empty text payloads.
   - Compaction resilience: repeatedly compacting an empty table, compacting after deleting chunks, compacting during simulated reads.
2. Verify stability, determinism, and zero panic guarantees.
3. Write your findings and verdict (APPROVE or REQUEST_CHANGES) to `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_2\handoff.md` and send a message when done.
