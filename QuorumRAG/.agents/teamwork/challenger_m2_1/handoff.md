# Handoff Report — Milestone 2 Adversarial Verification (LanceDB Vector Storage & Tantivy FTS)

**Author**: Challenger 1 (Milestone 2 Adversarial Verifier)  
**Date**: 2026-10-31  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_1/`  
**Target Module**: `crates/quorum-storage/`  
**Verdict**: **REQUEST_CHANGES**

---

## 1. Observation

Adversarial stress tests were designed, implemented, and executed in `crates/quorum-storage/tests/adversarial_stress_tests.rs`. Verification revealed 2 critical bugs and 2 architectural deficiencies:

### Observation 1.1: Integer Overflow Panic on Extreme `top_k` in Hybrid Search
- **Location**: `crates/quorum-storage/src/engine.rs:171:31`
- **Code snippet**:
  ```rust
  170: pub async fn search_hybrid(&self, text: &str, vector: &[f32], max_tier: u8, top_k: usize) -> Result<Vec<ScoredChunk>, StorageError> {
  171:     let candidate_limit = top_k.max(5) * 2;
  ```
- **Execution Command**:
  ```powershell
  $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"; $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
  cargo test -p quorum-storage --test adversarial_stress_tests -- test_adv_top_k_overflow_in_hybrid --nocapture
  ```
- **Verbatim Failure Output**:
  ```
  thread '<unnamed>' (20856) panicked at crates\quorum-storage\src\engine.rs:171:31:
  attempt to multiply with overflow
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
  CONFIRMED PANIC: search_hybrid panicked on usize::MAX: attempt to multiply with overflow
  ```

### Observation 1.2: Silent Failure of `delete_by_file_path` in Tantivy (Phantom Documents & Index Drift)
- **Location**: `crates/quorum-storage/src/tantivy_fts.rs:47` and `crates/quorum-storage/src/tantivy_fts.rs:274-275`
- **Code snippet**:
  ```rust
  47:  let field_file_path = builder.add_text_field("file_path", TEXT | STORED);
  ...
  274: let term = Term::from_field_text(self.field_file_path, file_path);
  275: let _ = writer.delete_term(term);
  ```
- **Execution Command**:
  ```powershell
  $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"; $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
  cargo test -p quorum-storage --test adversarial_storage_tests -- test_adversarial_compaction_after_deletions_and_reinsertions --nocapture
  ```
- **Verbatim Failure Output**:
  ```
  thread 'test_adversarial_compaction_after_deletions_and_reinsertions' (29420) panicked at crates\quorum-storage\tests\adversarial_storage_tests.rs:594:5:
  Deleted file_b chunks must not appear in search results
  ```
- **Root Cause**: `field_file_path` is indexed as tokenized `TEXT | STORED`. Paths like `"dir/file_b.rs"` are tokenized into sub-tokens (`dir`, `file_b`, `rs`). Calling `delete_term(Term::from_field_text(self.field_file_path, "dir/file_b.rs"))` searches for an exact term matching the entire string, which does not exist in tokenized text. Thus, Tantivy deletes 0 documents, leaving deleted chunks permanently present in Tantivy search results.

### Observation 1.3: Tantivy BM25 Keyword Search Omits Filenames
- **Location**: `crates/quorum-storage/src/tantivy_fts.rs:146`
- **Code snippet**:
  ```rust
  146: let query_parser = QueryParser::for_index(&self.index, vec![self.field_text_content]);
  ```
- **Observation**: In `test_adv_filename_search_in_tantivy`, searching for a filename (e.g. `"special_handler"`) where `file_path = "crates/network/src/special_handler.rs"` and the text payload does not repeat the filename returned **0 hits**.
- **Contract Violation**: `ORIGINAL_REQUEST.md` (Follow-up 2026-10-31T16:12:21Z, Item 1) mandates: *"Keyword / Lexical Search (Tantivy BM25 exact/fuzzy keyword & filename search)"*. Because `field_file_path` is excluded from `QueryParser::for_index`, filename searches fail completely.

### Observation 1.4: Robustness & Concurrency Findings (Passing Invariants)
- **Concurrency**: `test_adv_high_concurrency_rapid_insertions_and_queries` passed cleanly with 8 concurrent worker tasks writing 200 chunks while 4 reader tasks queried simultaneously. All 200 chunks persisted without data corruption or deadlocks.
- **5MB Payload**: `test_adv_massive_chunk_payload_5mb` passed without Arrow buffer overflow or memory leaks.
- **NaN / Inf Vectors**: Insertion of chunks with `NaN` vectors was cleanly rejected by LanceDB with `StorageError::Lance("Compute error: Vector column contains NaN or null values")`. Queries with `NaN` or `Inf` were cleanly rejected with `StorageError::Lance("Invalid input, query vector must contain only finite values")`.
- **Zero Vector**: Queries with all-zero vectors `vec![0.0; 768]` returned a valid 0.0 score without NaN or panics.
- **Access Tier Isolation**: LanceDB pre-filtering and Tantivy range filters properly bound access tier queries across tiers 0..=3 and 255.

---

## 2. Logic Chain

1. **Panic Invariant Violation (Objective 3)**:
   - *Premise*: Objective 3 of Milestone 2 verification requires that `quorum-storage` *never panics*.
   - *Observation*: In `StorageEngine::search_hybrid`, `let candidate_limit = top_k.max(5) * 2;` performs unchecked multiplication.
   - *Inference*: When a caller provides `top_k = usize::MAX` (or any `top_k > usize::MAX / 2`), the multiplication overflows and panics in debug mode (`attempt to multiply with overflow`).
   - *Impact*: Any consumer querying with large bounds crashes the runtime thread.

2. **Data Integrity & Consistency Invariant Violation (Objective 3 & Follow-up Req 2)**:
   - *Premise*: Deleting a file via `delete_by_file_path` must remove all associated chunks from both LanceDB and Tantivy indexes.
   - *Observation*: `crates/quorum-storage/src/tantivy_fts.rs` defines `field_file_path` as tokenized `TEXT | STORED`. `delete_by_file_path` calls `writer.delete_term(Term::from_field_text(self.field_file_path, file_path))`.
   - *Inference*: Because tokenized `TEXT` fields do not store the verbatim path string as a single term, Tantivy cannot match the term. `writer.delete_term` deletes zero documents.
   - *Impact*: Chunks deleted from LanceDB remain indexed in Tantivy indefinitely, creating severe index desync (hybrid drift) and returning phantom deleted documents to users.

3. **Feature Contract Non-Conformance**:
   - *Premise*: User requirement specifies BM25 search over keywords and filenames.
   - *Observation*: `QueryParser::for_index` in `tantivy_fts.rs` only indexes `self.field_text_content`.
   - *Inference*: Keyword queries matching filenames cannot match chunks unless the filename is repeated in the body text.
   - *Impact*: Users cannot search by filename in the ChatGPT interface.

---

## 3. Caveats

- **Protoc Requirement**: Compilation requires `$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"` and `$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"`.
- **Review-Only Constraint**: In accordance with the Challenger protocol, no implementation files in `crates/quorum-storage/src/` were modified by Challenger 1. All findings are empirically reproducible using tests in `crates/quorum-storage/tests/`.
- No other caveats.

---

## 4. Conclusion

**Verdict: REQUEST_CHANGES**

Milestone 2 cannot be approved in its current state due to:
1. **Critical panic**: Arithmetic overflow in `crates/quorum-storage/src/engine.rs:171` when `top_k` is large.
2. **Critical data leakage / drift**: Deletion failure in `crates/quorum-storage/src/tantivy_fts.rs:47, 274` leaving deleted documents in the full-text search index.
3. **Missing contract feature**: Lack of `field_file_path` in `tantivy_fts.rs:146` for filename search.

### Required Remediations for Worker M2:
1. In `crates/quorum-storage/src/engine.rs:171`:
   Change:
   ```rust
   let candidate_limit = top_k.max(5) * 2;
   ```
   To:
   ```rust
   let candidate_limit = top_k.saturating_mul(2).max(10);
   ```
2. In `crates/quorum-storage/src/tantivy_fts.rs:47`:
   Change `field_file_path` from `TEXT | STORED` to `STRING | STORED` (untokenized exact term matching) so that `delete_term(Term::from_field_text(self.field_file_path, file_path))` successfully deletes matching documents.
3. In `crates/quorum-storage/src/tantivy_fts.rs:146`:
   Include `self.field_file_path` in `QueryParser::for_index`:
   ```rust
   let query_parser = QueryParser::for_index(&self.index, vec![self.field_text_content, self.field_file_path]);
   ```

---

## 5. Verification Method

To independently reproduce and verify these findings:

1. **Verify `top_k` Overflow Panic**:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"; $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo test -p quorum-storage --test adversarial_stress_tests -- test_adv_top_k_overflow_in_hybrid --nocapture
   ```
   *Expected Output*: Panic at `engine.rs:171:31` (`attempt to multiply with overflow`).

2. **Verify Tantivy Deletion Failure**:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"; $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo test -p quorum-storage --test adversarial_storage_tests -- test_adversarial_compaction_after_deletions_and_reinsertions --nocapture
   ```
   *Expected Output*: Test failure (`Deleted file_b chunks must not appear in search results`).

3. **Verify Full Adversarial Stress Suite**:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"; $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo test -p quorum-storage --test adversarial_stress_tests -- --test-threads=2
   ```
   *Expected Output*: 13 tests execute, showing the verified invariants and caught panics.
