# Handoff Report: Milestone 6 Storage Engine Tantivy Remediation

- **Agent Name**: explorer_m6_remediation_1
- **Role**: Milestone 6 Storage Remediation Explorer
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\`
- **Target File**: `crates/quorum-storage/src/tantivy_fts.rs`
- **Date**: 2026-11-01T07:25:00Z
- **Status**: COMPLETE

---

## 1. Observation

Direct observations and evidence from `crates/quorum-storage/src/tantivy_fts.rs`, reviewer report (`reviewer_m6_1/handoff.md`), and test suites:

### 1.1 Silent Failure and Lack of Retry on `IndexReader::reload`
In `crates/quorum-storage/src/tantivy_fts.rs`, there are 10 call sites of `self.reader.reload()`. Five call sites completely discard the return value:
- Line 173 (`search`): `let _ = self.reader.reload();`
- Line 236 (`has_chunk_hash`): `let _ = self.reader.reload();`
- Line 248 (`delete_by_file_path`): `let _ = self.reader.reload();`
- Line 382 (`scan_all_chunks`): `let _ = self.reader.reload();`
- Line 405 (`delete_by_chunk_id`): `let _ = self.reader.reload();`

The other five call sites execute `self.reader.reload()` once without retry logic:
- Lines 154–157 (`index_chunks`):
  ```rust
  self.reader
      .reload()
      .map_err(|e| StorageError::Tantivy(format!("Tantivy reload error: {}", e)))?;
  ```
- Lines 278–280 (`delete_by_file_path`): `self.reader.reload().map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;`
- Lines 303–305 (`compact`): `self.reader.reload().map_err(|e| StorageError::Tantivy(format!("Reload error: {}", e)))?;`
- Lines 435–437 (`delete_by_chunk_id`): `self.reader.reload().map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;`
- Lines 469–471 (`clear`): `self.reader.reload().map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;`

In contrast, `commit_with_retry` (lines 478–492) retries up to 10 times with exponential backoff (`20ms * attempts`). When concurrent threads or rapid write-read cycles execute on Windows, file locking violations (`ERROR_SHARING_VIOLATION` = 32 or `ERROR_ACCESS_DENIED` = 5) on `meta.json` cause transient errors in `reload()`. Because `search()` ignores this, the searcher continues to read the stale snapshot, returning 0 hits.
Directly reproduced in `cargo test -p quorum-storage --test adversarial_stress_tests`:
```
thread 'test_adv_massive_chunk_payload_5mb' panicked at crates\quorum-storage\tests\adversarial_stress_tests.rs:270:5:
assertion `left == right` failed
  left: 0
 right: 1
```
When run in isolation, the test immediately passed (`1 passed; 0 failed in 1.14s`), confirming transient reader desynchronization under concurrent load.

### 1.2 In-Flight Batch Drop upon Writer Crash
In `crates/quorum-storage/src/tantivy_fts.rs` lines 130–152:
```rust
let mut add_res = writer.add_document(doc.clone());
if let Err(ref e) = add_res {
    let msg = e.to_string();
    if msg.contains("index writer was killed") || msg.contains("panicked") {
        tracing::warn!("Tantivy writer was killed; recovering by re-instantiating index writer: {}", msg);
        *writer = Self::create_writer_inner(&self.index)?;
        add_res = writer.add_document(doc);
    }
}
add_res.map_err(|e| StorageError::Tantivy(format!("Add document error: {}", e)))?;
count += 1;
```
And lines 143–152:
```rust
if let Err(e) = commit_with_retry(&mut writer) {
    let msg = e.to_string();
    if msg.contains("index writer was killed") || msg.contains("panicked") {
        tracing::warn!("Tantivy writer killed during commit; recreating: {}", msg);
        *writer = Self::create_writer_inner(&self.index)?;
        commit_with_retry(&mut writer)?;
    } else {
        return Err(e);
    }
}
```
If document index $k > 0$ fails due to a writer crash, `*writer = create_writer_inner()` instantiates an empty writer buffer. Documents $0 \dots k-1$ in the batch remain in the killed writer and are dropped. Only document $k$ and subsequent documents are added.
If commit fails due to writer crash, `*writer` is recreated empty and committed empty, dropping all $N$ documents in the batch.
In both scenarios, `index_chunks` returns `Ok(count)`, falsely reporting that all chunks were successfully indexed.

### 1.3 Lack of Segment Merging in `compact()`
In `crates/quorum-storage/src/tantivy_fts.rs` lines 44 and lines 286–308:
- Line 44: `writer.set_merge_policy(Box::new(NoMergePolicy));` explicitly disables Tantivy's automatic background merging.
- Lines 286–308: `compact()` only executes `commit_with_retry(&mut writer)` and `self.reader.reload()`. It never enumerates segments or calls `writer.merge(&segment_ids)`.
- As verified in Tantivy 0.27.0 documentation (`tantivy::indexer::IndexWriter`), `pub fn merge(&mut self, segment_ids: &[SegmentId]) -> FutureResult<Option<SegmentMeta>>` is the public API to consolidate segments.
- Without invoking `merge(&segment_ids).wait()`, segment files and deleted-document tombstones accumulate indefinitely without compaction.

---

## 2. Logic Chain

1. **Root Cause 1 (Reader Desync)**:
   - On Windows, file replaces and lock releases on `meta.json` during rapid commit cycles are not instantaneous.
   - Calling `self.reader.reload()` without retry during an active lock yields an `Err`.
   - In `search()` and `has_chunk_hash()`, discarding this `Err` leaves `self.reader.searcher()` viewing the previous commit generation.
   - Consequently, newly inserted chunks are invisible, leading to test assertions like `left: 0, right: 1`.
   - **Remedy**: Introduce `reload_with_retry(reader: &IndexReader)` with 10 attempts and `15ms * attempts` linear backoff (matching `commit_with_retry`). Call `reload_with_retry(&self.reader)?;` across all 10 call sites.

2. **Root Cause 2 (Batch Drop)**:
   - When an `IndexWriter` worker thread panics or is killed, all documents accumulated in that writer's uncommitted segment buffer are lost.
   - Re-instantiating the writer only re-adds the single current document `doc`, leaving all previously added chunks from the batch lost.
   - **Remedy**: Enclose the chunk iteration and commit inside a `'batch_attempt` loop with a maximum recovery budget (`MAX_RECOVERIES = 3`). If a writer crash is detected during either `add_document` or `commit_with_retry`, re-instantiate the writer, log the recovery event, and restart the batch iteration from chunk index `0`.

3. **Root Cause 3 (Unconsolidated Segments)**:
   - `NoMergePolicy` prevents any background consolidation during write traffic.
   - Therefore, `compact()` is responsible for executing explicit segment merging.
   - Tantivy provides `searcher.segment_readers()` to inspect active segments and `writer.merge(&segment_ids).wait()` to merge them.
   - **Remedy**: In `compact()`, commit pending changes, reload reader with retry, extract segment IDs from `self.reader.searcher().segment_readers().iter().map(|r| r.segment_id()).collect()`, and if `segment_ids.len() >= 2`, execute `writer.merge(&segment_ids).wait()`, commit the merge with retry, and reload the reader.

---

## 3. Caveats

- Tantivy's `writer.merge(&segment_ids).wait()` requires at least 1 segment, but merging a single segment without deletes is a no-op. Guarding with `if segment_ids.len() >= 2` avoids redundant rewrite overhead when the index is already a single segment.
- `writer.merge()` returns `FutureResult<Option<SegmentMeta>>`. In synchronous contexts, `.wait()` blocks until the background merge thread completes the consolidation.
- `NoMergePolicy` remains appropriate for indexing throughput as long as `compact()` performs the scheduled consolidation.
- No other storage subsystems (e.g. LanceDB vector store, arrow interop) require modification.

---

## 4. Conclusion & Ready-to-Apply Remediation Code

All 3 defects have been diagnosed and remedied. The exact replacements are detailed below.
Full replacement file is stored at:
`d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\proposed_tantivy_fts.rs`
Unified diff patch is stored at:
`d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_1\proposed_tantivy_fts.patch`

### 4.1 Exact Code Remediations for `worker_m6_remediation_1`

#### Remediation 1: Add `SegmentId` to Tantivy Imports
In `crates/quorum-storage/src/tantivy_fts.rs` line 16:
```rust
// BEFORE:
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument, Term};

// AFTER:
use tantivy::{Index, IndexReader, IndexWriter, SegmentId, TantivyDocument, Term};
```

#### Remediation 2: Add `chunk_to_doc` Helper and Batch Replay Loop in `index_chunks`
Replace lines 99–159 with:
```rust
    /// Converts a domain DocumentChunk into a TantivyDocument.
    fn chunk_to_doc(&self, c: &DocumentChunk) -> TantivyDocument {
        let mut doc = TantivyDocument::default();
        doc.add_text(self.field_chunk_id, &c.chunk_id);
        doc.add_text(self.field_document_id, &c.document_id);
        doc.add_text(self.field_file_path, &c.file_path);
        doc.add_text(self.field_file_path_text, &c.file_path);
        doc.add_u64(self.field_line_start, c.line_start as u64);
        doc.add_u64(self.field_line_end, c.line_end as u64);
        doc.add_text(self.field_blake3_hash, &c.blake3_hash);
        doc.add_u64(self.field_access_tier, c.access_tier as u64);
        doc.add_text(self.field_text_content, &c.text_content);
        doc.add_text(self.field_metadata_json, &c.metadata_json);

        let mut vec_bytes = Vec::with_capacity(c.vector.len() * 4);
        for &f in &c.vector {
            vec_bytes.extend_from_slice(&f.to_le_bytes());
        }
        doc.add_bytes(self.field_vector_bytes, &vec_bytes);
        doc
    }

    /// Indexes a slice of chunks into the Tantivy BM25 inverted index.
    ///
    /// Implements full batch-replay recovery: if the underlying Tantivy IndexWriter
    /// is killed or panics during `add_document` or `commit`, a fresh writer is
    /// instantiated and all chunks in the batch are replayed from chunk 0.
    pub(crate) fn index_chunks(&self, chunks: &[DocumentChunk]) -> Result<usize, StorageError> {
        if chunks.is_empty() {
            return Ok(0);
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| StorageError::Tantivy("Tantivy index writer mutex poisoned".into()))?;

        let mut recovery_attempts = 0;
        const MAX_RECOVERIES: usize = 3;

        'batch_attempt: loop {
            let mut crash_occurred = false;
            for c in chunks {
                let doc = self.chunk_to_doc(c);
                if let Err(e) = writer.add_document(doc) {
                    let msg = e.to_string();
                    if msg.contains("index writer was killed") || msg.contains("panicked") {
                        tracing::warn!(
                            "Tantivy writer was killed during add_document; re-creating and replaying batch: {}",
                            msg
                        );
                        recovery_attempts += 1;
                        if recovery_attempts > MAX_RECOVERIES {
                            return Err(StorageError::Tantivy(format!(
                                "Exceeded maximum writer recoveries ({}) during add_document: {}",
                                MAX_RECOVERIES, msg
                            )));
                        }
                        *writer = Self::create_writer_inner(&self.index)?;
                        crash_occurred = true;
                        break;
                    } else {
                        return Err(StorageError::Tantivy(format!("Add document error: {}", e)));
                    }
                }
            }

            if crash_occurred {
                // Replay the full batch from index 0 on the fresh writer
                continue 'batch_attempt;
            }

            if let Err(e) = commit_with_retry(&mut writer) {
                let msg = e.to_string();
                if msg.contains("index writer was killed") || msg.contains("panicked") {
                    tracing::warn!(
                        "Tantivy writer killed during commit; re-creating and replaying batch: {}",
                        msg
                    );
                    recovery_attempts += 1;
                    if recovery_attempts > MAX_RECOVERIES {
                        return Err(StorageError::Tantivy(format!(
                            "Exceeded maximum writer recoveries ({}) during commit: {}",
                            MAX_RECOVERIES, msg
                        )));
                    }
                    *writer = Self::create_writer_inner(&self.index)?;
                    // Replay the full batch from index 0 on the fresh writer
                    continue 'batch_attempt;
                } else {
                    return Err(e);
                }
            }

            break 'batch_attempt;
        }

        reload_with_retry(&self.reader)?;

        Ok(chunks.len())
    }
```

#### Remediation 3: Segment Consolidation in `compact()`
Replace `compact()` (lines 286–308) with:
```rust
    /// Merges Tantivy index segments to reclaim deleted space and consolidate inverted indexes.
    pub(crate) fn compact(&self) -> Result<(), StorageError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| StorageError::Tantivy("Tantivy writer mutex poisoned".into()))?;

        // 1. Commit any pending uncommitted additions/deletions before merging
        if let Err(e) = commit_with_retry(&mut writer) {
            let msg = e.to_string();
            if msg.contains("index writer was killed") || msg.contains("panicked") {
                tracing::warn!("Tantivy writer killed during compact pre-commit; recreating: {}", msg);
                *writer = Self::create_writer_inner(&self.index)?;
                commit_with_retry(&mut writer)?;
            } else {
                return Err(e);
            }
        }

        // 2. Reload reader so searcher reflects the latest commit state
        reload_with_retry(&self.reader)?;

        // 3. Enumerate all active segment IDs currently in the index
        let segment_ids: Vec<SegmentId> = self
            .reader
            .searcher()
            .segment_readers()
            .iter()
            .map(|r| r.segment_id())
            .collect();

        // 4. Consolidate segments if multiple segments exist
        if segment_ids.len() >= 2 {
            tracing::info!(
                segment_count = segment_ids.len(),
                "Consolidating Tantivy segments during compaction"
            );
            match writer.merge(&segment_ids).wait() {
                Ok(_merged) => {
                    commit_with_retry(&mut writer)?;
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("index writer was killed") || msg.contains("panicked") {
                        tracing::warn!("Tantivy writer killed during merge; recreating: {}", msg);
                        *writer = Self::create_writer_inner(&self.index)?;
                        commit_with_retry(&mut writer)?;
                    } else {
                        return Err(StorageError::Tantivy(format!("Segment merge error: {}", e)));
                    }
                }
            }
        }

        // 5. Final reader reload so searchers immediately pick up consolidated state
        reload_with_retry(&self.reader)?;

        Ok(())
    }
```

#### Remediation 4: Replace All Bare `let _ = self.reader.reload()` and Unretried Reloads
1. In `search()` (line 173):
   ```rust
   // REPLACE:
   let _ = self.reader.reload();
   // WITH:
   reload_with_retry(&self.reader)?;
   ```
2. In `has_chunk_hash()` (line 236):
   ```rust
   // REPLACE:
   let _ = self.reader.reload();
   // WITH:
   reload_with_retry(&self.reader)?;
   ```
3. In `delete_by_file_path()` (line 248):
   ```rust
   // REPLACE:
   let _ = self.reader.reload();
   // WITH:
   reload_with_retry(&self.reader)?;
   ```
4. In `delete_by_file_path()` (line 278):
   ```rust
   // REPLACE:
   self.reader
       .reload()
       .map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;
   // WITH:
   reload_with_retry(&self.reader)?;
   ```
5. In `scan_all_chunks()` (line 382):
   ```rust
   // REPLACE:
   let _ = self.reader.reload();
   // WITH:
   reload_with_retry(&self.reader)?;
   ```
6. In `delete_by_chunk_id()` (line 405):
   ```rust
   // REPLACE:
   let _ = self.reader.reload();
   // WITH:
   reload_with_retry(&self.reader)?;
   ```
7. In `delete_by_chunk_id()` (line 435):
   ```rust
   // REPLACE:
   self.reader
       .reload()
       .map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;
   // WITH:
   reload_with_retry(&self.reader)?;
   ```
8. In `clear()` (line 469):
   ```rust
   // REPLACE:
   self.reader
       .reload()
       .map_err(|e| StorageError::Tantivy(format!("Reload failed: {}", e)))?;
   // WITH:
   reload_with_retry(&self.reader)?;
   ```

#### Remediation 5: Implement `reload_with_retry`
Add the helper function at the bottom of `crates/quorum-storage/src/tantivy_fts.rs`:
```rust
/// Helper that retries Tantivy reader reload operations when encountering transient Windows file locks or I/O contention.
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

---

## 5. Verification Method

To independently verify this remediation:

1. **Apply the patch or copy proposed file**:
   ```powershell
   git apply .agents/teamwork/explorer_m6_remediation_1/proposed_tantivy_fts.patch
   # OR replace crates/quorum-storage/src/tantivy_fts.rs with proposed_tantivy_fts.rs
   ```

2. **Run Compiler & Clippy Check**:
   ```powershell
   cargo clippy -p quorum-storage --tests -- -D warnings
   ```
   Must compile with 0 errors and 0 warnings.

3. **Verify Tier 5 Adversarial Stress Tests (5 Consecutive Runs under Concurrency)**:
   ```powershell
   for ($i=1; $i -le 5; $i++) {
       Write-Host "=== Run $i ==="
       cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2
       if ($LASTEXITCODE -ne 0) { throw "Run $i failed!" }
   }
   ```
   All 8 tests must pass 100% reliably in every run.

4. **Verify Adversarial Stress Suite**:
   ```powershell
   cargo test -p quorum-storage --test adversarial_stress_tests -- --nocapture
   ```
   All 13 tests (including `test_adv_massive_chunk_payload_5mb`) must pass cleanly.

5. **Verify All Milestone 6 Crates**:
   ```powershell
   cargo test -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types -- --test-threads=2
   ```

6. **Invalidation Condition**:
   Any run under `--test-threads=2` returning an assertion mismatch (`left: 0, right: 1`) or failing compaction invalidates the fix.
