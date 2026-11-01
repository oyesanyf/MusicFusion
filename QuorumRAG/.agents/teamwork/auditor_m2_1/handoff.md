# Forensic Audit Report — Milestone 2 (Embedded LanceDB Storage & Tantivy FTS with Arrow Isolation)

**Author**: auditor_m2_1 (Milestone 2 Forensic Auditor)  
**Date**: 2026-10-31  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m2_1/`  
**Target Module**: `crates/quorum-storage/`  
**Integrity Mode**: Development (from `ORIGINAL_REQUEST.md`)  
**Verdict**: **INTEGRITY VIOLATION** (Rejected pending required remediation)

---

## Forensic Audit Report Summary

**Work Product**: `crates/quorum-storage/` (Milestone 2 Embedded LanceDB & Tantivy Engine)  
**Profile**: General Project  
**Verdict**: **INTEGRITY VIOLATION**

### Phase Results
- **Check 1: No Mocks or Hardcoded Test Results**: PASS — Real vectors, genuine cosine similarity math, real BM25 token scoring, and genuine RRF ranking.
- **Check 2: Genuine LanceDB & Tantivy Disk Execution**: PASS — Empirically verified physical `.lance` table files (parquet data files, transaction logs, manifests) and physical `.tantivy` inverted index files on disk.
- **Check 3: Strict Arrow Boundary Isolation**: PASS — ZERO `RecordBatch`, `ArrayRef`, `DataType`, or foreign LanceDB types are exposed across the public crate interface.
- **Check 4: Fail-Closed Security Gating**: PASS — Ed25519 `access_tier` pre-filtering pushed down into columnar scanner; SQL injection and malformed query syntax handled fail-closed.
- **Check 5: Facade / Silent Deletion Failure**: **FAIL** — `TantivyFtsIndex::delete_by_file_path` defines `file_path` as `TEXT | STORED`, which tokenizes paths into words and causes `Term::from_field_text` to delete 0 documents. The function unconditionally returns `Ok(1)`, creating a facade of successful deletion while ghost records persist in the search index.
- **Check 6: Behavioral Test Suite Execution**: **FAIL** — `cargo test -p quorum-storage` fails in `crates/quorum-storage/tests/adversarial_storage_tests.rs`: `test_adversarial_compaction_after_deletions_and_reinsertions` panicked at `Deleted file_b chunks must not appear in search results`.

---

## 1. Observation

### 1.1 Empirical Verification of Physical Disk Artifacts
Running an empirical verification probe (`audit_probe`) against an isolated directory produced actual physical files on disk:

```
Probe Root: "C:\\Users\\oyesanyf\\AppData\\Local\\Temp\\quorum_audit_probe_39520"
Lance Path: "C:\\Users\\oyesanyf\\AppData\\Local\\Temp\\quorum_audit_probe_39520\\.lance"
Tantivy Path: "C:\\Users\\oyesanyf\\AppData\\Local\\Temp\\quorum_audit_probe_39520\\.tantivy"

LanceDB physical files count: 6
  Lance artifact: "1111000101010100111101004dd80c45858f5904977125bde8.lance" (41626 bytes)
  Lance artifact: "0-b84dba11-b403-470d-8f9b-b8e2d5b39283.txn" (427 bytes)
  Lance artifact: "1-6703835c-270b-41ce-a33f-b2573a856620.txn" (139 bytes)
  Lance artifact: "18446744073709551613.manifest" (740 bytes)
  Lance artifact: "18446744073709551614.manifest" (930 bytes)
  Lance artifact: "latest_version_hint.json" (13 bytes)

Tantivy physical files count: 16
  Tantivy artifact: ".managed.json" (502 bytes)
  Tantivy artifact: ".tantivy-meta.lock" (0 bytes)
  Tantivy artifact: ".tantivy-writer.lock" (0 bytes)
  Tantivy artifact: "39299725842143ddba3bb5528fbb3ffa.fast" (229 bytes)
  Tantivy artifact: "39299725842143ddba3bb5528fbb3ffa.fieldnorm" (190 bytes)
  Tantivy artifact: "39299725842143ddba3bb5528fbb3ffa.idx" (443 bytes)
  Tantivy artifact: "39299725842143ddba3bb5528fbb3ffa.pos" (275 bytes)
  Tantivy artifact: "39299725842143ddba3bb5528fbb3ffa.store" (1380 bytes)
  Tantivy artifact: "39299725842143ddba3bb5528fbb3ffa.term" (1702 bytes)
  Tantivy artifact: "meta.json" (2700 bytes)
```

### 1.2 Arrow Encapsulation Verification
Inspection of `crates/quorum-storage/src/lib.rs` confirms zero leakage of Arrow, LanceDB, or Tantivy types:
```rust
pub(crate) mod arrow_interop;
pub mod config;
pub mod engine;
pub mod error;
pub(crate) mod tantivy_fts;
pub(crate) mod vector_store;

pub use config::StorageConfig;
pub use engine::{CompactionStats, SearchMode, StorageEngine, StorageSearchQuery};
pub use error::StorageError;
```
All method signatures in `StorageEngine` accept and return domain types (`DocumentChunk`, `ScoredChunk`, `StorageSearchQuery`, `CompactionStats`, `StorageError`). `StorageError` defines domain error variants (`Lance(String)`, `Arrow(String)`, `Tantivy(String)`) with zero foreign error encapsulation.

### 1.3 Security Gating Verification
1. Access tier pre-filtering in `vector_store.rs:112`:
   `.only_if(format!("access_tier <= {}", max_tier))`
   Since `max_tier` is a typed `u8`, SQL injection through this parameter is mathematically impossible.
2. Filter escaping in `vector_store.rs:134, 145`:
   `let escaped = blake3_hash.replace('\'', "''");`
   `let escaped = file_path.replace('\'', "''");`
   Adversarial injection string `foo' OR '1'='1' --` affected 0 rows and failed to delete unauthorized data.
3. Tantivy tier range filtering in `tantivy_fts.rs:167-172`:
   `RangeQuery::new(Bound::Included(Term::from_field_u64(self.field_access_tier, 0)), Bound::Included(Term::from_field_u64(self.field_access_tier, max_tier as u64)))` combined via `BooleanQuery::intersection` ensures fail-closed tier isolation even under malformed text queries.

### 1.4 Test Suite Failure & Facade Deletion Observation
Execution of `cargo test -p quorum-storage` produced an empirical test failure:

```
Command: cargo test -p quorum-storage
Running tests\adversarial_storage_tests.rs (target\debug\deps\adversarial_storage_tests-83259c9136b3f749.exe)
running 11 tests
...
test test_adversarial_compaction_after_deletions_and_reinsertions ... FAILED

failures:
---- test_adversarial_compaction_after_deletions_and_reinsertions stdout ----
thread 'test_adversarial_compaction_after_deletions_and_reinsertions' panicked at crates\quorum-storage\tests\adversarial_storage_tests.rs:594:5:
Deleted file_b chunks must not appear in search results

test result: FAILED. 10 passed; 1 failed; finished in 14.63s
```

Inspection of `crates/quorum-storage/src/tantivy_fts.rs`:
- Line 47:
  ```rust
  let field_file_path = builder.add_text_field("file_path", TEXT | STORED);
  ```
- Lines 274–282:
  ```rust
  let term = Term::from_field_text(self.field_file_path, file_path);
  let _ = writer.delete_term(term);
  commit_with_retry(&mut writer)?;
  self.reader.reload()?;
  Ok(1)
  ```

---

## 2. Logic Chain

1. **Physical Execution**: The empirical disk probe confirms that genuine LanceDB `.lance` files (41KB columnar parquet format) and Tantivy `.tantivy` files are created, verifying Invariant 2.
2. **Arrow Encapsulation**: Source code inspection confirms that `RecordBatch`, `ArrayRef`, `DataType`, and LanceDB foreign types are strictly private to `pub(crate)` modules, verifying Invariant 3.
3. **Security Gating**: Empirical injection testing confirms SQL escaping and strict numeric bounds enforcement across Tiers 0..=3, verifying Invariant 4.
4. **Root Cause of Test Failure**:
   - In Tantivy, fields declared with `TEXT` are tokenized by the default tokenizer into words split by non-alphanumeric characters (slashes `/`, backslashes `\`, and dots `.`).
   - Consequently, for a file path such as `dir/file_b.rs`, Tantivy stores individual tokens: `["dir", "file_b", "rs"]`.
   - In `delete_by_file_path`, `Term::from_field_text(self.field_file_path, "dir/file_b.rs")` attempts to find an exact term match for `"dir/file_b.rs"`. Because the field was tokenized, no such exact term exists in the lexicon.
   - `writer.delete_term(term)` deletes 0 documents.
   - The method unconditionally returns `Ok(1)`, creating a facade of successful deletion while 0 documents were actually removed.
   - While LanceDB deletes its vector rows, Tantivy retains the deleted documents as ghost records.
   - Subsequent keyword queries (`search_text`) return deleted documents, failing `test_adversarial_compaction_after_deletions_and_reinsertions`.
5. **Integrity Mandate**: Under Integrity Forensics Rule 4 (Build and run) and Rule 2 (Facade implementations), a work product with a failing test suite and a silent no-op facade cannot be certified clean. The verdict must be **INTEGRITY VIOLATION**.

---

## 3. Caveats

- **Scope**: Audit focused strictly on `crates/quorum-storage/`.
- **Environment**: Verified using process-scoped `$env:PROTOC` and `$env:PROTOC_INCLUDE` without mutating system/user PATH.
- **Audit-Only Constraint**: No implementation code was modified by this auditor.

---

## 4. Conclusion & Required Remediation

The Milestone 2 work product is **REJECTED** pending remediation of the Tantivy deletion schema.

### Actionable Remediation for Worker:

1. **Fix Tantivy Schema (`crates/quorum-storage/src/tantivy_fts.rs`)**:
   In line 47, change:
   ```rust
   let field_file_path = builder.add_text_field("file_path", TEXT | STORED);
   ```
   To:
   ```rust
   let field_file_path = builder.add_text_field("file_path", STRING | STORED);
   ```
   Using `STRING | STORED` indexes `file_path` as an untokenized, exact string literal, allowing `Term::from_field_text` to match and delete documents matching the path.

2. **Fix Vector Tie-Breaker (`crates/quorum-storage/src/vector_store.rs`)**:
   In line 125, add a deterministic secondary tie-breaker:
   ```rust
   all_scored.sort_by(|a, b| {
       b.score
           .partial_cmp(&a.score)
           .unwrap_or(std::cmp::Ordering::Equal)
           .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
   });
   ```

3. **Remediate Return Value in `delete_by_file_path`**:
   Return actual deletion count rather than hardcoded `Ok(1)`.

---

## 5. Verification Method

To independently verify the reported failure:

```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_compaction_after_deletions_and_reinsertions -- --nocapture
```

*Expected Failure*:
Panics at line 594: `Deleted file_b chunks must not appear in search results`.
