# Handoff Report — Explorer M8.5 2 (Classifier & Retrieval Isolation Explorer)

**Agent**: Explorer M8.5 2 (`explorer_m8_5_2`)  
**Roles**: `investigator`, `synthesizer`  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_5_2`  
**Parent Conversation ID**: `facfe77d-94d3-44be-95df-eec2b9dfe368`  
**Milestone**: Milestone M8.5 (Source Code & Temp File Ingestion Exclusion & RAG Isolation)  

---

## 1. Observation

### 1.1 Preservation of `FileCategory::Code` in `crates/quorum-classifier`
Direct inspection of `crates/quorum-classifier` confirms that `FileCategory::Code` identification is preserved:
- **Enum Definition** (`crates/quorum-classifier/src/lib.rs:33`):
  ```rust
  pub enum FileCategory {
      Code,
      Text,
      Document,
      Data,
      Archive,
      Executable,
      Image,
      Audio,
      Video,
      Unknown,
  }
  ```
- **Deep & Shebang Inspection** (`crates/quorum-classifier/src/lib.rs:883–930, 1033–1097`):
  - Correctly detects code shebangs (`#!/usr/bin/env python`, `#!/bin/bash`, `#!/usr/bin/node`) and code-specific AST tokens (`fn `, `impl `, `let mut `, `pub struct `, `def `, `class `, `import `), assigning `FileCategory::Code`.
- **Extension Fallback Mapping** (`crates/quorum-classifier/src/lib.rs:1118–1162`):
  - Maps source code extensions to `FileCategory::Code`:
    - `.rs` -> `"rust"`, `FileCategory::Code`
    - `.py`, `.pyi`, `.pyw` -> `"python"`, `FileCategory::Code`
    - `.js`, `.mjs`, `.cjs` -> `"javascript"`, `FileCategory::Code`
    - `.ts`, `.mts`, `.cts` -> `"typescript"`, `FileCategory::Code`
    - `.tsx`, `.jsx` -> `FileCategory::Code`
    - `.c`, `.h`, `.cpp`, `.hpp`, `.cc` -> `FileCategory::Code`
    - `.go`, `.java`, `.kt`, `.cs` -> `FileCategory::Code`
    - `.sh`, `.bash`, `.ps1` -> `FileCategory::Code`
    - `.sql`, `.html`, `.css`, `.bat`, `.cmd` -> `FileCategory::Code`
- **Unit and Integration Test Verification**:
  - `crates/quorum-classifier/src/lib.rs:1361`: `test_rust_code_deep_inspection` asserts `id.category == FileCategory::Code`.
  - `crates/quorum-classifier/src/lib.rs:1374`: `test_python_code_shebang_inspection` asserts `id.category == FileCategory::Code`.
  - `crates/quorum-classifier/tests/adversarial_magika_tests.rs:263`: asserts `id2.category == FileCategory::Code`.
  - `crates/quorum-classifier/tests/corner_cases_multibyte_utf8.rs:126, 150`: asserts `id.category == FileCategory::Code`.
  - `crates/quorum-classifier/tests/sharing_violation_stress_tests.rs:77`: asserts `id_res.unwrap().category == FileCategory::Code`.
- **Direct Test Run & Result**:
  Command executed: `cargo test -p quorum-classifier -- --test-threads=2`
  ```
  running 10 tests in src/lib.rs ... ok. 10 passed; 0 failed
  running 11 tests in tests/adversarial_magika_tests.rs ... ok. 11 passed; 0 failed
  running 3 tests in tests/binary_fast_path_tests.rs ... ok. 3 passed; 0 failed
  running 8 tests in tests/corner_cases_multibyte_utf8.rs ... ok. 8 passed; 0 failed
  running 2 tests in tests/sharing_violation_stress_tests.rs ... ok. 2 passed; 0 failed
  Doc-tests quorum_classifier ... ok. 0 passed; 0 failed
  Total: 34 passed; 0 failed; 0 ignored; finished in 0.58s
  ```

### 1.2 Code and Temp File Filtering in `crates/quorum-coordinator`
Direct inspection of `crates/quorum-coordinator` shows how code and temporary files are filtered out during RAG retrieval and synthesis:

1. **Mesh Retrieval Node (`crates/quorum-coordinator/src/agentic/retrieve.rs`)**:
   - Lines 106–110: Reads daemon status:
     ```rust
     let daemon_active = crate::dlp_mesh::global_mesh_dlp_controller()
         .settings_file_path
         .read()
         .map(|p| p.is_some())
         .unwrap_or(false);
     ```
   - Lines 148–154: Determines if the user explicitly requested code:
     ```rust
     let query_lower = state.current_query.to_ascii_lowercase();
     let requests_code = query_lower.contains("code")
         || query_lower.contains("unit test")
         || query_lower.contains("implement")
         || query_lower.contains("rrfcombiner")
         || query_lower.contains("struct ")
         || query_lower.contains("fn ");
     ```
   - Lines 155–195: When `daemon_active && !requests_code`, filters candidate chunks:
     - Discards temporary files: `file_l.starts_with('~') || file_l.starts_with(".~lock") || file_l == "chat.html" || (file_l.starts_with("conversations-") && file_l.ends_with(".json"))`
     - Discards fenced code blocks / Colab tutorials: `text_l.contains("```python")`, `drive.mount(`, `import numpy as np`, etc.
     - Discards source code extensions: `!EXCLUDED_EXTS.iter().any(|ext| file_l.ends_with(ext))` covering `.rs`, `.py`, `.js`, `.ts`, `.c`, `.cpp`, `.go`, `.java`, `.sh`, `.sql`, `.tmp`, `.bak`, etc.

2. **Offline Synthesis Engine (`crates/quorum-coordinator/src/offline_engine.rs`)**:
   - Lines 136–142: Defines `SOURCE_CODE_EXTENSIONS` (`.rs`, `.py`, `.js`, `.ts`, `.jsx`, `.tsx`, `.c`, `.cpp`, `.h`, `.hpp`, `.go`, `.java`, `.sh`, `.sql`, `.tmp`, `.bak`, `.wbk`).
   - Lines 151–157: Checks `requests_code` with the same criteria.
   - Lines 247–273: Unconditionally filters chunks during synthesis:
     - Rejects temporary files: `file_l.starts_with('~') || file_l.starts_with(".~lock") || file_l == "chat.html" ...`
     - Rejects code extensions when `!requests_code`:
       ```rust
       if !requests_code
           && (SOURCE_CODE_EXTENSIONS
               .iter()
               .any(|ext| file_l.ends_with(ext))
               || path_l.contains("\\harfile\\")
               || path_l.contains("/harfile/")
               || text_l.contains("```python")
               || text_l.contains("```javascript")
               ...
       ```
   - Lines 370–383: If no valid non-code documents remain, returns empty citations:
     ```rust
     if matched_chunks.is_empty() {
         let fallback = format!(
             "No matching documents were found in your local RAG index for **{}**.",
             prompt
         );
         ...
         return (fallback, Vec::new());
     }
     ```

### 1.3 Exact Test Failure in `adversarial_tier5_coordinator_daemon_stress.rs`
- **Location**: `crates/quorum-coordinator/tests/adversarial_tier5_coordinator_daemon_stress.rs:652–736` (`challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops`).
- **Verbatim Fixture Setup**:
  ```rust
  let storage_chunk = DocumentChunk::new(
      "c_agentic_01",
      "doc_agentic",
      "crypto.rs",
      1,
      20,
      0,
      vec![0.5; 768],
      "Ed25519 signature algorithm requires 32-byte public key and 64-byte signature vector.",
      None,
  )
  .expect("chunk creation");
  ```
- **Verbatim Prompt**:
  ```rust
  let prompt = "How are Ed25519 signatures verified in QuorumRAG?";
  ```
- **Failing Assertion** (`line 722–725`):
  ```rust
  assert!(
      !state.citations.is_empty(),
      "State graph must produce grounded citations"
  );
  ```
- **Mechanism of Failure**:
  1. The prompt `"How are Ed25519 signatures verified in QuorumRAG?"` does NOT contain the keyword `"code"`. Thus `requests_code` evaluates to `false`.
  2. The mock chunk file path is `"crypto.rs"`, which ends with `".rs"`.
  3. In `RetrieveMeshNode::execute`, `daemon_active` is `false` (unit test environment without settings file), so `retrieve.rs` does not exclude `"crypto.rs"`, passing it to `state.retrieved_chunks`.
  4. In `SynthesizeWithCitationsNode::execute`, `DeterministicOfflineSynthesisEngine::synthesize_with_history` is called.
  5. `offline_engine.rs` checks `if !requests_code && SOURCE_CODE_EXTENSIONS.iter().any(|ext| file_l.ends_with(ext))` **unconditionally** (without checking `daemon_active`). Because `file_l` is `"crypto.rs"` and `requests_code` is `false`, `"crypto.rs"` is excluded (`continue;`).
  6. As a result, `matched_chunks` is empty, synthesis returns `Vec::new()` citations, and `assert!(!state.citations.is_empty())` panics with an assertion failure.

- **Verbatim Empirical Test Execution Output**:
  Command: `cargo test -p quorum-coordinator --test adversarial_tier5_coordinator_daemon_stress challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops -- --test-threads=1`
  ```
  running 1 test
  test challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops ... FAILED

  failures:

  ---- challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops stdout ----
  [CHALLENGE 6 AGENTIC] Completed at hop 3/3, Citations: 0, Telemetry Events: 29

  thread 'challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops' (46164) panicked at crates\quorum-coordinator\tests\adversarial_tier5_coordinator_daemon_stress.rs:722:5:
  State graph must produce grounded citations
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

  failures:
      challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops

  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.30s
  ```

---

## 2. Logic Chain

1. **Classification Isolation Integrity**:
   - `crates/quorum-classifier` is the source-of-truth file identification engine. Preserving `FileCategory::Code` in `quorum-classifier` ensures that when Magika or extension fallbacks inspect `.rs`, `.py`, etc., they return `FileCategory::Code`. This is verified by all 34 classifier tests passing.

2. **RAG Isolation Policy (Milestone M8.5 Directive)**:
   - User directive (commit `ea953bb` and M8.5 Scope): *"The ingestion pipeline strictly indexes documents, text/data documents, and image assets ... and EXCLUDES source code files ... Also purge/skip any indexed source code chunks during reindexing or retrieval so RAG searches only return user documents and images!"*
   - Therefore, `offline_engine.rs` intentionally and correctly excludes source code files (`.rs`, `.py`, etc.) when users submit general/document queries (where `requests_code` is `false`).

3. **Origin of the Challenge 6 Conflict**:
   - `challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops` was written in an earlier milestone (M6) to test the state machine's hop-count bounded loop (`state.hop_count <= state.max_hops`).
   - The test author wrote mock chunk text describing Ed25519 verification: `"Ed25519 signature algorithm requires 32-byte public key and 64-byte signature vector."` (which is natural language documentation text).
   - However, the author arbitrarily assigned the chunk file path `"crypto.rs"` instead of a document extension such as `"crypto_spec.md"`.
   - The test query `"How are Ed25519 signatures verified in QuorumRAG?"` is a natural language question.
   - When M8.5 implemented strict source code exclusion for non-code queries, `"crypto.rs"` was rightfully excluded during synthesis, leaving 0 citations and causing the legacy assertion `assert!(!state.citations.is_empty())` to fail.

4. **Consistency between `retrieve.rs` and `offline_engine.rs`**:
   - `retrieve.rs` contained `if daemon_active && !requests_code`, whereas `offline_engine.rs` contained `if !requests_code && (SOURCE_CODE_EXTENSIONS...)` unconditionally.
   - This caused an inconsistency where `retrieve.rs` kept the chunk in memory during unit tests, but `offline_engine.rs` dropped it during synthesis.
   - Aligning the mock test chunk to a document extension (`.md`) resolves this cleanly and adheres to M8.5's core invariant.

---

## 3. Caveats

- **Read-Only Scope**: Per Explorer protocol, this report documents observations, root-cause analysis, and exact proposed diffs, but does not modify source files directly.
- **Concurrent Test Lock Contention**: The cargo virtual workspace shares a single build directory (`target/debug/.cargo-lock`). When other tasks/agents run cargo tests (e.g. `quorum-ingest`), compiler file locks block subsequent cargo commands until the preceding compilation completes. Tests must be sequenced cleanly.
- **Other Tests in Coordinator**: All other unit and adversarial tests in `crates/quorum-coordinator` (`coordinator_tests`, `adversarial_challenge_m8_2_tests`, `m8_adversarial_p2p_tests`, `m8_p2p_consensus_latency_tests`, `adversarial_challenge_m4_tests`) pass cleanly because their code-query tests include code keywords (`"code"`, `"rrfcombiner"`), or test internal modules directly (`RrfCombiner`, `ZeroizeOnDrop`, `CacheManager`, `PeerAclManager`).

---

## 4. Conclusion

1. **`crates/quorum-classifier` Status**:
   - **PASS**: 100% compliant. Preserves `FileCategory::Code` for all source code extensions and content-detected code streams. All 34 tests pass with zero errors.

2. **`crates/quorum-coordinator` Code/Temp Exclusion Status**:
   - **PASS**: Successfully excludes `.rs`, `.py`, `.js`, `.ts`, etc., Colab dumps, and Office temporary lock files (`~*`, `.~lock*`, `.tmp`, `.bak`) from non-code RAG queries.

3. **Challenge 6 Test Failure Root Cause & Actionable Fix**:
   - The failure in `challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops` is a **test fixture classification mismatch**, caused by giving a natural language crypto spec chunk a `.rs` source code extension (`"crypto.rs"`).
   - **Proposed Code Snippet (Diff)** for `crates/quorum-coordinator/tests/adversarial_tier5_coordinator_daemon_stress.rs`:
     ```rust
     <<<<<<< BEFORE (Line 657)
             "doc_agentic",
             "crypto.rs",
             1,
     =======
             "doc_agentic",
             "crypto_spec.md",
             1,
     >>>>>>> AFTER
     ```
   - Changing `"crypto.rs"` to `"crypto_spec.md"` correctly classifies the chunk as a Markdown document, allowing it to be cited during natural language document Q&A and making `challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops` pass cleanly.
   - (Alternative): If the test is intended to verify source code synthesis, add `"code"` to the prompt (`"How are Ed25519 signatures verified in QuorumRAG code?"`).

4. **Recommended Coordinator Consistency Polish**:
   - In `crates/quorum-coordinator/src/agentic/retrieve.rs:163–179`:
     Ensure temporary lock files (`~*`, `.~lock*`, `chat.html`, `.tmp`, `.bak`) are filtered out unconditionally, even if `requests_code` is true, since temporary lock files are never valid source code.

---

## 5. Verification Method

To independently verify all findings:

1. **Verify `quorum-classifier` Code Preservation**:
   ```powershell
   cargo test -p quorum-classifier -- --test-threads=2
   ```
   *Expected output*: 34 passed; 0 failed.

2. **Verify Challenge 6 Behavior with Document Extension**:
   Inspect `crates/quorum-coordinator/tests/adversarial_tier5_coordinator_daemon_stress.rs:657`.
   Temporarily update `"crypto.rs"` to `"crypto_spec.md"` and run:
   ```powershell
   cargo test -p quorum-coordinator --test adversarial_tier5_coordinator_daemon_stress challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops -- --test-threads=1
   ```
   *Expected output*: `test challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops ... ok`.

3. **Verify Coordinator Lib & Adversarial Suites**:
   ```powershell
   cargo test -p quorum-coordinator --lib -- --test-threads=2
   cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2
   cargo test -p quorum-coordinator --test m8_adversarial_p2p_tests -- --test-threads=2
   cargo test -p quorum-coordinator --test m8_p2p_consensus_latency_tests -- --test-threads=2
   cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2
   ```
   *Expected output*: All test suites pass 100% with zero failures.
