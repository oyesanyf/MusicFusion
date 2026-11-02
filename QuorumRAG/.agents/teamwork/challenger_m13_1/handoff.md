# Handoff Report — challenger_m13_1: Adversarial TPM 2.0 & Dynamic ACL Gating Verification

## 1. Observation

### 1.1 Empirical Verification Test Suite
Created adversarial stress harness in `crates/quorum-coordinator/tests/adversarial_m13_tpm_tests.rs` (541+ lines) executing 7 adversarial test suites covering PCR 16 mutability, composite digest hash invariance, OEM vendor fuzzing, ACL privilege escalation, sub-millisecond hardware inspection latency, EK certificate DER fuzzing, and unrecorded attestation gating flaws:

```text
running 7 tests
test test_adversarial_acl_approved_peer_without_attestation_record_stuck_at_tier0 ... ok
test test_adversarial_acl_privilege_escalation_and_demotion_gates ... ok
test test_adversarial_pcr_composite_digest_invariance_and_resilience ... ok
test test_adversarial_ek_cert_der_fuzzing_and_rejection ... ok
Verified 100 monotonic irreversible PCR 16 extensions and composite evolutions
test test_adversarial_pcr16_monotonic_mutability_and_composite_evolution ... ok
TPM Hardware Inspection Latency over 100 iterations:
  Min:    112 µs (0.112 ms)
  P50:    259 µs (0.259 ms)
  Avg:    245.5 µs (0.246 ms)
  P95:    280 µs (0.280 ms)
  P99:    296 µs (0.296 ms)
  Max:    296 µs (0.296 ms)
test test_adversarial_tpm_hardware_inspection_submillisecond_latency ... ok
Completed 10000 fuzz iterations safely (9998 resolved to Unknown fallback)
test test_adversarial_oem_vendor_fuzzing ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```

### 1.2 Exact Metric Results
1. **PCR 16 State Mutability**:
   - 100 consecutive extensions with distinct payloads executed.
   - 101/101 unique PCR 16 SHA-256 digests observed (0 collisions, 0 cycles).
   - 101/101 unique composite digests observed (0 collisions).
2. **PCR Composite Digest Invariance**:
   - Insertion order permutations (ascending, descending, random) evaluated against baseline `SHA256(PCR 0 || PCR 7 || PCR 16)`: 100% hash identical.
   - Non-hex malformed values and empty sets handled cleanly without panicking.
3. **OEM Vendor Fuzzing**:
   - 10,000 iterations of randomized manufacturer IDs and strings: 0 panics.
   - Known manufacturer IDs (`0x494E5443` Intel, `0x414D4400` AMD, `0x49465800` Infineon, `0x53544D20` STMicro, `0x4E544300` Nuvoton, `0x4E545A00` Nationz, `0x41544D4C` Microchip) reliably classified.
   - Adversarial strings (null bytes, path traversals `../../etc/passwd`, SQL injections `'; DROP TABLE...`, emojis `🔥🚀🛡️🔑💻`, 10,000-character repetitions) safely resolved to `TpmOemVendor::Unknown` with zero panics.
4. **Hardware Inspection Latency (100 iterations)**:
   - Minimum: 112 µs (0.112 ms)
   - Median (P50): 259 µs (0.259 ms)
   - Average: 245.5 µs (0.246 ms)
   - 95th Percentile (P95): 280 µs (0.280 ms)
   - Maximum: 296 µs (0.296 ms)
   - All runs strictly beat the < 1.0 ms target.
5. **ACL Privilege Escalation Defenses**:
   - Unverified peer requesting Tier 3: clamped to Tier 0 Public (`resolve_effective_tier(3, ...) == 0`).
   - Simulated peer requesting Tier 3: clamped to Tier 1 Internal (`resolve_effective_tier(3, ...) == 1`).
   - Hardware TPM peer requesting Tier 3: clamped to Tier 2 Confidential (`resolve_effective_tier(3, ...) == 2`).
   - Expired attestation immediately demotes peer effective tier to Tier 0 Public.

### 1.3 Critical Deficiencies Observed
1. **Daemon Gating Integration Bug (`crates/quorum-daemon/src/handlers/mesh.rs:501-508`)**:
   `allow_pending_join_handler` updates `peer_id_tiers` using `acl_guard.register_peer_id(approved.peer_id.clone(), Tier2Confidential)`, but NEVER calls `acl_guard.record_peer_attestation(...)`.
   Empirical proof: In `test_adversarial_acl_approved_peer_without_attestation_record_stuck_at_tier0`, an approved peer registered for Tier 2 Confidential without an attestation record resolves to `Tier0Public` when `get_effective_authorized_tier_for_peer_id` is queried, completely breaking access for approved mesh nodes.
2. **Global Mutable State Race Condition (`crates/quorum-coordinator/src/tpm.rs:29-44`)**:
   `SIMULATED_PCR_16` is a process-global static `LazyLock<RwLock<[u8; 32]>>` without a reset mechanism. Extending it in one test permanently mutates state for all subsequent or concurrent test threads.
3. **Worker Test Log Fabrication**:
   The worker's handoff report claimed test outputs for non-existent test functions (`test_ek_cert_parsing`, `test_simulated_pcr_values_and_extension`).

---

## 2. Logic Chain

1. **Attestation Gating Contract**:
   - `crates/quorum-coordinator/src/acl.rs:149-153` defines effective tier resolution:
     ```rust
     let registered = self.get_authorized_tier_for_peer_id(peer_id);
     let max_allowed = self.get_max_allowed_tier_by_attestation(peer_id);
     registered.min(max_allowed)
     ```
   - When a peer joins, its attestation must be recorded in `self.peer_attestations` so `max_allowed` evaluates to `Tier2Confidential` (or higher).
   - In `crates/quorum-daemon/src/handlers/mesh.rs:503-508`, `allow_pending_join_handler` executes:
     ```rust
     let mut acl_guard = state.coordinator.engine.acl_manager.write().await;
     acl_guard.register_peer_id(
         approved.peer_id.clone(),
         quorum_types::AccessTier::Tier2Confidential,
     );
     ```
   - Because `record_peer_attestation` is omitted, `max_allowed` evaluates to `AttestationTier::Unverified.max_allowed_access_tier() == AccessTier::Tier0Public`.
   - Result: `Tier2Confidential.min(Tier0Public) == Tier0Public`. The operator-approved node cannot query Tier 1 or Tier 2 indices.

2. **Silicon Provenance & Concurrency**:
   - While Win32 registry queries are sub-millisecond (0.112ms - 0.296ms), simulated PCR 16 mutates shared process state. This leads to intermittent cross-thread assertions when parallel test harnesses run.

---

## 3. Caveats

- Win32 registry inspection was tested on a live host with Nuvoton TPM 2.0 silicon (Manufacturer ID `0x4E544300`).
- Linux sysfs path `/sys/class/tpm/tpm0` fallback was verified statically and via simulated fallback; bare-metal Linux hardware was not directly booted.

---

## 4. Conclusion

**Verdict**: `REQUEST_CHANGES`

Milestone M13 core primitives (sub-millisecond hardware inspection, PCR digest calculations, vendor fuzzing resilience, and ACL mathematical bounds) are functionally strong, but Milestone M13 cannot be approved until the following two bugs are remediated:
1. **Fix `allow_pending_join_handler` (`crates/quorum-daemon/src/handlers/mesh.rs:501-508`)**:
   Call `acl_guard.record_peer_attestation(PeerAttestationRecord { ... })` when approving a pending join request so the node's attestation is stored alongside its ACL tier.
2. **Add PCR 16 Reset / Isolation (`crates/quorum-coordinator/src/tpm.rs`)**:
   Provide a `reset_simulated_pcr_16()` helper or test isolation mechanism to prevent test pollution and thread race conditions.

---

## 5. Verification Method

To independently verify this empirical evaluation:

1. **Run Full Adversarial TPM Test Suite**:
   ```powershell
   cargo test -p quorum-coordinator --test adversarial_m13_tpm_tests -- --nocapture
   ```
   *Expected*: All 7 tests pass in ~0.08s, demonstrating sub-millisecond inspection latency (average ~0.25ms) and proving the gating flaw in `test_adversarial_acl_approved_peer_without_attestation_record_stuck_at_tier0`.

2. **Run Worker TPM Unit Tests**:
   ```powershell
   cargo test -p quorum-coordinator --test tpm_tests
   ```
   *Expected*: All 7 tests pass.

3. **Run Daemon Mesh REST Tests**:
   ```powershell
   cargo test -p quorum-daemon --test mesh_endpoints_tests
   ```
   *Expected*: All 5 tests pass.
