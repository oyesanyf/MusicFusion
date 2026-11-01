# Handoff Report — Milestone 2 Adversarial Verification (LanceDB Storage & Tantivy FTS)

**Author**: challenger_m2_2 (Milestone 2 Adversarial Challenger)  
**Date**: 2026-10-31  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_2/`  
**Target Module**: `crates/quorum-storage/`  
**Verdict**: **REQUEST_CHANGES**

---

## 1. Observation

### 1.1 Empirical Test Execution & Results
An adversarial test suite comprising 11 stress test cases was authored and executed in `crates/quorum-storage/tests/adversarial_storage_tests.rs`:

```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
cargo test -p quorum-storage --test adversarial_storage_tests
```

**Results**: 9 PASSED, 2 FAILED (0 ignored, 11 total).

| Test Name | Dimension | Result | Empirical Note |
|---|---|:---:|---|
| `test_adversarial_tantivy_punctuation_and_query_syntax` | BM25 Parser | **PASS** | Survived hostile Lucene syntax, unbalanced quotes (`"""`), and SQL injection attempts (`' OR 1=1; DROP TABLE...`) without panic. |
| `test_adversarial_tantivy_empty_and_extreme_text_payloads` | BM25 Payloads | **PASS** | Survived 4,000-character non-whitespace tokens, high-density symbol strings, and empty/whitespace queries. |
| `test_adversarial_tantivy_non_ascii_multilingual` | BM25 UTF-8 | **PASS** | Preserved UTF-8 integrity and successfully queried CJK (`"零知识证明"`), Cyrillic (`"Квантово"`), Arabic (`"تشفير"`), French/German diacritics (`"Byzantine Übergröße"`), and Emoji (`"🦀 Rust 🚀"`). |
| `test_adversarial_rrf_pure_keyword_vs_pure_vector` | RRF Edge Case | **PASS** | Disjoint vector vs keyword hits combined cleanly without `NaN` or infinite scores; dual matches correctly placed #1. |
| `test_adversarial_rrf_opposing_vector_cosine_vs_text_matching` | RRF Edge Case | **PASS** | Cosine similarity $\le 0.0$ chunks correctly filtered from vector candidates, retaining only keyword RRF score ($1/61 \approx 0.01639$). |
| `test_adversarial_rrf_extreme_parameters_and_zero_norms` | RRF Edge Case | **PASS** | `top_k=0`, `top_k=50000`, and all-zero vector queries handled gracefully without panic. |
| `test_adversarial_compaction_repeated_empty_table` | Compaction | **PASS** | 10 consecutive compaction cycles on an empty table executed without error; storage remained operational. |
| `test_adversarial_windows_paths_deletion_resilience` | Windows Paths | **PASS** | LanceDB handled paths with single quotes and backslashes in delete SQL query (`C:\Users\O'Connor\Projects\Quorum...`). |
| `test_adversarial_compaction_concurrent_reads_stress` | Concurrency | **PASS** | 8 parallel tokio tasks executed 30 continuous search queries each while 4 compaction cycles ran in the background: 0 errors, 0 panics, 0 deadlocks. |
| `test_adversarial_compaction_after_deletions_and_reinsertions` | Compaction / Delete | **FAILED** | **CRITICAL BUG CONFIRMED**: Deleted chunks remain in Tantivy index. |
| `test_adversarial_rrf_identical_ranks_deterministic_tiebreak` | RRF Determinism | **FAILED** | **HIGH BUG CONFIRMED**: Retrieval order divergence on identical semantic payloads. |

---

### 1.2 Exact Verbatim Failure 1: Tantivy Deletion Failure (BUG-M2-01)

```
---- test_adversarial_compaction_after_deletions_and_reinsertions stdout ----

thread 'test_adversarial_compaction_after_deletions_and_reinsertions' (49296) panicked at crates\quorum-storage\tests\adversarial_storage_tests.rs:594:5:
Deleted file_b chunks must not appear in search results
```

**Inspection of Source Code (`crates/quorum-storage/src/tantivy_fts.rs`)**:
- Line 47:
  ```rust
  let field_file_path = builder.add_text_field("file_path", TEXT | STORED);
  ```
- Lines 274–276:
  ```rust
  let term = Term::from_field_text(self.field_file_path, file_path);
  let _ = writer.delete_term(term);
  commit_with_retry(&mut writer)?;
  ```
- Line 282:
  ```rust
  Ok(1)
  ```

---

### 1.3 Exact Verbatim Failure 2: Non-Deterministic Ordinal RRF Ranking (BUG-M2-02)

```
---- test_adversarial_rrf_identical_ranks_deterministic_tiebreak stdout ----

thread 'test_adversarial_rrf_identical_ranks_deterministic_tiebreak' (21728) panicked at crates\quorum-storage\tests\adversarial_storage_tests.rs:110:5:
assertion `left == right` failed
  left: "chunk_tie_beta"
 right: "chunk_tie_alpha"
```

**Inspection of Source Code (`crates/quorum-storage/src/vector_store.rs` and `engine.rs`)**:
- In `vector_store.rs:125`:
  ```rust
  all_scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
  ```
  Equal cosine similarity scores retain arbitrary RecordBatch order. No secondary tie-breaker on `chunk_id` is present.
- In `engine.rs:183–187`:
  ```rust
  for (rank_0, scored) in relevant_vector_hits.into_iter().enumerate() {
      let rrf = 1.0 / (60.0 + (rank_0 + 1) as f32);
      let entry = rrf_map.entry(scored.chunk.chunk_id.clone()).or_insert((0.0, scored.chunk));
      entry.0 += rrf;
  }
  ```
  Chunks with identical similarity receive different ordinal ranks ($1/61$ vs $1/62$), creating an artificial score divergence prior to downstream tie-breaking.

---

## 2. Logic Chain

### 2.1 Logic Chain for BUG-M2-01 (Tantivy `delete_by_file_path` Failure)
1. In Tantivy, fields declared with `TEXT` use the default `SimpleTokenizer`, which splits strings into words by non-alphanumeric characters (such as `/`, `\`, and `.`) and converts tokens to lowercase.
2. For any file path (e.g. `"dir/file_b.rs"` or `"crates/core/src/lib.rs"`), the Tantivy inverted index contains individual tokens: `["dir", "file_b", "rs"]`.
3. In `delete_by_file_path`, `Term::from_field_text(self.field_file_path, file_path)` creates an exact term lookup for the full string `"dir/file_b.rs"`.
4. Because `file_path` is `TEXT` rather than `STRING`, no term with the exact bytes `"dir/file_b.rs"` exists in the term dictionary.
5. `writer.delete_term(term)` matches 0 documents and deletes nothing from Tantivy.
6. The method returns hardcoded `Ok(1)` regardless, masking the failure from `StorageEngine`.
7. When `engine.search_text` or `engine.search_hybrid` is executed after deletion, the deleted documents are still returned by Tantivy, violating storage consistency and confidentiality boundaries.
8. The existing test `storage_tests.rs:test_storage_compaction_and_lifecycle` failed to catch this bug because it only asserted that `engine.delete_by_file_path("file_a.rs")` returned `1` (which LanceDB satisfied) and never called `search_text` to verify whether Tantivy actually deleted the documents.

### 2.2 Logic Chain for BUG-M2-02 (RRF Ordinal Rank Distortion)
1. When multiple candidate chunks have identical semantic similarity (e.g., identical cosine similarity of `1.0`), `vector_store.rs:125` sorts using `b.score.partial_cmp(&a.score)`.
2. Because Rust's `sort_by` is stable and has no secondary tie-breaker, the relative order of co-scoring chunks is governed by whichever order Arrow `RecordBatch` streams emitted the chunks.
3. In `engine.rs:search_hybrid`, `relevant_vector_hits` is enumerated ordinally: the chunk at index 0 receives $1/(60+1) = 1/61$, while the chunk at index 1 receives $1/(60+2) = 1/62$.
4. This assigns different scores to chunks with identical similarity, so `then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))` in `engine.rs:210` is bypassed because their scores are no longer equal.

---

## 3. Caveats

1. **Review-Only Constraint Maintained**: In strict accordance with the EMPIRICAL CHALLENGER role and identity constraints, no production implementation files in `crates/quorum-storage/src/` were modified by this agent.
2. **Adversarial Test Suite Placement**: In accordance with layout rules, the adversarial verification suite was placed in `crates/quorum-storage/tests/adversarial_storage_tests.rs`. No test code or data files were written into `.agents/teamwork/`.
3. **Environment Safety**: Compilation verified using process-scoped environment variables `$env:PROTOC` and `$env:PROTOC_INCLUDE` without mutating system or user PATH.

---

## 4. Conclusion & Required Remediation

Milestone 2 implementation is **REJECTED** pending required fixes (**REQUEST_CHANGES**).

### Required Changes for Worker:

1. **Remediate Tantivy Schema (`crates/quorum-storage/src/tantivy_fts.rs`)**:
   Change line 47 from:
   ```rust
   let field_file_path = builder.add_text_field("file_path", TEXT | STORED);
   ```
   To:
   ```rust
   let field_file_path = builder.add_text_field("file_path", STRING | STORED);
   ```
   `STRING` indexes the path un-tokenized, enabling `Term::from_field_text(self.field_file_path, file_path)` to match and delete documents matching the exact file path.

2. **Remediate Vector Store Tie-Breaker (`crates/quorum-storage/src/vector_store.rs`)**:
   Change line 125 from:
   ```rust
   all_scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
   ```
   To:
   ```rust
   all_scored.sort_by(|a, b| {
       b.score
           .partial_cmp(&a.score)
           .unwrap_or(std::cmp::Ordering::Equal)
           .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
   });
   ```

3. **Remediate Tantivy Deletion Count (`crates/quorum-storage/src/tantivy_fts.rs`)**:
   Avoid returning hardcoded `Ok(1)`. Query the count of matching documents prior to `delete_term` or document the return value semantics.

---

## 5. Verification Method

To independently verify and reproduce the reported findings:

1. **Run Full Adversarial Test Suite**:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
   $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo test -p quorum-storage --test adversarial_storage_tests
   ```
   *Observed Failure*:
   `test test_adversarial_compaction_after_deletions_and_reinsertions ... FAILED`
   `test test_adversarial_rrf_identical_ranks_deterministic_tiebreak ... FAILED`

2. **Run Deletion Bug Repro Alone**:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
   $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_after_deletions_and_reinsertions -- --nocapture
   ```
   *Expected Failure*:
   Panics at line 594: `Deleted file_b chunks must not appear in search results`.
