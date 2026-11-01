## 2026-10-31T22:25:36Z
You are worker_m4_1, the implementation worker for Milestone 4 (rust-libp2p Swarm, Coordinator, Fusion & GLM Selection).
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\worker_m4_1\
Your parent conversation ID is: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

MANDATORY INSTRUCTIONS:
1. You MUST read:
   - `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`
   - `d:\harfile\QuorumRAG\PROJECT.md`
   - `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m4_1\handoff.md` (Swarm & Transports blueprint)
   - `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m4_2\handoff.md` (Coordinator, RRF & Volatile Memory blueprint)
   - `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m4_3\handoff.md` (State Graph, Model Router & Telemetry blueprint)
   - `d:\harfile\QuorumRAG\tests\e2e\test_federated_mesh.rs` (Opaque-box E2E test scenarios to satisfy)
2. Initialize BRIEFING.md and progress.md in your working directory.
3. Exclusively owned files for Milestone 4:
   - Root `Cargo.toml` (add `crates/quorum-network` and `crates/quorum-coordinator` to workspace `members`)
   - `crates/quorum-network/**`
   - `crates/quorum-coordinator/**`

4. Implementation Scope:
   A. **`crates/quorum-network`**:
      - `Cargo.toml`: `libp2p 0.57.1` with exact features (`tokio`, `quic`, `tcp`, `noise`, `yamux`, `mdns`, `kad`, `gossipsub`, `request-response`, `cbor`, `identify`, `macros`, `ed25519`, `dns`).
      - `config.rs`, `error.rs`, `behaviour.rs`, `transport.rs`, `handshake.rs`, `handle.rs`, `actor.rs`, `lib.rs`.
      - Dual transports: QUIC (`libp2p-quic`) and TCP + Noise (`libp2p-noise`) + Yamux (`libp2p-yamux`) with port 0 dynamic binding.
      - Composite `QuorumNetworkBehaviour` combining mDNS, Kademlia DHT, GossipSub, Request-Response CBOR (`libp2p-request-response` 0.30.0 with `cbor4ii`), and Identify.
      - Multi-tier Gemma contract verification: Identify protocol metadata (`/quorum-rag/1.0.0/contract-<hash_hex>`), constant-time check (`subtle::ConstantTimeEq`), immediately disconnecting mismatched peers.
      - Actor pattern: `NetworkSwarmActor` + `NetworkSwarmHandle` with `oneshot` query reply channels and `tokio::sync::broadcast` for network events.
      - Integration test in `crates/quorum-network/tests/network_tests.rs`.

   B. **`crates/quorum-coordinator`**:
      - `Cargo.toml`: Dependencies on `quorum-types`, `quorum-contract`, `quorum-storage`, `tokio`, `futures`, `ed25519-dalek`, `blake3`, `subtle`, `zeroize`, `serde`, `serde_json`, `uuid`, `chrono`, `thiserror`, `tracing`, `sysinfo`.
      - `error.rs`, `config.rs`, `types.rs`, `crypto.rs`, `acl.rs`, `fusion.rs`, `volatile.rs`, `mesh.rs`, `scatter_gather.rs`, `coordinator.rs`, `hardware.rs`, `model_router.rs`, `offline_engine.rs`, `telemetry.rs`, `agentic/` (`mod.rs`, `classify.rs`, `retrieve.rs`, `grade.rs`, `rewrite.rs`, `synthesize.rs`), `lib.rs`.
      - `MeshQueryDispatcher` trait: cleanly bridges `MockMeshDispatcher` (for E2E/unit testing without physical networking) and `NetworkSwarmHandle` (for production mesh).
      - 400ms Scatter-Gather Query Coordinator (`ScatterGatherEngine` & `FederatedQueryCoordinator`):
        - Dispatches local LanceDB/Tantivy search and remote mesh queries in parallel.
        - Enforces 400ms hard timeout guard with graceful fallback to local results when peers lag or time out.
        - Ed25519 signing and timestamp anti-replay verification (< 60s tolerance).
        - Access-tier SQL pre-filtering in LanceDB before vector scan.
        - Reciprocal Rank Fusion ($k=60.0$) with BLAKE3 deduplication and deterministic tie-breaking on `(chunk_id, blake3_hash)`.
        - Volatile Memory Protection: remote chunks held strictly in `VolatileRemoteChunk` (`ZeroizeOnDrop`) memory structs and scrubbed on drop.
      - LangGraph-style Agentic State Graph:
        - `ClassifyIntent -> RetrieveMesh -> GradeRelevance -> RewriteQuery (bounded loop max 3 hops) -> SynthesizeWithCitations`.
        - Mathematical termination proof ($h_{n+1} = h_n + 1$, capped at 3 hops, anti-cycle detection).
        - Live SSE telemetry event schemas (`AgenticTelemetryEvent`).
      - ModelFusion Hardware-Aware GLM Selection:
        - `SynthesisModelFamily::{Glm4Agentic, Qwen25Code, GemmaProse, AutoFusion}`.
        - `Glm4Agentic` as default reasoning engine (`glm-4-9b-chat` / `glm4:9b`, 131K context).
        - `Qwen25Code` sized strictly by runtime free RAM/VRAM (32b, 14b, 7b, 3b, 1.5b), never appending `-coder`.
        - `GemmaProse` for natural English prose.
        - `AutoFusion` adaptive router analyzing intent, code AST ratio, and token volume.
        - Hardware memory probe via `sysinfo` + `nvidia-smi` (with `CREATE_NO_WINDOW 0x08000000`).
        - Deterministic offline synthesis engine fallback when Ollama is offline.
      - Unit and integration tests in `crates/quorum-coordinator/tests/coordinator_tests.rs` and `tests/agentic_tests.rs`.

   C. **Integration & Arrow Encapsulation Invariants**:
      - Zero Apache Arrow or Lance types may leak into `quorum-network` or `quorum-coordinator`.
      - All 10 scenarios in `tests/e2e/test_federated_mesh.rs` must pass cleanly.

   D. **Hardware & Testing Invariants**:
      - All test commands MUST be executed sequentially with `-- --test-threads=2` (respecting `jobs = 2` in `.cargo/config.toml` and `debug = 0` in `Cargo.toml`). Never run cargo commands concurrently.

5. Mandatory Verification Matrix:
   ```powershell
   cargo test -p quorum-network -- --test-threads=2
   cargo test -p quorum-coordinator -- --test-threads=2
   cargo test --manifest-path tests/e2e/Cargo.toml --test test_federated_mesh -- --test-threads=2
   cargo test -p quorum-storage -- --test-threads=2
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace -- --test-threads=2
   ```

6. Automatic Git Commit Protocol:
   Once verified, stage all changes (`git add -A`) and commit with clear descriptive message.

7. Deliver a comprehensive 5-component handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m4_1\handoff.md` and send your completion message to parent upon finishing.
