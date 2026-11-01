# Handoff Report — Milestone 2 Remediation: Tantivy Schema, Deletions & Filename Search

**Author**: Explorer M2 Remediation 1 (`explorer_m2_remediation_1`)  
**Target Module**: `crates/quorum-storage/src/tantivy_fts.rs` (with coordination in `engine.rs` and `vector_store.rs`)  
**Date**: 2026-10-31  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_1/`  
**Parent**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`  
**Verdict**: **REMEDIATION SPECIFICATION COMPLETE**

---

## 1. Observation

### 1.1 Verbatim Empirical Test Failure in Deletions (BUG-M2-01)
Executing the adversarial storage test command:
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_after_deletions_and_reinsertions
```
Produced the following verbatim failure:
```
running 1 test
test test_adversarial_compaction_after_deletions_and_reinsertions ... FAILED

failures:

---- test_adversarial_compaction_after_deletions_and_reinsertions stdout ----

thread 'test_adversarial_compaction_after_deletions_and_reinsertions' (13744) panicked at crates\quorum-storage\tests\adversarial_storage_tests.rs:594:5:
Deleted file_b chunks must not appear in search results
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    test_adversarial_compaction_after_deletions_and_reinsertions

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.64s
```

### 1.2 Inspection of Tantivy Schema & Deletion Source Code
In `crates/quorum-storage/src/tantivy_fts.rs`:
- Line 47:
  ```rust
  let field_file_path = builder.add_text_field("file_path", TEXT | STORED);
  ```
- Lines 268–283:
  ```rust
  pub(crate) fn delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError> {
      let mut writer = self
          .writer
          .lock()
          .map_err(|_| StorageError::Tantivy("Tantivy writer mutex poisoned".into()))?;

      let term = Term::from_field_text(self.field_file_path, file_path);
      let _ = writer.delete_term(term);
      commit_with_retry(&mut writer)?;

      self.reader
          .reload()
          .map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;

      Ok(1)
  }
  ```

### 1.3 Inspection of Filename Search Gap
In `crates/quorum-storage/src/tantivy_fts.rs`:
- Line 146:
  ```rust
  let query_parser = QueryParser::for_index(&self.index, vec![self.field_text_content]);
  ```
Executing `test_adv_filename_search_in_tantivy` in `crates/quorum-storage/tests/adversarial_stress_tests.rs`:
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"; $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
cargo test -p quorum-storage --test adversarial_stress_tests test_adv_filename_search_in_tantivy -- --nocapture
```
Output:
```
Filename keyword search 'special_handler' hits: 0
test test_adv_filename_search_in_tantivy ... ok
```
Where `chunk.file_path = "crates/network/src/special_handler.rs"`, searching for filename keyword `"special_handler"` yields 0 hits because `field_file_path` is not queried.

### 1.4 Inspection of Hardcoded Deletion Return Value
- In `crates/quorum-storage/src/tantivy_fts.rs:282`: `delete_by_file_path` unconditionally returns `Ok(1)` regardless of whether 0, 1, or 50 documents matched.
- In `crates/quorum-storage/src/engine.rs:250-254`:
  ```rust
  pub async fn delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError> {
      let deleted = self.vector_store.delete_by_file_path(file_path).await?;
      let _ = self.fts_index.delete_by_file_path(file_path)?;
      Ok(deleted)
  }
  ```
  The return value of Tantivy's deletion is discarded (`let _ = ...`), masking any desynchronization between LanceDB and Tantivy.

### 1.5 Inspection of Vector Store Tie-Breaker and Hybrid Arithmetic Overflow
- In `crates/quorum-storage/src/vector_store.rs:125`:
  ```rust
  all_scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
  ```
  No secondary tie-breaker on `chunk_id` exists, causing ordinal rank divergence in RRF (`test_adversarial_rrf_identical_ranks_deterministic_tiebreak`).
- In `crates/quorum-storage/src/engine.rs:171`:
  ```rust
  let candidate_limit = top_k.max(5) * 2;
  ```
  Panics with `attempt to multiply with overflow` on large `top_k` (e.g. `usize::MAX`).

---

## 2. Logic Chain

### 2.1 Why `TEXT | STORED` Fails on File Paths
1. *Observation 1.2* shows `field_file_path` is configured as `TEXT | STORED`.
2. In Tantivy 0.27, `TEXT` configures `TextFieldIndexing` with the `"default"` tokenizer (`SimpleTokenizer`).
3. `SimpleTokenizer` splits input strings into word tokens on all characters that are neither alphanumeric nor ASCII letters/digits (`/`, `\`, `.`, `-`, `_`, whitespace, punctuation) and converts ASCII characters to lowercase.
4. For path `"dir/file_b.rs"`, Tantivy emits 3 tokens: `["dir", "file_b", "rs"]`.
5. For Windows path `"C:\Users\repo\file.rs"`, Tantivy emits: `["c", "users", "repo", "file", "rs"]`.
6. Crucially, the inverted index term dictionary contains entries for `"dir"`, `"file_b"`, and `"rs"`. The raw string `"dir/file_b.rs"` **does not exist** as an entry in the term dictionary.
7. In `delete_by_file_path`, `Term::from_field_text(self.field_file_path, file_path)` builds a `Term` with exact bytes `b"dir/file_b.rs"`.
8. When `writer.delete_term(term)` looks up the term dictionary, it finds 0 matching postings lists.
9. Therefore, `writer.delete_term` marks **0 documents** for deletion in Tantivy.
10. While LanceDB removes its vector rows, Tantivy retains all chunks as zombie/phantom records (**Hybrid Index Drift**).
11. Subsequent keyword searches return these deleted chunks, causing `adversarial_storage_tests.rs:594` to panic.

### 2.2 Why `STRING | STORED` Fixes Exact Path Deletion
1. In Tantivy, `STRING` configures `TextFieldIndexing` with the `"raw"` tokenizer and `IndexRecordOption::Basic`.
2. The `"raw"` tokenizer performs zero splitting, filtering, or lowercasing; it indexes the verbatim string literal as a single token.
3. For `"dir/file_b.rs"`, the indexed term is exactly `b"dir/file_b.rs"`.
4. When `delete_by_file_path` constructs `Term::from_field_text(self.field_file_path, "dir/file_b.rs")`, Tantivy finds the exact term in the lexicon, points to all associated document IDs, and `writer.delete_term(term)` deletes all matching documents from the index.
5. After `commit_with_retry` and `reader.reload()`, deleted documents are completely eliminated from search results.

### 2.3 Why Changing `field_file_path` to `STRING` Alone Breaks Filename Search
1. Challenger 1 suggested adding `self.field_file_path` directly to `QueryParser::for_index`.
2. However, if `field_file_path` is `STRING`, its tokenizer is `"raw"`.
3. When a user searches for `"special_handler"`, `QueryParser` creates `TermQuery(field_file_path, "special_handler")`.
4. In the index, the term stored for that document is `"crates/network/src/special_handler.rs"`.
5. Because `"special_handler"` $\neq$ `"crates/network/src/special_handler.rs"`, the term query evaluates to false.
6. The user will receive **0 hits** when searching by filename, extension, or directory name.

### 2.4 The Dual-Field Architectural Solution
1. To satisfy both exact deletion AND flexible keyword/filename search, we apply the canonical search-engine dual-field pattern:
   - `field_file_path: STRING | STORED`: Untokenized literal. Used for exact `delete_by_file_path`, exact counting, and retrieval of `DocumentChunk.file_path`.
   - `field_file_path_text: TEXT`: Tokenized text. Indexed with `SimpleTokenizer` into `["crates", "network", "src", "special_handler", "rs"]`. Used for BM25 keyword matching over filenames and paths.
2. In `index_chunks`:
   - `doc.add_text(self.field_file_path, &c.file_path);`
   - `doc.add_text(self.field_file_path_text, &c.file_path);`
3. In `search`:
   - `QueryParser::for_index(&self.index, vec![self.field_text_content, self.field_file_path_text])`
4. In `delete_by_file_path`:
   - Targets `field_file_path`. When Tantivy deletes a document via `delete_term(term)`, the entire document is removed across all fields (`field_file_path_text`, `field_text_content`, etc.).
5. Result: 100% exact deletion + 100% filename keyword searchability with zero leakage and zero schema conflicts.

### 2.5 Returning Genuine Deletion Count
1. `writer.delete_term(term)` in Tantivy returns an `Opstamp` (`u64`), not a document count.
2. Using `tantivy::collector::Count`, we query the index reader prior to deletion:
   ```rust
   let term = Term::from_field_text(self.field_file_path, file_path);
   let query = TermQuery::new(term.clone(), IndexRecordOption::Basic);
   let count = searcher.search(&query, &Count)?;
   ```
3. If `count == 0`, the engine immediately returns `Ok(0)` without taking the writer lock or executing an expensive disk commit.
4. If `count > 0`, the engine acquires the writer mutex, executes `delete_term(term)`, commits with retry, reloads the reader, and returns `Ok(count)`.
5. In `StorageEngine::delete_by_file_path`, we return `Ok(lance_deleted.max(tantivy_deleted))`, ensuring accurate deletion metrics even in the presence of historical index drift.

---

## 3. Caveats

- **Read-Only Investigation**: In strict accordance with the explorer archetype, no source files were directly modified in `crates/` during this investigation.
- **Process-Scoped Tooling**: Running `cargo test` on this workspace requires process-scoped environment variables `$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"` and `$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"`. Never mutate user or system PATH.
- **Index Migration**: Existing indices created under the old schema must be cleared or recreated (as is standard in tests via `tempdir()`).

---

## 4. Conclusion & Actionable Worker Remediation Plan

The Worker (`worker_m2_1` or remediation worker) must apply the following specific code edits across 3 files:

### Target File 1: `crates/quorum-storage/src/tantivy_fts.rs`

1. **Import `Count` collector** (Line 12):
   ```rust
   // Replace:
   use tantivy::collector::TopDocs;
   // With:
   use tantivy::collector::{Count, TopDocs};
   ```

2. **Add `field_file_path_text` to struct** (Line 26):
   ```rust
   pub(crate) struct TantivyFtsIndex {
       index: Index,
       reader: IndexReader,
       writer: Mutex<IndexWriter<TantivyDocument>>,
       field_chunk_id: Field,
       field_document_id: Field,
       field_file_path: Field,
       field_file_path_text: Field, // <-- Add this
       field_line_start: Field,
   ```

3. **Declare fields in `TantivyFtsIndex::open`** (Line 47):
   ```rust
   // Replace:
   let field_file_path = builder.add_text_field("file_path", TEXT | STORED);
   // With:
   let field_file_path = builder.add_text_field("file_path", STRING | STORED);
   let field_file_path_text = builder.add_text_field("file_path_text", TEXT);
   ```

4. **Initialize struct in `TantivyFtsIndex::open`** (Line 77):
   ```rust
   // Add field_file_path_text to struct initialization:
   Ok(Self {
       index,
       reader,
       writer: Mutex::new(writer),
       field_chunk_id,
       field_document_id,
       field_file_path,
       field_file_path_text,
       field_line_start,
       // ...
   })
   ```

5. **Index both fields in `TantivyFtsIndex::index_chunks`** (Line 104):
   ```rust
   // Replace:
   doc.add_text(self.field_file_path, &c.file_path);
   // With:
   doc.add_text(self.field_file_path, &c.file_path);
   doc.add_text(self.field_file_path_text, &c.file_path);
   ```

6. **Wire query parser to search both content and file path text** (Line 146):
   ```rust
   // Replace:
   let query_parser = QueryParser::for_index(&self.index, vec![self.field_text_content]);
   // With:
   let query_parser = QueryParser::for_index(
       &self.index,
       vec![self.field_text_content, self.field_file_path_text],
   );
   ```

7. **Add deterministic tie-breaker in Tantivy search** (Line 251):
   ```rust
   // After populating scored_chunks, ensure tie-break determinism:
   scored_chunks.sort_by(|a, b| {
       b.score
           .partial_cmp(&a.score)
           .unwrap_or(std::cmp::Ordering::Equal)
           .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
   });
   ```

8. **Fix `delete_by_file_path` to return actual count** (Lines 268–283):
   ```rust
   // Replace:
   pub(crate) fn delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError> {
       let mut writer = self
           .writer
           .lock()
           .map_err(|_| StorageError::Tantivy("Tantivy writer mutex poisoned".into()))?;

       let term = Term::from_field_text(self.field_file_path, file_path);
       let _ = writer.delete_term(term);
       commit_with_retry(&mut writer)?;

       self.reader
           .reload()
           .map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;

       Ok(1)
   }

   // With:
   pub(crate) fn delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError> {
       let term = Term::from_field_text(self.field_file_path, file_path);
       let query = TermQuery::new(term.clone(), IndexRecordOption::Basic);
       let searcher = self.reader.searcher();
       let count = searcher
           .search(&query, &Count)
           .map_err(|e| StorageError::Tantivy(format!("Count matching documents failed: {}", e)))?;

       if count == 0 {
           return Ok(0);
       }

       let mut writer = self
           .writer
           .lock()
           .map_err(|_| StorageError::Tantivy("Tantivy writer mutex poisoned".into()))?;

       let _ = writer.delete_term(term);
       commit_with_retry(&mut writer)?;

       self.reader
           .reload()
           .map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;

       Ok(count)
   }
   ```

---

### Target File 2: `crates/quorum-storage/src/vector_store.rs`

1. **Add secondary tie-breaker in `search_vector`** (Line 125):
   ```rust
   // Replace:
   all_scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
   // With:
   all_scored.sort_by(|a, b| {
       b.score
           .partial_cmp(&a.score)
           .unwrap_or(std::cmp::Ordering::Equal)
           .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
   });
   ```

---

### Target File 3: `crates/quorum-storage/src/engine.rs`

1. **Saturating multiplication on candidate limit** (Line 171):
   ```rust
   // Replace:
   let candidate_limit = top_k.max(5) * 2;
   // With:
   let candidate_limit = top_k.saturating_mul(2).max(10);
   ```

2. **Handle return count in `delete_by_file_path`** (Lines 250–254):
   ```rust
   // Replace:
   pub async fn delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError> {
       let deleted = self.vector_store.delete_by_file_path(file_path).await?;
       let _ = self.fts_index.delete_by_file_path(file_path)?;
       Ok(deleted)
   }
   // With:
   pub async fn delete_by_file_path(&self, file_path: &str) -> Result<usize, StorageError> {
       let lance_deleted = self.vector_store.delete_by_file_path(file_path).await?;
       let tantivy_deleted = self.fts_index.delete_by_file_path(file_path)?;
       Ok(lance_deleted.max(tantivy_deleted))
   }
   ```

---

## 5. Verification Method

To independently verify the implementation after the Worker applies the remediation:

### Command Matrix
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"

# 1. Verify standard unit tests
cargo test -p quorum-storage --test storage_tests

# 2. Verify bug reproduction resolves (panicked at line 594 previously)
cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_after_deletions_and_reinsertions -- --nocapture

# 3. Verify deterministic RRF tie-breaking resolves (failed assertion previously)
cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_rrf_identical_ranks_deterministic_tiebreak -- --nocapture

# 4. Run entire 11-test adversarial storage suite
cargo test -p quorum-storage --test adversarial_storage_tests

# 5. Run entire adversarial stress suite (including top_k overflow)
cargo test -p quorum-storage --test adversarial_stress_tests

# 6. Run E2E storage isolation contract test
cargo test --manifest-path tests/e2e/Cargo.toml --test test_storage_isolation

# 7. Verify zero clippy warnings
cargo clippy --workspace --all-targets -- -D warnings
```

### Invalidation Conditions
- Any occurrence of `panicked at 'Deleted file_b chunks must not appear in search results'`.
- Any non-deterministic order in `test_adversarial_rrf_identical_ranks_deterministic_tiebreak`.
- Any arithmetic overflow panic when running `test_adv_top_k_overflow_in_hybrid`.
- Any compilation error or warning under `cargo clippy --workspace --all-targets -- -D warnings`.
