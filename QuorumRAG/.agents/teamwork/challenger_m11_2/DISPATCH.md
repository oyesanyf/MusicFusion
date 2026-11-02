# Task Assignment: Challenger M11 2 (Question-Restating Affirmative Synthesis & Invariant Stress Verification)

## Working Directory
d:\harfile\QuorumRAG\.agents\teamwork\challenger_m11_2\

## Mission & Scope
Empirically stress-test and verify Milestone M11 question-restating affirmative synthesis and invariants:
- Read `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
- Read `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md`
- Read `d:\harfile\QuorumRAG\.agents\teamwork\worker_m11_1\handoff.md`

Test:
1. Verify affirmative question-restating prefix for various interrogative structures:
   - "Who is femi" -> "Femi is "
   - "what is RedactX" -> "RedactX is "
   - "where did Femi study" -> "Femi studied at "
   - "where did Femi go to school" -> "Femi attended "
   - "what the Cigna policy" -> "The Cigna Policy is "
   - "tell me about Femi" -> "Femi is "
2. Hard Invariant checks on synthesized outputs:
   - ZERO occurrences of `(Lines X–Y)` or `:line_start-line_end`.
   - ZERO raw section headers `### 📄 ` in synthesized body text.
   - Sources block strictly at bottom: `- [📄 <file_name>](<file_path>) — **<pct>% accuracy**`.
   - Sources deduplicated: no document listed twice.
3. Target document prioritization:
   - Queries for Femi Oyesanya must prioritize resume/CV files over general docs.
4. Run:
   `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --test m11_langextract_tests -- test_question_restating_synthesis_no_line_numbers_and_clean_sources`
   `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --lib offline_engine`
5. Report verdict (APPROVE or REQUEST_CHANGES) with empirical evidence in `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m11_2\handoff.md` and send message to orchestrator.


## 2026-11-02T15:14:58Z
You are Challenger M11 2 (teamwork_challenger).
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\challenger_m11_2\
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md, d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md, d:\harfile\QuorumRAG\.agents\teamwork\worker_m11_1\handoff.md, and d:\harfile\QuorumRAG\.agents\teamwork\challenger_m11_2\DISPATCH.md immediately.

Empirically stress-test question-restating synthesis and user invariants:
- Assert zero line numbers anywhere in synthesized responses.
- Assert zero raw file headers (`### 📄 `) in response bodies.
- Assert clean Sources block at end with accuracy score percentage, deduplicated by filename.
- Assert profile queries prioritize CV/Resume documents.
Run:
`$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --test m11_langextract_tests -- test_question_restating_synthesis_no_line_numbers_and_clean_sources`
`$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --lib offline_engine`
Write your adversarial evaluation report and verdict (APPROVE or REQUEST_CHANGES) to d:\harfile\QuorumRAG\.agents\teamwork\challenger_m11_2\handoff.md and notify parent orchestrator via send_message.


## 2026-11-02T15:25:38Z
**Context**: Challenger M11 2 handoff finalization
**Content**: Your adversarial test suite `adversarial_m11_challenger_tests.rs` (5/5 passed) has been verified. Please write your handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m11_2\handoff.md` and send your verdict.
**Action**: Write handoff.md and send completion message.
