# Handoff Report: Adversarial Challenge 2 (Milestone 1)

**Agent**: challenger_m1_2 (Empirical Challenger 2)  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\challenger_m1_2\`  
**Target Milestone**: Milestone 1 (Core Types, Gemma Contract & Magika Classifier)  
**Date**: 2026-10-31T15:34:00Z  
**Verdict**: **`REQUEST_CHANGES`**

---

## 1. Observation

### 1.1 Empirical Test Suite Execution
Created independent, empirical stress-test harness:
`crates/quorum-types/tests/adversarial_challenge.rs` (12 tests evaluating memory zeroization, raw pointer inspection, 20-thread concurrency, AccessTier 4x4 authorization matrix, SQL predicate generation, line number boundary stress, and serialization fuzzing).

Command executed:
```powershell
cargo test --package quorum-types --test adversarial_challenge -- --nocapture
```

### 1.2 Verbatim Test Failures & Observed Panic Traces

#### Defect 1: Hard Panic (DoS) in `serde_sig64::deserialize` on Multi-Byte UTF-8
- **File**: `crates/quorum-types/src/lib.rs:417:51`
- **Verbatim Error**:
```text
thread 'test_federated_query_serde_sig64_multibyte_panic_probe' panicked at crates\quorum-types\src\lib.rs:417:51:
end byte index 2 is not a char boundary; it is inside '🔥' (bytes 0..4 of string)
```
- **Code snippet observed**:
```rust
409: if v.len() != 128 {
410:     return Err(E::custom(...));
411: }
412: let mut out = [0u8; 64];
413: for i in 0..64 {
414:     out[i] = u8::from_str_radix(&v[i * 2..i * 2 + 2], 16)
415:         .map_err(E::custom)?;
416: }
```
`v.len()` is byte length, not char count. When an untrusted JSON payload contains a 128-byte string with multi-byte UTF-8 characters (e.g. `🔥`), `&v[i * 2..i * 2 + 2]` slices in the middle of a Unicode scalar value, triggering an unhandled panic that crashes the deserializing worker thread.

#### Defect 2: Hard Panic (DoS) via Integer Subtraction Overflow in `FederatedQueryRequest::is_fresh()`
- **File**: `crates/quorum-types/src/lib.rs:486:21`
- **Verbatim Error**:
```text
thread 'test_federated_query_timestamp_overflow_panic_probe' panicked at crates\quorum-types\src\lib.rs:486:21:
attempt to subtract with overflow
```
- **Code snippet observed**:
```rust
484: pub fn is_fresh(&self, max_drift_secs: i64) -> Result<(), TypeError> {
485:     let now = Utc::now().timestamp();
486:     let drift = (now - self.timestamp).abs();
487:     if drift > max_drift_secs {
```
When an untrusted peer passes `self.timestamp = i64::MIN` (`-9223372036854775808`), `now - i64::MIN` overflows signed 64-bit integer arithmetic in debug and overflow-checked builds, immediately crashing the node. Furthermore, `(-9223372036854775808i64).abs()` is also an overflow panic in Rust.

#### Defect 3: In-Flight Query Tampering via Unsigned Fields in `canonical_signing_bytes()`
- **File**: `crates/quorum-types/src/lib.rs:466:5`
- **Observation**:
In `test_federated_query_canonical_signing_tampering_oracle`:
```rust
pub fn canonical_signing_bytes(&self) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64 + self.sender_peer_id.len() + self.query_text.len());
    bytes.extend_from_slice(b"QUORUM_QUERY_V1\n");
    bytes.extend_from_slice(self.query_id.as_bytes());
    bytes.extend_from_slice(self.sender_peer_id.as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(&self.timestamp.to_le_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(self.query_text.as_bytes());
    bytes
}
```
`canonical_signing_bytes` signs ONLY `query_id`, `sender_peer_id`, `timestamp`, and `query_text`.
Fields `top_k`, `embedding`, and `sender_pubkey` are completely omitted from the signed payload.
Empirical test demonstrated that modifying `req.top_k = 1_000_000` or replacing `req.embedding` with an adversarial vector results in `digest_orig == digest_tampered`, allowing malicious relays to mount DoS amplification or vector injection attacks without breaking signature validation.

#### Defect 4: Missing AccessTier Validation in `DocumentChunk::new()`
- **File**: `crates/quorum-types/src/lib.rs:184`
- **Observation**:
`DocumentChunk::new(..., access_tier: 255, ...)` returns `Ok(chunk)`. Later, calling `chunk.access_tier_typed()` fails with `Err(TypeError::InvalidAccessTier(255))`. `new()` fails to validate `AccessTier::from_u8(access_tier)`.

#### Defect 5: JSON Deserialization Invariant Bypass in `DocumentChunk`
- **File**: `crates/quorum-types/src/lib.rs:137`
- **Observation**:
In `test_document_chunk_boundary_line_numbers`, a crafted JSON payload with `line_start: 500, line_end: 10` deserialized without error, bypassing the `line_start <= line_end` validation enforced by `DocumentChunk::new()`.

### 1.3 Verified Positive Invariants
1. **Volatile Memory Security**:
   - `test_volatile_memory_zeroize_inspection` verified that `volatile.zeroize()` actively zeroes the heap memory buffer. Raw pointer slice inspection verified all bytes transitioned from the sensitive token string to `0x00`.
   - `test_volatile_memory_concurrency_stress` verified 10,000 concurrent allocations and drops across 20 threads with zero data races, leaks, or deadlocks.
2. **AccessTier Authorization & SQL Predicates**:
   - Exhaustive 4x4 authorization matrix confirmed $req \le max \iff \text{authorized}$.
   - All SQL filter predicates generated strictly formatted scalar comparisons (`"access_tier <= 0"`, `"access_tier <= 1"`, `"access_tier <= 2"`, `"access_tier <= 3"`), injection-free.
3. **Citation Truncation Safety**:
   - `to_citation()` correctly truncated 4-byte and 3-byte unicode strings at character boundaries without panicking.

---

## 2. Logic Chain

1. **Premise 1 (Untrusted Network Boundary Requirement)**:
   - In P2P swarm architectures (`rust-libp2p`), deserializing remote peer requests is an untrusted security boundary. Any unexpected input that induces a process panic constitutes a Remote Denial of Service (DoS) vulnerability.
2. **Step 2 (Deduction of Defect 1 Vulnerability)**:
   - In `serde_sig64::deserialize`, `v.len() != 128` only validates byte length. A 128-byte string containing UTF-8 characters of width > 1 byte (such as emojis or accented characters) will have fewer than 128 chars.
   - Slicing `&v[i * 2..i * 2 + 2]` slices into non-char boundaries, which triggers `std::panic` in Rust's string slice indexing.
   - This was empirically confirmed by `test_federated_query_serde_sig64_multibyte_panic_probe`.
3. **Step 3 (Deduction of Defect 2 Vulnerability)**:
   - In `FederatedQueryRequest::is_fresh`, subtracting `self.timestamp` from `now` without checked arithmetic overflows `i64` when `self.timestamp` is a extreme negative value (`i64::MIN`).
   - This was empirically confirmed by `test_federated_query_timestamp_overflow_panic_probe`.
4. **Step 4 (Deduction of Defect 3 Invariant Violation)**:
   - PROJECT.md Feature 25 specifies "Ed25519 Query Verification: Digital signature signing".
   - If `top_k` and `embedding` are not part of `canonical_signing_bytes`, the signature does not authenticate the query payload. An attacker can change `top_k` to `1_000_000` (causing excessive memory allocation on the responder) or substitute arbitrary query vectors.
5. **Step 5 (Deduction of Verdict)**:
   - Because Defect 1 and Defect 2 cause reproducible panics during untrusted deserialization/query handling, and Defect 3 allows signature bypass/tampering, Milestone 1 cannot be approved in its current state.
   - Therefore, the verdict is **`REQUEST_CHANGES`**.

---

## 3. Caveats

- Candle neural weights were not loaded in these tests; tests exercised the deterministic offline execution and domain type boundaries.
- Full libp2p network transport fuzzing is scoped for Milestone 4; current tests fuzzed the DTO serialization layer.

---

## 4. Conclusion

**Verdict: REQUEST_CHANGES**

Required Remediations before Approval:
1. **Fix `serde_sig64` Deserialization Panic**:
   Validate that `v.is_ascii()` and `v.chars().all(|c| c.is_ascii_hexdigit())` BEFORE slicing, or decode bytes directly without `&str` slicing. Return a serde error (`E::custom(...)`), never panic.
2. **Fix `is_fresh()` Overflow Panic**:
   Replace `(now - self.timestamp).abs()` with `now.checked_sub(self.timestamp)`. If subtraction overflows or result exceeds `max_drift_secs`, return `Err(TypeError::QueryExpired { ... })`.
3. **Bind `top_k`, `embedding`, and `sender_pubkey` into `canonical_signing_bytes()`**:
   Include `top_k.to_le_bytes()`, `sender_pubkey`, and the BLAKE3 digest of `embedding` in `canonical_signing_bytes()` to prevent in-flight tampering.
4. **Validate AccessTier in `DocumentChunk::new()`**:
   Invoke `AccessTier::from_u8(access_tier)?` in `DocumentChunk::new()` so chunks with unmapped tiers cannot be created.

---

## 5. Verification Method

To reproduce and verify these findings independently:

1. **Run the Empirical Challenge Test Suite**:
   ```powershell
   cd d:\harfile\QuorumRAG
   cargo test --package quorum-types --test adversarial_challenge -- --nocapture
   ```
   *Observed Result: 10 passed; 2 failed (`test_federated_query_serde_sig64_multibyte_panic_probe`, `test_federated_query_timestamp_overflow_panic_probe`).*

2. **Run Existing Adversarial Test Suites**:
   ```powershell
   cargo test --package quorum-types --test adversarial_types_tests -- --nocapture
   ```
   *Observed Result: Fails with panic at `test_signature_deserialization_panic_resilience`.*

3. **Check Invalidation Conditions**:
   When the worker replaces unchecked string slicing with ASCII hex verification and checked timestamp subtraction, all 12 tests in `adversarial_challenge.rs` will pass with code 0.
