# Task Assignment: Explorer M2 Remediation 1 — Tantivy Schema & Deletion Integrity

## Mission
Investigate and formulate the exact remediation strategy for Tantivy FTS deletion failure (BUG-M2-01) and filename keyword search in `crates/quorum-storage/src/tantivy_fts.rs`. Do NOT implement code directly. Produce a structured investigation report in `.agents/teamwork/explorer_m2_remediation_1/handoff.md`.

## Mandatory Reading & Input Context
You MUST read these authoritative files in full before beginning:
1. `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (verbatim user request)
2. `d:\harfile\QuorumRAG\PROJECT.md` (architecture, Milestone 2 specifications)
3. `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m2_1\handoff.md` (FULL UNFILTERED FORENSIC AUDIT EVIDENCE)
4. `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m2_2\handoff.md` (Reviewer 2 finding on `field_file_path`)
5. `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_2\handoff.md` (Challenger 2 empirical reproduction of deletion failure)
6. `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_1\handoff.md` (Challenger 1 findings on deletion & filename search)
7. `crates/quorum-storage/src/tantivy_fts.rs` (source code under investigation)

## Specific Investigation Focus
1. **Tantivy Field Schema (`tantivy_fts.rs:47`)**:
   - Why `builder.add_text_field("file_path", TEXT | STORED)` tokenizes path strings (`/`, `\`, `.`) into word tokens.
   - Why `Term::from_field_text(self.field_file_path, file_path)` in `delete_by_file_path` fails to match the exact path in the term dictionary.
   - Why changing to `STRING | STORED` indexes the path as an untokenized string literal and allows exact term matching for deletion.
2. **Deletion Return Value (`tantivy_fts.rs:282`)**:
   - `delete_by_file_path` returns hardcoded `Ok(1)`, creating a facade of successful deletion.
   - Formulate how to return the actual count of deleted documents or query matching docs beforehand.
3. **Filename Keyword Search**:
   - Challenger 1 noted `tantivy_fts.rs:146` only queries `field_text_content`. Investigate whether a tokenized filename/path field should be added alongside `field_file_path` (or how `QueryParser::for_index` can search both text and file path) so users can search by filename while preserving `STRING | STORED` for deletion.
4. **Remediation Strategy**:
   - Provide exact line-by-line recommendations for the Worker.
   - Specify exact cargo test commands to verify the fix without regressions.

## Completion Criteria
Write your complete analysis and remediation plan to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_1\handoff.md`, update `progress.md`, and notify orchestrator via `send_message`.


## 2026-10-31T17:44:00Z
Received dispatch from e13403c5-b0e6-48a4-a1ec-a1f096f9f53e:
You are `explorer_m2_remediation_1`, a read-only exploration agent assigned to Milestone 2 Remediation.
Your working directory is: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_1\`
Your task description is located at: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_1\DISPATCH.md`

MANDATORY INSTRUCTIONS:
1. You MUST read `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` in full before starting.
2. Read `d:\harfile\QuorumRAG\PROJECT.md`.
3. Read the FULL UNFILTERED FORENSIC AUDIT EVIDENCE in `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m2_1\handoff.md`.
4. Read `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m2_2\handoff.md`, `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_2\handoff.md`, and `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_1\handoff.md`.
5. Focus: Tantivy FTS Schema & Deletions in `crates/quorum-storage/src/tantivy_fts.rs`. Explain why `TEXT | STORED` fails on file paths, why `STRING | STORED` fixes it, how to handle the deletion count return value, and how filename keyword search should be wired.
6. Do NOT implement code directly. Produce a structured investigation and remediation plan in `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_1\handoff.md`.
7. Once finished, send a message to orchestrator (`e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`) with your key findings and handoff path.
