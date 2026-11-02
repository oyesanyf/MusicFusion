# Handoff Report: Milestone M12 Remediation

**Worker**: `worker_m12_remediation_1` (implementer, qa)  
**Parent Orchestrator ID**: `6e43b521-b448-4762-842a-0bc8d66e2854`  
**Target Milestone**: M12 Remediation (Tantivy FTS Capacity Overflow & WebUI Line Number / Citation Sanitization)  
**Commit**: `3ce1f7fa4c04054054d41a9cf958c01a07d19dc6`  
**Date**: 2026-11-02T16:48:00Z  

---

## 1. Observation

### Observation 1: Tantivy FTS Capacity Overflow Panic
- In `crates/quorum-storage/src/tantivy_fts.rs:400` (pre-remediation):
  ```rust
  let mut primary_chunks = Vec::with_capacity(top_k);
  ```
  When invoked with extreme adversarial `top_k == usize::MAX` (as tested by `tests/adversarial_stress_tests.rs:187-204` `test_adv_top_k_overflow_in_hybrid`), executing:
  ```powershell
  $env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-storage --test adversarial_stress_tests -- test_adv_top_k_overflow_in_hybrid
  ```
  produced a fatal panic in `alloc::raw_vec`:
  ```text
  thread 'test_adv_top_k_overflow_in_hybrid' panicked at /rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/library\alloc\src\raw_vec\mod.rs:28:5:
  capacity overflow
  ```
  Additionally, line 388 contained `(top_k * 8).max(400).min(num_docs.max(1))`, which risked multiplication overflow when `top_k > usize::MAX / 8`.

### Observation 2: Line Number Leakage in WebUI Markdown Rendering
- In `crates/quorum-ui/assets/app.js:664` (pre-remediation):
  ```javascript
  cleanedRaw = cleanedRaw.replace(/\(Lines?\s*\d+\s*[–-]\s*\d+\)/gi, '').replace(/:(\d+)-(\d+)/g, '');
  ```
  Running `node target/challenger_m12_stress_test.js` failed 4 cases due to:
  1. Unicode em-dash (`—` U+2014) in `(Lines 35—39)` was unhandled (regex only tested en-dash `–` and hyphen `-`).
  2. Singular line references `(Line 42)` were unhandled (regex required range `\d+\s*[–-]\s*\d+`).
  3. Colon line numbers with Unicode en-dash (`:35–39`) and em-dash (`:35—39`) were unhandled (regex only tested ASCII hyphen `-`).

### Observation 3: Citation Badge Line Number Leakage and Deduplication Failure
- In `crates/quorum-ui/assets/app.js:738-744` (pre-remediation):
  ```javascript
  const fileName = String(cit.file_path || '').split(/[\\/]/).pop() || cit.file_path;
  const cleanFileName = fileName.trim();
  if (!cleanFileName || citationsEl._seenFiles.has(cleanFileName.toLowerCase())) {
    return;
  }
  citationsEl._seenFiles.add(cleanFileName.toLowerCase());
  ```
  If `cit.file_path` contained line numbers (e.g. `Complete Assignments__.docx:35-39`), the badge HTML retained `:35-39`, and multiple citations pointing to the same document with different line ranges (e.g. `:35-39` and `:10-20`) were treated as distinct files, creating duplicate badges. Furthermore, `citationsEl._seenFiles` was re-initialized within `if (!citationsEl.querySelector('.sources-heading'))`, risking state loss across re-renders.

---

## 2. Logic Chain

1. **Storage Capacity Guard**:
   - `top_k.saturating_mul(8)` prevents integer overflow during search limit calculation.
   - Initializing `Vec::with_capacity(top_k.min(top_docs.len()))` guarantees that memory allocation is strictly bounded by the number of retrieved documents `top_docs.len()`, completely eliminating allocation overflow when `top_k == usize::MAX`.
2. **Markdown Regex Normalization**:
   - Replacing the regex with:
     ```javascript
     cleanedRaw = cleanedRaw
       .replace(/\(Lines?\s*\d+(\s*[–—\-]\s*\d+)?\)/gi, '')
       .replace(/:(\d+)\s*[–—\-]\s*(\d+)/g, '');
     ```
     matches singular `(Line 42)`, ranges with optional end bounds `(Lines 35–39)`, and all dash variants (ASCII `-`, en-dash `–`, em-dash `—`).
3. **Citation Badge Normalization & Persistent Deduplication**:
   - Applying the identical regex sanitization to `cleanFileName` before checking `citationsEl._seenFiles`:
     ```javascript
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
     ensures both badge text and `_seenFiles` lookup keys are purely canonical document names without line artifacts.
   - Initializing `citationsEl._seenFiles = new Set()` once, combined with `citationsEl._hasSourcesHeading` guard, prevents re-initializing deduplication state on subsequent badge additions.

---

## 3. Caveats

No caveats. All failure modes from `auditor_m12_1` and `challenger_m12_1` were resolved directly in production files without introducing mocks or dummy code.

---

## 4. Conclusion

Both issues identified in Milestone M12 audits have been remediated:
1. `crates/quorum-storage/src/tantivy_fts.rs`: Capacity allocation is bounded by `top_k.min(top_docs.len())` with `top_k.saturating_mul(8)`.
2. `crates/quorum-ui/assets/app.js`: Markdown and citation badges reliably strip singular/range line numbers with all dash variants, and citations deduplicate accurately by canonical filename.
3. All changes committed cleanly to git: `3ce1f7fa4c04054054d41a9cf958c01a07d19dc6`.

---

## 5. Verification Method

### 1. Storage Adversarial Tests
```powershell
$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-storage --test adversarial_stress_tests
```
**Result**: 13 passed, 0 failed (including `test_adv_top_k_overflow_in_hybrid`).

### 2. UI Test Suite
```powershell
$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-ui
```
**Result**: 16 passed, 0 failed.

### 3. Challenger Adversarial Stress Suite
```powershell
node target/challenger_m12_stress_test.js
```
**Result**: 41 passed, 0 failed.

### 4. Workspace Compilation Health
```powershell
$env:CARGO_INCREMENTAL = "0"; cargo check --workspace
```
**Result**: Clean compilation with 0 errors and 0 warnings.
