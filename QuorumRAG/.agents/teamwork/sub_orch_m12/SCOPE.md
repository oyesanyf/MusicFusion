# Scope: Milestone M12 - Retrieval Engine Bugfixes

## Mission
Fix retrieval bugs and conversational context handling in `crates/quorum-coordinator/src/offline_engine.rs`:
1. Fix `file_hits` in `offline_engine.rs:499`:
   - Check filename tokens (`file_l` / `file_tokens`), NOT chunk body text (`matches_whole_word`).
2. Remove broken RRF similarity cutoff in `offline_engine.rs:505`:
   - Remove `if scored.score < 0.35 && scored.score > 0.0 && file_hits == 0 { continue; }`.
   - RRF scores are ~0.01..0.05; this filter was improperly dropping valid candidate chunks when filename did not match.
3. Conversational follow-up query context:
   - When user queries "tell me more", "continue", "elaborate", or similar follow-ups, extract context keywords from the conversation history (`history`) so retrieval does not return 0 chunks or stall on empty query terms.

## File Boundaries (Exclusive Write Ownership)
- `crates/quorum-coordinator/src/offline_engine.rs`

## Acceptance Criteria
- [ ] `file_hits` checks filename tokens without checking chunk body text.
- [ ] `scored.score < 0.35` cutoff is removed.
- [ ] Conversational follow-ups like "tell me more" retrieve relevant context from conversation history.
- [ ] `cargo test -p quorum-coordinator` passes.
- [ ] Reviewer APPROVE, Challenger PASS, Forensic Auditor CLEAN.
