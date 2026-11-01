# Milestone 4 Adversarial Verification Report: Coordinator, 400ms Timeout & Agentic Loops

**Agent**: `challenger_m4_2`  
**Parent**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`  
**Milestone**: Milestone 4  
**Date**: 2026-10-31  
**Verdict**: **`REQUEST_CHANGES`**

---

## 1. Observation

Direct tool outputs, commands, file paths, line numbers, and verbatim test execution logs:

1. **Baseline Suite Execution**:
   - Command: `cargo test -p quorum-coordinator -- --test-threads=2`
   - Result: Exited code 0.
   - Tests: 12 passed (4 in `agentic_tests.rs`, 8 in `coordinator_tests.rs`).
   ```text
   running 4 tests
   test test_autofusion_router_intent_and_ast_detection ... ok
   test test_hardware_sizing_matrix ... ok
   test test_telemetry_event_serialization_and_sse_formatting ... ok
   test test_agentic_loop_convergence_max_hops ... ok
   test result: ok. 4 passed; 0 failed

   running 8 tests
   test test_acl_effective_tier_resolution ... ok
   test test_crypto_signing_and_verification ... ok
   test test_crypto_stale_timestamp_rejection ... ok
   test test_rrf_combiner_consensus_boost ... ok
   test test_crypto_forged_signature_rejection ... ok
   test test_volatile_memory_scrubbing ... ok
   test test_zero_peer_graceful_local_fallback ... ok
   test test_scatter_gather_400ms_timeout_aborts_slow_peer ... ok
   test result: ok. 8 passed; 0 failed
   ```

2. **Empirical Adversarial Stress Test Execution**:
   - File created: `crates/quorum-coordinator/tests/adversarial_challenge_m4_tests.rs`
   - Command: `cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2`
   - Result: Exited code 1 (4 passed, 2 failed).
   - Verbatim error log:
   ```text
   running 6 tests
   test challenge_agentic_loop_empty_retrieval_terminates ... ok
   test challenge_agentic_loop_terminates_strictly_at_max_hops ... ok
   test challenge_scatter_gather_engine_slow_peer_first ... FAILED
   test challenge_volatile_remote_chunk_implicit_drop_zeroizes ... ok
   test challenge_volatile_remote_chunk_zeroizes_buffer ... ok
   test challenge_slow_peer_first_must_not_drop_fast_peer ... FAILED

   failures:

   ---- challenge_scatter_gather_engine_slow_peer_first stdout ----
   thread 'challenge_scatter_gather_engine_slow_peer_first' (48348) panicked at crates\quorum-coordinator\tests\adversarial_challenge_m4_tests.rs:182:5:
   CRITICAL BUG: Fast peer response was dropped because slow peer was earlier in the tasks list!

   ---- challenge_slow_peer_first_must_not_drop_fast_peer stdout ----
   thread 'challenge_slow_peer_first_must_not_drop_fast_peer' (13392) panicked at crates\quorum-coordinator\tests\adversarial_challenge_m4_tests.rs:102:5:
   CRITICAL BUG: Fast peer response was dropped because slow peer was earlier in the tasks list!
   ```

3. **Vulnerable Code Locations in Codebase**:
   - Location A: `crates/quorum-coordinator/src/scatter_gather.rs` lines 195-203:
     ```rust
     let mut peer_lists: Vec<Vec<CandidateChunkDto>> = Vec::new();
     let _ = tokio::time::timeout(self.timeout_duration, async {
         for task in tasks {
             if let Ok(chunks) = task.await {
                 peer_lists.push(chunks);
             }
         }
     })
     .await;
     ```
   - Location B: `crates/quorum-coordinator/src/mesh.rs` lines 83-91 (`MockMeshDispatcher`):
     ```rust
     let mut responses = Vec::new();
     let _ = tokio::time::timeout(timeout, async {
         for task in tasks {
             if let Ok(resp) = task.await {
                 responses.push(resp);
             }
         }
     })
     .await;
     ```
   - Location C: `crates/quorum-coordinator/src/mesh.rs` lines 140-148 (`SwarmMeshDispatcher`):
     ```rust
     let mut responses = Vec::new();
     let _ = tokio::time::timeout(timeout, async {
         for task in tasks {
             if let Ok(Ok(resp)) = task.await {
                 responses.push(resp);
             }
         }
     })
     .await;
     ```

4. **Existing Baseline Test Masking**:
   - `crates/quorum-coordinator/tests/coordinator_tests.rs` lines 218-247 (`test_scatter_gather_400ms_timeout_aborts_slow_peer`):
     ```rust
     // Fast peer (30ms) added FIRST
     coord.add_peer(MockRemotePeer { peer_id: "peer_fast", latency: Duration::from_millis(30), ... });
     // Slow peer (800ms) added SECOND
     coord.add_peer(MockRemotePeer { peer_id: "peer_slow", latency: Duration::from_millis(800), ... });
     ```
     The baseline test only passed because the fast peer happened to be registered *before* the slow peer in `self.remote_peers`.

5. **Agentic Loop & Volatile Chunk Verification**:
   - `challenge_agentic_loop_terminates_strictly_at_max_hops`: Passed (stopped at exactly `hop_count == 3` under continuous 0.0 scores).
   - `challenge_agentic_loop_empty_retrieval_terminates`: Passed (`hop_count == 0`, proceeded to synthesis without looping).
   - `challenge_volatile_remote_chunk_zeroizes_buffer`: Passed (`Zeroize` zeroed content and pointer buffer).
   - `challenge_volatile_remote_chunk_implicit_drop_zeroizes`: Passed (`ZeroizeOnDrop` zeroed the heap slice upon implicit drop).

---

## 2. Logic Chain

1. **Root Cause of the Scatter-Gather Bug**:
   - `tasks` is constructed as a `Vec<tokio::task::JoinHandle<...>>`.
   - The gathering loop inside `tokio::time::timeout` iterates sequentially over `tasks`:
     `for task in tasks { if let Ok(...) = task.await { ... } }`
   - In Rust, `for task in tasks` evaluates elements in vector index order (`task = tasks[0]`, then `task = tasks[1]`, etc.).
   - When `tasks[0]` belongs to a slow peer (e.g. latency = 800ms) or an unresponsive peer (hangs indefinitely):
     - The async block awaits `tasks[0].await`.
     - In the background, `tasks[1]` (a fast peer with latency = 30ms) completes successfully, and its result is ready in its JoinHandle.
     - However, the gathering loop is suspended awaiting `tasks[0]`.
     - At 400ms, `tokio::time::timeout` expires and cancels the entire `async { ... }` block future.
     - Because the loop was still suspended awaiting `tasks[0]`, it never reached `tasks[1]`.
     - As a direct consequence, `peer_lists` (or `responses`) never receives the chunks from `tasks[1]`, and all subsequent fast peers in `tasks` are silently dropped.

2. **Real-World Impact on P2P Mesh Retrieval**:
   - In a production libp2p swarm, `handle.connected_peers()` returns connected peers in arbitrary order (e.g., PeerId hash order or socket table order).
   - If any single peer in the mesh is experiencing network congestion, packet drop, or a delay >400ms, all fast peers that appear after it in the vector are starved and omitted from RRF rank fusion.
   - This directly violates requirement R3 ("The query coordinator must dispatch parallel local and mesh queries with a 400ms timeout... aggregate candidate chunks via Reciprocal Rank Fusion") and the prompt's mandate ("Empirically verify that slow or unresponsive peers (delaying >400ms) do not hang or block the coordinator, and fast peer responses are merged cleanly").

3. **Remediation via Concurrent Task Stream (`FuturesUnordered`)**:
   - Replacing the sequential `for task in tasks` loop with `futures::stream::FuturesUnordered` (or `tokio::task::JoinSet` with `join_next().await`):
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
     })
     .await;
     ```
   - Under `FuturesUnordered`, futures yield in completion order (out-of-order execution). As soon as `tasks[1]` completes at 30ms, `futures.next()` yields it immediately, collecting the fast peer's chunks. At 400ms, when the timeout fires, all completed peers are safely retained regardless of their original list position.

---

## 3. Caveats

1. **Hardware Telemetry on Systems without Dedicated GPU**:
   - `HardwareTelemetry::probe()` defaults `free_vram_mb` to 0 on machines where `nvidia-smi` is not installed, gracefully falling back to free RAM sizing tiers (`qwen2.5:32b`, `qwen2.5:14b`, etc.).
2. **Review-Only Constraint**:
   - In accordance with review constraints, Challenger 2 did not modify production code in `src/`. The bug was empirically demonstrated via the integration test in `crates/quorum-coordinator/tests/adversarial_challenge_m4_tests.rs`.

---

## 4. Conclusion

**Verdict**: **`REQUEST_CHANGES`**

While the agentic state graph loop bounds (terminating strictly at max 3 hops) and volatile remote chunk zeroization on drop pass empirical verification, the 400ms scatter-gather timeout mechanism contains a critical flaw:
- `FederatedQueryCoordinator::execute_query` (`scatter_gather.rs:196`), `MockMeshDispatcher::query_peers` (`mesh.rs:84`), and `SwarmMeshDispatcher::query_peers` (`mesh.rs:141`) sequentially await JoinHandles.
- When an earlier peer is slow (>400ms) or hangs, all subsequent fast peers are starved and dropped upon timeout.

### Required Changes for Remediating Agent:
1. In `crates/quorum-coordinator/src/scatter_gather.rs` lines 195-203, replace the sequential `for task in tasks { task.await }` loop with `futures::stream::FuturesUnordered` or `tokio::task::JoinSet::join_next()`.
2. In `crates/quorum-coordinator/src/mesh.rs` lines 83-91 (`MockMeshDispatcher`) and lines 140-148 (`SwarmMeshDispatcher`), replace the sequential `for task in tasks` loops with `FuturesUnordered`.
3. Verify that all 6 tests in `crates/quorum-coordinator/tests/adversarial_challenge_m4_tests.rs` pass cleanly:
   `cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2`
4. Verify that all 12 baseline unit/integration tests in `crates/quorum-coordinator` pass cleanly.

---

## 5. Verification Method

To independently reproduce this finding and verify the remediation:

```powershell
# 1. Run the empirical adversarial challenge suite reproducing the failure:
cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2

# Current Output:
# test challenge_scatter_gather_engine_slow_peer_first ... FAILED
# test challenge_slow_peer_first_must_not_drop_fast_peer ... FAILED
# (2 failed; 4 passed)

# 2. Invalidation Condition (Remediation Verification):
# After converting the sequential await loop to FuturesUnordered / JoinSet in:
# - crates/quorum-coordinator/src/scatter_gather.rs:196
# - crates/quorum-coordinator/src/mesh.rs:84
# - crates/quorum-coordinator/src/mesh.rs:141
# Re-run:
cargo test -p quorum-coordinator --test adversarial_challenge_m4_tests -- --test-threads=2
# Invalidation: All 6 tests in adversarial_challenge_m4_tests pass with 0 failures.

# 3. Full coordinator test suite regression check:
cargo test -p quorum-coordinator -- --test-threads=2
# Invalidation: All 18 tests (12 original + 6 adversarial) pass with 0 failures.
```
