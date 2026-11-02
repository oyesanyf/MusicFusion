# Forensic Integrity Audit Report: Milestone M12

**Auditor**: Auditor M12 1 (`auditor_m12_1`, forensic_auditor)  
**Parent Orchestrator ID**: `6e43b521-b448-4762-842a-0bc8d66e2854`  
**Target Milestone**: M12 (WebUI Clean Interface, Dynamic Spellcheck, Action Buttons & Server ChatRequest Fix)  
**Target Commit**: `db03bfe2165251015b137eb77704deefd5e83f62`  
**Date**: 2026-11-02T16:23:00Z  

---

## Forensic Audit Report

**Work Product**: Milestone M12 implementation in `db03bfe` and current working tree  
**Profile**: General Project  
**Integrity Mode**: Benchmark Mode / Production Code Only  
**Verdict**: **INTEGRITY VIOLATION**

### Phase Results
- **Phase 1: Source Code Analysis & Zero Mocks Verification**: PASS — `server.rs:115`, `app.js`, `styles.css`, `index.html`, and `chat.rs` contain authentic production implementations. Zero hardcoded mocks, fake personas, or simulated placeholders detected.
- **Phase 2: Zero Line Numbers Invariant**: PASS — Line numbers are genuinely excluded and stripped from DOM nodes, badge innerHTML, and titles. Zero CSS trickery (`opacity:0`, `font-size:0`, `display:none`, `color:transparent`, `visibility:hidden`, `text-indent:-9999px`) is used to hide line numbers.
- **Phase 3: Universal Zero-Touch PATH & Environment Protection**: PASS — Zero registry mutations (`HKCU\Environment`, `HKLM\SYSTEM`), zero persistent PATH overwrites (`setx`, `reg add`), and all compilation/test commands enforce process-scoped `$env:CARGO_INCREMENTAL = "0"`.
- **Phase 4: Git Commit Hygiene**: PASS — Commit `db03bfe` is recorded in git history. `git diff db03bfe -- crates/` is 100% clean with zero uncommitted changes in project code.
- **Phase 5: Behavioral Verification & Test Suite Execution**: **FAIL** — `cargo test -p quorum-storage --test adversarial_stress_tests` failed with a `capacity overflow` panic in `test_adv_top_k_overflow_in_hybrid`. The failure was introduced directly in commit `db03bfe` in `crates/quorum-storage/src/tantivy_fts.rs:400` (`Vec::with_capacity(top_k)`).

---

## 1. Observation

### Observation 1: Behavioral Verification Failure in `quorum-storage` Test Suite
- Executing `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-storage --test adversarial_stress_tests -- test_adv_top_k_overflow_in_hybrid` produces verbatim failure:
  ```text
  running 1 test
  test test_adv_top_k_overflow_in_hybrid ... FAILED

  failures:

  ---- test_adv_top_k_overflow_in_hybrid stdout ----

  thread 'test_adv_top_k_overflow_in_hybrid' (66372) panicked at /rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/library\alloc\src\raw_vec\mod.rs:28:5:
  capacity overflow
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

  failures:
      test_adv_top_k_overflow_in_hybrid

  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.18s
  ```

### Observation 2: Code Origin of the Failure in Commit `db03bfe`
- Inspecting `git diff db03bfe~1 db03bfe -- :/:QuorumRAG/crates/quorum-storage/src/tantivy_fts.rs` shows that commit `db03bfe` modified `crates/quorum-storage/src/tantivy_fts.rs`:
  ```rust
  @@ -383,20 +384,70 @@ impl TantivyFtsIndex {
  ...
  -        let effective_limit = top_k.min(num_docs.max(1));
  +        // Over-fetch when the index is large so monolithic dumps (chat.html, conversations-*.json)
  +        // or source code files never crowd out real PDF/DOCX documents before top_k truncation.
  +        let fetch_limit = if num_docs > 500 {
  +            (top_k * 8).max(400).min(num_docs.max(1))
  +        } else {
  +            top_k.min(num_docs.max(1))
  +        };
           let top_docs = searcher
  -            .search(&combined, &TopDocs::with_limit(effective_limit).order_by_score())
  +            .search(&combined, &TopDocs::with_limit(fetch_limit).order_by_score())
               .map_err(|e| StorageError::Tantivy(format!("Search execution failed: {}", e)))?;
   
  -        let mut scored_chunks = Vec::with_capacity(top_docs.len());
  +        let mut primary_chunks = Vec::with_capacity(top_k);
  ```
- Line 400 (`let mut primary_chunks = Vec::with_capacity(top_k);`) attempts to allocate capacity for `top_k` elements directly.
- In `tests/adversarial_stress_tests.rs:187-204`, `test_adv_top_k_overflow_in_hybrid` queries `search_text("content", 0, usize::MAX)`.
- When `top_k == usize::MAX`, allocating `Vec::with_capacity(usize::MAX)` causes `capacity overflow` panic inside `alloc::raw_vec::RawVec`.
- Prior to commit `db03bfe`, line 391 used `Vec::with_capacity(top_docs.len())`, which is bounded by `num_docs` and never panics.
- In addition, line 388 `(top_k * 8)` risks debug arithmetic multiplication overflow when `top_k > usize::MAX / 8` and `num_docs > 500`.

### Observation 3: Milestone M12 UI & Daemon Modules
- **`server.rs:115`**: Genuine struct initialization `bypass_cache: false`, compiling cleanly and passing `test_chat_stream_compat_get_endpoint`.
- **Dynamic Spellcheck**: `app.js:133-203` and `chat.rs:228-249` provide non-destructive cursor preservation, debouncing (80ms/320ms), and genuine Ollama/Levenshtein processing.
- **Action Buttons**: `app.js:845-948` provides browser-style Copy (clipboard API), Regenerate (`bypassCache: true`), Read Aloud (`window.speechSynthesis`), and Thumbs Up/Down.
- **Logo Retention**: `#welcome-banner` anchored and forced with `display: block !important;` in `styles.css`.
- **Zero Line Numbers**: `appendCitation` in `app.js` and `renderMarkdownHtml` omit and strip all line numbers without CSS hiding.
- **Environment Protection**: Zero registry mutations (`HKCU\Environment`, `HKLM\SYSTEM`) and process-scoped `$env:CARGO_INCREMENTAL = "0"`.

---

## 2. Logic Chain

1. Ground-truth constraint from user instructions:
   "Behavioral Verification: Build the project from source and run its test suite. The build must succeed and tests must execute — a project that doesn't build or whose tests don't run is automatically flagged."
   "Block on failure: If ANY check fails, the verdict is INTEGRITY VIOLATION and the work product must be rejected."
2. Commit `db03bfe` touched `crates/quorum-storage/src/tantivy_fts.rs` and replaced `Vec::with_capacity(top_docs.len())` with `Vec::with_capacity(top_k)`.
3. Executing `cargo test -p quorum-storage --test adversarial_stress_tests -- test_adv_top_k_overflow_in_hybrid` fails with panic: `capacity overflow`.
4. Because commit `db03bfe` introduced this test breakage into the codebase, the work product cannot be approved as clean.
5. In accordance with the auditor mandate, the auditor does not modify implementation code directly, but reports the finding and issues an **INTEGRITY VIOLATION** verdict to reject the work product.

---

## 3. Caveats

- The WebUI, logo banner, dynamic spellchecking, response action buttons, and `server.rs:115` fix themselves meet all functional requirements. The violation is strictly due to the regression introduced in `crates/quorum-storage/src/tantivy_fts.rs:400` within commit `db03bfe`.

---

## 4. Conclusion

The work product in commit `db03bfe` is **REJECTED** with verdict **INTEGRITY VIOLATION** due to a behavioral test failure in `quorum-storage`.

### Required Worker Remediation
In `crates/quorum-storage/src/tantivy_fts.rs`:
1. Line 400: Change `let mut primary_chunks = Vec::with_capacity(top_k);` to:
   ```rust
   let mut primary_chunks = Vec::with_capacity(top_k.min(top_docs.len()));
   ```
2. Line 388: Change `(top_k * 8)` to:
   ```rust
   let fetch_limit = if num_docs > 500 {
       top_k.saturating_mul(8).max(400).min(num_docs.max(1))
   } else {
       top_k.min(num_docs.max(1))
   };
   ```
3. Re-run `cargo test -p quorum-storage --test adversarial_stress_tests` to confirm 13 passed, 0 failed.

---

## 5. Verification Method

To independently reproduce the failure:
```powershell
$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-storage --test adversarial_stress_tests -- test_adv_top_k_overflow_in_hybrid
```
Expected output:
```text
test test_adv_top_k_overflow_in_hybrid ... FAILED
thread 'test_adv_top_k_overflow_in_hybrid' panicked at 'capacity overflow'
```
