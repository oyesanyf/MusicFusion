## 2026-11-02T16:25:44Z
You are worker_m12_remediation_1, an implementation worker.
Working directory: d:\harfile\QuorumRAG\.agents\teamwork\worker_m12_remediation_1\
Parent Conversation ID: 6e43b521-b448-4762-842a-0bc8d66e2854

MANDATORY FIRST STEPS:
1. Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md.
2. Read d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md.
3. Read d:\harfile\QuorumRAG\.agents\teamwork\auditor_m12_1\handoff.md.
4. Read d:\harfile\QuorumRAG\.agents\teamwork\challenger_m12_1\handoff.md.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

OWNED FILES:
You exclusively own:
- crates/quorum-storage/src/tantivy_fts.rs
- crates/quorum-ui/assets/app.js

TASKS:
1. Fix Tantivy FTS Capacity Overflow on Extreme top_k (`crates/quorum-storage/src/tantivy_fts.rs`):
   - At line 400: change `let mut primary_chunks = Vec::with_capacity(top_k);` to:
     `let mut primary_chunks = Vec::with_capacity(top_k.min(top_docs.len()));`
   - At line 388: change `(top_k * 8)` to:
     `top_k.saturating_mul(8)`
   - Run verification:
     `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-storage --test adversarial_stress_tests` (must pass 13/13 tests, especially `test_adv_top_k_overflow_in_hybrid`).

2. Fix Line-Number Leakage and Citation Deduplication (`crates/quorum-ui/assets/app.js`):
   - In `renderMarkdownHtml` (around line 664):
     Replace:
     ```javascript
     cleanedRaw = cleanedRaw.replace(/\(Lines?\s*\d+\s*[–-]\s*\d+\)/gi, '').replace(/:(\d+)-(\d+)/g, '');
     ```
     With Unicode-aware regex matching singular lines, ranges, and all dash variants (`-`, `–` U+2013, `—` U+2014):
     ```javascript
     cleanedRaw = cleanedRaw
       .replace(/\(Lines?\s*\d+(\s*[–—\-]\s*\d+)?\)/gi, '')
       .replace(/:(\d+)\s*[–—\-]\s*(\d+)/g, '');
     ```
   - In `appendCitation` (around lines 738-744):
     Strip line numbers from `cleanFileName` before checking `citationsEl._seenFiles` and creating the badge:
     ```javascript
     const fileName = String(cit.file_path || '').split(/[\\/]/).pop() || cit.file_path;
     let cleanFileName = fileName.trim();
     cleanFileName = cleanFileName
       .replace(/\(Lines?\s*\d+(\s*[–—\-]\s*\d+)?\)/gi, '')
       .replace(/:(\d+)\s*[–—\-]\s*(\d+)/g, '')
       .trim();
     if (!cleanFileName || citationsEl._seenFiles.has(cleanFileName.toLowerCase())) {
       return;
     }
     citationsEl._seenFiles.add(cleanFileName.toLowerCase());
     ```
   - Run verification:
     `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-ui` (must pass 16/16 tests).
     `node target/challenger_m12_stress_test.js` (must pass 41/41 tests).

3. Verify compiler and workspace health:
   `$env:CARGO_INCREMENTAL = "0"; cargo check --workspace`

4. Stage and commit immediately (`NO COMMIT SHOULD WAIT`):
   `git add crates/quorum-storage/src/tantivy_fts.rs crates/quorum-ui/assets/app.js`
   `git commit -m "fix(storage,ui): fix top_k capacity overflow and sanitize em-dash line references and citation file paths"`

OUTPUT:
Write your full report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m12_remediation_1\handoff.md`.
Send completion message to Parent `6e43b521-b448-4762-842a-0bc8d66e2854` using `send_message`.
