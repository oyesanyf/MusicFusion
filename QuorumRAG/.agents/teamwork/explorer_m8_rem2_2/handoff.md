# Handoff Report — Explorer M8 Remediation 2 (Normalization & Spellcheck Investigation)

**Author**: Explorer M8 Remediation 2 (`explorer_m8_rem2_2`)  
**Role**: teamwork_explorer (Read-Only Investigation)  
**Target Milestone**: M8 Remediation 2  
**Target Crate**: `crates/quorum-coordinator` (`src/offline_engine.rs`)  
**Date**: 2026-11-02T05:30:00Z  

---

## 1. Observation

### 1.1 Direct Reproduction of Test Failure
Executed command:
```powershell
cargo test -p quorum-coordinator --lib -- --test-threads=2
```
**Verbatim Output**:
```
running 13 tests
test agentic::grade::tests::test_grade_relevance_empty_chunks_fails ... ok
test agentic::grade::tests::test_grade_relevance_high_cosine_score_passes ... ok
test agentic::grade::tests::test_grade_relevance_max_hops_exhaustion_does_not_rewrite ... ok
test agentic::grade::tests::test_grade_relevance_rrf_matching_content_passes ... ok
test agentic::grade::tests::test_grade_relevance_rrf_unrelated_content_fails_and_rewrites ... ok
test cache::key::tests::test_derive_embedding_cache_key ... ok
test cache::key::tests::test_derive_query_cache_key ... ok
test cache::key::tests::test_text_normalization ... ok
test dlp_mesh::tests::test_mesh_shared_folder_enforcement ... ok
test offline_engine::tests::test_clean_chunk_text_collapses_broken_pdf_lines_and_strips_metadata ... ok
test offline_engine::tests::test_offline_engine_synthesizes_compilable_rrf_code ... ok
test offline_engine::tests::test_spellcheck_corrects_chat_box_typos_without_rewriting ... FAILED
test dlp_mesh::tests::test_phi_redaction_and_blocking_modes ... ok

failures:

---- offline_engine::tests::test_spellcheck_corrects_chat_box_typos_without_rewriting stdout ----

thread 'offline_engine::tests::test_spellcheck_corrects_chat_box_typos_without_rewriting' panicked at crates\quorum-coordinator\src\offline_engine.rs:1206:9:
assertion `left == right` failed
  left: "where did femi Oyesanya go to school"
 right: "where did Femi Oyesanya go to school"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    offline_engine::tests::test_spellcheck_corrects_chat_box_typos_without_rewriting

test result: FAILED. 12 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
error: test failed, to rerun pass `-p quorum-coordinator --lib`
```

### 1.2 Code Inspection in `crates/quorum-coordinator/src/offline_engine.rs`
- **Location 1: `EXACT_TYPO_MAP` (lines 961–975)**:
  ```rust
  961:             ("angla", "Angela"),
  962:             ("anglea", "Angela"),
  963:             ("angelaa", "Angela"),
  964:             ("anjela", "Angela"),
  965:             ("davs", "Davis"),
  966:             ("dvis", "Davis"),
  967:             ("femii", "Femi"),
  968:             ("femy", "Femi"),
  969:             ("oyesnya", "Oyesanya"),
  970:             ("oyesana", "Oyesanya"),
  971:             ("oyesany", "Oyesanya"),
  972:             ("oyesania", "Oyesanya"),
  973:             ("oyesnaya", "Oyesanya"),
  974:             ("oyesanay", "Oyesanya"),
  975:             ("cinga", "Cigna"),
  ```
  `EXACT_TYPO_MAP` includes typo variants `("femii", "Femi")` and `("femy", "Femi")`, but omits lowercase canonical `("femi", "Femi")`.

- **Location 2: Token Matching & Levenshtein Loop (lines 1039–1060)**:
  ```rust
  1039:             for &(typo, fixed) in EXACT_TYPO_MAP {
  1040:                 if core_lower == typo {
  1041:                     replacement = Some(fixed.to_string());
  1042:                     break;
  1043:                 }
  1044:             }
  1045: 
  1046:             // If not in exact typo map and word is >= 6 chars, check edit distance 1 against VOCABULARY
  1047:             if replacement.is_none() && core_lower.len() >= 6 {
  1048:                 let already_valid = VOCABULARY
  1049:                     .iter()
  1050:                     .any(|v| v.eq_ignore_ascii_case(&core_lower));
  1051:                 if !already_valid {
  1052:                     for &vocab_word in VOCABULARY {
  ...
  1059:                     }
  1060:                 }
  1061:             }
  ```
  When the token is `"femi"`:
  1. `core_lower == "femi"` does not match any entry in `EXACT_TYPO_MAP`. `replacement` remains `None`.
  2. `core_lower.len() == 4 < 6`, so line 1047 evaluates to `false` and skips the Levenshtein block.
  3. Even if `len() >= 6` were not present, line 1048 evaluates `already_valid = true` because `"Femi"` exists in `VOCABULARY`. Line 1051 (`if !already_valid`) thus guards the inner loop, ensuring words in `VOCABULARY` are never modified by the edit-distance block.
  4. As a result, `"femi"` remains unchanged as lowercase `"femi"`.

- **Location 3: Test Assertion (lines 1202–1207)**:
  ```rust
  1202:         let (fixed2, _) = DeterministicOfflineSynthesisEngine::correct_query_spelling(
  1203:             "wher did femi oyesnya go to schol",
  1204:             false,
  1205:         );
  1206:         assert_eq!(fixed2, "where did Femi Oyesanya go to school");
  ```
  The test expects `"femi"` to be capitalized to `"Femi"`.

---

## 2. Logic Chain

1. **Test Failure Mechanism**: In `test_spellcheck_corrects_chat_box_typos_without_rewriting`, the input query contains `"wher did femi oyesnya go to schol"`.
   - `"wher"` matches `("wher", "where")` in `EXACT_TYPO_MAP`.
   - `"did"` is a common word, left untouched.
   - `"femi"` is lowercase.
   - `"oyesnya"` matches `("oyesnya", "Oyesanya")` in `EXACT_TYPO_MAP`.
   - `"go"` and `"to"` are left untouched.
   - `"schol"` matches `("schol", "school")` in `EXACT_TYPO_MAP`.
   The actual result is `"where did femi Oyesanya go to school"`, causing the assertion against `"where did Femi Oyesanya go to school"` to panic on `"femi"` vs `"Femi"`.

2. **Architectural Intent of `EXACT_TYPO_MAP` vs `VOCABULARY`**:
   - `EXACT_TYPO_MAP` (lines 926–1000) is the deterministic map for exact typo corrections and canonical proper noun casing. It explicitly maps improper casing and typos:
     `("angla", "Angela")`, `("davs", "Davis")`, `("femii", "Femi")`, `("femy", "Femi")`, `("oyesnya", "Oyesanya")`, `("cinga", "Cigna")`, `("westrn", "Western")`, `("govenors", "Governors")`.
   - `VOCABULARY` (lines 1002–1011) is solely a candidate list for fuzzy Levenshtein distance-1 checks on words of length $\ge 6$. It is explicitly gated by `if !already_valid`, meaning words already in `VOCABULARY` (case-insensitively) are considered valid and intentionally bypassed.

3. **Evaluation of Remediation Approaches**:
   - **Approach A: Add Lowercase Names to `EXACT_TYPO_MAP`**
     - Add `("femi", "Femi")` (and canonical domain names `("davis", "Davis")`, `("cigna", "Cigna")`, `("angela", "Angela")`, `("oyesanya", "Oyesanya")`, `("patientpoint", "PatientPoint")`).
     - **Complexity**: $O(1)$ change, zero new allocations, zero algorithmic branching changes.
     - **Side Effects**: **Zero**. Does not alter common words, does not downcase capitalized sentence starters, and does not alter words outside the table.
     - **Performance**: Zero overhead; `EXACT_TYPO_MAP` is a compile-time `const &[(&str, &str)]`.
   - **Approach B: Dynamic Case Normalization Against `VOCABULARY` in the Token Loop**
     - Iterate `VOCABULARY` for proper nouns: `if vocab_word.chars().any(char::is_ascii_uppercase) && vocab_word.eq_ignore_ascii_case(&core_lower) { replacement = Some(vocab_word.to_string()); }`.
     - **Hazards**:
       1. In `VOCABULARY` (line 1009), the word `"Transcript"` has an uppercase `'T'`. Under Approach B, any common query with `"transcript"` (e.g., `"request college transcript"`) would unexpectedly be forced to capitalized `"Transcript"`.
       2. `"Western"` (line 1007) would cause lowercase `"western"` (as in `"western boundary"`) to be forced to capitalized `"Western"`.
       3. Iterating `VOCABULARY` on every token adds quadratic string comparison overhead for every word in every user prompt.

4. **Recommendation**:
   Approach A is strictly superior. It aligns 100% with the existing architecture, is zero-allocation, eliminates all unintended capitalization side effects, and directly resolves the test failure.

---

## 3. Caveats

- **Read-Only Scope**: In strict accordance with explorer role guidelines, no source code was edited during this investigation.
- **Affected Surface**: The defect and its fix are completely isolated to `crates/quorum-coordinator/src/offline_engine.rs`.
- **Zero Impact on Invariants**: Neither Approach A nor the test failure affects:
  - Phase 1 Crypto Invariants (Ed25519 dalek signing/verification, BLAKE3 deduplication),
  - Loopback/RFC1918 dial restrictions,
  - Merkle / Multi-term precision retrieval (`test_adv_multi_term_entity_matching_precision`),
  - Arrow type isolation or environment safety.

---

## 4. Conclusion

The sole cause of the failure in `cargo test -p quorum-coordinator --lib` is the omission of `("femi", "Femi")` from `EXACT_TYPO_MAP` in `crates/quorum-coordinator/src/offline_engine.rs`.

### Surgical Fix Proposal (For Implementer / Worker)
Target file: `crates/quorum-coordinator/src/offline_engine.rs`  
Location: Lines 961–975 (inside `const EXACT_TYPO_MAP: &[(&str, &str)] = &[ ... ]`)

#### Proposed Diff:
```diff
--- a/crates/quorum-coordinator/src/offline_engine.rs
+++ b/crates/quorum-coordinator/src/offline_engine.rs
@@ -961,12 +961,16 @@ impl DeterministicOfflineSynthesisEngine {
+            ("angela", "Angela"),
             ("angla", "Angela"),
             ("anglea", "Angela"),
             ("angelaa", "Angela"),
             ("anjela", "Angela"),
+            ("davis", "Davis"),
             ("davs", "Davis"),
             ("dvis", "Davis"),
+            ("femi", "Femi"),
             ("femii", "Femi"),
             ("femy", "Femi"),
+            ("oyesanya", "Oyesanya"),
             ("oyesnya", "Oyesanya"),
             ("oyesana", "Oyesanya"),
             ("oyesany", "Oyesanya"),
@@ -975,6 +979,7 @@ impl DeterministicOfflineSynthesisEngine {
             ("oyesnaya", "Oyesanya"),
             ("oyesanay", "Oyesanya"),
+            ("cigna", "Cigna"),
             ("cinga", "Cigna"),
             ("cigana", "Cigna"),
```

Adding these entries guarantees that:
1. `"wher did femi oyesnya go to schol"` deterministically normalizes to `"where did Femi Oyesanya go to school"`.
2. All 13/13 tests in `cargo test -p quorum-coordinator --lib` pass with exit code 0.
3. Domain names (`Femi`, `Oyesanya`, `Davis`, `Angela`, `Cigna`) are consistently canonicalized when typed in lowercase by users in the chat box.

---

## 5. Verification Method

To independently verify after the implementer applies the surgical fix:

1. **Coordinator Unit Tests (Must pass 13/13 cleanly)**:
   ```powershell
   cargo test -p quorum-coordinator --lib -- --test-threads=2
   ```
   *Expected Result*: 13 passed; 0 failed; exit code 0. Specifically, `offline_engine::tests::test_spellcheck_corrects_chat_box_typos_without_rewriting` passes.

2. **Adversarial Challenge M8.2 Test Suite**:
   ```powershell
   cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2
   ```
   *Expected Result*: 6 passed; 0 failed; exit code 0.

3. **P2P Consensus & Latency Test Suite**:
   ```powershell
   cargo test -p quorum-coordinator --test m8_p2p_consensus_latency_tests -- --test-threads=2
   ```
   *Expected Result*: 6 passed; 0 failed; exit code 0.

4. **Workspace Type Check**:
   ```powershell
   cargo check --workspace
   ```
   *Expected Result*: Finished `dev` profile [unoptimized] with exit code 0.

**Invalidation Condition**: If `cargo test -p quorum-coordinator --lib` exits with code 1 or any of the 13 tests fail, this analysis is invalidated.
