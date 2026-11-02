# Task Assignment: Reviewer M11 2 (Question-Restating Synthesis, Sources Block & RAG Retrieval Review)

## 2026-11-02T15:14:43Z
You are Reviewer M11 2 (teamwork_reviewer).
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m11_2\
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md, d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md, d:\harfile\QuorumRAG\.agents\teamwork\worker_m11_1\handoff.md, and d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m11_2\DISPATCH.md immediately.

Review Milestone M11 implementation in `retrieve.rs` and `offline_engine.rs`.
Verify entity-guided retrieval, target document boosting (+200), affirmative question-restating synthesis, zero line numbers, zero raw file headers, and clean sources block with accuracy percentages.
Run:
`$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --test m11_langextract_tests -- test_question_restating_synthesis_no_line_numbers_and_clean_sources`
`$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator --lib offline_engine`
Write your review report and verdict (APPROVE or REQUEST_CHANGES) to d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m11_2\handoff.md and notify parent orchestrator via send_message.
