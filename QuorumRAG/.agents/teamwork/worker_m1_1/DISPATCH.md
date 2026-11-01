# DISPATCH — worker_m1_1

## Task Assignment
You are Worker 1: Milestone 1 Implementation Worker (Core Types, Gemma Contract & Magika Classifier).
Working directory: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m1_1/`

Read `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` and `d:\harfile\QuorumRAG\PROJECT.md`.
Also review the survey findings in:
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_1\handoff.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_2\handoff.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\spec_miner_survey_1\handoff.md`

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Write Ownership
You exclusively own and may create/modify:
- `d:\harfile\QuorumRAG\Cargo.toml` (root workspace file)
- `d:\harfile\QuorumRAG\crates\quorum-types\`
- `d:\harfile\QuorumRAG\crates\quorum-classifier\`
- `d:\harfile\QuorumRAG\crates\quorum-contract\`

## Implementation Scope
1. **Root `Cargo.toml`**:
   - Virtual workspace declaring members: `crates/quorum-types`, `crates/quorum-classifier`, `crates/quorum-contract`.
   - Workspace-level package settings and shared dependencies (blake3, serde, serde_json, uuid, chrono, tokio, thiserror, anyhow, subtle, zeroize, etc.).
2. **`crates/quorum-types`**:
   - Pure domain models: `DocumentChunk`, `ScoredChunk`, `AccessTier` (0=Public, 1=Internal, 2=Confidential, 3=Restricted), `Citation`, `VolatileRemoteChunk` (with `zeroize`), `FederatedQueryRequest`, `FederatedQueryResponse`, `CandidateChunkDto`.
   - Invariant: ZERO dependencies on `arrow-array` or `lancedb`.
3. **`crates/quorum-classifier`**:
   - Google Magika file identification engine (`magika` crate or deep-learning/MIME content inspection).
   - `FileTypeClassifier`: inspects file bytes (first 1024-4096 bytes or full buffer), identifies MIME type, detects binaries vs source code/markdown/documents.
   - Support for common extensions and content types.
4. **`crates/quorum-contract`**:
   - Canonical `EmbeddingContract` (version 1, model "google/embeddinggemma-300m", dimension 768, normalization "L2", metric "Cosine", pooling "MeanPooling", sequence length 512).
   - Canonical BLAKE3 contract hash derivation `derive_contract_hash(&self) -> [u8; 32]`.
   - Handshake validation `verify_contract_handshake(local_hash, remote_hash) -> Result<(), ContractMismatchError>` with constant-time equality check (`subtle::ConstantTimeEq`).
   - `EmbeddingGenerator` trait with:
     - `DeterministicOfflineEngine`: fast, zero-network, reproducible L2-normalized 768-dim embeddings derived from token BLAKE3 hashes for instant testing and CI.
     - `GemmaCandleEngine`: Candle-based offline Gemma embedding loader.
5. **Automated Verification**:
   - Run `cargo test -p quorum-types`, `cargo test -p quorum-classifier`, `cargo test -p quorum-contract`.
   - Verify all tests compile cleanly with 0 warnings/errors.
   - Document commands, test logs, and layout in `d:\harfile\QuorumRAG\.agents\teamwork\worker_m1_1\handoff.md`.


## 2026-10-31T15:08:16Z
You are Worker 1: Milestone 1 Implementation Worker (Core Types, Gemma Contract & Magika Classifier).
Working directory: d:\harfile\QuorumRAG\.agents\teamwork\worker_m1_1\
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md, d:\harfile\QuorumRAG\PROJECT.md, and d:\harfile\QuorumRAG\.agents\teamwork\worker_m1_1\DISPATCH.md.
MANDATORY INTEGRITY WARNING: DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
Your write ownership is:
- d:\harfile\QuorumRAG\Cargo.toml
- d:\harfile\QuorumRAG\crates\quorum-types\
- d:\harfile\QuorumRAG\crates\quorum-classifier\
- d:\harfile\QuorumRAG\crates\quorum-contract\
Implement genuine, production-grade code for all three crates, build and execute cargo test across them, verify 0 warnings/errors, and write a complete handoff report to d:\harfile\QuorumRAG\.agents\teamwork\worker_m1_1\handoff.md. Send a message when done.
