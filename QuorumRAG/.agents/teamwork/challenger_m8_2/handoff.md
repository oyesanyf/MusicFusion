# Adversarial Challenge Handoff Report — Milestone M8 (challenger_m8_2)

**Final Verdict**: **REJECT**

## 1. Observation
1. **Byzantine Quorum Consensus Hardening (`crates/quorum-coordinator/src/fusion.rs`)**:
   - `RrfCombiner::fuse_rankings` (lines 47-80):
     ```rust
     for (remote_idx, list) in remote_peer_lists.into_iter().enumerate() {
         let remote_peer_id = remote_idx + 1;
         let mut seen_peer_hashes: HashSet<String> = HashSet::new();

         for (rank_0, candidate) in list.into_iter().enumerate() {
             let content_hash = DocumentChunk::compute_blake3_hash(&candidate.content);

             // Byzantine consensus hardening: a remote peer contributes at most ONCE per unique chunk content
             if !seen_peer_hashes.insert(content_hash.clone()) {
                 continue;
             }
     ```
   - Empirically verified in `adversarial_challenge_m8_2_tests.rs`:
     - Test `test_adv_byzantine_sybil_score_stuffing_and_honest_peer_superiority`: Rogue peer flooded 100 duplicate chunks at ranks 0..99 under distinct `chunk_id`s. Output was deduplicated to exactly 1 entry with score $1 / (60 + 1) \approx 0.01639344$. An honest chunk supported by 2 peers received score $((1/61) + (1/61)) \times 1.25 \approx 0.0409836$ and outranked the rogue submission at rank #0 (`PASSED`).
     - Test `test_adv_aggregation_key_blake3_hash_anti_malleability`: Chunks sharing the identical `chunk_id` but with different content did not collide or aggregate (`PASSED`). Different `chunk_id`s with identical content correctly aggregated into one entry and received the $1.25\times$ boost (`PASSED`).
     - Test `test_adv_independent_honest_peers_accumulate_consensus_boost`: 3 independent peers accumulated $(3 / 61) \times 1.25 \approx 0.0614754$ (`PASSED`).

2. **Remote Chunk Isolation & Access Tier Clamping (`crates/quorum-coordinator/src/scatter_gather.rs`)**:
   - Lines 264-275 in `execute_query_with_cached_vector`:
     ```rust
     if chunk.line_start > chunk.line_end {
         tracing::warn!(...);
         continue;
     }
     chunk.access_tier = chunk.access_tier.min(access_tier);
     ```
   - Empirically verified in `adversarial_challenge_m8_2_tests.rs`:
     - Test `test_adv_remote_chunk_access_tier_clamping_to_requested`: Remote chunk with `access_tier = 5` requested under `access_tier = 1` was clamped to `1` in both `VolatileRemoteChunk` and `ScoredChunk` (`PASSED`).
     - Test `test_adv_remote_chunk_inverted_lines_rejected`: Remote chunk with `line_start = 50, line_end = 10` was rejected (`PASSED`).

3. **Multi-Term Entity Matching Precision Failure (`crates/quorum-coordinator/src/agentic/retrieve.rs`)**:
   - In `crates/quorum-coordinator/src/agentic/retrieve.rs` lines 28-32:
     ```rust
     pub const GENERIC_ATTRIBUTE_TERMS: &[&str] = &[
         ...
         "income", "tax", "taxes", "statement", "statements", ...
         "life", "time", "world", "reading", "book", "books", "universal", "system", "systems",
     ];
     ```
   - In `RetrieveMeshNode::execute` (lines 83-99 and 243-256):
     ```rust
     let entity_terms: Vec<String> = query_terms
         .iter()
         .filter(|t| !GENERIC_ATTRIBUTE_TERMS.contains(&t.as_str()))
         .cloned()
         .collect();
     ...
     let required_entity_hits = if entity_terms.len() >= 2 {
         ((entity_terms.len() + 1) / 2).max(2).min(entity_terms.len())
     } else {
         1
     };

     let is_relevant = if !entity_terms.is_empty() {
         entity_hits >= required_entity_hits && entity_hits > 0
     } else if query_terms.len() >= 2 {
         content_matched_terms >= 2
     } else {
         content_matched_terms > 0
     };
     ```
   - Empirical test execution command:
     `cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2`
   - Test output and verbatim panic:
     ```
     ---- test_adv_multi_term_entity_matching_precision stdout ----
     Retrieved chunk IDs: ["doc_all_three", "doc_two_terms", "doc_mineral_only"]

     thread 'test_adv_multi_term_entity_matching_precision' (49732) panicked at crates\quorum-coordinator\tests\adversarial_challenge_m8_2_tests.rs:654:5:
     Document containing ONLY 'mineral' in isolation (1 term < 2 threshold) MUST be filtered out (is_relevant = false)
     ```

## 2. Logic Chain
1. For the query `"what is universal mineral income"`:
   - Non-stopword `query_terms` are `["universal", "mineral", "income"]` (length 3).
   - In `retrieve.rs`, `"universal"` and `"income"` are in `GENERIC_ATTRIBUTE_TERMS`.
   - Therefore, `entity_terms` is reduced to `["mineral"]` (length 1).
2. Inside `precompute_criteria` in `retrieve.rs`:
   - `entity_terms.len() >= 2` evaluates to false, setting `required_entity_hits = 1`.
   - Because `!entity_terms.is_empty()` is true, the code evaluates:
     `is_relevant = entity_hits >= required_entity_hits && entity_hits > 0`
   - For a document containing ONLY `"mineral"` in isolation (e.g. geological survey text with 0 hits for "universal" and 0 hits for "income"), `entity_hits = 1`.
   - Since `1 >= 1`, `is_relevant` evaluates to `true`.
3. The multi-term quorum requirement specified in Milestone M8 (`(len + 1) / 2` distinct terms, requiring $\ge 2$ distinct terms for a 3-word query) is completely bypassed when generic terms reduce `entity_terms.len()` to 1.
4. As a result, `doc_mineral_only` is marked `is_relevant = true` and returned in the candidate set, causing single-word accidental matches on multi-word queries to pollute retrieved context.
5. In contrast, `crates/quorum-coordinator/src/offline_engine.rs` does not include `"universal"` in `GENERIC_ATTRIBUTE_TERMS`, preserving `entity_terms` as `["universal", "mineral"]` (length 2) where `doc_mineral_only` correctly yields `is_relevant = false`.

## 3. Caveats
- `fusion.rs` (Byzantine Sybil deduplication, BLAKE3 content keying, and multi-peer score boosting) is solid and robust.
- `scatter_gather.rs` (access tier clamping to `requested_tier` and inverted line range rejection) is solid and robust.
- The failure is isolated strictly to the relevance gating logic in `crates/quorum-coordinator/src/agentic/retrieve.rs`.

## 4. Conclusion
Milestone M8 cannot be approved in its current state.
**Verdict: REJECT.**

### Required Remediation
In `crates/quorum-coordinator/src/agentic/retrieve.rs`:
1. In `precompute_criteria` (around lines 243-256), enforce that for any multi-word query (`query_terms.len() >= 2`), the total distinct matched query terms (`content_matched_terms`) must satisfy the quorum threshold:
   ```rust
   let required_content_hits = if query_terms.len() >= 2 {
       ((query_terms.len() + 1) / 2).max(2).min(query_terms.len())
   } else {
       1
   };

   let is_relevant = if !entity_terms.is_empty() {
       content_matched_terms >= required_content_hits && entity_hits > 0
   } else {
       content_matched_terms >= required_content_hits
   };
   ```
2. Re-run `cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2` to confirm all 6 adversarial tests pass cleanly.

## 5. Verification Method
- Execute:
  `cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2`
- Files to inspect:
  - `crates/quorum-coordinator/tests/adversarial_challenge_m8_2_tests.rs` (lines 530-660)
  - `crates/quorum-coordinator/src/agentic/retrieve.rs` (lines 243-260)
- Invalidation condition:
  When `crates/quorum-coordinator/src/agentic/retrieve.rs` is corrected as outlined, `test_adv_multi_term_entity_matching_precision` will pass with 6/6 tests green.
