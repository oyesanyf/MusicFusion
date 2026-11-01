# Handoff Report — Milestone 2 Remediation Adversarial Verification

**Author**: `challenger_m2_remediation_1` (Critic & Specialist Empirical Challenger)  
**Date**: 2026-10-31  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m2_remediation_1/`  
**Parent / Orchestrator**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`  
**Target Crates**: `crates/quorum-storage/`  
**Verdict**: **`REQUEST_CHANGES`**

---

## 1. Observation

### 1.1 Tantivy Exact Deletion & Compaction Lifecycle (VERIFIED PASS)
Command executed:
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_after_deletions_and_reinsertions -- --nocapture
```
Result:
```
running 1 test
test test_adversarial_compaction_after_deletions_and_reinsertions ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.78s
```
- Inserting 30 chunks across `dir/file_a.rs`, `dir/file_b.rs`, and `dir/file_c.rs` resulted in `engine.count() == 30`.
- Deleting `dir/file_b.rs` deleted exactly 10 chunks from both Tantivy and LanceDB (`deleted_b == 10`, `engine.count() == 20`).
- Post-compaction, searching `"file_b"` and `"file_b segment"` returned 0 results for the deleted document.
- Deleting remaining files and compacting an empty table succeeded.
- Re-inserting new chunk `dir/file_d.rs` post-compaction succeeded and was retrievable by keyword search.

---

### 1.2 Extreme `top_k: usize::MAX` Capacity Overflow Panic (DEFECT FOUND)
Command executed:
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
$env:RUST_BACKTRACE = "1"
cargo test -p quorum-storage --test adversarial_stress_tests test_adv_top_k_overflow_in_hybrid -- --nocapture
```
Output:
```
running 1 test

thread '<unnamed>' (68824) panicked at /rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/library\alloc\src\raw_vec\mod.rs:28:5:
capacity overflow
stack backtrace:
   0: std::panicking::panic_handler
   1: core::panicking::panic_fmt
   2: alloc::raw_vec::capacity_overflow
   3: alloc::raw_vec::handle_error
   4: alloc::raw_vec::RawVecInner<alloc::alloc::Global>::with_capacity_in<alloc::alloc::Global>
   5: alloc::raw_vec::RawVec<core::cmp::Reverse<tantivy::collector::sort_key::sort_by_score::ScoreHeapEntry>,alloc::alloc::Global>::with_capacity_in
   6: alloc::vec::Vec<core::cmp::Reverse<tantivy::collector::sort_key::sort_by_score::ScoreHeapEntry>,alloc::alloc::Global>::with_capacity_in
   7: alloc::vec::Vec<core::cmp::Reverse<tantivy::collector::sort_key::sort_by_score::ScoreHeapEntry>,alloc::alloc::Global>::with_capacity
   8: alloc::collections::binary_heap::BinaryHeap<core::cmp::Reverse<tantivy::collector::sort_key::sort_by_score::ScoreHeapEntry>,alloc::alloc::Global>::with_capacity
   9: tantivy::collector::sort_key::sort_by_score::TopNHeap::new
  16: tantivy::core::searcher::Searcher::search
  17: quorum_storage::tantivy_fts::TantivyFtsIndex::search
  18: quorum_storage::engine::StorageEngine::search_hybrid
CONFIRMED PANIC: search_hybrid panicked on usize::MAX: capacity overflow
test test_adv_top_k_overflow_in_hybrid ... ok
```
**Defect Analysis**:
1. In `engine.rs:175`: `let candidate_limit = top_k.saturating_mul(2).max(10);`. When `top_k == usize::MAX`, `candidate_limit` saturates to `usize::MAX`.
2. In `tantivy_fts.rs:181`: `searcher.search(&combined, &TopDocs::with_limit(top_k).order_by_score())`. Tantivy attempts to instantiate a `TopNHeap` with capacity `usize::MAX`, triggering Rust standard library `capacity overflow` panic.
3. The test `test_adv_top_k_overflow_in_hybrid` in `crates/quorum-storage/tests/adversarial_stress_tests.rs:209-219` caught the thread panic and asserted `assert!(msg.contains("overflow"))`, passing on `capacity overflow`. However, the dispatch requirement states: *"Extreme top_k (including 0, 1, usize::MAX, 50000) WITHOUT PANIC"*. Calling `search_hybrid` or `search_text` with `usize::MAX` in production causes unhandled thread panics.

---

### 1.3 Filename Keyword Search in Tantivy (ASSERTION GAP)
Command executed:
```powershell
cargo test -p quorum-storage --test adversarial_stress_tests test_adv_filename_search_in_tantivy -- --nocapture
```
Output:
```
running 1 test
Filename keyword search 'special_handler' hits: 1
test test_adv_filename_search_in_tantivy ... ok
```
In `crates/quorum-storage/tests/adversarial_stress_tests.rs:336-339`:
```rust
    // Search for filename keyword "special_handler"
    let hits = engine.search_text("special_handler", 0, 5).await.unwrap();
    println!("Filename keyword search 'special_handler' hits: {}", hits.len());
    // Note: If Tantivy only queries text_content, hits will be 0!
```
While the dual-field implementation in `tantivy_fts.rs` (`field_file_path_text: TEXT`) correctly indexed path tokens and returned 1 hit, the test `test_adv_filename_search_in_tantivy` lacks an `assert_eq!(hits.len(), 1)` assertion, meaning any regression dropping filename search would pass unnoticed.

---

### 1.4 Intermittent Failure in `test_adversarial_rrf_opposing_vector_cosine_vs_text_matching`
During repeated multi-run testing:
```powershell
1..10 | ForEach-Object { cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_rrf_opposing_vector_cosine_vs_text_matching -- --quiet; if ($LASTEXITCODE -ne 0) { Write-Host "FAILED on run $_"; exit 1 } else { Write-Host "Run $_ OK" } }
```
Output:
```
Run 1 OK
Run 2 OK
Run 3 OK
Run 4 OK
Run 5 OK
Run 6 OK
Run 7 OK
test_adversarial_rrf_opposing_vector_cosine_vs_text_matching --- FAILED

failures:

---- test_adversarial_rrf_opposing_vector_cosine_vs_text_matching stdout ----

thread 'test_adversarial_rrf_opposing_vector_cosine_vs_text_matching' (41756) panicked at crates\quorum-storage\tests\adversarial_storage_tests.rs:259:5:
Opposing vector chunk with keyword match must be retained in hybrid results
```
On Run 8, `c_opposing` was absent from `hits`, indicating that `self.search_text` returned 0 hits due to Tantivy reader sync / segment visibility timing on Windows following commit.

---

## 2. Logic Chain

1. **Observation 1.1** demonstrates that the exact deletion mechanism (`field_file_path: STRING | STORED` + `TermQuery` + `IndexWriter::delete_term`) and post-compaction search integrity are fully functional and verified.
2. **Observation 1.2** proves that passing `top_k = usize::MAX` causes `candidate_limit = usize::MAX` in `engine.rs:175`. Tantivy's `TopDocs::with_limit(top_k)` unconditionally invokes `BinaryHeap::with_capacity(top_k)`. Because `usize::MAX` cannot be allocated in memory, Rust's allocator throws a `capacity overflow` panic. Clamping `top_k` to `searcher.num_docs() as usize` (or a maximum candidate ceiling such as `100_000`) in `tantivy_fts.rs:search` and in `engine.rs:search_hybrid` is required to guarantee zero panics under boundary inputs.
3. **Observation 1.3** shows that `test_adv_filename_search_in_tantivy` only executes `println!`. Without `assert_eq!(hits.len(), 1)`, the test does not assert the intended invariant.
4. **Observation 1.4** shows that `commit_with_retry` and `reader.reload()` can intermittently race with query execution on Windows filesystem operations, causing `search_text` to return 0 hits for recently committed chunks.

---

## 3. Caveats

- **Vector search resilience**: Dense vector search via LanceDB (`search_vector`) handled `top_k = 1_000_000` without panicking (`test_adv_top_k_huge`). The panic is isolated to Tantivy's heap allocation in `search_text` and `search_hybrid`.
- **Review-Only Constraint**: As an adversarial challenger, I did not modify any source code. Remediation must be implemented by the worker.

---

## 4. Conclusion

**Verdict: `REQUEST_CHANGES`**

Required remediation actions:
1. **Safety Bound on `top_k` in Tantivy Search**:
   In `crates/quorum-storage/src/tantivy_fts.rs`:
   Inside `TantivyFtsIndex::search`:
   Clamp `top_k` to avoid unbounded allocation in `TopDocs`:
   ```rust
   let num_docs = searcher.num_docs() as usize;
   let effective_limit = top_k.min(num_docs).max(1);
   let top_docs = searcher
       .search(&combined, &TopDocs::with_limit(effective_limit).order_by_score())
   ```
2. **Candidate Limit Clamping in Hybrid Search**:
   In `crates/quorum-storage/src/engine.rs`:
   Inside `StorageEngine::search_hybrid`:
   Clamp `candidate_limit` to a safe upper bound:
   ```rust
   let candidate_limit = top_k.saturating_mul(2).clamp(10, 50_000);
   ```
3. **Test Assertion in `test_adv_filename_search_in_tantivy`**:
   In `crates/quorum-storage/tests/adversarial_stress_tests.rs`:
   Add the missing assertion:
   ```rust
   let hits = engine.search_text("special_handler", 0, 5).await.unwrap();
   assert_eq!(hits.len(), 1, "Filename keyword search must match indexed file_path_text tokens");
   assert_eq!(hits[0].chunk.chunk_id, "c_fn_test");
   ```
4. **Update `test_adv_top_k_overflow_in_hybrid` Assertion**:
   In `crates/quorum-storage/tests/adversarial_stress_tests.rs`:
   Ensure `test_adv_top_k_overflow_in_hybrid` asserts that `search_hybrid` completes successfully with `Ok(_)` and does NOT panic.

---

## 5. Verification Method

To independently reproduce the findings:
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
$env:RUST_BACKTRACE = "1"

# 1. Reproduce capacity overflow panic:
cargo test -p quorum-storage --test adversarial_stress_tests test_adv_top_k_overflow_in_hybrid -- --nocapture

# 2. Inspect missing assertion in test_adv_filename_search_in_tantivy:
# Notice it prints hits but lacks assert_eq!

# 3. Verify exact deletion passes:
cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_after_deletions_and_reinsertions -- --nocapture
```
Invalidation conditions:
- `search_hybrid(..., top_k = usize::MAX)` returns `Ok(hits)` without panicking.
- `test_adv_filename_search_in_tantivy` has an explicit `assert_eq!(hits.len(), 1)`.
