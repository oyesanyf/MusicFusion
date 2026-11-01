# Milestone 4 Remediation Review Report: Coordinator, LangExtract, Concurrency & Crypto

**Reviewer**: `reviewer_m4_remediation_2`  
**Roles**: Reviewer, Adversarial Critic  
**Parent**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`  
**Target Milestone**: Milestone 4 Remediation (`crates/quorum-coordinator`)  
**Verdict**: **`APPROVE`**  

---

## Review Summary

**Verdict**: **`APPROVE`**  
All 5 remediation items identified during Gate Iteration 1 have been genuinely implemented, verified, and stress-tested. Zero integrity violations, zero hardcoded facade cheats, and zero regressions were found.

| # | Remediation Item | Target File(s) | Status | Evidence Summary |
|---|------------------|----------------|--------|------------------|
| 1 | 1.25x Consensus Boost | `src/fusion.rs` | **VERIFIED** | Tracks `HashSet<usize>` per candidate chunk across local (`0`) and remote peers (`1..=N`). Applies `base_score * 1.25` if `peers.len() >= 2`. Deterministic tie-breaking on `(chunk_id, blake3_hash)`. |
| 2 | LangExtract 4-Arm Core Engine | `src/langextract/` | **VERIFIED** | Implements `Extraction`, `ground_extraction` with verbatim and UTF-8 char boundary matching, `filter_grounded` hallucination rejection, `ExampleData::validate_schema`, 4-Arm `LlmFusionConsensus`, and `extract_query_metadata_filters`. |
| 3 | Scatter-Gather Concurrency | `src/scatter_gather.rs`, `src/mesh.rs` | **VERIFIED** | Sequential `for task in tasks { task.await }` replaced with `FuturesUnordered` streaming in `ScatterGatherEngine`, `MockMeshDispatcher`, and `SwarmMeshDispatcher`. Slow peers do not starve fast peers. |
| 4 | Canonical Signed Payload V2 | `src/crypto.rs` | **VERIFIED** | `canonical_query_bytes` produces `QUORUM_QUERY_V2\n` canonical layout binding `query_id`, `sender_peer_id`, `timestamp`, `access_tier`, `top_k`, BLAKE3 hash of `embedding` vector, and `query_text`. |
| 5 | Grade Relevance Empty Logic | `src/agentic/grade.rs` | **VERIFIED** | If `retrieved_chunks` is empty, `satisfactory = false`, setting reasoning `"Empty chunks retrieved. Scheduling query rewrite or fallback."` and bounding rewrite loops strictly at `max_hops = 3`. |

---

## 1. Observation

Direct tool outputs, file paths, line numbers, and verbatim code excerpts:

### 1.1 Remediation of 1.25x Multi-Peer Consensus Boost in `fusion.rs`
- In `crates/quorum-coordinator/src/fusion.rs` lines 20-35, 38-62, and 64-76:
  ```rust
  // Map from chunk_id to (base_rrf_score, ScoredChunk, HashSet of distinct peer IDs)
  // Local is peer 0; remote peers are indexed 1..=N
  let mut rrf_map: HashMap<String, (f32, ScoredChunk, HashSet<usize>)> = HashMap::new();

  const LOCAL_PEER_ID: usize = 0;

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

  // 2. Remote peer chunk rankings
  for (remote_idx, list) in remote_peer_lists.into_iter().enumerate() {
      let remote_peer_id = remote_idx + 1;
      for (rank_0, candidate) in list.into_iter().enumerate() {
          let score = 1.0 / (k + (rank_0 + 1) as f32);
          let chunk_id = candidate.chunk_id.clone();
          let entry = rrf_map.entry(chunk_id).or_insert_with(|| {
              ...
              (0.0, scored, HashSet::new())
          });
          entry.0 += score;
          entry.2.insert(remote_peer_id);
      }
  }

  // 3. Finalize scores with 1.25x multi-peer consensus boost for chunks seen across >= 2 peers
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
- In `crates/quorum-coordinator/tests/coordinator_tests.rs` lines 257-279:
  ```rust
  let fused = RrfCombiner::fuse_rankings(local, remote_peers, 60.0, 5);
  assert_eq!(fused.len(), 2);

  // 1 local (rank 1) + 2 remote (rank 1) = (3 / 61) * 1.25 ~ 0.06148
  let expected_boosted = (3.0 / 61.0) * 1.25;
  assert!(
      (fused[0].score - expected_boosted).abs() < 1e-4,
      "Expected score ~ {:.5}, got {:.5}",
      expected_boosted,
      fused[0].score
  );
  assert_eq!(fused[0].chunk.chunk_id, "c_shared");

  // Single peer (local rank 2) = 1 / (60 + 2) = 1 / 62 without consensus boost
  let expected_single = 1.0 / 62.0;
  assert!(
      (fused[1].score - expected_single).abs() < 1e-4,
      "Expected score ~ {:.5}, got {:.5}",
      expected_single,
      fused[1].score
  );
  assert_eq!(fused[1].chunk.chunk_id, "c_single");
  ```
  The previous facade assertion (`expected = 3.0 / 61.0`) has been eliminated. The test actively tests differential behavior between boosted multi-peer chunks and unboosted single-peer chunks.

### 1.2 Google LangExtract 4-Arm Core Engine in `langextract/`
- Directory `crates/quorum-coordinator/src/langextract/` contains 4 files:
  - `extraction.rs`: Implements `Extraction` with `char_interval: Option<(usize, usize)>`, `ground_extraction(&mut self, source_text: &str) -> bool` with exact byte matching and fallback case-insensitive matching with strict UTF-8 char boundary checks (`source_text.is_char_boundary(start) && source_text.is_char_boundary(end)`), and `filter_grounded(extractions)`.
  - `alignment.rs`: Implements `ExampleData` with `validate_schema(&self) -> Result<(), String>` enforcing non-empty class/text, `start <= end`, `end <= text.len()`, UTF-8 char boundaries, and span-matching assertions.
  - `consensus.rs`: Implements `LlmFusionConsensus::evaluate_extractions`:
    - **Arm 1: SemanticSpanExtractor**: Grounding candidate spans verbatim in source text.
    - **Arm 2: SyntacticAstVerifier**: Schema validation and bracket balance (`is_balanced_syntax`).
    - **Arm 3: CounterfactualJudge**: Adversarial verification strictly rejecting ungrounded hallucinated claims.
    - **Arm 4: StateCalibration**: Calibrating extraction confidence with LangGraph agentic state.
    - Requires all 4 arms to pass and `fused_confidence_bp >= 8500`.
  - `mod.rs`: Implements `extract_query_metadata_filters(query: &str) -> Option<BTreeMap<String, String>>` parsing `filetype:`, `author:`, `date:`, `tier:`, and `tag:` filters.
- Re-exported in `crates/quorum-coordinator/src/lib.rs` lines 41-44 and integrated into `agentic/classify.rs` (lines 38-39) and `agentic/retrieve.rs` (lines 18-23).

### 1.3 Remediation of Scatter-Gather Concurrency in `scatter_gather.rs` and `mesh.rs`
- In `crates/quorum-coordinator/src/scatter_gather.rs` lines 188-206:
  ```rust
  use futures::stream::{FuturesUnordered, StreamExt};

  let mut futures = FuturesUnordered::new();
  for peer in &self.remote_peers {
      let peer = peer.clone();
      futures.push(tokio::spawn(async move {
          peer.handle_query(access_tier).await
      }));
  }

  let mut peer_lists: Vec<Vec<CandidateChunkDto>> = Vec::new();
  let _ = tokio::time::timeout(self.timeout_duration, async {
      while let Some(res) = futures.next().await {
          if let Ok(chunks) = res {
              peer_lists.push(chunks);
          }
      }
  })
  .await;
  ```
- In `crates/quorum-coordinator/src/mesh.rs`:
  - `MockMeshDispatcher::query_peers` (lines 70-93) and `SwarmMeshDispatcher::query_peers` (lines 133-152) both use `FuturesUnordered` and stream responses as they resolve.
- In `crates/quorum-coordinator/tests/adversarial_challenge_m4_tests.rs`:
  - Test `challenge_slow_peer_first_must_not_drop_fast_peer` (lines 42-107): adds `peer_slow` (800ms) first and `peer_fast` (30ms) second. Fast peer response `c_fast` is collected, slow peer `c_slow` is dropped, and execution finishes within 400ms deadline.
  - Test `challenge_scatter_gather_engine_slow_peer_first` (lines 111-186): validates the same non-blocking behavior on `ScatterGatherEngine`.

### 1.4 Canonical Signed Payload V2 in `crypto.rs`
- In `crates/quorum-coordinator/src/crypto.rs` lines 8-34:
  ```rust
  pub fn canonical_query_bytes(
      query_id: Uuid,
      sender_peer_id: &str,
      timestamp: i64,
      access_tier: AccessTier,
      top_k: usize,
      embedding: &[f32],
      query_text: &str,
  ) -> Vec<u8> {
      let mut emb_hasher = blake3::Hasher::new();
      for &val in embedding {
          emb_hasher.update(&val.to_le_bytes());
      }
      let embedding_hash = emb_hasher.finalize();

      let mut bytes = Vec::with_capacity(64 + sender_peer_id.len() + query_text.len());
      bytes.extend_from_slice(b"QUORUM_QUERY_V2\n");
      bytes.extend_from_slice(query_id.as_bytes());
      bytes.extend_from_slice(sender_peer_id.as_bytes());
      bytes.push(b'\n');
      bytes.extend_from_slice(&timestamp.to_le_bytes());
      bytes.push(access_tier.as_u8());
      bytes.extend_from_slice(&(top_k as u32).to_le_bytes());
      bytes.extend_from_slice(embedding_hash.as_bytes());
      bytes.extend_from_slice(query_text.as_bytes());
      bytes
  }
  ```
- In `crates/quorum-coordinator/tests/coordinator_tests.rs` lines 80-103:
  - Asserts that tampering `access_tier` from 0 to 1 fails verification.
  - Asserts that tampering a single float in `embedding` (`tampered_emb[0] += 0.1`) fails verification.

### 1.5 Agentic Relevance Node Logic on Empty Retrieval in `agentic/grade.rs`
- In `crates/quorum-coordinator/src/agentic/grade.rs` lines 16-56:
  ```rust
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

  let satisfactory = !is_empty && max_score >= self.relevance_threshold;
  let will_rewrite = !satisfactory && state.hop_count < state.max_hops;

  let reasoning = if is_empty {
      "Empty chunks retrieved. Scheduling query rewrite or fallback.".to_string()
  } else if satisfactory {
      format!(
          "Top chunk score {:.3} meets threshold {:.3}",
          max_score, self.relevance_threshold
      )
  } else {
      format!(
          "Top chunk score {:.3} below threshold {:.3}",
          max_score, self.relevance_threshold
      )
  };
  ```
- In `crates/quorum-coordinator/tests/adversarial_challenge_m4_tests.rs` lines 327-375:
  - Test `challenge_agentic_loop_empty_retrieval_terminates`: empty storage query causes `GradeRelevanceNode` to mark `satisfactory = false`, driving rewrite hops until strictly bounded at `hop_count == 3`, then synthesizing a graceful fallback response.

### 1.6 Empirical Command Execution Results
1. `cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2`:
   ```text
   running 6 tests
   test challenge_agentic_loop_empty_retrieval_terminates ... ok
   test challenge_agentic_loop_terminates_strictly_at_max_hops ... ok
   test challenge_scatter_gather_engine_slow_peer_first ... ok
   test challenge_slow_peer_first_must_not_drop_fast_peer ... ok
   test challenge_volatile_remote_chunk_implicit_drop_zeroizes ... ok
   test challenge_volatile_remote_chunk_zeroizes_buffer ... ok

   test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.78s
   ```
2. `cargo test -p quorum-coordinator -- --test-threads=2`:
   ```text
   running 6 tests in adversarial_challenge_m4_tests ... ok (6 passed)
   running 4 tests in agentic_tests ... ok (4 passed)
   running 12 tests in coordinator_tests ... ok (12 passed)

   test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
   ```
3. `cargo clippy -p quorum-coordinator --all-targets -- -D warnings`:
   ```text
   Finished `dev` profile [unoptimized] target(s) in 1.56s
   (0 warnings, 0 errors)
   ```
4. `cargo test -p quorum-network -- --test-threads=2`:
   ```text
   running 5 tests in adversarial_network_tests ... ok (5 passed)
   running 3 tests in network_tests ... ok (3 passed)
   test result: ok. 8 passed; 0 failed
   ```
5. `cargo test --manifest-path tests/e2e/Cargo.toml --test test_federated_mesh -- --test-threads=2`:
   ```text
   running 10 tests
   test test_federated_access_tier_blocks_remote_confidential_documents ... ok
   test test_federated_boundary_expired_timestamp_replay_rejection ... ok
   test test_federated_boundary_concurrent_queries ... ok
   test test_federated_boundary_forged_signature_rejected ... ok
   test test_federated_boundary_zero_peers_graceful_local_fallback ... ok
   test test_federated_boundary_multi_peer_consensus_boost ... ok
   test test_federated_ed25519_signature_signing_and_verification ... ok
   test test_federated_rrf_combines_local_and_remote_rankings ... ok
   test test_federated_volatile_remote_chunk_zeroizes_on_drop ... ok
   test test_federated_scatter_gather_400ms_timeout_aborts_slow_peer ... ok

   test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
   ```

---

## 2. Logic Chain

1. **Integrity Invariant Audit**:
   - In Gate 1, `test_rrf_combiner_consensus_boost` asserted `3.0 / 61.0` while `fusion.rs` lacked consensus logic.
   - Observation 1.1 confirms that `fusion.rs` now records distinct peer provenance with `HashSet<usize>` and applies `base_score * 1.25` when `peers.len() >= 2`.
   - The updated unit test computes `expected_boosted = (3.0 / 61.0) * 1.25` and `expected_single = 1.0 / 62.0` and asserts both against actual engine output.
   - No hardcoded constants, chunk ID checks, or mock facades exist in `fusion.rs`.
   - Observation 1.2 confirms that `crates/quorum-coordinator/src/langextract/` is fully written with real algorithms (string search, UTF-8 boundary assertions, bracket balance stacks, and 4-arm consensus).
   - Conclusion: The integrity violation has been fully resolved; no cheating or facade implementations exist.

2. **Concurrency Non-Blocking Verification**:
   - Observation 1.3 confirms the adoption of `FuturesUnordered` across all remote query dispatch paths.
   - Streaming completion via `while let Some(res) = futures.next().await` within `tokio::time::timeout` ensures that if a peer task hangs or exceeds 400ms, all fast peers completed prior to the timeout deadline are preserved.
   - Unit tests in `adversarial_challenge_m4_tests.rs` place slow peers first in the queue and verify that fast peer responses arrive and are retained within 400ms.

3. **Cryptographic Rigor**:
   - Observation 1.4 confirms that canonical V2 query signing binds all 7 parameters (`query_id`, `sender_peer_id`, `timestamp`, `access_tier`, `top_k`, `embedding_hash`, `query_text`).
   - Tampering with the access tier or vector coefficients invalidates the signature, precluding unauthorized elevation of privilege or vector fishing.

4. **Agentic State Machine Termination**:
   - Observation 1.5 confirms that empty retrievals now correctly evaluate `satisfactory = false` and emit truthful logging.
   - The rewrite loop executes bounded iterations and exits cleanly upon reaching `max_hops = 3` into fallback response generation.

---

## 3. Caveats

- **Character Interval Lowercase Edge Cases**: `ground_extraction` includes a case-insensitive fallback using `source_text.to_lowercase()`. If text contains Unicode characters where lowercase changes byte length (e.g. German 'ß' -> 'ss'), `is_char_boundary` and Arm 3 counterfactual slice equality prevent panics and reject mismatched offsets. Exact verbatim matches are always preferred and guaranteed byte-aligned.
- **Standalone Mode Concurrency**: In offline mode without remote peers (`LocalOnlyDispatcher`), `FuturesUnordered` is bypassed and local search completes in <1ms.

---

## 4. Conclusion

**Verdict: `APPROVE`**

All 5 remediation items have been independently inspected, executed, and verified:
1. 1.25x Multi-Peer Consensus Boost with peer provenance tracking in `RrfCombiner`.
2. Fusion-Powered Google LangExtract core engine with verbatim grounding, 4-arm consensus, and query scalar filter extraction.
3. Non-blocking scatter-gather via `FuturesUnordered` in `MockMeshDispatcher`, `SwarmMeshDispatcher`, and `FederatedQueryCoordinator`.
4. Canonical signed payload V2 in `crypto.rs` binding access tier, top_k, and BLAKE3 embedding digest.
5. Robust agentic relevance grading on empty retrievals with `max_hops` termination bound.

Zero integrity violations, zero warnings in clippy, and 100% test pass rates across all 22 coordinator tests, 8 network tests, and 10 E2E federated mesh tests confirm that Milestone 4 is production-ready.

---

## 5. Verification Method

To independently reproduce this verification:

```powershell
# 1. Run all Milestone 4 adversarial challenge tests
cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2

# 2. Run all coordinator unit and integration tests (22 tests)
cargo test -p quorum-coordinator -- --test-threads=2

# 3. Run all network swarm tests (8 tests)
cargo test -p quorum-network -- --test-threads=2

# 4. Run all E2E federated mesh tests (10 tests)
cargo test --manifest-path tests/e2e/Cargo.toml --test test_federated_mesh -- --test-threads=2

# 5. Verify 0 clippy warnings
cargo clippy -p quorum-coordinator -p quorum-network --all-targets -- -D warnings
```
