# Adversarial Verification Report: UI Assets, Citations & Action Buttons (Milestone M12)

**Challenger**: Challenger M12 1 (`teamwork_challenger`)  
**Parent Orchestrator ID**: `6e43b521-b448-4762-842a-0bc8d66e2854`  
**Milestone**: M12 (WebUI Clean Interface, Dynamic Spellcheck, Action Buttons & Citations)  
**Date**: 2026-11-02T16:03:00Z  
**Verdict**: **`REQUEST_CHANGES`**

---

## 1. Observation

1. **Rust Test Suite Execution (`cargo test -p quorum-ui`)**:
   - Running `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-ui` executed 16 tests across 3 suites:
     - `tests/adversarial_ui_tests.rs`: 8 passed, 0 failed.
     - `tests/m9_ui_cancellation_and_progress_tests.rs`: 4 passed, 0 failed.
     - `tests/ui_tests.rs`: 4 passed, 0 failed.
   - Result: 16 passed, 0 failed in 0.06s.

2. **Empirical Adversarial Test Suite Execution (`target/challenger_m12_stress_test.js`)**:
   - Executing `node target/challenger_m12_stress_test.js` ran 41 automated stress test cases directly against production `crates/quorum-ui/assets/app.js` and `styles.css`.
   - Result: **32 passed, 9 failed**.

3. **Line Number Leakage in `renderMarkdownHtml` (`crates/quorum-ui/assets/app.js:664`)**:
   - In `app.js:664`:
     ```javascript
     // Strip line number references e.g. (Lines 35–39), (Lines 1-10), :35-39 from prose body
     cleanedRaw = cleanedRaw.replace(/\(Lines?\s*\d+\s*[–-]\s*\d+\)/gi, '').replace(/:(\d+)-(\d+)/g, '');
     ```
   - **Leak 1 (Unicode em-dash `—` U+2014)**:
     - Input: `"📄 Complete Assignments__.docx (Lines 35—39) details his leadership."`
     - Verbatim Output: `"<p style=\"margin:0 0 10px 0;line-height:1.65;\">📄 Complete Assignments__.docx (Lines 35—39) details his leadership.</p>"`
     - `/\(Lines?\s*\d+\s*[–-]\s*\d+\)/gi` failed to match em-dash `—`.
   - **Leak 2 (Singular line reference `(Line X)`)**:
     - Input: `"Femi verified compliance in audit (Line 42)."`
     - Verbatim Output: `"<p style=\"margin:0 0 10px 0;line-height:1.65;\">Femi verified compliance in audit (Line 42).</p>"`
     - The regex requires `\d+\s*[–-]\s*\d+`, failing to match single line numbers.
   - **Leak 3 (Colon with Unicode en-dash `:35–39` U+2013)**:
     - Input: `"Sources [📄 Complete Assignments__.docx:35–39](http://127.0.0.1:3031/#)"`
     - Verbatim Output: `"<p style=\"margin:0 0 10px 0;line-height:1.65;\">Sources [📄 Complete Assignments__.docx:35–39](http://127.0.0.1:3031/#)</p>"`
     - `replace(/:(\d+)-(\d+)/g, '')` matches only ASCII hyphen `-` (`\x2D`), completely failing on Unicode en-dash `–`.
   - **Leak 4 (Colon with Unicode em-dash `:35—39` U+2014)**:
     - Input: `"Sources [📄 Complete Assignments__.docx:35—39](http://127.0.0.1:3031/#)"`
     - Verbatim Output: `"<p style=\"margin:0 0 10px 0;line-height:1.65;\">Sources [📄 Complete Assignments__.docx:35—39](http://127.0.0.1:3031/#)</p>"`
     - Fails to match Unicode em-dash `—`.

4. **Line Number Leakage & Deduplication Failure in `appendCitation` (`crates/quorum-ui/assets/app.js:738-743`)**:
   - In `app.js:738-743`:
     ```javascript
     const fileName = String(cit.file_path || '').split(/[\\/]/).pop() || cit.file_path;
     const cleanFileName = fileName.trim();
     if (!cleanFileName || citationsEl._seenFiles.has(cleanFileName.toLowerCase())) {
       return;
     }
     citationsEl._seenFiles.add(cleanFileName.toLowerCase());
     ```
   - When a citation object arrives with line numbers embedded in `file_path`:
     - Input 1: `{ file_path: 'C:\\docs\\Complete Assignments__.docx:35-39', score: 0.95 }`
     - Input 2: `{ file_path: 'Complete Assignments__.docx (Lines 35–39)', score: 0.95 }`
     - Input 3: `{ file_path: 'C:\\docs\\Complete Assignments__.docx:10-20', score: 0.92 }`
   - Verbatim Badge HTML generated:
     - `📄 Complete Assignments__.docx:35-39 <span class="score-pill" ...>95%</span>`
     - `📄 Complete Assignments__.docx (Lines 35–39) <span class="score-pill" ...>95%</span>`
   - Verbatim Deduplication Result:
     - `seenFiles` records `'complete assignments__.docx:35-39'` and `'complete assignments__.docx:10-20'`.
     - Result: 2 separate badges were rendered for the same document instead of 1 deduplicated badge.

5. **Action Buttons Verification (`crates/quorum-ui/assets/app.js:845-948`)**:
   - **Copy Button**: Correctly checks `navigator.clipboard && navigator.clipboard.writeText` and provides fallback via `document.createElement('textarea')` + `document.execCommand('copy')`. Rejections and errors are captured cleanly in `catch (_e) {}` without uncaught promise exceptions.
   - **Regenerate Button**: Correctly guards against rapid clicking collisions with `if (isGenerating || !turnPrompt) return;`.
   - **Read Aloud Button**: Correctly verifies `'speechSynthesis' in window`, toggles `window.speechSynthesis.cancel()`, ignores whitespace-only text, and hooks both `utterance.onend` and `utterance.onerror` to restore button state.
   - **Feedback Thumbs**: Mutually exclusive toggle states implemented cleanly.
   - **Spellcheck API**: Handled with `AbortController` 1800ms timeout, non-200 HTTP check (`if (!res.ok) return trimmed;`), `try...catch` wrapper, cursor preservation (`setSelectionRange`), and trailing whitespace preservation.

6. **`#welcome-banner` Visibility Invariant (`crates/quorum-ui/assets/styles.css` & `app.js`)**:
   - In `styles.css:459-469`:
     ```css
     #welcome-banner,
     .welcome-banner {
       display: block !important;
       text-align: center;
       padding: 40px 20px 24px 20px;
     }

     #welcome-banner.hidden,
     .welcome-banner.hidden {
       display: block !important;
     }
     ```
   - Specificity of `#welcome-banner.hidden` is `(1, 1, 0)`, which strictly overrides `.hidden { display: none !important; }` `(0, 1, 0)`.
   - In `app.js`: Zero calls to `welcomeBanner.classList.add('hidden')` or `welcomeBanner.style.display = 'none'`. All references (`lines 120-121, 259-260, 3380-3381`) invoke `welcomeBanner.style.display = ''` and `welcomeBanner.classList.remove('hidden')`.

---

## 2. Logic Chain

1. **User Requirement**:
   - ORIGINAL_REQUEST §3 explicitly specifies:
     > "NO Line Numbers Anywhere: Remove `(Lines X–Y)` and `:line_start-line_end` completely from both the synthesized response body AND the bottom Sources citation badges in `app.js` (`appendCitation`)."
     > "Source Only + Accuracy Score at the Bottom of Response: Only source document name/link and accuracy/relevance percentage (e.g. `📄 Femi_Oyesanya_CV_2026.pdf · 96%`), deduplicated by file name."

2. **Defect in Markdown Sanitization**:
   - Observation 3 proves that line numbers with em-dashes (`—`), singular line markers (`(Line X)`), and colon line numbers formatted with Unicode en-dashes/em-dashes (`:35–39`, `:35—39`) bypass `renderMarkdownHtml` and leak directly into rendered paragraphs and headings.

3. **Defect in Citation Normalization & Deduplication**:
   - Observation 4 proves that `appendCitation` does not sanitize `cleanFileName`.
   - If `cit.file_path` contains `:35-39` or `(Lines 35–39)`:
     - The line numbers are directly interpolated into the badge text: `📄 Complete Assignments__.docx:35-39 95%`.
     - Deduplication via `_seenFiles.has(cleanFileName.toLowerCase())` evaluates different line spans as distinct files, creating duplicate badges for the same document in the Sources bar.

4. **Action Buttons & Welcome Banner**:
   - Observations 5 & 6 confirm that action buttons (Copy, Regenerate, Read Aloud, Thumbs Up/Down, Spellcheck) and `#welcome-banner` meet all edge case robustness requirements.

5. **Verdict**:
   - Because the line number leakage and citation deduplication failures directly violate the user requirements in ORIGINAL_REQUEST §3, the verification verdict must be `REQUEST_CHANGES`.

---

## 3. Caveats

- **No Caveats**: All failure modes were reproduced deterministically via automated headless script execution directly on the codebase assets.

---

## 4. Conclusion

**Verdict: `REQUEST_CHANGES`**

The following two concrete fixes are required in `crates/quorum-ui/assets/app.js`:

1. **In `renderMarkdownHtml` (`crates/quorum-ui/assets/app.js:664`)**:
   Replace:
   ```javascript
   cleanedRaw = cleanedRaw.replace(/\(Lines?\s*\d+\s*[–-]\s*\d+\)/gi, '').replace(/:(\d+)-(\d+)/g, '');
   ```
   With comprehensive line number sanitization supporting singular lines, ranges, and all dash variants (`-`, `–` U+2013, `—` U+2014):
   ```javascript
   cleanedRaw = cleanedRaw
     .replace(/\(Lines?\s*\d+(\s*[–—\-]\s*\d+)?\)/gi, '')
     .replace(/:(\d+)\s*[–—\-]\s*(\d+)/g, '');
   ```

2. **In `appendCitation` (`crates/quorum-ui/assets/app.js:739-744`)**:
   Normalize `cleanFileName` before checking `_seenFiles` and rendering the badge:
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

---

## 5. Verification Method

To independently reproduce the failures:

1. Run the empirical stress test harness:
   ```powershell
   node target/challenger_m12_stress_test.js
   ```
   *Observed Failure*:
   ```text
   [FAIL] Strip em-dash (Lines 35—39)
   [FAIL] Strip singular line reference (Line 42)
   [FAIL] Strip colon with Unicode en-dash :35–39
   [FAIL] Strip colon with Unicode em-dash :35—39
   [FAIL] appendCitation: cleanFileName excludes :35-39
   [FAIL] appendCitation: cleanFileName excludes (Lines 35–39)
   [FAIL] appendCitation deduplicates same file across line variations
   ```

2. Run the Rust test suite:
   ```powershell
   $env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-ui
   ```
   *Expected*: 16 passed; 0 failed.
