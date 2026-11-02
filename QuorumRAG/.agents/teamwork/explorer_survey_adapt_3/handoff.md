# Technical Investigation Handoff Report: OshoosiClaw Mesh Configuration UI & TPM Hardware Identity Adaptation & Server Port 3031 Health

**Agent**: Survey Explorer 3 (`teamwork_explorer`)  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_3\`  
**Date**: 2026-11-02T14:21:00Z  
**Target Milestone**: Requirements 4 & 5 Investigation, Architectural Blueprint & Implementation Specification  
**Parent Orchestrator ID**: `6e43b521-b448-4762-842a-0bc8d66e2854`

---

## 1. Observation

### 1.1 OshoosiClaw TPM Hardware Identity & Attestation Implementation
Direct inspection of `d:\harfile\OshoosiClaw` revealed the complete TPM 2.0 hardware identity, silicon provenance, and remote mutual attestation architecture:

1. **Hardware Detection & PCR Reading (`d:\harfile\OshoosiClaw\crates\osoosi-audit\src\tpm.rs`)**:
   - Lines 20–33: Defines application audit PCR bank `const AUDIT_PCR_INDEX: u32 = 16` and `AttestationResult` struct:
     ```rust
     pub struct AttestationResult {
         pub hardware_backed: bool,
         pub pcr_index: Option<u32>,
         pub attestation_hash: String,
         pub timestamp: chrono::DateTime<chrono::Utc>,
     }
     ```
   - Lines 38–53: Defines in-memory `SIMULATED_PCR_16` using `LazyLock<RwLock<[u8; 32]>>` for software fallback when running in CI or virtualized environments.
   - Lines 60–85: Detects physical TPM 2.0 on Windows via Win32 Registry API `RegOpenKeyExW` against `HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\TPM\WMI`.
   - Lines 165–241: Direct low-latency Windows registry inspection reads `ManufacturerId` (DWORD) and `TaskFirmwareVersion` / `FirmwareVersionAtLastProvision` (SZ) to construct hardware-bound PCR hashes in `< 1ms`.
   - Lines 144–163: Direct Linux read via `tpm2_pcrread sha256:<index>`.
   - Lines 247–268: `simulated_pcr(pcr_index: u32)` provides deterministic fallback for PCR 0 (`SIMULATED_PCR_0_PLATFORM_FIRMWARE_CORE`), PCR 7 (`SIMULATED_PCR_7_SECURE_BOOT_STATE_ACTIVE`), and PCR 16 (`SIMULATED_PCR_16_APPLICATION_AUDIT_LOG`).
   - Lines 280–294: `compute_pcr_composite_digest(pcr_values: &BTreeMap<u32, String>) -> String` computes canonical composite hash `Sha256(sum_i (idx_i || PCR_i))`.

2. **Decentralized Trust Models & Golden Baseline Verification (`d:\harfile\OshoosiClaw\crates\osoosi-types\src\trust.rs`)**:
   - Lines 11–14: `NodeDID { id: String, public_key: String }`.
   - Lines 75–101: `TpmOemVendor` enum supporting `Intel`, `Amd`, `Infineon`, `StMicro`, `Nuvoton`, `Nationz`, `Microchip`, `Unknown`.
   - Lines 103–180: `TpmEkCertificate` struct parsing DER X.509 Endorsement Key certificates provisioned into TPM NVRAM (indices `0x01C00002` RSA, `0x01C0000A` ECC) and detecting silicon OEM vendor.
   - Lines 184–197: `TpmQuote` struct:
     ```rust
     pub struct TpmQuote {
         pub pcr_digest: String,
         pub pcr_indices: Vec<u32>,
         pub quoted_digest: String,
         pub hardware_backed: bool,
         pub raw_quote: Option<Vec<u8>>,
     }
     ```
   - Lines 48–73 & 200–213: `AttestationChallenge` (32-byte CSPRNG nonce, challenger DID, timestamp, pcr_selection) and `AttestationResponse` (challenge nonce, binary hash, config hash, responder DID, signature, pcr_values, tpm_quote, ek_certificate).
   - Lines 215–318: `GoldenBaseline` policy specifying acceptable binary hashes, mesh config hashes, expected PCR banks, `require_hardware_tpm: bool`, `require_tpm_ek_validation: bool`, allowed OEM vendors, and pinned EK fingerprints.
   - Lines 361–515: `verify_tpm_ek_certificate(ek_cert, policy)` performs cryptographic signature verification against OEM Root CAs and enforces Golden Baseline vendor constraints.
   - Lines 517–548: Defines peer management entities: `ReputationScore`, `PendingJoinRequest`, and `QuarantinedPeer`.

3. **TBS Inspection Utilities (`d:\harfile\OshoosiClaw\crates\osoosi-trust\src\tbs.rs`)**:
   - Lines 11–21: Defines `TpmHardwareInspection`:
     ```rust
     pub struct TpmHardwareInspection {
         pub available: bool,
         pub tpm_version: String,
         pub manufacturer_id: u32,
         pub manufacturer_str: String,
         pub vendor: TpmOemVendor,
         pub nvram_ek_present: bool,
         pub spec_version: String,
         pub interface_type: String,
     }
     ```
   - Lines 28–60: `inspect_tpm_hardware_tbs()` queries Windows registry/WMI, Linux `/sys/class/tpm/tpm0`, or returns simulated fallback.
   - Lines 235–277: `read_hardware_ek_certificate_nvram()` retrieves the physical hardware EK certificate from NVRAM.

4. **Mesh Join Gate & Anti-Replay Attestation (`d:\harfile\OshoosiClaw\crates\osoosi-wire\src\join_gate.rs`)**:
   - Lines 18–42: `JoinGate` manages pending peer joins, rate-limiting discovery, tracking active nonces, and enforcing `require_tpm_attestation`.
   - Lines 108–138: Anti-replay defense: tracks `seen_announce_nonces` (`DashSet<[u8; 32]>`), rejects replay attacks, enforces timestamp freshness (TTL <= 300s), and evaluates TPM attestation.

---

### 1.2 OshoosiClaw Mesh Configuration UI Implementation
Direct inspection of OshoosiClaw's dashboard backend and frontend scripts:

1. **Backend REST API (`d:\harfile\OshoosiClaw\crates\osoosi-dashboard\src\routes\mesh.rs`)**:
   - Exposes full endpoints (lines 2055–2117):
     - `GET /api/mesh-stats`: peer count, gossip stats, p2p status.
     - `GET /api/mesh/p2p` & `POST /api/mesh/p2p` (toggle P2P mesh on/off).
     - `GET /api/mesh/topology`: returns node graph (id, label, group, status, attestation_state, latency, IP, packets_tx/rx).
     - `GET /api/mesh/bootstrap-peers` & `POST /api/mesh/bootstrap-peers`: manages WAN multiaddresses and DuckDNS auto-bootstrap domains.
     - `GET /api/pending-joins`, `POST /api/pending-joins/:peer_id/allow`, `POST /api/pending-joins/:peer_id/deny`, `POST /api/pending-joins/clear`.
     - `GET /api/quarantined-peers`, `POST /api/quarantined-peers/:peer_id/release`, `POST /api/quarantined-peers/:peer_id/false-positive`.
   - Multiaddr sanitization (lines 602–634): ensures peer addresses strictly begin with `/ip4/`, `/dns4/`, `/dns/`, `/dns6/`, or `/ip6/` and rejects localhost/loopback in remote peer configs.

2. **Frontend UI Components (`d:\harfile\OshoosiClaw\dashboard\src\views\mesh.js` & `index.html`)**:
   - **P2P Swarm Status Banner** (lines 67–144):
     - Disabled: Red border banner *"P2P MESH SWARM: DISABLED BY USER (0 PEERS)"* with *"Enable Peer-to-Peer"* button.
     - Connected: Green border banner *"P2P MESH SWARM: CONNECTED & OPERATIONAL (N Active Peers)"* with GossipSub v1.2, Cryptographic Trust, and Direct TCP peering indicators.
     - Standalone: Gray banner *"P2P MESH SWARM: STANDALONE MODE (0 PEERS)"* listening on port 4001.
   - **Active Mesh Nodes & Telemetry** (lines 166–251):
     - Local Core Node: Label, local IP/port, *"TPM 2.0 RoT Verified"*, Latency, Tx/Rx counters.
     - Remote Peers: Host/peer ID, IP, Attestation State badge (*"TPM 2.0 Hardware RoT Verified"* vs *"Attestation Pending"*), Latency.
   - **Pending Joins Operator Approval Queue** (lines 253–317):
     - Renders pending join requests with Discovered time, Multiaddr, Reputation score & tier, Verified alerts, Host OS & patch status, Attestation state, and Transport.
     - Actions: `✓ Approve / Allow` (`meshAllowPeer`), `✕ Deny` (`meshDenyPeer`), `✓ Approve All` (`meshAllowAllPending`), and `Deny / Clear All` (`meshClearPendingJoins`).
   - **Quarantined Peers Panel** (lines 319–343):
     - Peer ID, Quarantine time, Reason, Reputation, with `Release` and `False Positive` actions.
   - **WAN Mesh & Remote Peering Settings Card (`index.html:1221-1295`)**:
     - *"This Node's Public WAN Multiaddr"* input with copy button.
     - *"DuckDNS Zero-Touch Auto-Bootstrap Peering"* with domain input (e.g. `quorumrag.duckdns.org`).
     - *"Remote Bootstrap Peers to Dial"* table with `+ Add Peer`, `Remove`, and `Save Config & Dial Now` button.

---

### 1.3 QuorumRAG Existing Subsystems & Implementation Gaps
Inspection across QuorumRAG crates:

1. **`crates/quorum-network`**:
   - `transport.rs:12-98`: Builds libp2p swarm with mDNS, Kademlia DHT, GossipSub v1.2, Request-Response (CBOR on `/quorum-rag/query/1.0.0`), and Identify (`/quorum-rag/1.0.0/contract-<hash>`).
   - `actor.rs:109-154`: Peer validation is strictly limited to comparing the 64-character Gemma embedding contract hash inside `identify::Event::Received`.
   - **Gap**: No TPM 2.0 hardware quote exchange, no EK certificate verification, and no `AttestationChallenge` handshake exists in `quorum-network`.

2. **`crates/quorum-coordinator`**:
   - `hardware.rs:1-116`: `HardwareTelemetry::probe()` reads total/free RAM and Nvidia GPU VRAM. It has ZERO TPM hardware inspection.
   - `acl.rs:1-60`: `PeerAclManager` manages static mapping from peer ID / pubkey to `AccessTier` (`Tier0Public` .. `Tier3Restricted`).
   - **Gap**: `PeerAclManager` does not dynamically gate access tiers based on TPM hardware attestation validation status. If a peer has verified physical TPM 2.0 hardware provenance, it should be authorized for higher tiers (`Tier2Confidential` / `Tier3Restricted`), whereas unverified peers should be clamped to `Tier0Public`.

3. **`crates/quorum-daemon`**:
   - `server.rs:115-123`:
     ```rust
     let req = chat::ChatRequest {
         prompt,
         search_mode: None,
         model_family: family,
         top_k: Some(12),
         access_tier: Some(0),
         history: Vec::new(),
         query_id: None,
     };
     ```
     **Missing field**: `bypass_cache: false`. Confirmed via `cargo check -p quorum-daemon`:
     `error[E0063]: missing field bypass_cache in initializer of ChatRequest --> crates\quorum-daemon\src\server.rs:115:15`.
   - `main.rs:136-139`: HTTP listener binds dynamically to `cli.port` (advancing if busy) and P2P swarm binds to `find_next_open_p2p_port` (advancing past HTTP port).
   - Currently, `quorumrag_live` is running on PID **80036**, bound to `0.0.0.0:3031` and actively responding to `http://127.0.0.1:3031/api/health` and `http://10.0.0.3:3031/api/health` with `{"status":"healthy","uptime_secs":431,...}`.
   - **Gap**: Missing TPM CLI flags (`--tpm`, `--require-hardware-tpm`, `--tpm-pcr`), and missing REST endpoints (`/api/mesh/tpm/status`, `/api/mesh/topology`, `/api/mesh/pending-joins`, `/api/mesh/quarantined-peers`, `/api/mesh/bootstrap-peers`).

4. **`crates/quorum-ui`**:
   - `index.html:327-330`: Settings modal already contains `data-tab="mesh"` ("Mesh & Shared Folders").
   - `app.js:2200-2346`: Renders shared folders allowlist and basic peer alias input, but lacks:
     - TPM Silicon RoT status badge, manufacturer ID, and PCR banks readout.
     - Operator Pending Joins HITL approval queue.
     - Quarantined Peers management table.
     - Public WAN multiaddr copy card & DuckDNS auto-bootstrap settings.

---

## 2. Logic Chain

```
[Observation: OshoosiClaw has mature TBS/WMI TPM 2.0 inspection, PCR 0/7/16 tracking, EK X.509 parsing, GoldenBaseline, JoinGate, and Mesh UI]
        │
        ├── Step 1: Adapt TPM hardware inspection & attestation data structures into QuorumRAG:
        │           - Create `crates/quorum-coordinator/src/tpm.rs` (or `quorum-types/src/trust.rs`)
        │           - Integrate Windows TBS/WMI registry probe (< 1ms) + Linux sysfs + deterministic simulated fallback
        │           - Compute PCR composite digest Sha256(sum_i (idx_i || PCR_i)) for PCRs [0, 7, 16]
        │           - Parse X.509 EK DER bytes and detect silicon OEM vendors (Intel, AMD, Infineon, etc.)
        │
        ├── Step 2: Integrate TPM Hardware Handshake & Gating into `quorum-network` and `quorum-coordinator`:
        │           - In `quorum-network`: Extend `Identify` or Request-Response with attestation handshake
        │           - In `quorum-coordinator/src/acl.rs`: Update `PeerAclManager` with dynamic attestation-based tier gating:
        │             * Verified Hardware TPM 2.0 -> Authorized up to Tier2Confidential / Tier3Restricted
        │             * Software Fallback -> Capped at Tier1Internal
        │             * Attestation Pending / Failed -> Restrict to Tier0Public or Quarantine
        │
        ├── Step 3: Wire Daemon State, CLI Flags & REST Endpoints in `quorum-daemon`:
        │           - Add `--tpm`, `--require-hardware-tpm`, `--tpm-pcr` in `cli.rs`
        │           - Initialize TPM hardware inspection in `main.rs` and store in `DaemonState`
        │           - Register `/api/mesh/tpm/status`, `/api/mesh/topology`, `/api/mesh/pending-joins`, `/api/mesh/quarantined-peers`, `/api/mesh/bootstrap-peers`
        │
        ├── Step 4: Adapt Mesh Configuration UI in `quorum-ui`:
        │           - In `assets/index.html` & `assets/app.js` (`data-tab="mesh"`):
        │             * Render TPM Hardware Identity card with RoT badge, vendor, and PCR banks
        │             * Render Connected Mesh Nodes with live TPM attestation state badges
        │             * Render Pending Joins HITL approval queue (Approve, Deny, Approve All, Clear All)
        │             * Render Quarantined Peers management list (Release, False Positive)
        │             * Render Public WAN Multiaddr with copy button & DuckDNS auto-bootstrap domain
        │
        └── Step 5: Fix Build & Server Health (Requirement 5):
                    - Fix `crates/quorum-daemon/src/server.rs:115` by adding `bypass_cache: false`
                    - Enforce `$env:CARGO_INCREMENTAL = "0"` across builds to prevent rustc ICE
                    - Keep `quorumrag.exe` running on `0.0.0.0:3031` accessible via `10.0.0.3:3031` and `127.0.0.1:3031`
```

---

## 3. Caveats

1. **Physical TPM vs. VM/CI Hardware**: Machines running in Windows Subsystem for Linux (WSL2), Hyper-V VMs, or cloud GitHub Actions runners may lack physical TPM 2.0 access. The adaptation MUST strictly preserve OshoosiClaw's fail-safe fallback to deterministic pure-Rust simulated PCRs (`SIMULATED_PCR_16`, simulated Intel/AMD vendor) unless `--require-hardware-tpm` is explicitly configured.
2. **Elevated Permissions**: Querying TPM Base Services (`TBS.dll`) directly sometimes requires administrative privileges if accessing raw TPM command interfaces. OshoosiClaw's approach of reading the Windows Registry subkey `HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\TPM\WMI` provides non-elevated read access in `< 1ms` with 100% reliability.
3. **P2P Swarm vs. HTTP Port Allocation**: When `--port 3031` is specified for the HTTP web UI, `p2p_port` defaults to 3031. QuorumRAG's `find_next_open_p2p_port` automatically excludes `bound_http_port` and binds P2P to 3032. This behavior must be retained so both HTTP (3031) and libp2p (3032) run without conflict.
4. **Read-Only Explorer Constraint**: As Survey Explorer 3, all findings and proposed code changes are detailed in this report without modifying files outside our working folder.

---

## 4. Conclusion & Actionable Blueprint

### 4.1 Concrete Proposed Changes

#### A. Fix `crates/quorum-daemon/src/server.rs:115`
In `crates/quorum-daemon/src/server.rs:115-123`, add `bypass_cache: false`:
```rust
<<<< BEFORE (Line 115)
    let req = chat::ChatRequest {
        prompt,
        search_mode: None,
        model_family: family,
        top_k: Some(12),
        access_tier: Some(0),
        history: Vec::new(),
        query_id: None,
    };
==== AFTER
    let req = chat::ChatRequest {
        prompt,
        search_mode: None,
        model_family: family,
        top_k: Some(12),
        access_tier: Some(0),
        history: Vec::new(),
        query_id: None,
        bypass_cache: false,
    };
>>>>
```

#### B. Add TPM Types to `crates/quorum-coordinator/src/tpm.rs` (or `quorum-types/src/trust.rs`)
Integrate:
- `TpmOemVendor`: `Intel`, `Amd`, `Infineon`, `StMicro`, `Nuvoton`, `Nationz`, `Microchip`, `Unknown`.
- `TpmHardwareInspection`: available, tpm_version, manufacturer_id, manufacturer_str, vendor, nvram_ek_present, spec_version, interface_type.
- `TpmQuote`: pcr_digest, pcr_indices, quoted_digest, hardware_backed, raw_quote.
- `AttestationChallenge` & `AttestationResponse`.
- `inspect_tpm_hardware()`: Windows registry query (`SYSTEM\CurrentControlSet\Services\TPM\WMI`) + simulated fallback.
- `get_pcr_value(index)`: PCR 0, 7, 16.
- `compute_pcr_composite_digest()`.

#### C. Extend `crates/quorum-coordinator/src/acl.rs` for Attestation-Tier Gating
Add dynamic attestation-based tier evaluation to `PeerAclManager`:
```rust
impl PeerAclManager {
    pub fn update_peer_attestation(&mut self, peer_id: &str, is_hw_tpm: bool, attestation_passed: bool) {
        if !attestation_passed {
            self.register_peer_id(peer_id.to_string(), AccessTier::Tier0Public);
        } else if is_hw_tpm {
            // Hardware TPM 2.0 RoT grants access up to Tier 2 (Confidential)
            self.register_peer_id(peer_id.to_string(), AccessTier::Tier2Confidential);
        } else {
            // Software-anchored fallback grants access up to Tier 1 (Internal)
            self.register_peer_id(peer_id.to_string(), AccessTier::Tier1Internal);
        }
    }
}
```

#### D. Expose TPM & Mesh Management Endpoints in `crates/quorum-daemon`
In `crates/quorum-daemon/src/handlers/settings.rs` and `server.rs`:
- `GET /api/mesh/tpm/status` -> returns local TPM hardware inspection and PCR digests.
- `GET /api/mesh/topology` -> returns node graph with local node + remote peers with attestation states.
- `GET /api/mesh/pending-joins`, `POST /api/mesh/pending-joins/:peer_id/allow`, `POST /api/mesh/pending-joins/:peer_id/deny`, `POST /api/mesh/pending-joins/clear`.
- `GET /api/mesh/quarantined-peers`, `POST /api/mesh/quarantined-peers/:peer_id/release`, `POST /api/mesh/quarantined-peers/:peer_id/false-positive`.
- `GET /api/mesh/bootstrap-peers`, `POST /api/mesh/bootstrap-peers`.

#### E. Adapt Mesh Configuration UI in `crates/quorum-ui`
In `crates/quorum-ui/assets/app.js` under `renderSettingsTab('mesh')`:
1. **TPM Hardware Silicon RoT Card**:
   - Status Badge: `🟢 TPM 2.0 Hardware RoT Verified (Intel)` or `🟡 Software-Anchored Simulated TPM`.
   - PCR Readout: PCR 0 (Firmware), PCR 7 (Secure Boot), PCR 16 (Audit Log).
   - Silicon Manufacturer & Spec Version.
2. **WAN Peering & Remote Bootstrap Card**:
   - This Node's Public WAN Multiaddr with `Copy Address` button.
   - DuckDNS Zero-Touch Auto-Bootstrap Domain input with `Update Domain`.
   - Remote Bootstrap Peers table (`+ Add Peer`, `Remove`, `Save Config & Dial Now`).
3. **Connected Mesh Nodes Table**:
   - Peer ID, IP/Port, Attestation state, Latency, Packets Tx/Rx.
4. **Pending Joins Operator Approval Queue (HITL)**:
   - Peer requests with Discovered time, Reputation, Host OS, Patch status, Attestation state.
   - Actions: `✓ Approve / Allow`, `✕ Deny`, `✓ Approve All`, `Deny / Clear All`.
5. **Quarantined Peers Panel**:
   - Quarantined peers with reason and `Release` / `False Positive` actions.

---

## 5. Verification Method

To verify the implementation once applied:

1. **Compile Verification**:
   ```powershell
   $env:CARGO_INCREMENTAL = "0"
   cargo check --workspace
   cargo test -p quorum-network -p quorum-coordinator -p quorum-daemon -p quorum-ui
   ```
   *Pass Condition*: Zero compiler errors; `ChatRequest` initializes cleanly with `bypass_cache: false`.

2. **TPM Status Verification**:
   ```powershell
   Invoke-RestMethod -Uri "http://127.0.0.1:3031/api/mesh/tpm/status" -Method Get
   ```
   *Pass Condition*: Returns JSON with `available`, `tpm_version: "2.0"`, `vendor`, `pcr_values`, and `hardware_backed`.

3. **Mesh UI & Endpoints Verification**:
   ```powershell
   Invoke-RestMethod -Uri "http://127.0.0.1:3031/api/mesh/topology" -Method Get
   Invoke-RestMethod -Uri "http://127.0.0.1:3031/api/mesh/bootstrap-peers" -Method Get
   Invoke-RestMethod -Uri "http://127.0.0.1:3031/api/mesh/pending-joins" -Method Get
   ```
   *Pass Condition*: All return 200 OK with valid arrays/objects.

4. **Live Server Verification**:
   ```powershell
   Invoke-RestMethod -Uri "http://127.0.0.1:3031/api/health" -Method Get
   Invoke-RestMethod -Uri "http://10.0.0.3:3031/api/health" -Method Get
   ```
   *Pass Condition*: Returns `status: "healthy"` on both loopback (`127.0.0.1:3031`) and LAN (`10.0.0.3:3031`).
