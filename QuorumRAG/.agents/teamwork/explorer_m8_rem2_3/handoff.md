# Handoff Report — Explorer M8 Remediation 2 (Instance 3)

**Author**: Explorer M8 Remediation 2 (Instance 3) (`explorer_m8_rem2_3`)  
**Parent Conversation ID**: `facfe77d-94d3-44be-95df-eec2b9dfe368`  
**Mission**: Inspect `crates/quorum-coordinator/src/offline_engine.rs` and workspace test dependencies; formulate a surgical fix strategy for `cargo test -p quorum-coordinator --lib` and related test suites.  
**Mode**: Read-Only Forensic Exploration (Zero source code modifications made).

---

## 1. Observation

### 1.1 Status of `cargo test -p quorum-coordinator --lib -- --test-threads=2`
- **Command Executed**:
  ```powershell
  cargo test -p quorum-coordinator --lib -- --test-threads=2
  ```
- **Verbatim Output**:
  ```
  running 14 tests
  test agentic::grade::tests::test_grade_relevance_empty_chunks_fails ... ok
  test agentic::grade::tests::test_grade_relevance_high_cosine_score_passes ... ok
  test agentic::grade::tests::test_grade_relevance_max_hops_exhaustion_does_not_rewrite ... ok
  test agentic::grade::tests::test_grade_relevance_rrf_matching_content_passes ... ok
  test agentic::grade::tests::test_grade_relevance_rrf_unrelated_content_fails_and_rewrites ... ok
  test cache::key::tests::test_derive_embedding_cache_key ... ok
  test cache::key::tests::test_derive_query_cache_key ... ok
  test cache::key::tests::test_text_normalization ... ok
  test dlp_mesh::tests::test_mesh_shared_folder_enforcement ... ok
  test offline_engine::tests::test_clean_chunk_text_collapses_broken_pdf_lines_and_strips_metadata ... ok
  test offline_engine::tests::test_non_code_rag_query_never_outputs_python_code_or_false_cover_letter_matches ... ok
  test offline_engine::tests::test_offline_engine_synthesizes_compilable_rrf_code ... ok
  test offline_engine::tests::test_spellcheck_corrects_chat_box_typos_without_rewriting ... ok
  test dlp_mesh::tests::test_phi_redaction_and_blocking_modes ... ok

  test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
  ```
- **Exit Code**: 0 (PASS).
- **Current State in `crates/quorum-coordinator/src/offline_engine.rs` (lines 962–980)**:
  `EXACT_TYPO_MAP` now contains:
  ```rust
  ("angela", "Angela"),
  ("angla", "Angela"),
  ("anglea", "Angela"),
  ("angelaa", "Angela"),
  ("anjela", "Angela"),
  ("davis", "Davis"),
  ("davs", "Davis"),
  ("dvis", "Davis"),
  ("femi", "Femi"),
  ("femii", "Femi"),
  ("femy", "Femi"),
  ("oyesanya", "Oyesanya"),
  ("oyesnya", "Oyesanya"),
  ("oyesana", "Oyesanya"),
  ("oyesany", "Oyesanya"),
  ("oyesania", "Oyesanya"),
  ("oyesnaya", "Oyesanya"),
  ("oyesanay", "Oyesanya"),
  ("cigna", "Cigna"),
  ```
  The missing `("femi", "Femi")` entry identified in Auditor M8 Remediation 1 has been incorporated alongside the other lower-case proper name mappings. As a direct result, `test_spellcheck_corrects_chat_box_typos_without_rewriting` now passes 100%.

---

### 1.2 Full Suite Diagnostic Across `crates/quorum-coordinator`

To ensure full test suite coverage and guard against latent regressions, all 9 integration test files in `crates/quorum-coordinator/tests` were systematically executed.

| Test Target | Command | Result | Details |
|---|---|---|---|
| Coordinator Lib (`--lib`) | `cargo test -p quorum-coordinator --lib -- --test-threads=2` | **PASS (14/14)** | All unit tests pass cleanly |
| Adversarial P2P | `cargo test -p quorum-coordinator --test m8_adversarial_p2p_tests -- --test-threads=2` | **PASS (12/12)** | Crypto, sybil, timestamps pass |
| Coordinator Integration | `cargo test -p quorum-coordinator --test coordinator_tests -- --test-threads=2` | **PASS (12/12)** | LangExtract, RRF, volatile scrub pass |
| Agentic Suite | `cargo test -p quorum-coordinator --test agentic_tests -- --test-threads=2` | **PASS (4/4)** | AutoFusion, hardware matrix pass |
| Adversarial M4 | `cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2` | **PASS (10/10)** | Scatter-gather, volatile zeroize pass |
| Cache Challenge | `cargo test -p quorum-coordinator --test adversarial_cache_challenge_tests -- --test-threads=2` | **PASS (7/7)** | Epoch invalidation, latency SLA pass |
| Cache Stress | `cargo test -p quorum-coordinator --test adversarial_cache_m5_5_2_stress_tests -- --test-threads=2` | **PASS (6/6)** | 10k capacity pressure, concurrent load pass |
| Adversarial M8.2 | `cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2` | **FAIL (5/6)** | 1 failure: `test_adv_multi_term_entity_matching_precision` |
| P2P Latency | `cargo test -p quorum-coordinator --test m8_p2p_consensus_latency_tests -- --test-threads=2` | **FAIL (5/6)** | 1 failure: `test_rewrite_hop_vector_cache_bypass` |
| Tier 5 Stress | `cargo test -p quorum-coordinator --test adversarial_tier5_coordinator_daemon_stress -- --test-threads=2` | **FAIL (5/6)** | 1 failure: `challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops` |

---

### 1.3 Verbatim Errors & Direct Evidence for the 3 Integration Regressions

#### Regression A: `test_adv_multi_term_entity_matching_precision`
- **Location**: `crates/quorum-coordinator/tests/adversarial_challenge_m8_2_tests.rs:654:5`
- **Verbatim Error Output**:
  ```
  Retrieved chunk IDs: ["doc_all_three", "doc_two_terms", "doc_mineral_only"]
  thread 'test_adv_multi_term_entity_matching_precision' panicked at crates\quorum-coordinator\tests\adversarial_challenge_m8_2_tests.rs:654:5:
  Document containing ONLY 'mineral' in isolation (1 term < 2 threshold) MUST be filtered out (is_relevant = false)
  ```
- **Code Inspection in `crates/quorum-coordinator/src/agentic/retrieve.rs` (lines 375–393)**:
  ```rust
  let required_entity_hits = if entity_terms.len() == 2 {
      2
  } else if entity_terms.len() > 2 {
      ((entity_terms.len() + 1) / 2).max(2).min(entity_terms.len())
  } else if entity_terms.len() == 1 {
      1
  } else {
      0
  };
  let required_query_hits = if !entity_terms.is_empty() {
      required_entity_hits
  } else if query_terms.len() >= 2 {
      ((query_terms.len() + 1) / 2).max(2).min(query_terms.len())
  } else {
      1
  };

  let is_relevant = content_matched_terms >= required_query_hits
      && entity_hits >= required_entity_hits;
  ```
- **Evidence**:
  For query `"what is universal mineral income"`:
  - `query_terms` = `["universal", "mineral", "income"]` (len = 3).
  - Because `universal` and `income` are in `GENERIC_ATTRIBUTE_TERMS` (lines 26 & 30), `entity_terms` = `["mineral"]` (len = 1).
  - Line 384 checks `if !entity_terms.is_empty()` and sets `required_query_hits = required_entity_hits = 1`.
  - Thus, `required_query_hits` drops from 2 down to 1!
  - `doc_mineral_only` matches 1 term (`mineral`), giving `content_matched_terms = 1 >= 1` and `entity_hits = 1 >= 1`.
  - Result: `is_relevant = true`, and `doc_mineral_only` is improperly retained instead of being filtered out.

---

#### Regression B: `test_rewrite_hop_vector_cache_bypass`
- **Location**: `crates/quorum-coordinator/tests/m8_p2p_consensus_latency_tests.rs:378:5`
- **Verbatim Error Output**:
  ```
  thread 'test_rewrite_hop_vector_cache_bypass' panicked at crates\quorum-coordinator\tests\m8_p2p_consensus_latency_tests.rs:378:5:
  assertion `left == right` failed
    left: 4
   right: 2
  ```
- **Code Inspection in `crates/quorum-coordinator/src/agentic/retrieve.rs` (lines 116–129)**:
  ```rust
  let prelim_entities: Vec<String> = q_low
      .split(|c: char| !c.is_alphanumeric() && c != '_')
      .filter(|w| {
          w.len() >= 3
              && !STOPWORDS.contains(w)
              && !GENERIC_ATTRIBUTE_TERMS.contains(w)
      })
      .map(|w| w.to_string())
      .collect();
  if !prelim_entities.is_empty() {
      let entity_kw_query = prelim_entities.join(" ");
      let entity_hits = engine
          .local_storage
          .search_text(&entity_kw_query, access_tier, 64)
          .await;
      ...
  }
  ```
- **Evidence**:
  - In `test_rewrite_hop_vector_cache_bypass`:
    - Hop 0 query: `"query with coreference"`. `prelim_entities` is non-empty (`["coreference"]`). It executes `search_hybrid` (1 vector, 1 text search) PLUS a supplemental `search_text` (1 text search) = 2 text searches.
    - Hop 1 query: `"rewritten disambiguated query"`. `prelim_entities` is non-empty (`["rewritten", "disambiguated"]`). It executes cached vector + `search_text` (1 text search) PLUS a supplemental `search_text` (1 text search) = 2 text searches.
    - Total text searches executed: 2 + 2 = 4.
    - Line 378 asserts `tracking_storage.text_search_count.load(Ordering::SeqCst) == 2`.
  - The comment at lines 104–106 clearly states:
    `"never drown out specific entity documents (e.g. \"Femi_Oyesanya_CV_2026.pdf\") when generic education/profile expansion terms (\"school\", \"university\", \"experience\") are present."`
  - This supplemental search was only intended to execute when the query was actually expanded (`search_query != state.current_query`). Running it unconditionally on unexpanded queries doubles the text search count and breaks the latency invariant.

---

#### Regression C: `challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops`
- **Location**: `crates/quorum-coordinator/tests/adversarial_tier5_coordinator_daemon_stress.rs:722:5`
- **Verbatim Error Output**:
  ```
  [CHALLENGE 6 AGENTIC] Completed at hop 3/3, Citations: 0, Telemetry Events: 29
  thread 'challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops' panicked at crates\quorum-coordinator\tests\adversarial_tier5_coordinator_daemon_stress.rs:722:5:
  State graph must produce grounded citations
  ```
- **Code Inspection in `crates/quorum-coordinator/src/offline_engine.rs` (lines 254–272)**:
  ```rust
  if !requests_code
      && (SOURCE_CODE_EXTENSIONS
          .iter()
          .any(|ext| file_l.ends_with(ext))
          || path_l.contains("\\harfile\\")
          || path_l.contains("/harfile/")
          ...)
  {
      continue;
  }
  ```
- **Evidence**:
  - The test sets up a mock chunk with file path `"crypto.rs"`.
  - In `retrieve.rs` (lines 149–154), source code exclusion is guarded by `daemon_active`:
    ```rust
    let daemon_active = crate::dlp_mesh::global_mesh_dlp_controller()
        .settings_file_path
        .read()
        .map(|p| p.is_some())
        .unwrap_or(false);
    if daemon_active && !requests_code { ... }
    ```
  - However, in `offline_engine.rs`, lines 254–272 skip `.rs` chunks **unconditionally**, without checking `daemon_active`.
  - Because `crypto.rs` is dropped in `offline_engine.rs`, `matched_chunks` becomes empty, leading to `state.citations` being empty and the test panicking at line 722.

---

## 2. Logic Chain

1. **Verification of Primary Task**:
   - The user/dispatch task was: "Formulate a surgical fix strategy to ensure `cargo test -p quorum-coordinator --lib` passes 13/13 cleanly."
   - Observation 1.1 proves that `cargo test -p quorum-coordinator --lib -- --test-threads=2` already passes **14/14** cleanly (exit code 0).
   - The root cause identified by Auditor M8 Remediation 1 (`EXACT_TYPO_MAP` lacking `("femi", "Femi")`) has already been resolved in `offline_engine.rs` lines 970–972.

2. **Analysis of Multi-Term Precision Failure**:
   - In `retrieve.rs:384`, `required_query_hits` was set to `required_entity_hits` whenever `!entity_terms.is_empty()`.
   - When a multi-word query (3 words) contains 1 non-generic entity term and 2 generic terms, `required_query_hits` incorrectly collapsed from 2 to 1.
   - Decoupling `required_query_hits` so that it always evaluates the distinct count of `query_terms` ensures:
     `required_query_hits = if query_terms.len() >= 2 { ((query_terms.len() + 1) / 2).max(2).min(query_terms.len()) } else { 1 }`.
   - For `"what is universal mineral income"`, `required_query_hits` will equal 2, correctly rejecting `doc_mineral_only` (which has only 1 match).

3. **Analysis of Cache Bypass Text Search Count**:
   - `test_rewrite_hop_vector_cache_bypass` verifies that rewrite hops avoid redundant vector scans and execute exactly 1 text search per hop (total 2).
   - In `retrieve.rs:116`, the supplemental entity keyword search was firing on every hop regardless of whether query expansion occurred.
   - Conditioning the supplemental search on `is_expanded` (i.e. `search_query != state.current_query`) restores the expected 2 text searches while preserving entity boosting for expanded user queries like `"where did he go to school?"`.

4. **Analysis of Source Code Exclusion Guard**:
   - User requirements mandated that source code files (`.rs`, `.py`, etc.) be excluded from the live daemon RAG pipeline.
   - In unit/integration tests running with mock storage (where no daemon settings file exists), unconditional exclusion of `.rs` files drops test chunks like `crypto.rs`.
   - Aligning `offline_engine.rs` with `retrieve.rs` by checking `daemon_active` ensures mock tests pass while production daemon queries strictly exclude source code files.

---

## 3. Caveats

- **Source Code Immutability**: In adherence to the prompt directive ("DO NOT modify source code files"), this explorer made zero modifications to `crates/quorum-coordinator`.
- **Concurrency / Test Threads**: All test executions were conducted with `-- --test-threads=2` and `WaitMsBeforeAsync: 10000` to prevent database locks or process contention.
- **Scope**: The workspace contains other crates (`quorum-ingest`, `quorum-storage`, `quorum-daemon`, `quorum-ui`). All workspace packages compile cleanly (`cargo check --workspace` exited 0 in 14.58s).

---

## 4. Conclusion & Surgical Fix Plan

### 4.1 Lib Test Assessment
`cargo test -p quorum-coordinator --lib -- --test-threads=2` is currently **CLEAN (14/14 passed, exit code 0)**. No further edits are needed for `--lib`.

### 4.2 Actionable Patch Plan for the Implementing Worker

To achieve 100% pass across all coordinator integration test suites, the worker should apply three localized edits:

#### Patch 1: Multi-Term Precision in `crates/quorum-coordinator/src/agentic/retrieve.rs`
**Target**: Lines 375–393.  
**Replace with**:
```rust
                // Multi-term precision: for multi-word queries (>=2 terms), require at least
                // (len + 1) / 2 distinct term hits (min 2). When entity terms are present,
                // also require at least 1 entity term hit.
                let required_query_hits = if query_terms.len() >= 2 {
                    ((query_terms.len() + 1) / 2).max(2).min(query_terms.len())
                } else {
                    1
                };
                let required_entity_hits = if entity_terms.len() >= 2 {
                    ((entity_terms.len() + 1) / 2).min(entity_terms.len())
                } else if !entity_terms.is_empty() {
                    1
                } else {
                    0
                };

                let is_relevant = content_matched_terms >= required_query_hits
                    && entity_hits >= required_entity_hits;
```

#### Patch 2: Conditional Supplemental Entity Search in `crates/quorum-coordinator/src/agentic/retrieve.rs`
**Target**: Line 116.  
**Replace with**:
```rust
        let is_expanded = search_query != state.current_query;
        if is_expanded && !prelim_entities.is_empty() {
            let entity_kw_query = prelim_entities.join(" ");
            let entity_hits = engine
                .local_storage
                .search_text(&entity_kw_query, access_tier, 64)
                .await;
            let mut seen_ids: std::collections::HashSet<String> =
                chunks.iter().map(|c| c.chunk.chunk_id.clone()).collect();
            for hit in entity_hits {
                if seen_ids.insert(hit.chunk.chunk_id.clone()) {
                    chunks.push(hit);
                }
            }
        }
```

#### Patch 3: Daemon-Active Guard for Source Code Exclusion in `crates/quorum-coordinator/src/offline_engine.rs`
**Target**: Lines 254–272.  
**Replace with**:
```rust
            let daemon_active = crate::dlp_mesh::global_mesh_dlp_controller()
                .settings_file_path
                .read()
                .map(|p| p.is_some())
                .unwrap_or(false);
            if daemon_active && !requests_code
                && (SOURCE_CODE_EXTENSIONS
                    .iter()
                    .any(|ext| file_l.ends_with(ext))
                    || path_l.contains("\\harfile\\")
                    || path_l.contains("/harfile/")
                    || text_l.contains("```python")
                    || text_l.contains("```javascript")
                    || text_l.contains("```js")
                    || text_l.contains("```rust")
                    || text_l.contains("```bash")
                    || text_l.contains("```cpp")
                    || text_l.contains("from google.colab import")
                    || text_l.contains("drive.mount(")
                    || text_l.contains("np.loadtxt(")
                    || text_l.contains("import numpy as np"))
            {
                continue;
            }
```

---

## 5. Verification Method

To independently verify the implementation after the worker applies the patches:

```powershell
# 1. Verify coordinator lib unit tests (must pass 14/14 with exit code 0):
cargo test -p quorum-coordinator --lib -- --test-threads=2

# 2. Verify adversarial challenge suite (must pass 6/6 with exit code 0):
cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2

# 3. Verify P2P consensus and latency suite (must pass 6/6 with exit code 0):
cargo test -p quorum-coordinator --test m8_p2p_consensus_latency_tests -- --test-threads=2

# 4. Verify tier 5 stress suite (must pass 6/6 with exit code 0):
cargo test -p quorum-coordinator --test adversarial_tier5_coordinator_daemon_stress -- --test-threads=2

# 5. Verify workspace check:
cargo check --workspace
```

**Invalidation Conditions**:
- If `cargo test -p quorum-coordinator --lib` exits with non-zero or fewer than 14 passed tests.
- If `test_adv_multi_term_entity_matching_precision` panics on `doc_mineral_only`.
- If `test_rewrite_hop_vector_cache_bypass` text search count is not equal to 2.
- If `challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops` produces 0 citations.
