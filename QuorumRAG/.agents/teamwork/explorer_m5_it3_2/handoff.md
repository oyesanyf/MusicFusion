# Investigation Report: Relevance Grading Calibration Fix in `GradeRelevanceNode`

- **Author**: `explorer_m5_it3_2` (Agentic Grader Convergence Explorer)
- **Recipient**: Parent Orchestrator (`e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`)
- **Date**: 2026-11-01T03:32:00Z
- **Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_2\`
- **Target**: `crates/quorum-coordinator/src/agentic/grade.rs`

---

## 1. Observation

### 1.1 Direct Source Code Observations

1. **Current Implementation in `crates/quorum-coordinator/src/agentic/grade.rs`** (lines 16–38):
   ```rust
   impl GradeRelevanceNode {
       pub fn execute(&self, state: &mut AgenticGraphState) -> (RelevanceGrade, bool) {
           let is_empty = state.retrieved_chunks.is_empty();
           let max_score = if is_empty {
               0.0f32
           } else {
               state
                   .retrieved_chunks
                   .iter()
                   .map(|c| c.score)
                   .fold(0.0f32, f32::max)
           };

           state.relevance_score = max_score;

           let effective_threshold = if max_score > 0.0 && max_score <= 0.1 {
               self.relevance_threshold * (1.0 / 60.0)
           } else {
               self.relevance_threshold
           };

           let satisfactory = !is_empty && max_score >= effective_threshold;
           let will_rewrite = !satisfactory && state.hop_count < state.max_hops;
   ```

2. **RRF Score Overwrite in `crates/quorum-coordinator/src/fusion.rs`** (lines 27–35, 67–75):
   ```rust
   // 1. Local chunk rankings
   for (rank_0, scored) in local_results.into_iter().enumerate() {
       let score = 1.0 / (k + (rank_0 + 1) as f32);
       let chunk_id = scored.chunk.chunk_id.clone();
       let entry = rrf_map
           .entry(chunk_id)
           .or_insert_with(|| (0.0, scored, HashSet::new()));
       entry.0 += score;
       entry.2.insert(LOCAL_PEER_ID);
   }
   ...
   let mut fused: Vec<ScoredChunk> = rrf_map
       .into_iter()
       .map(|(_id, (base_score, mut chunk, peers))| {
           let final_score = if peers.len() >= 2 {
               base_score * 1.25
           } else {
               base_score
           };
           chunk.score = final_score;
           chunk
       })
       .collect();
   ```
   At rank 0 with single peer and $k=60.0$, `final_score = 1.0 / (60.0 + 1.0) = 0.016393441`.
   `chunk.score` is unconditionally overwritten with `final_score`.

3. **Mock Storage in Coordinator Tests**:
   - `crates/quorum-coordinator/tests/adversarial_challenge_m4_tests.rs` (lines 204–222):
     ```rust
     // Return candidate with score 0.0, far below threshold 0.60
     vec![ScoredChunk::new(
         DocumentChunk::new(
             "zero_chunk",
             "doc_zero",
             "irrelevant.txt",
             1,
             1,
             0,
             vec![0.0; 768],
             "Completely irrelevant text",
             None,
         )
         .unwrap(),
         0.0,
         "local",
         false,
     )]
     ```
   - `crates/quorum-coordinator/tests/agentic_tests.rs` (lines 22–40):
     ```rust
     // Return a chunk with intentionally low score (0.2 < threshold 0.6) to force query rewrites
     vec![ScoredChunk::new(
         DocumentChunk::new(
             "low_chunk",
             "doc",
             "notes.txt",
             1,
             2,
             0,
             vec![0.0; 768],
             "Low relevance context",
             None,
         )
         .unwrap(),
         0.20,
         "local",
         false,
     )]
     ```

### 1.2 Verbatim Command Execution & Failure Outputs

1. **`cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- challenge_agentic_loop_terminates_strictly_at_max_hops`**:
   ```text
   running 1 test
   test challenge_agentic_loop_terminates_strictly_at_max_hops ... FAILED

   failures:

   ---- challenge_agentic_loop_terminates_strictly_at_max_hops stdout ----

   thread 'challenge_agentic_loop_terminates_strictly_at_max_hops' panicked at crates\quorum-coordinator\tests\adversarial_challenge_m4_tests.rs:259:5:
   assertion `left == right` failed: State graph must terminate at exactly max_hops (3)
     left: 0
    right: 3
   ```

2. **`cargo test -p quorum-coordinator --test agentic_tests -- test_agentic_loop_convergence_max_hops`**:
   ```text
   running 1 test
   test test_agentic_loop_convergence_max_hops ... FAILED

   failures:

   ---- test_agentic_loop_convergence_max_hops stdout ----

   thread 'test_agentic_loop_convergence_max_hops' panicked at crates\quorum-coordinator\tests\agentic_tests.rs:208:5:
   assertion `left == right` failed: Monotonically bounded loop must stop at exactly max_hops (3)
     left: 0
    right: 3
   ```

3. **`coordinator_tests.rs`**:
   `cargo test -p quorum-coordinator --test coordinator_tests`: 12 passed, 0 failed.

4. **Live IDE Execution Step 5 Citation Requirement**:
   In `scripts/verify_live_ide_execution.ps1` lines 185–194:
   ```powershell
   $foundFusionCitation = $false
   foreach ($cit in $citations) {
       Write-Host "  - [$($cit.peer_id):$($cit.file_path):$($cit.line_start)-$($cit.line_end)] (Score: $($cit.score))" -ForegroundColor Gray
       if ($cit.file_path -like "*fusion.rs*") {
           $foundFusionCitation = $true
       }
   }
   if (-not $foundFusionCitation) {
       throw "Assertion Failed: Did not receive citation pointing to fusion.rs!"
   }
   ```

---

## 2. Logic Chain

1. **The RRF Score Transformation**:
   - In `ScatterGatherEngine::execute_query`, `local_storage.search_hybrid(...)` returns local chunks.
   - Even when `ZeroScoreStorage` returns `score: 0.0` or `DummyStorage` returns `score: 0.20`, `ScatterGatherEngine` passes the chunks into `RrfCombiner::fuse_rankings`.
   - Because the test mock returns exactly 1 chunk, that chunk occupies rank 0 in `local_results`.
   - `RrfCombiner::fuse_rankings` computes $score = \frac{1}{60 + (0 + 1)} = \frac{1}{61} \approx 0.016393441$ and unconditionally overwrites `chunk.score = final_score` (line 73).
   - Thus, when chunks enter `GradeRelevanceNode::execute`, `max_score` is mathematically equal to `0.016393441`.

2. **Why the M5 Iteration 2 Blanket Threshold Broke Loop Convergence**:
   - In commit `57e426d`, the worker introduced:
     `let effective_threshold = if max_score > 0.0 && max_score <= 0.1 { self.relevance_threshold * (1.0 / 60.0) } else { self.relevance_threshold };`
   - With `self.relevance_threshold = 0.60`, `effective_threshold` is evaluated as $0.60 \times \frac{1}{60} = 0.010$.
   - Because rank-1 RRF hits always score $\frac{1}{61} \approx 0.016393441$, and $0.016393441 \ge 0.010$, `satisfactory` evaluates to `true` on the very first hop (hop 0) for ANY retrieval returning at least one chunk.
   - Consequently, `will_rewrite = !satisfactory && state.hop_count < state.max_hops` is `false`, the query rewrite loop is bypassed entirely, and both tests panic with `left: 0, right: 3`.

3. **Why the Unscaled 0.60 Threshold Fails Live IDE Execution**:
   - In live IDE execution, `ScatterGatherEngine` returns genuine matching chunks (e.g. from `crates/quorum-coordinator/src/fusion.rs`).
   - Because the results are RRF-fused, the top chunk's score is $\le 0.01639$.
   - If an unscaled threshold of $0.60$ is applied, $0.01639 < 0.60$ causes `GradeRelevanceNode` to mark the retrieval as unsatisfactory on every hop.
   - `RewriteQueryNode` appends `"(disambiguation hop N)"` to the query. Because Tantivy BM25 indexes all files in the workspace, the rare words "disambiguation" and "hop" match `crates/quorum-coordinator/src/agentic/rewrite.rs` and `agentic_tests.rs` with higher BM25 weights, displacing `fusion.rs` from top citations and causing `scripts/verify_live_ide_execution.ps1` to fail.

4. **The Root Asymmetry & Resolution**:
   - An RRF score of $0.01639$ indicates **relative rank within a result list**, not **absolute relevance to the query intent**.
   - If a retrieval returns irrelevant text (e.g. "Completely irrelevant text" in `ZeroScoreStorage` or "Low relevance context" in `DummyStorage`), the chunk still receives rank 1 and score $0.01639$, but has ZERO semantic overlap with the query.
   - If a retrieval returns relevant code (e.g. `fusion.rs` containing `pub struct RrfCombiner;`), it also receives rank 1 and score $0.01639$, but possesses high lexical and semantic overlap with the query terms.
   - Therefore, in the RRF score regime ($0.0 < max\_score \le 0.10$), `GradeRelevanceNode` must gate threshold scaling on **content relevance**:
     - Extract significant query terms from `state.original_prompt` (alphanumeric tokens with length $\ge 3$, excluding standard English stop words).
     - Check if any retrieved chunk contains any of the significant query terms in its `text_content` or `file_path`.
     - If content relevance is confirmed AND `max_score >= self.relevance_threshold * (1.0 / 60.0)` (0.010): `satisfactory = true`.
     - If content relevance fails (as in `ZeroScoreStorage` and `DummyStorage`): `effective_threshold` remains `self.relevance_threshold` (0.60), evaluating to `satisfactory = false` and triggering query rewrites up to `max_hops`.
     - When scores are outside the RRF regime ($max\_score > 0.10$), the unscaled threshold (0.60) applies directly, cleanly handling raw cosine similarities.

---

## 3. Caveats

1. **Standalone Verification Harness in `scripts/verify_live_ide_execution.ps1`**:
   - While `grade.rs` is the root cause of the 2 test failures in `quorum-coordinator`, `scripts/verify_live_ide_execution.ps1` contains a separate issue noted in `auditor_m5_remediation_1` and `reviewer_m5_remediation_2`:
     - Line 205: `Remove-Item` on a static `$verifyHarnessDir` without `-ErrorAction SilentlyContinue` can crash with `PermissionDenied` on Windows when executable files are locked by the OS.
     - Line 215: Including `quorum-coordinator` in the temporary verification crate harness's `Cargo.toml` pulls `aws-lc-sys` into an external build, which fails on machines without NASM.
     - The worker should use a unique directory (`Join-Path $env:TEMP ("quorumrag_codegen_verify_" + [System.Guid]::NewGuid().ToString("N"))`) and omit `quorum-coordinator` from dependencies in the temporary test harness.

2. **No Assumptions on Upstream Storage**:
   - The design in `GradeRelevanceNode` does not assume `ScatterGatherEngine` or `RrfCombiner` will be modified, ensuring full backward compatibility and zero side-effects on other crates.

---

## 4. Conclusion & Proposed Calibration Design

### 4.1 Proposed Implementation for `crates/quorum-coordinator/src/agentic/grade.rs`

Replace `crates/quorum-coordinator/src/agentic/grade.rs` with the following clean, robust implementation:

```rust
use crate::types::{AgenticGraphState, RelevanceGrade};
use std::collections::HashSet;

pub struct GradeRelevanceNode {
    pub relevance_threshold: f32,
}

impl Default for GradeRelevanceNode {
    fn default() -> Self {
        Self {
            relevance_threshold: 0.60,
        }
    }
}

impl GradeRelevanceNode {
    /// Determines whether any retrieved chunk contains significant query terms from the prompt.
    pub fn has_content_relevance(prompt: &str, chunks: &[quorum_types::ScoredChunk]) -> bool {
        let stop_words: HashSet<&'static str> = [
            "the", "and", "for", "with", "show", "that", "this", "from", "have",
            "what", "where", "when", "some", "into", "write", "about", "using",
            "are", "you", "can", "how", "why", "who", "all", "any", "will",
            "would", "should", "could", "did", "does", "been", "has", "had"
        ]
        .into_iter()
        .collect();

        let query_terms: Vec<String> = prompt
            .to_ascii_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() >= 3 && !stop_words.contains(w))
            .map(|w| w.to_string())
            .collect();

        if query_terms.is_empty() {
            return false;
        }

        chunks.iter().any(|c| {
            let content_lower = c.chunk.text_content.to_ascii_lowercase();
            let path_lower = c.chunk.file_path.to_ascii_lowercase();
            query_terms.iter().any(|term| {
                content_lower.contains(term) || path_lower.contains(term)
            })
        })
    }

    pub fn execute(&self, state: &mut AgenticGraphState) -> (RelevanceGrade, bool) {
        let is_empty = state.retrieved_chunks.is_empty();
        let max_score = if is_empty {
            0.0f32
        } else {
            state
                .retrieved_chunks
                .iter()
                .map(|c| c.score)
                .fold(0.0f32, f32::max)
        };

        state.relevance_score = max_score;

        let has_content_match = !is_empty && Self::has_content_relevance(&state.original_prompt, &state.retrieved_chunks);

        let effective_threshold = if max_score > 0.0 && max_score <= 0.1 {
            if has_content_match {
                self.relevance_threshold * (1.0 / 60.0)
            } else {
                self.relevance_threshold
            }
        } else {
            self.relevance_threshold
        };

        let satisfactory = !is_empty && max_score >= effective_threshold;
        let will_rewrite = !satisfactory && state.hop_count < state.max_hops;

        let reasoning = if is_empty {
            "Empty chunks retrieved. Scheduling query rewrite or fallback.".to_string()
        } else if satisfactory {
            format!(
                "Top chunk score {:.3} meets threshold {:.3}{}",
                max_score,
                effective_threshold,
                if has_content_match && max_score <= 0.1 { " (RRF content verified)" } else { "" }
            )
        } else if max_score <= 0.1 && !has_content_match {
            format!(
                "Top chunk score {:.3} in RRF regime but failed content relevance check against query intent",
                max_score
            )
        } else {
            format!(
                "Top chunk score {:.3} below threshold {:.3}",
                max_score, effective_threshold
            )
        };

        let grade = RelevanceGrade {
            is_relevant: satisfactory,
            score: max_score,
            reasoning,
            satisfactory,
        };

        state.relevance_grade = Some(grade.clone());
        (grade, will_rewrite)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quorum_types::{DocumentChunk, ScoredChunk};

    fn make_test_chunk(id: &str, path: &str, content: &str, score: f32) -> ScoredChunk {
        let doc = DocumentChunk::new(
            id,
            "test_doc",
            path,
            1,
            5,
            0,
            vec![0.0; 768],
            content,
            None,
        )
        .unwrap();
        ScoredChunk::new(doc, score, "local", false)
    }

    #[test]
    fn test_grade_relevance_empty_chunks_fails() {
        let grader = GradeRelevanceNode::default();
        let mut state = AgenticGraphState::new("Test query", crate::types::SynthesisModelFamily::AutoFusion, 3);
        let (grade, will_rewrite) = grader.execute(&mut state);
        assert!(!grade.satisfactory);
        assert!(!grade.is_relevant);
        assert!(will_rewrite);
    }

    #[test]
    fn test_grade_relevance_rrf_unrelated_content_fails_and_rewrites() {
        let grader = GradeRelevanceNode::default();
        let mut state = AgenticGraphState::new(
            "Adversarial query testing loop limit",
            crate::types::SynthesisModelFamily::AutoFusion,
            3,
        );
        state.retrieved_chunks = vec![make_test_chunk("c1", "notes.txt", "Low relevance context", 0.01639)];
        let (grade, will_rewrite) = grader.execute(&mut state);
        assert!(!grade.satisfactory);
        assert!(will_rewrite);
    }

    #[test]
    fn test_grade_relevance_rrf_matching_content_passes() {
        let grader = GradeRelevanceNode::default();
        let mut state = AgenticGraphState::new(
            "Show me the RrfCombiner implementation in QuorumRAG",
            crate::types::SynthesisModelFamily::AutoFusion,
            3,
        );
        state.retrieved_chunks = vec![make_test_chunk("c1", "fusion.rs", "pub struct RrfCombiner;", 0.01639)];
        let (grade, will_rewrite) = grader.execute(&mut state);
        assert!(grade.satisfactory);
        assert!(!will_rewrite);
    }

    #[test]
    fn test_grade_relevance_high_cosine_score_passes() {
        let grader = GradeRelevanceNode::default();
        let mut state = AgenticGraphState::new("Any query", crate::types::SynthesisModelFamily::AutoFusion, 3);
        state.retrieved_chunks = vec![make_test_chunk("c1", "doc.txt", "Arbitrary text", 0.85)];
        let (grade, will_rewrite) = grader.execute(&mut state);
        assert!(grade.satisfactory);
        assert!(!will_rewrite);
    }

    #[test]
    fn test_grade_relevance_max_hops_exhaustion_does_not_rewrite() {
        let grader = GradeRelevanceNode::default();
        let mut state = AgenticGraphState::new("Unrelated query", crate::types::SynthesisModelFamily::AutoFusion, 3);
        state.hop_count = 3;
        state.retrieved_chunks = vec![make_test_chunk("c1", "doc.txt", "Completely irrelevant", 0.01639)];
        let (grade, will_rewrite) = grader.execute(&mut state);
        assert!(!grade.satisfactory);
        assert!(!will_rewrite, "Must not rewrite when hop_count reached max_hops");
    }
}
```

---

## 5. Verification Method

To independently verify this solution:

1. **Verify Unit Tests in `quorum-coordinator`**:
   ```powershell
   cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- challenge_agentic_loop_terminates_strictly_at_max_hops
   cargo test -p quorum-coordinator --test agentic_tests -- test_agentic_loop_convergence_max_hops
   ```
   *Expected Result*: Both tests pass cleanly (`test result: ok. 1 passed; 0 failed`).

2. **Verify Full `quorum-coordinator` Test Suite**:
   ```powershell
   cargo test -p quorum-coordinator -- --test-threads=2
   ```
   *Expected Result*: All 27 tests across `coordinator_tests.rs`, `adversarial_challenge_m4_tests.rs`, `agentic_tests.rs`, and unittests pass cleanly (0 failed).

3. **Verify Full Workspace & Compiler Lints**:
   ```powershell
   cargo clippy -p quorum-coordinator --all-targets -- -D warnings
   ```
   *Expected Result*: 0 warnings, clean compilation.

4. **Verify Live IDE Execution Script**:
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/verify_live_ide_execution.ps1
   ```
   *Expected Result*: All 5 live IDE execution gates pass cleanly, citing `fusion.rs` with exact line numbers and verifying production code generation.
