# Task Assignment: Reviewer M2 Remediation 1 — Storage Architecture & Deletion Review

## Mission
Perform comprehensive code review and verification of Milestone 2 remediation in `crates/quorum-storage/`. Produce your verdict (APPROVE or REQUEST_CHANGES) in `.agents/teamwork/reviewer_m2_remediation_1/handoff.md`.

## Mandatory Reading & Input Context
1. `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (verbatim user request)
2. `d:\harfile\QuorumRAG\PROJECT.md` (architecture, specifications)
3. `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_1\handoff.md` (Worker's remediation handoff)
4. `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m2_1\handoff.md` (Previous forensic audit report)
5. `crates/quorum-storage/src/tantivy_fts.rs`, `vector_store.rs`, `engine.rs`

## Review Checklist
- [ ] Strict Arrow encapsulation: verify zero Arrow/LanceDB types leak into public interface.
- [ ] Tantivy dual-field schema: verify `field_file_path: STRING | STORED` and `field_file_path_text: TEXT`.
- [ ] Deletion integrity: verify `delete_by_file_path` accurately queries document count and removes matching documents from Tantivy without ghost records.
- [ ] Filename search: verify `QueryParser::for_index` queries both text and path text fields.
- [ ] Run build, unit tests, and E2E storage isolation tests.

## Verification Commands
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
cargo test -p quorum-storage --test storage_tests
cargo test --manifest-path tests/e2e/Cargo.toml --test test_storage_isolation
cargo clippy --workspace --all-targets -- -D warnings
```

## Completion Criteria
Write your complete review report to `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m2_remediation_1\handoff.md`, update `progress.md`, and notify orchestrator via `send_message`.


## 2026-10-31T18:32:33Z
You are `reviewer_m2_remediation_1`, a high-reliability review agent assigned to Milestone 2 Remediation.
Your working directory is: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m2_remediation_1\`
Your task assignment is in: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m2_remediation_1\DISPATCH.md`

MANDATORY INSTRUCTIONS:
1. You MUST read `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` in full before starting.
2. Read `d:\harfile\QuorumRAG\PROJECT.md`.
3. Read the Worker's handoff report at `d:\harfile\QuorumRAG\.agents\teamwork\worker_m2_remediation_1\handoff.md`.
4. Read the previous audit report at `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m2_1\handoff.md`.
5. Verify `crates/quorum-storage/src/tantivy_fts.rs`, `vector_store.rs`, `engine.rs`.
6. Verify strict Arrow encapsulation, dual-field schema (`STRING | STORED` + `TEXT`), exact count deletion, and filename search.
7. Run the verification commands specified in `DISPATCH.md` using process-scoped environment variables:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
   $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo test -p quorum-storage --test storage_tests
   cargo test --manifest-path tests/e2e/Cargo.toml --test test_storage_isolation
   cargo clippy --workspace --all-targets -- -D warnings
   ```
8. Write your complete handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m2_remediation_1\handoff.md` with your verdict (APPROVE or REQUEST_CHANGES), and notify orchestrator (`e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`) via `send_message`.
