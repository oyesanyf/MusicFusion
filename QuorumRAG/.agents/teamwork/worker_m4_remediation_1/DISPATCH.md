# DISPATCH — 2026-10-31T23:55:00Z

## Target
`worker_m4_remediation_1`

## Role
Milestone 4 Remediation Worker (crates/quorum-coordinator)

## Objective
Remediate all issues identified during Milestone 4 Gate Iteration 1 by Reviewer 2 (`reviewer_m4_2`) and Challenger 2 (`challenger_m4_2`). Ensure 100% of coordinator tests, network tests, federated mesh tests, adversarial tests, and clippy pass cleanly.

## Context & Inputs
- `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md` (lines 165-212 for LangExtract mandate)
- `d:\harfile\QuorumRAG\PROJECT.md` (Milestone 4 specification)
- `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m4_2\handoff.md` (Detailed review findings)
- `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m4_2\handoff.md` (Adversarial challenge test failure details)
- `d:\harfile\QuorumRAG\crates\quorum-coordinator\tests\adversarial_challenge_m4_tests.rs` (Failing test suite to make green)

## Exclusive Write Ownership
- `crates/quorum-coordinator/src/fusion.rs`
- `crates/quorum-coordinator/src/langextract/` (new module: `mod.rs`, `extraction.rs`, `consensus.rs`, `alignment.rs`)
- `crates/quorum-coordinator/src/scatter_gather.rs`
- `crates/quorum-coordinator/src/mesh.rs`
- `crates/quorum-coordinator/src/crypto.rs`
- `crates/quorum-coordinator/src/agentic/grade.rs`
- `crates/quorum-coordinator/src/lib.rs`
- `crates/quorum-coordinator/tests/coordinator_tests.rs`
- `crates/quorum-coordinator/tests/adversarial_challenge_m4_tests.rs`

## Detailed Remediation Instructions

### 1. [CRITICAL - Integrity Fix] 1.25x Multi-Peer Consensus Boost (`crates/quorum-coordinator/src/fusion.rs`)
- In `RrfCombiner::fuse_rankings`:
  - Track distinct peer provenance for each chunk: count how many unique peers (local counts as 1 peer; each remote peer list is identified by index or peer provenance) returned the chunk.
  - Compute base RRF score: $\sum \frac{1.0}{k + \text{rank}}$.
  - If a chunk is returned by $\ge 2$ distinct sources (e.g. local + 1 remote, or 2 remote peers), multiply its final score by $1.25$: `final_score = base_score * 1.25`.
  - In `crates/quorum-coordinator/tests/coordinator_tests.rs`: Update `test_rrf_combiner_consensus_boost` to assert the boosted score:
    `let expected = (3.0 / 61.0) * 1.25;`
    Verify that an unboosted single-peer chunk gets $1.0 / 61.0$ without multiplier.

### 2. [CRITICAL - Missing Requirement] Implement Fusion-Powered LangExtract Core Engine (`crates/quorum-coordinator/src/langextract/`)
- Adapt architecture from `D:\harfile\OshoosiClaw\crates\osoosi-agent-anomaly\src\breakout_defense.rs` (lines 3440–3697) and `arxiv-langextract` skill:
  - `extraction.rs`:
    - `Extraction` struct:
      - `pub extraction_class: String`
      - `pub extraction_text: String`
      - `pub char_interval: Option<(usize, usize)>`
      - `pub attributes: BTreeMap<String, String>`
      - `pub source_chunk_id: Option<String>`
      - `pub source_file_path: Option<String>`
    - Verbatim text grounding function `ground_extraction(extraction: &mut Extraction, source_text: &str) -> bool`:
      - Searches `source_text` for verbatim substring match of `extraction.extraction_text`.
      - If found at byte index `start`, sets `extraction.char_interval = Some((start, start + len))`.
      - If not found (or case-insensitive match fallback), returns false / sets `char_interval = None`.
    - Hallucination filter: `filter_grounded(extractions: Vec<Extraction>) -> Vec<Extraction>` strictly retains only items where `extraction.char_interval.is_some()`.
  - `alignment.rs`:
    - `ExampleData` struct for few-shot schema alignment:
      - `pub text: String`
      - `pub extractions: Vec<Extraction>`
    - Schema validation function: checks that example extractions match class rules and have valid bounds within example text.
  - `consensus.rs`:
    - `LlmFusionConsensus` struct implementing 4-arm verification:
      - Arm 1: `SemanticSpanExtractor` (extracts candidate spans with verbatim grounding).
      - Arm 2: `SyntacticAstVerifier` (verifies syntax / balance / schema constraints).
      - Arm 3: `CounterfactualJudge` (adversarial check: verifies candidate is grounded in source text, rejecting ungrounded claims).
      - Arm 4: `StateCalibration` (calibrates extraction confidence with LangGraph state).
    - Returns high-confidence, verified extraction entities and structured metadata attributes.
  - Query-time scalar filter hook:
    - Provide helper `extract_query_metadata_filters(query: &str) -> Option<BTreeMap<String, String>>` so `ClassifyIntentNode` can extract file-type, date, or author constraints to guide `RetrieveMeshNode`.
  - Expose `pub mod langextract;` in `crates/quorum-coordinator/src/lib.rs`.

### 3. [CRITICAL BUG - Scatter-Gather Head-of-Line Blocking]
- In `crates/quorum-coordinator/src/scatter_gather.rs` lines 195-203 (`FederatedQueryCoordinator::execute_query`):
  - Replace sequential `for task in tasks { task.await }` loop with `futures::stream::FuturesUnordered`:
    ```rust
    use futures::stream::{FuturesUnordered, StreamExt};

    let mut futures = FuturesUnordered::new();
    for task in tasks {
        futures.push(task);
    }
    let _ = tokio::time::timeout(self.timeout_duration, async {
        while let Some(res) = futures.next().await {
            if let Ok(chunks) = res {
                peer_lists.push(chunks);
            }
        }
    }).await;
    ```
- In `crates/quorum-coordinator/src/mesh.rs` lines 83-91 (`MockMeshDispatcher::query_peers`):
  - Replace sequential task loop with `FuturesUnordered`.
- In `crates/quorum-coordinator/src/mesh.rs` lines 140-148 (`SwarmMeshDispatcher::query_peers`):
  - Replace sequential task loop with `FuturesUnordered`.
- Run `cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2` and verify that both `challenge_slow_peer_first_must_not_drop_fast_peer` and `challenge_scatter_gather_engine_slow_peer_first` PASS!

### 4. [MAJOR - Signed Payload Incompleteness] (`crates/quorum-coordinator/src/crypto.rs`)
- In `canonical_query_bytes`:
  - Update signature to accept `(query_id: Uuid, sender_peer_id: &str, timestamp: i64, access_tier: AccessTier, top_k: usize, embedding: &[f32], query_text: &str)`.
  - Compute BLAKE3 hash of `embedding` vector (as bytes: `blake3::hash(bytemuck::cast_slice(embedding))` or manual byte iteration).
  - Canonical format:
    ```rust
    bytes.extend_from_slice(b"QUORUM_QUERY_V2\n");
    bytes.extend_from_slice(query_id.as_bytes());
    bytes.extend_from_slice(sender_peer_id.as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(&timestamp.to_le_bytes());
    bytes.push(access_tier.as_u8());
    bytes.extend_from_slice(&(top_k as u32).to_le_bytes());
    bytes.extend_from_slice(embedding_hash.as_bytes());
    bytes.extend_from_slice(query_text.as_bytes());
    ```
  - Update `sign_query` and `verify_query` accordingly.
  - Update `coordinator_tests.rs` tests for signing/verification.

### 5. [MINOR - Relevance Node Logic on Empty Retrieval] (`crates/quorum-coordinator/src/agentic/grade.rs`)
- In `GradeRelevanceNode::execute`:
  - If `retrieved_chunks` is empty:
    `let satisfactory = false;` (do NOT treat empty as satisfactory with misleading "0.000 meets threshold 0.600" log).
    Log: `"Empty chunks retrieved. Scheduling query rewrite or fallback."`
  - Ensure agentic loop handles rewrite properly or terminates at `hop_count == state.max_hops`.

### 6. Verification Commands (Run sequentially with `-- --test-threads=2`)
- `cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2` (All 6 tests must pass)
- `cargo test -p quorum-coordinator -- --test-threads=2` (All tests must pass)
- `cargo test -p quorum-network -- --test-threads=2` (All tests must pass)
- `cargo test --manifest-path tests/e2e/Cargo.toml --test test_federated_mesh -- --test-threads=2` (All 10 tests must pass)
- `cargo clippy -p quorum-coordinator -p quorum-network --all-targets -- -D warnings` (0 warnings)

### 7. Git Commit
- Stage and commit changes on `main`:
  `git add -A`
  `git commit -m "fix(coordinator): 1.25x consensus boost, LangExtract core engine, FuturesUnordered scatter-gather, and canonical signing payload"`

## 2026-11-01T00:04:10Z
[Message] sender=e13403c5-b0e6-48a4-a1ec-a1f096f9f53e priority=MESSAGE_PRIORITY_HIGH content=**Context**: Milestone 4 Remediation Status Check
**Content**: Checking in on your progress. I see you have created the `langextract` modules (`alignment.rs`, `consensus.rs`, `extraction.rs`, `mod.rs`). Please provide a brief update on your current step and remember to update your `progress.md` timestamp.
**Action**: Continue remediation, run tests with `-- --test-threads=2`, and report back.
