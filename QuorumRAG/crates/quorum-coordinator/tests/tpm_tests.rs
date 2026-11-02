use chrono::{Duration, Utc};
use quorum_coordinator::{
    compute_pcr_composite_digest, extend_simulated_pcr_16, inspect_tpm_hardware, simulated_pcr,
    AttestationTier, PeerAclManager, PeerAttestationRecord, TpmHardwareInspection, TpmOemVendor,
    PCR_APPLICATION_AUDIT, PCR_PLATFORM_FIRMWARE, PCR_SECURE_BOOT,
};
use quorum_types::AccessTier;
use std::collections::BTreeMap;
use std::time::Instant;

#[test]
fn test_simulated_pcr_deterministic_and_unique() {
    let pcr0 = simulated_pcr(PCR_PLATFORM_FIRMWARE);
    let pcr7 = simulated_pcr(PCR_SECURE_BOOT);
    let pcr16 = simulated_pcr(PCR_APPLICATION_AUDIT);

    assert_eq!(pcr0.len(), 64, "PCR 0 digest must be 64 hex characters (32 bytes)");
    assert_eq!(pcr7.len(), 64, "PCR 7 digest must be 64 hex characters (32 bytes)");
    assert_eq!(pcr16.len(), 64, "PCR 16 digest must be 64 hex characters (32 bytes)");

    assert_ne!(pcr0, pcr7, "PCR 0 and PCR 7 digests must not collide");
    assert_ne!(pcr7, pcr16, "PCR 7 and PCR 16 digests must not collide");
    assert_ne!(pcr0, pcr16, "PCR 0 and PCR 16 digests must not collide");

    // Repeated calls must be deterministic
    assert_eq!(pcr0, simulated_pcr(PCR_PLATFORM_FIRMWARE));
    assert_eq!(pcr7, simulated_pcr(PCR_SECURE_BOOT));
}

#[test]
fn test_extend_simulated_pcr_16() {
    let before = simulated_pcr(PCR_APPLICATION_AUDIT);
    extend_simulated_pcr_16(b"TEST_APPLICATION_EVENT_LOG_ENTRY");
    let after = simulated_pcr(PCR_APPLICATION_AUDIT);

    assert_eq!(after.len(), 64);
    assert_ne!(before, after, "Extending PCR 16 must irreversibly change its hash value");
}

#[test]
fn test_composite_digest_computation() {
    let mut map = BTreeMap::new();
    map.insert(0, simulated_pcr(0));
    map.insert(7, simulated_pcr(7));
    map.insert(16, simulated_pcr(16));

    let composite = compute_pcr_composite_digest(&map);
    assert_eq!(composite.len(), 64, "Composite digest must be 64 hex characters");

    // Reordering map insertions shouldn't change BTreeMap sorted order
    let mut map2 = BTreeMap::new();
    map2.insert(16, simulated_pcr(16));
    map2.insert(0, simulated_pcr(0));
    map2.insert(7, simulated_pcr(7));
    assert_eq!(compute_pcr_composite_digest(&map2), composite);
}

#[test]
fn test_inspect_tpm_hardware_returns_valid_struct_and_speed() {
    let start = Instant::now();
    let inspection: TpmHardwareInspection = inspect_tpm_hardware();
    let elapsed = start.elapsed();

    // Must be fast (< 50ms even on cold run, sub-millisecond on warm run)
    assert!(
        elapsed.as_millis() < 50,
        "TPM hardware inspection took too long: {:?}",
        elapsed
    );

    assert_eq!(inspection.tpm_version, "2.0");
    assert_eq!(inspection.pcr_values.len(), 3);
    assert!(inspection.pcr_values.contains_key(&0));
    assert!(inspection.pcr_values.contains_key(&7));
    assert!(inspection.pcr_values.contains_key(&16));
    assert_eq!(inspection.composite_digest.len(), 64);
}

#[test]
fn test_tpm_oem_vendor_mapping() {
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0x494E5443, ""),
        TpmOemVendor::Intel
    );
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0x414D4400, ""),
        TpmOemVendor::Amd
    );
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0x49465800, ""),
        TpmOemVendor::Infineon
    );
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0x53544D20, ""),
        TpmOemVendor::StMicro
    );
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0x4E544300, ""),
        TpmOemVendor::Nuvoton
    );
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0x4E545A00, ""),
        TpmOemVendor::Nationz
    );
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0x41544D4C, ""),
        TpmOemVendor::Microchip
    );
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0, "Intel Corporation"),
        TpmOemVendor::Intel
    );
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0, "Nuvoton Technology"),
        TpmOemVendor::Nuvoton
    );
    assert_eq!(
        TpmOemVendor::from_manufacturer_id_or_str(0, "Random Unregistered Silicon"),
        TpmOemVendor::Unknown
    );
}

#[test]
fn test_peer_acl_dynamic_attestation_gating() {
    let mut acl = PeerAclManager::new(AccessTier::Tier0Public);

    let peer_hw_ek = "peer_hw_ek";
    let peer_hw_tpm = "peer_hw_tpm";
    let peer_sim = "peer_sim";
    let peer_unver = "peer_unver";

    // All peers statically whitelisted for Tier 3 Restricted
    acl.register_peer_id(peer_hw_ek.to_string(), AccessTier::Tier3Restricted);
    acl.register_peer_id(peer_hw_tpm.to_string(), AccessTier::Tier3Restricted);
    acl.register_peer_id(peer_sim.to_string(), AccessTier::Tier3Restricted);
    acl.register_peer_id(peer_unver.to_string(), AccessTier::Tier3Restricted);

    let now = Utc::now();
    let expires = now + Duration::seconds(300);

    // Record attestation records
    acl.record_peer_attestation(PeerAttestationRecord {
        peer_id: peer_hw_ek.to_string(),
        pubkey: None,
        attestation_tier: AttestationTier::VerifiedHardwareEk,
        vendor: Some(TpmOemVendor::Amd),
        pcr_composite_digest: Some("c0".to_string()),
        hardware_backed: true,
        ek_fingerprint: Some("ek_fp".to_string()),
        verified_at: now,
        expires_at: expires,
    });

    acl.record_peer_attestation(PeerAttestationRecord {
        peer_id: peer_hw_tpm.to_string(),
        pubkey: None,
        attestation_tier: AttestationTier::VerifiedHardwareTpm,
        vendor: Some(TpmOemVendor::Intel),
        pcr_composite_digest: Some("c1".to_string()),
        hardware_backed: true,
        ek_fingerprint: None,
        verified_at: now,
        expires_at: expires,
    });

    acl.record_peer_attestation(PeerAttestationRecord {
        peer_id: peer_sim.to_string(),
        pubkey: None,
        attestation_tier: AttestationTier::Simulated,
        vendor: None,
        pcr_composite_digest: Some("c2".to_string()),
        hardware_backed: false,
        ek_fingerprint: None,
        verified_at: now,
        expires_at: expires,
    });

    // Check effective tiers bounded monotonically by attestation:
    // 1. Hardware EK -> Tier 3
    assert_eq!(
        acl.get_effective_authorized_tier_for_peer_id(peer_hw_ek),
        AccessTier::Tier3Restricted
    );

    // 2. Hardware TPM -> Tier 2
    assert_eq!(
        acl.get_effective_authorized_tier_for_peer_id(peer_hw_tpm),
        AccessTier::Tier2Confidential
    );

    // 3. Simulated -> Tier 1
    assert_eq!(
        acl.get_effective_authorized_tier_for_peer_id(peer_sim),
        AccessTier::Tier1Internal
    );

    // 4. Unverified -> Tier 0
    assert_eq!(
        acl.get_effective_authorized_tier_for_peer_id(peer_unver),
        AccessTier::Tier0Public
    );
}

#[test]
fn test_peer_acl_attestation_expiration_downgrade() {
    let mut acl = PeerAclManager::new(AccessTier::Tier0Public);
    let peer_id = "peer_expiring";

    acl.register_peer_id(peer_id.to_string(), AccessTier::Tier3Restricted);

    // Record attestation already expired in the past
    let now = Utc::now();
    let past = now - Duration::seconds(10);

    acl.record_peer_attestation(PeerAttestationRecord {
        peer_id: peer_id.to_string(),
        pubkey: None,
        attestation_tier: AttestationTier::VerifiedHardwareEk,
        vendor: Some(TpmOemVendor::Nuvoton),
        pcr_composite_digest: Some("c0".to_string()),
        hardware_backed: true,
        ek_fingerprint: Some("ek0".to_string()),
        verified_at: past - Duration::seconds(300),
        expires_at: past,
    });

    // Since attestation is expired, tier MUST downgrade to Tier 0 Public
    assert_eq!(acl.get_attestation_tier(peer_id), AttestationTier::Unverified);
    assert_eq!(
        acl.get_effective_authorized_tier_for_peer_id(peer_id),
        AccessTier::Tier0Public
    );
}
