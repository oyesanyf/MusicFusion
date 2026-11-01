# DISPATCH — worker_m1_2

## Task Assignment
You are Worker 2: Milestone 1 Remediation Worker (Iteration 2).
Working directory: `d:\harfile\QuorumRAG\.agents\teamwork\worker_m1_2/`

Read:
- `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m1_remediation\handoff.md`

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Write Ownership
You exclusively own and may create/modify:
- `d:\harfile\QuorumRAG\crates\quorum-types\`
- `d:\harfile\QuorumRAG\crates\quorum-classifier\`
- `d:\harfile\QuorumRAG\crates\quorum-contract\`

## Implementation Scope
Follow the detailed code blueprints in `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m1_remediation\handoff.md`:
1. **Fix Hex Slicing DoS Panics**:
   - `crates/quorum-types/src/lib.rs`: Update `serde_sig64::deserialize` to parse hex bytes safely without string slicing.
   - `crates/quorum-contract/src/lib.rs`: Update `parse_hex_32` to parse hex bytes safely without string slicing.
2. **Fix CJK & Multi-Byte UTF-8 Ingestion Rejection**:
   - `crates/quorum-classifier/src/lib.rs`: In `is_high_entropy_binary` and `classify_bytes`, use `valid_up_to()` on sample boundaries and inspect Unicode control characters (`c.is_control()`) so valid UTF-8 documents (CJK, Cyrillic, Arabic, etc.) are never rejected as binary.
3. **Fix Integer Subtraction Overflow Panic**:
   - `crates/quorum-types/src/lib.rs`: In `FederatedQueryRequest::is_fresh()`, replace unchecked subtraction with `now.checked_sub(self.timestamp).and_then(|d| d.checked_abs()).unwrap_or(i64::MAX)`.
4. **Cryptographically Bind Payload to Ed25519 Signature**:
   - `crates/quorum-types/src/lib.rs`: In `canonical_signing_bytes()`, bind `sender_pubkey`, `timestamp`, `top_k`, and the BLAKE3 digest of `embedding`.
   - Update `crates/quorum-types/tests/adversarial_challenge.rs` oracle assertion to assert `!=` on tampering.
5. **Fix Positional Token Index Hashing**:
   - `crates/quorum-contract/src/lib.rs`: In `DeterministicOfflineEngine::generate_vector`, remove positional `idx` hashing so tokens match position-invariantly.
6. **Fix Plaintext Memory Clone Leak**:
   - `crates/quorum-types/src/lib.rs`: Derive `Zeroize` on `CandidateChunkDto` and implement consuming `into_volatile_remote_chunk(self)`.
7. **Enforce AccessTier Validation**:
   - `crates/quorum-types/src/lib.rs`: In `DocumentChunk::new()`, call `AccessTier::from_u8(access_tier)?`.
   - Harmonize test expectation in `adversarial_challenge.rs`.
8. **Magika & Candle Engine Transparency**:
   - `crates/quorum-classifier/src/lib.rs`: Wire `magika::Session` under `#[cfg(feature = "onnx")]`, add ONNX error variants, and label structural inspection cleanly.
   - `crates/quorum-contract/src/lib.rs`: Accurately report `is_neural_loaded()` based on the actual presence of `model.safetensors` alongside `tokenizer.json`, add `has_tokenizer(&self) -> bool`, and handle token tensors cleanly.

## Verification Requirements
Run all test suites and verify 100% pass:
1. `cargo test --package quorum-classifier --test adversarial_magika_tests`
2. `cargo test --package quorum-contract --test adversarial_contract_tests`
3. `cargo test --package quorum-types --test adversarial_types_tests`
4. `cargo test --package quorum-types --test adversarial_challenge`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --manifest-path tests/e2e/Cargo.toml`

Document verbatim outputs in `d:\harfile\QuorumRAG\.agents\teamwork\worker_m1_2\handoff.md`.
