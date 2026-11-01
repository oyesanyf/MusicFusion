# BRIEFING — 2026-10-31T17:53:30Z

## Mission
Investigate and formulate the exact remediation strategy for non-deterministic ordinal RRF ranking (BUG-M2-02) across crates/quorum-storage/src/vector_store.rs, crates/quorum-storage/src/engine.rs, and crates/quorum-storage/tests/adversarial_storage_tests.rs without implementing direct code changes.

## 🔒 My Identity
- Archetype: explorer
- Roles: read-only investigation, forensic code analysis, synthesis, remediation planning
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_2\
- Original parent: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
- Milestone: Milestone 2 Remediation (M2)

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Write only to .agents/teamwork/explorer_m2_remediation_2/
- Follow 5-component handoff report structure (Observation, Logic Chain, Caveats, Conclusion, Verification Method)
- Never modify source code directly

## Current Parent
- Conversation ID: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
- Updated: 2026-10-31T17:53:30Z

## Investigation State
- **Explored paths**:
  - `ORIGINAL_REQUEST.md`, `PROJECT.md`
  - Forensic audit & challenger reports: `auditor_m2_1/handoff.md`, `reviewer_m2_2/handoff.md`, `challenger_m2_2/handoff.md`, `challenger_m2_1/handoff.md`
  - Codebase: `crates/quorum-storage/src/vector_store.rs`, `crates/quorum-storage/src/engine.rs`, `crates/quorum-storage/src/tantivy_fts.rs`, `crates/quorum-storage/src/arrow_interop.rs`
  - Tests: `crates/quorum-storage/tests/adversarial_storage_tests.rs`, `crates/quorum-storage/tests/storage_tests.rs`, `tests/e2e/src/lib.rs`
- **Key findings**:
  - Live empirical reproduction of BUG-M2-02: test oscillated between failing (`left: "chunk_tie_beta", right: "chunk_tie_alpha"`) and passing across consecutive runs.
  - Root cause 1: `vector_store.rs:125` sorts via `b.score.partial_cmp(&a.score)` without secondary tie-breaker on `chunk_id`, preserving arbitrary LanceDB RecordBatch physical scan order.
  - Root cause 2: `tantivy_fts.rs:253` returns `scored_chunks` in collector `DocAddress` order without secondary tie-breaker.
  - Root cause 3: `engine.rs:183-199` uses `enumerate()` ordinal rank ($1/(60 + \text{rank})$), converting identical raw cosine/BM25 scores into distinct RRF scores ($1/61$ vs $1/62$), which completely bypasses the downstream tie-breaker on line 210 (`.then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))`).
  - Level-2 BLAKE3 Deduplication in `adversarial_storage_tests.rs`: `engine.insert_chunks` hashes `text_content`. The comment claiming "chunk_a and chunk_b have identical text" is inaccurate. If fixtures had identical text, `chunk_b` would be deduplicated away! Fixture requires distinct prefixes ("alpha", "beta", "gamma") while retaining identical term frequencies and doc length to avoid dedup skips while yielding identical BM25 scores.
- **Unexplored areas**: None. Complete evidence chain established.

## Key Decisions Made
- Formulate a 4-layer defense-in-depth remediation:
  1. Add `.then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))` to `vector_store.rs:125`.
  2. Add deterministic `.sort_by` on `score DESC, chunk_id ASC` in `tantivy_fts.rs:250`.
  3. Implement equal-score competition ranking in `engine.rs:183-199` so identical cosine/BM25 scores share identical RRF rank, allowing line 210 tie-breaker to execute.
  4. Clarify test comment and add score equality assertions in `adversarial_storage_tests.rs`.

## Artifact Index
- `DISPATCH.md` — Task assignment and instructions
- `BRIEFING.md` — Persistent working memory and state
- `progress.md` — Liveness heartbeat and milestone tracker
- `handoff.md` — Final 5-component handoff report
