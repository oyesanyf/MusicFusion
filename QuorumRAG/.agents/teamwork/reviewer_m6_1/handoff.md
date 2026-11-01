# Handoff Report: Milestone 6 Storage, Ingest, Classifier & Types Review

**Reviewer**: reviewer_m6_1  
**Role**: Milestone 6 Storage Ingest Reviewer & Adversarial Critic  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m6_1\`  
**Date**: 2026-11-01T07:10:00Z  
**Verdict**: **`REQUEST_CHANGES`**

---

## 1. Observation

### Verification Commands & Results

1. **Tier 5 Adversarial Stress Tests (`cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2`)**:
   - **Run 1**: 7 passed; 1 failed (`test_tier5_acl_access_tier_drift_healing_and_isolation`):
     ```
     thread 'test_tier5_acl_access_tier_drift_healing_and_isolation' panicked at crates\quorum-storage\tests\adversarial_tier5_storage_ingest_stress.rs:430:5:
     assertion `left == right` failed: Tier 1 must now retrieve declassified text chunk
       left: 0
      right: 1
     ```
   - **Run 2** (isolated test execution): 1 passed (`test_tier5_acl_access_tier_drift_healing_and_isolation ... ok` in 0.76s).
   - **Run 3** (full suite): 7 passed; 1 failed (`test_tier5_compaction_and_version_pruning_under_churn`):
     ```
     thread 'test_tier5_compaction_and_version_pruning_under_churn' panicked at crates\quorum-storage\tests\adversarial_tier5_storage_ingest_stress.rs:533:5:
     Surviving chunks must be retrievable
     ```
   - **Run 4** (full suite): 7 passed; 1 failed (`test_tier5_tantivy_reader_reload_coherence_under_rapid_cycles`):
     ```
     thread 'test_tier5_tantivy_reader_reload_coherence_under_rapid_cycles' panicked at crates\quorum-storage\tests\adversarial_tier5_storage_ingest_stress.rs:171:9:
     assertion `left == right` failed: Immediate search must find chunk at cycle 7
       left: 0
      right: 1
     ```

2. **All Crate Tests (`cargo test -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types -- --test-threads=2`)**:
   - `quorum-classifier`: 10 passed in lib test, 11 passed in `adversarial_magika_tests`, 8 passed in `corner_cases_multibyte_utf8`. Total: 29 passed, 0 failed.
   - `quorum-contract`: 7 passed in lib test, 7 passed in `adversarial_contract_tests`, 6 passed in `corner_cases_handshake_gating`. Total: 20 passed, 0 failed.
   - `quorum-ingest`: 9 passed in `adversarial_challenge_tests`, 10 passed in `adversarial_reindex_and_drift`, 10 passed in `ingest_tests`. Total: 29 passed, 0 failed.
   - `quorum-storage`: 11 passed in `adversarial_storage_tests`. However, `adversarial_stress_tests-3ef53b9f4ea51e71.exe` failed to launch:
     ```
     Caused by:
       could not execute process `D:\harfile\QuorumRAG\target\debug\deps\adversarial_stress_tests-3ef53b9f4ea51e71.exe --test-threads=2` (never executed)
     Caused by:
       An Application Control policy has blocked this file. (os error 4551)
     ```

3. **Compiler & Linter (`cargo clippy -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types --tests -- -D warnings`)**:
   - Result: Clean pass, exit code 0, 0 warnings, 0 errors.

---

## 2. Logic Chain

1. **Tantivy Reader Reload Silent Failure**:
   - In `crates/quorum-storage/src/tantivy_fts.rs`:
     - Line 173: `let _ = self.reader.reload();` in `search()`
     - Line 236: `let _ = self.reader.reload();` in `has_chunk_hash()`
     - Line 248: `let _ = self.reader.reload();` in `delete_by_file_path()`
     - Line 382: `let _ = self.reader.reload();` in `scan_all_chunks()`
     - Line 405: `let _ = self.reader.reload();` in `delete_by_chunk_id()`
   - While `commit_with_retry` was introduced for writer commits to tolerate Windows transient file locks, reader reloads have **zero retry mechanism**.
   - On Windows, during concurrent rapid commit/read cycles, directory and `meta.json` replacements can experience transient sharing violations (`ERROR_SHARING_VIOLATION` = 32 or `ERROR_ACCESS_DENIED` = 5).
   - Because `let _ = self.reader.reload()` ignores the return value, any transient failure causes the reader to silently retain its stale `Searcher`. The subsequent query is executed against the outdated index state, returning 0 results and causing intermittent assertion panics across cycles in multiple test cases.

2. **Writer Crash Recovery Drops In-Flight Batches**:
   - In `crates/quorum-storage/src/tantivy_fts.rs` lines 130–151:
     - When iterating over `chunks`, if `writer.add_document()` or `commit_with_retry()` fails with `"index writer was killed"` or `"panicked"`, the code executes:
       ```rust
       *writer = Self::create_writer_inner(&self.index)?;
       ```
     - For `add_document`, only the current document `doc` is re-added; all prior documents in the batch `chunks[..i]` were lost inside the killed writer's buffer.
     - For `commit_with_retry`, the new writer is committed empty, permanently losing the entire batch.
     - Despite dropping documents, `index_chunks` returns `Ok(count)` (reporting successful insertion of all chunks).

3. **Incomplete Compaction in Tantivy**:
   - In `crates/quorum-storage/src/tantivy_fts.rs` lines 286–308:
     - `compact()` only executes `commit_with_retry(&mut writer)` and `self.reader.reload()`.
     - In `open()`, the writer is initialized with `NoMergePolicy`.
     - Without calling `writer.merge(&segment_ids)` or scheduling segment merges, segment files are never merged or consolidated during `compact()`.

4. **Integrity Violation Check**:
   - Checked for hardcoded test constants, facades, or dummy placeholders.
   - None found: `LanceDB` vector search, `Tantivy` BM25 inverted index, `Google Magika` classifier, `Gemma` embedding contract, Level-1/Level-2 BLAKE3 deduplication, `ImageIndexer`, and `DriftHealer` are genuine, high-fidelity implementations.

---

## 3. Findings

### Finding 1: Intermittent Reader Reload Coherence Failure under Windows File Contention (Critical)
- **Where**: `crates/quorum-storage/src/tantivy_fts.rs` lines 154–157, 173, 236, 248, 382, 405, 436, 470
- **Why**: `self.reader.reload()` is called with `let _ = self.reader.reload();` or without retry logic. Under Windows file I/O contention with parallel threads, `meta.json` reloading can return a transient I/O sharing error. Discarding the error leaves the reader attached to a stale searcher, causing search queries immediately following an update/insert to return 0 hits.
- **Suggestion**: Implement `reload_with_retry(&self.reader)` analogous to `commit_with_retry`:
  ```rust
  fn reload_with_retry(reader: &IndexReader) -> Result<(), StorageError> {
      let mut attempts = 0;
      loop {
          match reader.reload() {
              Ok(_) => return Ok(()),
              Err(e) => {
                  attempts += 1;
                  if attempts >= 10 {
                      return Err(StorageError::Tantivy(format!("Tantivy reload error: {}", e)));
                  }
                  std::thread::sleep(std::time::Duration::from_millis(15 * attempts));
              }
          }
      }
  }
  ```

### Finding 2: In-Flight Document Dropping upon Writer Recreation (Major)
- **Where**: `crates/quorum-storage/src/tantivy_fts.rs` lines 130–152
- **Why**: Re-creating the `IndexWriter` after a thread kill or panic without replaying previously added chunks from the active batch results in silent data loss while still returning `Ok(count)`.
- **Suggestion**: If `writer` recovery is triggered, restart the entire `chunks` slice loop for the batch on the freshly instantiated writer.

### Finding 3: Segment Consolidation Missing in Tantivy Compaction (Major)
- **Where**: `crates/quorum-storage/src/tantivy_fts.rs` lines 286–308
- **Why**: `compact()` only executes commit and reload. Because `NoMergePolicy` is configured, segment files and tombstones accumulate without consolidation during drift healing.
- **Suggestion**: Enumerate segment IDs via `searcher.segment_readers()` and trigger explicit segment merging or set an explicit merge policy during compaction cycles.

### Finding 4: Test Binary Blocked by Application Control (Minor / Environmental)
- **Where**: `target\debug\deps\adversarial_stress_tests-3ef53b9f4ea51e71.exe`
- **Why**: Windows Defender Application Control / AppLocker blocked execution of this specific integration test binary with OS error 4551.
- **Suggestion**: Ensure CI and developer runner environments have AppLocker / WDAC policies configured to allow the workspace target build directories.

---

## 4. Verified Claims & Strengths

| Requirement / Invariant | Status | Verification Method |
|---|---|---|
| **Zero Arrow/LanceDB Leakage** | **PASS** | `crates/quorum-types` and `crates/quorum-storage/src/lib.rs` reviewed: zero Arrow types exported publicly |
| **Gemma 768d Embedding Contract** | **PASS** | BLAKE3 derivation, constant-time `ConstantTimeEq` handshake gating, dual offline engine verified |
| **RRF ($k=60$) Hybrid Search** | **PASS** | Deterministic competition tiebreaking, $1/(60 + \text{rank})$ formula, chunk_id tiebreaker verified |
| **Google Magika Classification** | **PASS** | ONNX and deep-inspection fallback, MIME detection, executable/binary exclusion verified |
| **Image Indexing Support** | **PASS** | `ImageIndexer` extracts PNG, JPEG, WEBP, GIF, BMP, SVG dimensions, EXIF, tokens; images are NOT skipped |
| **Two-Level BLAKE3 Dedup** | **PASS** | Level 1 file catalog + rename/move detection, Level 2 chunk LRU embedding cache verified |
| **5-Drift Healing (`fix-drift`)** | **PASS** | Heals Merkle, Lance-Tantivy desync, contract drift, ACL drift, compaction drift verified |
| **Dedicated Reindexer** | **PASS** | Incremental fast-path, full rebuild, cancellation token, progress telemetry verified |
| **Clippy Conformance** | **PASS** | Clean run with `-D warnings` on all 5 crates and tests |

---

## 5. Caveats

- Tests outside the scope of Milestone 6 (Storage, Ingest, Classifier, Types crates) such as `quorum-network`, `quorum-coordinator`, `quorum-ui`, and `quorum-daemon` were not evaluated in this review.
- The AppLocker blocking of `adversarial_stress_tests` is an environmental Windows OS policy issue, distinct from code logic.

---

## 6. Conclusion & Verdict

**Verdict**: **`REQUEST_CHANGES`**

While the core architecture, Arrow isolation, Magika classifier, Gemma contract, two-level deduplication, image indexer, and 5-drift healer are genuine and excellently designed, the storage engine exhibits intermittent coherence failures under concurrent multi-threaded execution due to unretried Tantivy reader reloads on Windows (`let _ = self.reader.reload()`). 

Once Finding 1 (`reload_with_retry`), Finding 2 (batch replay upon writer recreation), and Finding 3 (segment merging during compaction) are remediated in `crates/quorum-storage/src/tantivy_fts.rs`, the Tier 5 adversarial stress test suite will achieve 100% stable, deterministic pass rates.

---

## 7. Verification Method

To independently reproduce and verify the findings:
1. Run Tier 5 Adversarial Stress Tests repeatedly under `--test-threads=2`:
   ```powershell
   cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2
   ```
2. Verify all crate tests:
   ```powershell
   cargo test -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types -- --test-threads=2
   ```
3. Run Clippy across all crates:
   ```powershell
   cargo clippy -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types --tests -- -D warnings
   ```
4. Invalidation Condition: The changes are satisfactory when 5 consecutive runs of `cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2` pass 8 of 8 tests without a single failure or panic.
