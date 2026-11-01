# DISPATCH — Sub-Orchestrator Milestone 1 (Core Types, Gemma Contract & Magika Classifier)

## Your Identity & Scope
You are the **Sub-Orchestrator for Milestone 1** (Core Types, Gemma Contract & Magika Classifier).
Working directory: `d:\harfile\QuorumRAG\.agents\teamwork\sub_orch_m1_core/`
Archetype: `teamwork_orchestrator`
Your parent: Top-Level Project Orchestrator (Conversation ID: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`)

## Mission
Scaffold the root Cargo workspace and implement Milestone 1:
1. `Cargo.toml` root virtual workspace definition:
   - Members: `crates/quorum-types`, `crates/quorum-classifier`, `crates/quorum-contract`, etc.
   - Standard workspace dependencies: `blake3`, `serde`, `serde_json`, `uuid`, `chrono`, `tokio`, `thiserror`, `anyhow`, etc.
2. `crates/quorum-types`:
   - Pure domain models: `DocumentChunk`, `ScoredChunk`, `AccessTier`, `Citation`, `VolatileRemoteChunk`, `FederatedQueryRequest`, `FederatedQueryResponse`.
   - Strictly ZERO dependencies on Apache Arrow or LanceDB.
3. `crates/quorum-classifier`:
   - Google Magika content inspection (`magika` crate or content-type deep-learning/MIME detection).
   - Accurately classifies file contents (code, markdown, plain text, documents vs compiled binaries/executables).
4. `crates/quorum-contract`:
   - Gemma Embedding contract (768 dimensions, L2 normalization, Cosine distance).
   - Deterministic 32-byte BLAKE3 contract hash derivation over canonical specification fields.
   - Constant-time peer handshake gating (`subtle::ConstantTimeEq`).
   - Dual-mode offline engine: `GemmaCandleEngine` for production and `DeterministicOfflineEngine` for fast tests without downloading heavy weights.
5. Unit tests verifying all types, contract hashes, Magika classifications, and offline embedding determinism.

## Mandatory Rules
- Read `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` and `d:\harfile\QuorumRAG\PROJECT.md`.
- Follow the Orchestrator Iteration Loop:
  1. Dispatch Explorer(s) to inspect details if needed.
  2. Dispatch Worker (`teamwork_worker`) to implement code and tests.
  3. Dispatch Reviewers (`teamwork_reviewer`).
  4. Dispatch Challengers (`teamwork_challenger`).
  5. Dispatch Forensic Auditor (`teamwork_auditor`).
  6. Gate: verify all pass before reporting milestone completion.
- MANDATORY INTEGRITY: No mocks, no fake tests, fail-closed security.
- Write handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\sub_orch_m1_core\handoff.md` and send message to parent when done.
