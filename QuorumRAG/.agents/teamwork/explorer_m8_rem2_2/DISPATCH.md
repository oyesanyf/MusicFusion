# Dispatch — Explorer M8 Remediation 2 (Instance 2)

## Identity & Role
- **Role**: teamwork_explorer
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_rem2_2`
- **Parent Conversation ID**: `facfe77d-94d3-44be-95df-eec2b9dfe368`

## Mandatory Reference Documents
- **`ORIGINAL_REQUEST.md`**: `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` (MANDATORY: read first)
- **`PROJECT.md`**: `d:\harfile\QuorumRAG\PROJECT.md`
- **Forensic Auditor Full Evidence Report**: `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m8_remediation_1\handoff.md`

## Audit Failure Evidence Summary
The Forensic Auditor reported INTEGRITY VIOLATION due to:
`cargo test -p quorum-coordinator --lib -- --test-threads=2` terminated with exit code 1.
Failed test: `offline_engine::tests::test_spellcheck_corrects_chat_box_typos_without_rewriting`.
Panicked at `crates\quorum-coordinator\src\offline_engine.rs:1166:9`:
```
assertion `left == right` failed
  left: "where did femi Oyesanya go to school"
 right: "where did Femi Oyesanya go to school"
```
Root Cause: In `offline_engine.rs` line 927, `EXACT_TYPO_MAP` has `("femii", "Femi")` and `("femy", "Femi")`, but omits `("femi", "Femi")`. `"femi"` is length 4 (< 6 chars), bypassing the Levenshtein distance check against `VOCABULARY`, so it remains lowercase `"femi"`.

## Exploration Task
1. Investigate the normalization loop in `correct_query_spelling` in `crates/quorum-coordinator/src/offline_engine.rs`:
   Check whether `VOCABULARY` should be checked directly for case correction (e.g. if `core_lower.eq_ignore_ascii_case(vocab_word)` -> replace with `vocab_word`), OR if `EXACT_TYPO_MAP` should include lowercase names (`("femi", "Femi")`, `("davis", "Davis")`, etc.).
2. Evaluate which approach is cleanest, zero-allocation, and robust against all test queries.
3. Deliver `handoff.md` with your recommended fix plan.
4. Do NOT modify source code files.


## 2026-11-02T05:24:18Z
From: facfe77d-94d3-44be-95df-eec2b9dfe368
Priority: MESSAGE_PRIORITY_HIGH
Content:
You are Explorer M8 Remediation 2 (Instance 2) (role: teamwork_explorer).
Your working directory is d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_rem2_2.
Your parent conversation ID is facfe77d-94d3-44be-95df-eec2b9dfe368.

MANDATORY READING:
1. Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md
2. Read d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_rem2_2\DISPATCH.md
3. Read FULL forensic auditor evidence report at d:\harfile\QuorumRAG\.agents\teamwork\auditor_m8_remediation_1\handoff.md

TASK:
1. Investigate normalization in correct_query_spelling in crates/quorum-coordinator/src/offline_engine.rs.
2. Formulate a surgical fix strategy to ensure cargo test -p quorum-coordinator --lib passes 13/13 cleanly.
3. Write your report to d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_rem2_2\handoff.md.
4. Send a message to parent facfe77d-94d3-44be-95df-eec2b9dfe368.
DO NOT modify source code files.
