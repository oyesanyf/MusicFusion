# Progress: Explorer M13-1 (TPM 2.0 Hardware Attestation Adaptation)

- Status: Completed investigation & full technical specification
- Last visited: 2026-11-02T17:11:30Z

## Tasks
- [x] Record DISPATCH.md and initialize BRIEFING.md
- [x] Read ORIGINAL_REQUEST.md and SCOPE.md
- [x] Inspect OshoosiClaw TPM 2.0 implementations:
  - `d:\harfile\OshoosiClaw\crates\osoosi-audit\src\tpm.rs`
  - `d:\harfile\OshoosiClaw\crates\osoosi-trust\src\tbs.rs`
  - `d:\harfile\OshoosiClaw\crates\osoosi-types\src\trust.rs`
- [x] Verify live host TPM registry keys:
  - `HKLM\SYSTEM\CurrentControlSet\Services\TPM\WMI` verified
  - `TaskManufacturerId`: 1314145024 (0x4E544300, Nuvoton 'NTC')
  - `TaskFirmwareVersion`: "1.3.2.8"
  - `FirmwareVersionAtLastProvision`: "1.3.2.8"
  - EK Cert store: `HKLM\SYSTEM\CurrentControlSet\Services\TPM\WMI\Endorsement\EKCertStore\Certificates`
- [x] Verify `Cargo.lock` dependencies:
  - `sha2 0.11.1`, `hex 0.4.3`, `x509-parser 0.18.1`, `windows-sys 0.61.2`, `blake3 1.8.7`
- [x] Synthesize specification for `crates/quorum-coordinator/src/tpm.rs`:
  - Fast Win32 Registry WMI inspection (< 1ms)
  - Linux sysfs `/sys/class/tpm/tpm0` inspection & simulated fallback
  - `compute_pcr_composite_digest` over PCR 0, 7, 16
  - `TpmOemVendor` detection & X.509 Endorsement Key (EK) parsing
- [x] Synthesize specification for dynamic attestation-tier gating in `crates/quorum-coordinator/src/acl.rs`:
  - Verified TPM 2.0 -> Tier 2/3
  - Simulated -> Tier 1
  - Unverified -> Tier 0
- [x] Write comprehensive `handoff.md` with 5 sections
- [x] Send completion message to parent
