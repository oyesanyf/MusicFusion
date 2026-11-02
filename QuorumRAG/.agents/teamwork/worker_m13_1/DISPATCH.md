## 2026-11-02T17:11:14Z
You are worker_m13_1. Your working directory is `d:\harfile\QuorumRAG\.agents\teamwork\worker_m13_1\`.
Follow the File Workspace Convention: write ONLY to your assigned directory for agent metadata (progress.md, handoff.md, BRIEFING.md, DISPATCH.md).
Never place source code or tests in `.agents/teamwork/`.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

CRITICAL INVARIANTS:
1. Universal Zero-Touch PATH & Environment Protection: Never alter, setx, or mutate user/system PATH or registry keys. Process-scoped env vars only (`$env:CARGO_INCREMENTAL = "0"`).
2. Production Code Only: No mocks or fake data.
3. Zero-Wait Incremental Commits (`NO COMMIT SHOULD WAIT`): As soon as your code compiles and tests pass, immediately stage with `git add -A` and commit with a clear commit message.
4. Keep the live daemon on port 3031 in mind: do not break port 3031 server configuration or `server.rs:115` (`bypass_cache: false`).

TASK OBJECTIVE:
Implement Milestone M13 — Adapt OshoosiClaw Mesh UI & TPM 2.0 Hardware Identity/Attestation into QuorumRAG:
1. Read the following authoritative documents FIRST:
   - `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
   - `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\SCOPE.md`
   - `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m13_1\handoff.md` (Contains complete Rust architecture and code for `crates/quorum-coordinator/src/tpm.rs` and `crates/quorum-coordinator/src/acl.rs`)
   - `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m13_2\handoff.md` (Contains complete Rust architecture and DTOs/handlers for daemon mesh REST endpoints)
   - `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m13_3\handoff.md` (Contains complete JavaScript and CSS for WebUI Settings "Mesh Network & Shared Folders")

2. Implement `crates/quorum-coordinator/src/tpm.rs`:
   - Fast Win32 Registry inspection on `HKLM\SYSTEM\CurrentControlSet\Services\TPM\WMI` reading `ManufacturerId` (or `TaskManufacturerId`), `TaskFirmwareVersion`, `FirmwareVersionAtLastProvision` in < 1ms.
   - Linux sysfs `/sys/class/tpm/tpm0` inspection & simulated fallback.
   - Deterministic pure-Rust simulated fallback (`simulated_pcr(0)`, `simulated_pcr(7)`, `simulated_pcr(16)` with irreversible `extend_simulated_pcr_16`).
   - TCG canonical `compute_pcr_composite_digest` over PCR 0, 7, 16.
   - `TpmOemVendor` detection (`Intel`, `Amd`, `Infineon`, `StMicro`, `Nuvoton`, `Nationz`, `Microchip`, `Unknown`).
   - `TpmEkCertificate` parsing and telemetry struct `TpmHardwareInspection`.
   - Export `pub mod tpm;` in `crates/quorum-coordinator/src/lib.rs`.
   - Add any needed dependencies in `crates/quorum-coordinator/Cargo.toml` (e.g. `sha2`, `hex`, `x509-parser`, `windows-sys` with `Win32_System_Registry` or std registry access, or windows-sys features). Ensure it compiles cleanly on Windows.

3. Update `crates/quorum-coordinator/src/acl.rs`:
   - Wire dynamic attestation-tier gating into `PeerAclManager`:
     * Verified TPM 2.0 with EK -> Tier 3
     * Verified TPM 2.0 -> Tier 2
     * Simulated TPM fallback -> Tier 1
     * Unverified / Expired -> Tier 0
     * `effective_tier = min(static_tier, attestation_tier_max)`.

4. Implement Mesh REST Endpoints in `crates/quorum-daemon`:
   - Add routes in `crates/quorum-daemon/src/server.rs` and handlers in `crates/quorum-daemon/src/handlers/settings.rs` (or `crates/quorum-daemon/src/handlers/mesh.rs`):
     * `GET /api/mesh/tpm/status` -> returns `TpmHardwareInspection`
     * `GET /api/mesh/topology` (and `/api/topology`, `/api/peers`) -> returns local node and peer collection with RoT badge, latency, attestation state, access tier.
     * `GET /api/mesh/bootstrap-peers` & `POST /api/mesh/bootstrap-peers`
     * `GET /api/mesh/pending-joins`, `POST /api/mesh/pending-joins/:peer_id/allow`, `POST /api/mesh/pending-joins/:peer_id/deny`, `POST /api/mesh/pending-joins/clear` (strictly reject legacy TPM 1.2 peers).
     * `GET /api/mesh/quarantined-peers`, `POST /api/mesh/quarantined-peers/:peer_id/release`.

5. Update WebUI in `crates/quorum-ui/assets/app.js` and `crates/quorum-ui/assets/styles.css`:
   - In Settings > "Mesh Network & Shared Folders" tab:
     * Render Local Hardware Root-of-Trust (RoT) Badge & Silicon Attestation Card (Green for hardware TPM 2.0 with manufacturer, version, and PCR 0/7/16 composite digest; Yellow for simulated fallback).
     * Render Connected Nodes & Attestation Topology Panel (Local host card and remote peer cards with latency and attestation badges).
     * Render Pending Joins Operator Approval Queue (Allow/Deny actions, legacy TPM 1.2 peers blocked).
     * Render Quarantined Peers Panel (Release button).

6. Verification:
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo check --workspace`
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-coordinator`
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-daemon`
   - Run `$env:CARGO_INCREMENTAL = "0"; cargo test -p quorum-ui`
   - Add unit tests verifying TPM inspection, ACL attestation-tier gating, daemon endpoints, and UI assets.
   - Stage (`git add -A`) and commit (`git commit -m "feat: adapt OshoosiClaw TPM 2.0 hardware identity, attestation-tier gating, and Mesh UI"`).

7. Output:
   - Write comprehensive `handoff.md` in `d:\harfile\QuorumRAG\.agents\teamwork\worker_m13_1\handoff.md` with:
     * Observation (what was implemented, files touched)
     * Logic Chain
     * Caveats
     * Conclusion
     * Verification Method and exact test results
   - Send completion message to parent via `send_message`.
