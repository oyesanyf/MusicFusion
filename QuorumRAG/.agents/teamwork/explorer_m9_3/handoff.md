# Milestone 9 Feature 53 Handoff Report: Monotonic Live Indexer Progress Reporting

## 1. Observation

A comprehensive inspection of `crates/quorum-ingest/src/reindex.rs`, `crawler.rs`, `crates/quorum-daemon/src/handlers/reindex.rs`, `server.rs`, `main.rs`, and `crates/quorum-ui/assets/app.js` and `index.html` revealed the following exact observations:

### Observation 1.1: Fake 25% Fallback and Artificial Minimum in UI
In `crates/quorum-ui/assets/app.js`, lines 813–820:
```javascript
      let pct = Number(data.percent || 0);
      if (totalEst > 0) {
        pct = Math.min(100, Math.max(2, (scanned / totalEst) * 100));
      } else if (phase === 'completed') {
        pct = 100;
      } else {
        pct = 25;
      }
```
And line 821:
```javascript
      const pctFormatted = totalEst > 0 ? `${pct.toFixed(1)}%` : (phase === 'completed' ? '100%' : 'Scanning...');
```
- **Finding**: Whenever `totalEst === 0` (e.g. while the filesystem is being scanned or on startup before file counts are known), `app.js` unconditionally assigns `pct = 25;`. This forces the progress bar to render at `25%` regardless of actual indexer activity. Furthermore, `Math.max(2, ...)` forces an arbitrary 2% floor even when 0 of 50,000 files have been processed.

### Observation 1.2: Hardcoded Initial Bar Widths in HTML
In `crates/quorum-ui/assets/index.html`, lines 76 and 143:
```html
Line 76:
<div id="live-indexer-bar" style="width:15%; height:100%; background:linear-gradient(90deg, #2563eb, #10a37f); border-radius:999px; transition:width 0.4s ease;"></div>

Line 143:
<div id="top-index-bar" style="width:20%; height:100%; background:linear-gradient(90deg, #2563eb, #10a37f); transition:width 0.4s ease;"></div>
```
- **Finding**: The HTML hardcodes initial CSS widths of `15%` and `20%` respectively before any JavaScript poll or backend state arrives, creating an immediate visual illusion of progress.

### Observation 1.3: Slice Undercounting in Candidate Discovery
In `crates/quorum-ingest/src/reindex.rs`, lines 275–319:
```rust
        let targets = Self::expand_drive_roots(&raw_targets);
        let chunker = TextChunker::new(ChunkerConfig::default());

        for target in &targets {
            if !target.exists() {
                warn!("Target path does not exist, skipping: {}", target.display());
                continue;
            }

            let cfg_clone = config.clone();
            let target_clone = target.clone();
            let candidates = tokio::task::spawn_blocking(move || {
                crate::throttling::set_background_thread_priority();
                let crawler = FilesystemCrawler::new(cfg_clone);
                crawler.crawl_path_sync(&target_clone)
            })
            .await
            .map_err(|e| IngestError::Io(std::io::Error::other(format!("Crawl task panicked: {}", e))))??;

            {
                let mut lock = progress.write().await;
                if let Some(ref mut p) = *lock {
                    p.files_total_estimated += candidates.len();
                    p.phase = ReindexPhase::ChunkingAndVectorizing;
                }
            }

            let mut batch_chunks: Vec<DocumentChunk> = Vec::with_capacity(options.batch_size);

            for candidate in candidates {
                ...
                {
                    let mut lock = progress.write().await;
                    if let Some(ref mut p) = *lock {
                        p.files_scanned += 1;
                        p.bytes_processed += candidate.size_bytes;
                    }
                }
```
- **Finding**: `expand_drive_roots` breaks drive roots (e.g., `D:\`) into multiple sub-slices (`femi/resume`, `femi/transcripts`, `school`, `articles`, etc.). The reindexer iterates over each slice sequentially: it crawls slice $i$, adds slice $i$'s candidate count to `files_total_estimated`, and immediately begins chunking and indexing slice $i$ while incrementing `files_scanned`.
- **Direct Consequence**: During slice 1 with 10 files, `files_total_estimated` is 10. When slice 1 completes, `files_scanned` is 10, resulting in `10/10 = 100%`. When slice 2 begins with 90 files, `files_total_estimated` increases to 100, but `files_scanned` is still 10, causing the percentage to suddenly plunge from 100% down to 10%! This cycle repeats for every slice target, causing violent progress bar oscillation and resets.

### Observation 1.4: Discontinuous Progress Resets on Background Indexer Passes
In `crates/quorum-daemon/src/main.rs`, lines 232–276:
```rust
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            loop {
                ...
                if !is_running {
                    let pass_options = quorum_ingest::ReindexOptions {
                        target_paths,
                        full_rebuild: false,
                        force_reembed: false,
                        batch_size: 64,
                        num_threads: 0,
                        dry_run: false,
                    };
                    if let Err(e) = bg_worker.start_reindex(pass_options).await {
                        error!("Continuous background indexer pass warning: {}", e);
                    }
                }

                tokio::select! {
                    _ = bg_shutdown.notified() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(12)) => {}
                }
            }
        });
```
And in `crates/quorum-ingest/src/reindex.rs`, lines 132–150:
```rust
        let initial_progress = ReindexProgress {
            task_id,
            phase: ReindexPhase::ScanningFilesystem,
            full_rebuild: options.full_rebuild,
            files_scanned: 0,
            files_total_estimated: 0,
            files_indexed: 0,
            files_skipped_unmodified: 0,
            files_failed: 0,
            chunks_reused_from_cache: 0,
            chunks_newly_embedded: 0,
            bytes_processed: 0,
            started_at: Utc::now(),
            completed_at: None,
            elapsed_secs: 0,
            error_message: None,
        };
        *active = Some(initial_progress);
```
- **Finding**: Every 12 seconds, the continuous background indexer starts a new pass to verify drive state. Every invocation of `start_reindex` immediately overwrites `*active` with `initial_progress`, setting `files_scanned = 0` and `files_total_estimated = 0`. For the seconds during which the crawler scans, the UI sees `totalEst == 0`, triggering the 25% fake fallback. The UI thus flips from 100% to 25% and back every 12 seconds even when 0 files changed on disk.

### Observation 1.5: REST API Status Route
In `crates/quorum-daemon/src/server.rs`, line 42:
```rust
        .route("/api/reindex/status", get(reindex::reindex_status_handler))
```
- **Finding**: `/api/reindex/status` is registered, but `/api/index/status` is not mapped.

---

## 2. Logic Chain

1. **Root Cause of Oscillating Progress (Slice Undercounting)**:
   - In Observation 1.3, `p.files_total_estimated += candidates.len()` occurs inside the loop over `&targets`.
   - Therefore, at iteration $k$, the denominator `files_total_estimated` represents only $\sum_{i=1}^{k} \text{len}(\text{candidates}_i)$ instead of the total across all targets $\sum_{i=1}^{N} \text{len}(\text{candidates}_i)$.
   - Because `files_scanned` completes all items in slice $k$ before slice $k+1$ is crawled, the ratio $\frac{\text{files\_scanned}}{\text{files\_total\_estimated}}$ reaches 1.0 (100%) at the end of slice $k$, then suddenly drops when slice $k+1$ expands the denominator without a corresponding initial increase in `files_scanned`.
   - **Remedy**: Decouple discovery from chunking/vectorization. First discover and aggregate all candidates across all targets during `ReindexPhase::ScanningFilesystem`. Set `files_total_estimated = total_candidates`. Then enter `ReindexPhase::ChunkingAndVectorizing` and iterate through the unified candidate list.

2. **Root Cause of the 25% Fake Fallback and 2% Minimum**:
   - In Observation 1.1, `app.js` line 819 explicitly sets `pct = 25` whenever `totalEst === 0`.
   - This was originally written as a crude placeholder for the "Scanning..." state before candidate discovery had completed.
   - Combined with Observation 1.2 (`style="width:15%"` and `style="width:20%"`), the UI displays arbitrary numbers (15%, 20%, 25%) that bear no relation to real disk or index status.
   - **Remedy**:
     - Remove `pct = 25` entirely.
     - When `totalEst === 0` and `storedChunks > 0`, progress is `100.0%` (or `"Indexed: N chunks (100%)"`).
     - When `totalEst === 0` and `storedChunks === 0`, progress is `0.0%` and text is `"Discovering files across drives..."`.
     - Remove `Math.max(2, ...)` and replace initial HTML widths with `0%` (or dynamic width).

3. **Root Cause of Periodic Telemetry Flicker**:
   - In Observation 1.4, `start_reindex` zeroes `files_scanned` and `files_total_estimated` on every 12-second background pass.
   - In `handlers/reindex.rs`, lines 87–94 calculate `pct = 0.0` when `files_total_estimated == 0` during this initial phase, even though `state.storage.count()` shows thousands of stored chunks.
   - **Remedy**:
     - In `handlers/reindex.rs`: if `progress.files_total_estimated == 0` but `total_stored_chunks > 0`, report `percent: 100.0` so the UI does not report a false 0% or drop during background verification.
     - In `app.js`: maintain a monotonic client-side maximum (`lastMonotonicPct`) during an active task pass, preventing backward jitter.

4. **API Endpoint Route Completeness**:
   - Adding `.route("/api/index/status", get(reindex::reindex_status_handler))` in `server.rs` ensures both `/api/reindex/status` and `/api/index/status` respond with identical monotonic status JSON.

---

## 3. Caveats

- **Upfront Discovery Memory Overhead**: Storing `DiscoveredCandidate` structs upfront across all drives takes approximately 80–100 bytes per candidate. For an extensive library of 50,000 files, this represents ~5 MB of RAM, which is completely negligible and well within host overhead invariants (R1).
- **Drive Disconnections during Ingestion**: If a physical drive or USB volume is unmounted during the chunking phase after discovery, `DocumentExtractor::extract_text` will gracefully handle I/O failure, increment `p.files_failed += 1`, and increment `p.files_scanned += 1`, maintaining monotonic advancement without panicking.
- **Office Temp Lock Files**: M8.5 already filters out `~$*`, `.~lock*`, `*.tmp`, `*.bak`, and `*.wbk` in `FilesystemCrawler::build_walker`, preventing transient lock files from skewing candidate totals.

---

## 4. Conclusion & Implementation Plan

The root causes of non-monotonic indexer progress, slice undercounting, and fake 25% fallbacks are completely identified with exact file paths and line numbers. The concrete implementation plan consists of four localized modifications:

### Step 1: Upfront Candidate Discovery in `crates/quorum-ingest/src/reindex.rs`
In `execute_pipeline`:
1. Crawl all `targets` in `ScanningFilesystem` phase before processing any files.
2. Accumulate candidates in `all_candidates: Vec<DiscoveredCandidate>` and deduplicate paths (`HashSet`).
3. Set `p.files_total_estimated = all_candidates.len()` and transition `p.phase = ReindexPhase::ChunkingAndVectorizing`.
4. Process `all_candidates` in one continuous loop, incrementing `p.files_scanned += 1` on every file.
5. Result: `(files_scanned as f64 / files_total_estimated as f64) * 100.0` is strictly monotonic from `0.0%` to `100.0%`.

### Step 2: Monotonic Status Reporting in `crates/quorum-daemon/src/handlers/reindex.rs` & `server.rs`
1. In `handlers/reindex.rs` (`reindex_status_handler`):
   - When `files_total_estimated > 0`: `pct = ((files_scanned as f64 / files_total_estimated as f64) * 100.0).clamp(0.0, 100.0)`.
   - When `files_total_estimated == 0 && (phase == Completed || total_stored_chunks > 0)`: `pct = 100.0`.
   - When `files_total_estimated == 0 && total_stored_chunks == 0`: `pct = 0.0`.
2. In `server.rs`: Add `.route("/api/index/status", get(reindex::reindex_status_handler))`.

### Step 3: Eliminate Fake 25% Fallback & Clamps in `crates/quorum-ui/assets/app.js`
In `checkReindexStatus()`:
1. Replace lines 813–820:
   ```javascript
   let pct = 0;
   if (typeof data.percent === 'number') {
     pct = data.percent;
   } else if (totalEst > 0) {
     pct = (scanned / totalEst) * 100;
   } else if (phase === 'completed' || storedChunks > 0) {
     pct = 100;
   } else {
     pct = 0;
   }
   pct = Math.min(100, Math.max(0, pct));
   ```
2. Track `lastMonotonicPct`:
   ```javascript
   if (isRunning && totalEst > 0) {
     pct = Math.max(lastMonotonicPct, pct);
     lastMonotonicPct = pct;
   } else if (!isRunning) {
     lastMonotonicPct = 0;
   }
   ```
3. Update `pctFormatted`:
   ```javascript
   const pctFormatted = (isRunning && totalEst === 0)
     ? 'Scanning...'
     : (totalEst > 0 ? `${pct.toFixed(1)}%` : (storedChunks > 0 ? '100%' : '0%'));
   ```

### Step 4: Fix Hardcoded Initial Bar Widths in `crates/quorum-ui/assets/index.html`
1. Line 76: Set initial `#live-indexer-bar` `style="width:0%; ..."` (or `100%` when chunks present).
2. Line 143: Set initial `#top-index-bar` `style="width:0%; ..."`.

---

## 5. Verification Method

To independently verify after implementation:

1. **Automated Unit & Integration Tests**:
   Execute the reindexing and daemon test suites with limited concurrency:
   ```powershell
   cargo test -p quorum-ingest --test adversarial_reindex_and_drift -- --test-threads=2
   cargo test -p quorum-daemon --test daemon_tests -- --test-threads=2
   cargo test -p quorum-daemon --test adversarial_endpoint_tests -- --test-threads=2
   ```
2. **Monotonicity Assertion Test**:
   Add a dedicated unit test in `crates/quorum-ingest/tests/m9_monotonic_progress_test.rs`:
   - Setup a temporary directory hierarchy with multiple subdirectories (mimicking drive root expansion).
   - Spawn `worker.start_reindex(...)`.
   - Poll `worker.get_progress()` every 20ms until completion.
   - Assert:
     a) `prog.files_total_estimated` never decreases.
     b) `prog.files_scanned` is monotonically non-decreasing.
     c) Computed percentage `prog.files_scanned as f64 / prog.files_total_estimated as f64` never decreases.
     d) Final percentage is exactly 100.0%.
3. **HTTP REST API Verification**:
   Query both `/api/reindex/status` and `/api/index/status`:
   ```powershell
   curl -s http://127.0.0.1:3031/api/reindex/status
   curl -s http://127.0.0.1:3031/api/index/status
   ```
   Verify HTTP 200 response containing `percent`, `files_scanned`, `files_total_estimated`, `total_stored_chunks`, and zero occurrence of `25.0` when idle.
4. **Invalidation Conditions**:
   The fix is invalidated if:
   - Progress percentage ever drops during an active reindex run.
   - Any API or UI response exhibits a hardcoded 25% value when files are unindexed.
   - Candidate counts are reset to zero during active indexing.
