# Forensic Investigation & Remediation Report — Vector Store RRF Determinism & Secondary Tie-Breaking (BUG-M2-02)

**Author**: `explorer_m2_remediation_2` (Milestone 2 Explorer & Forensic Investigator)  
**Date**: 2026-10-31  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m2_remediation_2/`  
**Target Crates**: `crates/quorum-storage/`, `crates/quorum-types/`  
**Target Tests**: `crates/quorum-storage/tests/adversarial_storage_tests.rs`  
**Classification**: High-Severity Determinism & Retrieval Quality Defect (BUG-M2-02)

---

## Executive Summary

Non-deterministic ordinal ranking during Reciprocal Rank Fusion (RRF) in `quorum-storage` causes retrieval order to arbitrarily flip between co-scoring chunks (`chunk_tie_beta` vs `chunk_tie_alpha`). This is caused by three compounding flaws: (1) `vector_store.rs:125` sorts via `b.score.partial_cmp(&a.score)` without a secondary tie-breaker, preserving arbitrary LanceDB `RecordBatch` row emission order; (2) `engine.rs:183-199` enumerates hits ordinally, artificially assigning $1/(60+1) = 1/61$ to the first hit and $1/(60+2) = 1/62$ to the second, which creates an artificial score divergence that completely bypasses the downstream tie-breaker on line 210 (`.then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))`); and (3) `tantivy_fts.rs:253` returns documents in collector `DocAddress` order without a secondary tie-breaker. Furthermore, analysis of `test_adversarial_rrf_identical_ranks_deterministic_tiebreak` proves that test chunks must possess distinct `text_content` to prevent Level-2 BLAKE3 deduplication from silently dropping them during `insert_chunks`. A 4-point defense-in-depth remediation plan is provided.

---

## 1. Observation

### 1.1 Live Empirical Reproduction of Non-Deterministic Ranking
Executing `test_adversarial_rrf_identical_ranks_deterministic_tiebreak` across consecutive runs reproduces flaky non-deterministic oscillation:

```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
1..10 | ForEach-Object { cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_rrf_identical_ranks_deterministic_tiebreak -- --nocapture }
```

**Verbatim Terminal Output (Run 1 — FAILED)**:
```text
thread 'test_adversarial_rrf_identical_ranks_deterministic_tiebreak' (43464) panicked at crates\quorum-storage\tests\adversarial_storage_tests.rs:110:5:
assertion `left == right` failed
  left: "chunk_tie_beta"
 right: "chunk_tie_alpha"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test test_adversarial_rrf_identical_ranks_deterministic_tiebreak ... FAILED

failures:
    test_adversarial_rrf_identical_ranks_deterministic_tiebreak

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.32s
```

**Verbatim Terminal Output (Run 2 — PASSED)**:
```text
test test_adversarial_rrf_identical_ranks_deterministic_tiebreak ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 1.12s
```

On Run 1, the test failed because `first_order[0]` was `"chunk_tie_beta"`. On Run 2, `first_order[0]` was `"chunk_tie_alpha"`. This empirically proves that retrieval ordering is non-deterministic across repeated executions.

---

### 1.2 Direct Source Code Observations

#### 1.2.1 Missing Vector Store Secondary Tie-Breaker (`crates/quorum-storage/src/vector_store.rs:118-128`)
```rust
118:         let mut all_scored = Vec::new();
119:         while let Some(batch_result) = stream.next().await {
120:             let batch = batch_result.map_err(|e| StorageError::Lance(format!("RecordBatch stream error: {}", e)))?;
121:             let scored = extract_scored_chunks(&batch, self.config.vector_dimension, Some(vector))?;
122:             all_scored.extend(scored);
123:         }
124: 
125:         all_scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
126:         all_scored.truncate(top_k);
127: 
128:         Ok(all_scored)
```
- Line 125 uses `b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)`.
- When two chunks have identical cosine similarity (e.g. 1.0), `partial_cmp` returns `Some(Ordering::Equal)`.
- Rust's `Vec::sort_by` is a stable sort algorithm; it preserves the arrival order of elements as yielded by `stream.next().await`.
- LanceDB's underlying DataFusion execution and parquet columnar reader scan fragments across thread boundaries. Row emission order for rows with identical distance is arbitrary.
- No secondary tie-breaker (such as `chunk_id`) is present.

#### 1.2.2 Downstream RRF Ordinal Enumeration and Tie-Breaker Bypass (`crates/quorum-storage/src/engine.rs:171, 183-199, 206-212`)
```rust
171:         let candidate_limit = top_k.max(5) * 2;
172:         let vector_hits = self.search_vector(vector, max_tier, candidate_limit).await?;
173:         let text_hits = self.search_text(text, max_tier, candidate_limit).await?;
174: 
175:         let mut rrf_map: HashMap<String, (f32, DocumentChunk)> = HashMap::new();
176: 
177:         // Only include vector hits that have positive cosine similarity
178:         let relevant_vector_hits: Vec<ScoredChunk> = vector_hits
179:             .into_iter()
180:             .filter(|s| s.score > 0.0)
181:             .collect();
182: 
183:         for (rank_0, scored) in relevant_vector_hits.into_iter().enumerate() {
184:             let rrf = 1.0 / (60.0 + (rank_0 + 1) as f32);
185:             let entry = rrf_map.entry(scored.chunk.chunk_id.clone()).or_insert((0.0, scored.chunk));
186:             entry.0 += rrf;
187:         }
...
195:         for (rank_0, scored) in relevant_text_hits.into_iter().enumerate() {
196:             let rrf = 1.0 / (60.0 + (rank_0 + 1) as f32);
197:             let entry = rrf_map.entry(scored.chunk.chunk_id.clone()).or_insert((0.0, scored.chunk));
198:             entry.0 += rrf;
199:         }
200: 
201:         let mut combined: Vec<ScoredChunk> = rrf_map
202:             .into_iter()
203:             .map(|(_id, (score, chunk))| ScoredChunk::new(chunk, score, "local", false))
204:             .collect();
205: 
206:         combined.sort_by(|a, b| {
207:             b.score
208:                 .partial_cmp(&a.score)
209:                 .unwrap_or(std::cmp::Ordering::Equal)
210:                 .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
211:         });
212:         combined.truncate(top_k);
```
- Lines 183–187 and 195–199 assign RRF scores ordinally based on loop index: `rank_0 = 0` gets $1/(60+1) = 1/61 \approx 0.01639344$, while `rank_0 = 1` gets $1/(60+2) = 1/62 \approx 0.01612903$.
- If `chunk_tie_beta` precedes `chunk_tie_alpha` in `relevant_vector_hits`, `chunk_tie_beta` receives score $0.01639344$ and `chunk_tie_alpha` receives $0.01612903$.
- On line 206, `b.score.partial_cmp(&a.score)` compares $0.01639344$ against $0.01612903$, returning `Some(Ordering::Greater)`.
- Because the scores are unequal, the `.then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))` clause on line 210 is **never evaluated**. The downstream tie-breaker is completely bypassed.

#### 1.2.3 Missing Tantivy Secondary Tie-Breaker (`crates/quorum-storage/src/tantivy_fts.rs:173-177, 250-254`)
```rust
173:         let top_docs = searcher
174:             .search(&combined, &TopDocs::with_limit(top_k).order_by_score())
175:             .map_err(|e| StorageError::Tantivy(format!("Search execution failed: {}", e)))?;
...
250:             scored_chunks.push(ScoredChunk::new(chunk, score, "local", false));
251:         }
252: 
253:         Ok(scored_chunks)
```
- `TopDocs::order_by_score()` returns documents ordered by BM25 score. When BM25 scores are identical (equal term frequencies and equal document length), ties in Tantivy's collector are broken by internal `DocAddress (SegmentLocalId, DocId)`.
- `scored_chunks` is returned directly without sorting on `chunk_id`.

#### 1.2.4 Level-2 BLAKE3 Chunk Deduplication (`crates/quorum-storage/src/engine.rs:119-135` & `crates/quorum-types/src/lib.rs:198-199`)
- In `crates/quorum-types/src/lib.rs:199`:
  ```rust
  let blake3_hash = Self::compute_blake3_hash(&text);
  ```
  `blake3_hash` is computed strictly from `text_content`.
- In `crates/quorum-storage/src/engine.rs:122-131`:
  ```rust
  for chunk in chunks {
      if seen_hashes.contains(&chunk.blake3_hash) {
          continue;
      }
      if self.has_chunk_hash(&chunk.blake3_hash).await.unwrap_or(false) {
          continue;
      }
      seen_hashes.insert(chunk.blake3_hash.clone());
      unique_chunks.push(chunk.clone());
  }
  ```
- If test fixtures supply identical `text_content` across two chunks (e.g. `chunk_a` and `chunk_b`), `chunk_b` has an identical `blake3_hash` and is discarded by `seen_hashes.contains(&chunk.blake3_hash)`.
- In `crates/quorum-storage/tests/adversarial_storage_tests.rs:53-55`:
  ```rust
  // Create 4 chunks with identical RRF rank potential:
  // Pair 1: chunk_a and chunk_b have identical vector and identical text
  // We want to verify that ties are broken deterministically by chunk_id ASC.
  ```
  The comment claiming "chunk_a and chunk_b have identical vector and identical text" is factually inaccurate. In the code (lines 65, 75, 85), each chunk has a distinct text prefix (`"alpha ..."`, `"beta ..."`, `"gamma ..."`). If they had identical text, Level-2 deduplication would drop `chunk_b` and `chunk_c`, causing `engine.insert_chunks` to return 1 instead of 3, and breaking `assert_eq!(first_result.len(), 3)`.

---

## 2. Logic Chain

1. **LanceDB Scan Ordering (Observation 1.2.1)**:
   - In LanceDB, `nearest_to(vector)` executes an ANN or flat scan across parquet columnar batches.
   - For candidate rows with identical distance (e.g. distance = 0.0), DataFusion and LanceDB do not specify an internal tie-breaking column.
   - Therefore, the order in which rows arrive in `RecordBatch` streams is non-deterministic.

2. **Vector Store Incomplete Sort (Observation 1.2.1)**:
   - `vector_store.rs:125` sorts candidate chunks with `b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)`.
   - Because `Vec::sort_by` is stable, elements with identical similarity scores preserve their arrival order from the `RecordBatch` stream.
   - When LanceDB emits `chunk_tie_beta` before `chunk_tie_alpha`, `all_scored` remains `[chunk_tie_beta, chunk_tie_alpha]`.

3. **RRF Ordinal Rank Distortion (Observation 1.2.2)**:
   - In `engine.rs:search_hybrid`, `relevant_vector_hits` is processed with `.enumerate()`.
   - Index 0 receives RRF score $1/(60+1) = 1/61 \approx 0.01639344$.
   - Index 1 receives RRF score $1/(60+2) = 1/62 \approx 0.01612903$.
   - Although `chunk_tie_beta` and `chunk_tie_alpha` have identical cosine similarity (1.0), `chunk_tie_beta` is assigned a higher RRF score solely because it happened to be yielded first by LanceDB.

4. **Downstream Tie-Breaker Bypass (Observation 1.2.2)**:
   - At line 206 of `engine.rs`, `combined.sort_by` attempts to sort results by `b.score.partial_cmp(&a.score).then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))`.
   - Because `0.01639344 > 0.01612903`, `b.score.partial_cmp(&a.score)` returns `Some(Ordering::Greater)`.
   - Because the scores are not equal, the secondary comparison `.then_with(...)` on `chunk_id` is never reached.
   - `chunk_tie_beta` is placed at position 0 in the result list.
   - The test assertion `assert_eq!(first_order[0], "chunk_tie_alpha")` fails (Observation 1.1).

5. **Tantivy Collector Ambiguity (Observation 1.2.3)**:
   - When keyword search yields documents with identical BM25 scores, Tantivy's `TopDocs` collector orders by `DocAddress`, which depends on segment boundaries and thread-pool execution.
   - Without an explicit secondary sort on `chunk_id` in `tantivy_fts.rs`, keyword search order also exhibits non-determinism.

6. **Level-2 Deduplication Invariant (Observation 1.2.4)**:
   - Deduplication in `StorageEngine::insert_chunks` computes `blake3_hash` over `text_content`.
   - If test fixtures use identical text strings across chunks, subsequent chunks are discarded as duplicates before reaching LanceDB or Tantivy.
   - To construct valid test fixtures for co-scoring chunks:
     - The text must contain distinct prefix tokens (`"alpha"`, `"beta"`, `"gamma"`) so that their BLAKE3 hashes are distinct.
     - The query terms must appear with identical frequencies across all chunks.
     - The chunks must have identical word counts (6 tokens) so that Tantivy computes identical BM25 scores.
     - The vectors must be identical (`vec![0.5; 768]`) so that LanceDB computes identical cosine similarity (1.0).

---

## 3. Caveats

- **Read-Only Explorer Constraint**: No production implementation files in `crates/quorum-storage/src/` or tests were modified by this explorer agent.
- **Environment Safety**: All test executions utilized process-scoped environment variables (`$env:PROTOC` and `$env:PROTOC_INCLUDE`) without modifying the user or machine system PATH.
- **Scope**: Analysis is focused strictly on vector store determinism, Tantivy tie-breaking, RRF ordinal scoring, and the adversarial test harness. `BUG-M2-01` (Tantivy `TEXT` vs `STRING` deletion) is investigated in parallel by `explorer_m2_remediation_1`.

---

## 4. Conclusion & Actionable Remediation Plan

BUG-M2-02 is fully diagnosed and reproducible. The remediation requires changes across four specific touchpoints to achieve a defense-in-depth determinism guarantee:

### 4.1 Remediation Touchpoint 1: `crates/quorum-storage/src/vector_store.rs:125`
**Problem**: Missing secondary tie-breaker in `VectorStore::search_vector`.  
**Remediation**: Update line 125 to break ties alphabetically by `chunk_id`:

```rust
// BEFORE:
all_scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

// AFTER:
all_scored.sort_by(|a, b| {
    b.score
        .partial_cmp(&a.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
});
```

### 4.2 Remediation Touchpoint 2: `crates/quorum-storage/src/tantivy_fts.rs:253`
**Problem**: `TantivyFtsIndex::search` returns results without secondary tie-breaking on `chunk_id`.  
**Remediation**: Sort `scored_chunks` before returning:

```rust
// BEFORE:
        scored_chunks.push(ScoredChunk::new(chunk, score, "local", false));
    }

    Ok(scored_chunks)

// AFTER:
        scored_chunks.push(ScoredChunk::new(chunk, score, "local", false));
    }

    scored_chunks.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
    });

    Ok(scored_chunks)
```

### 4.3 Remediation Touchpoint 3: `crates/quorum-storage/src/engine.rs:171, 183-199`
**Problem**:
1. Unchecked multiplication `top_k.max(5) * 2` can overflow on `top_k > usize::MAX / 2`.
2. Ordinal loop index enumeration assigns different RRF scores to co-scoring hits, bypassing the downstream tie-breaker.

**Remediation**:
1. Use `.saturating_mul(2).max(10)` for `candidate_limit`.
2. Implement competition ranking (assigning identical RRF rank to identical raw scores within float epsilon `1e-6`):

```rust
// BEFORE:
        let candidate_limit = top_k.max(5) * 2;
        let vector_hits = self.search_vector(vector, max_tier, candidate_limit).await?;
        let text_hits = self.search_text(text, max_tier, candidate_limit).await?;

        let mut rrf_map: HashMap<String, (f32, DocumentChunk)> = HashMap::new();

        // Only include vector hits that have positive cosine similarity
        let relevant_vector_hits: Vec<ScoredChunk> = vector_hits
            .into_iter()
            .filter(|s| s.score > 0.0)
            .collect();

        for (rank_0, scored) in relevant_vector_hits.into_iter().enumerate() {
            let rrf = 1.0 / (60.0 + (rank_0 + 1) as f32);
            let entry = rrf_map.entry(scored.chunk.chunk_id.clone()).or_insert((0.0, scored.chunk));
            entry.0 += rrf;
        }

        // Only include text hits with positive scores
        let relevant_text_hits: Vec<ScoredChunk> = text_hits
            .into_iter()
            .filter(|s| s.score > 0.0)
            .collect();

        for (rank_0, scored) in relevant_text_hits.into_iter().enumerate() {
            let rrf = 1.0 / (60.0 + (rank_0 + 1) as f32);
            let entry = rrf_map.entry(scored.chunk.chunk_id.clone()).or_insert((0.0, scored.chunk));
            entry.0 += rrf;
        }

// AFTER:
        let candidate_limit = top_k.saturating_mul(2).max(10);
        let vector_hits = self.search_vector(vector, max_tier, candidate_limit).await?;
        let text_hits = self.search_text(text, max_tier, candidate_limit).await?;

        let mut rrf_map: HashMap<String, (f32, DocumentChunk)> = HashMap::new();

        // Only include vector hits that have positive cosine similarity
        let relevant_vector_hits: Vec<ScoredChunk> = vector_hits
            .into_iter()
            .filter(|s| s.score > 0.0)
            .collect();

        let mut current_rank = 1;
        for (i, scored) in relevant_vector_hits.iter().enumerate() {
            if i > 0 && (scored.score - relevant_vector_hits[i - 1].score).abs() > 1e-6 {
                current_rank = i + 1;
            }
            let rrf = 1.0 / (60.0 + current_rank as f32);
            let entry = rrf_map
                .entry(scored.chunk.chunk_id.clone())
                .or_insert((0.0, scored.chunk.clone()));
            entry.0 += rrf;
        }

        // Only include text hits with positive scores
        let relevant_text_hits: Vec<ScoredChunk> = text_hits
            .into_iter()
            .filter(|s| s.score > 0.0)
            .collect();

        let mut current_rank = 1;
        for (i, scored) in relevant_text_hits.iter().enumerate() {
            if i > 0 && (scored.score - relevant_text_hits[i - 1].score).abs() > 1e-6 {
                current_rank = i + 1;
            }
            let rrf = 1.0 / (60.0 + current_rank as f32);
            let entry = rrf_map
                .entry(scored.chunk.chunk_id.clone())
                .or_insert((0.0, scored.chunk.clone()));
            entry.0 += rrf;
        }
```

### 4.4 Remediation Touchpoint 4: `crates/quorum-storage/tests/adversarial_storage_tests.rs:53-56`
**Problem**: Stale comment states "chunk_a and chunk_b have identical vector and identical text", which would break under Level-2 deduplication if implemented verbatim.  
**Remediation**: Clarify comment and add explicit assertions validating that co-scoring chunks receive equal RRF scores:

```rust
// In test_adversarial_rrf_identical_ranks_deterministic_tiebreak:
    // Create 3 chunks with identical RRF rank potential:
    // Chunks have identical vectors and equal-length text containing identical query terms,
    // but distinct initial tokens ("alpha", "beta", "gamma") so Level-2 BLAKE3 deduplication
    // does not skip inserting them.
    // We verify that ties are broken deterministically by chunk_id ASC across all iterations.
```
Add after line 102:
```rust
    // Verify that all co-scoring chunks receive identical RRF scores
    assert!(
        (first_result[0].score - first_result[1].score).abs() < 1e-6,
        "Co-scoring chunks must receive identical RRF score"
    );
    assert!(
        (first_result[1].score - first_result[2].score).abs() < 1e-6,
        "Co-scoring chunks must receive identical RRF score"
    );
```

---

## 5. Verification Method

To independently verify the bug and validate the proposed fix:

### 5.1 Reproduce Bug in Baseline (Before Fix)
```powershell
$env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
$env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
1..10 | ForEach-Object { cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_rrf_identical_ranks_deterministic_tiebreak -- --nocapture }
```
*Expected Failure*: Flaky panic at line 110: `assertion left == right failed; left: "chunk_tie_beta", right: "chunk_tie_alpha"`.

### 5.2 Validate Post-Remediation Stability (After Fix)
1. **Run 20 Iterations of the Tiebreak Test**:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
   $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   1..20 | ForEach-Object { cargo test -p quorum-storage --test adversarial_storage_tests test_adversarial_rrf_identical_ranks_deterministic_tiebreak -- --nocapture }
   ```
   *Expected Result*: 20/20 PASSED with 0 failures.

2. **Run Full Adversarial Storage Suite**:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
   $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo test -p quorum-storage --test adversarial_storage_tests -- --nocapture
   ```
   *Expected Result*: All 11 tests pass (when combined with BUG-M2-01 Tantivy deletion fix).

3. **Run Clippy Cleanliness**:
   ```powershell
   $env:PROTOC = "C:\Users\oyesanyf\.cargo\bin\protoc.exe"
   $env:PROTOC_INCLUDE = "C:\Users\oyesanyf\.cargo\include"
   cargo clippy -p quorum-storage --all-targets -- -D warnings
   ```
   *Expected Result*: 0 warnings, 0 errors.

---

## Invalidation Conditions
This analysis and remediation plan would be invalidated if:
1. LanceDB introduces a deterministic SQL `ORDER BY` clause inside its vector search pipeline that breaks vector distance ties natively before streaming `RecordBatch`es. (Currently not present in LanceDB Rust SDK `0.10.x`).
2. RRF is replaced with a score-weighted linear combiner (e.g. Cosine + BM25 linear blend) rather than reciprocal rank fusion. (Violates `PROJECT.md` and `ORIGINAL_REQUEST.md` architecture requirements).
