use quorum_types::AccessTier;
use std::collections::HashMap;

/// Manages access-tier authorization policies for mesh peers.
#[derive(Debug, Clone)]
pub struct PeerAclManager {
    peer_pubkey_tiers: HashMap<[u8; 32], AccessTier>,
    peer_id_tiers: HashMap<String, AccessTier>,
    default_tier: AccessTier,
}

pub type AccessTierPolicy = PeerAclManager;

impl Default for PeerAclManager {
    fn default() -> Self {
        Self {
            peer_pubkey_tiers: HashMap::new(),
            peer_id_tiers: HashMap::new(),
            default_tier: AccessTier::Tier0Public,
        }
    }
}

impl PeerAclManager {
    pub fn new(default_tier: AccessTier) -> Self {
        Self {
            peer_pubkey_tiers: HashMap::new(),
            peer_id_tiers: HashMap::new(),
            default_tier,
        }
    }

    pub fn register_pubkey(&mut self, pubkey: [u8; 32], tier: AccessTier) {
        self.peer_pubkey_tiers.insert(pubkey, tier);
    }

    pub fn register_peer_id(&mut self, peer_id: String, tier: AccessTier) {
        self.peer_id_tiers.insert(peer_id, tier);
    }

    pub fn get_authorized_tier_for_pubkey(&self, pubkey: &[u8; 32]) -> AccessTier {
        self.peer_pubkey_tiers
            .get(pubkey)
            .copied()
            .unwrap_or(self.default_tier)
    }

    pub fn get_authorized_tier_for_peer_id(&self, peer_id: &str) -> AccessTier {
        self.peer_id_tiers
            .get(peer_id)
            .copied()
            .unwrap_or(self.default_tier)
    }

    /// Resolves the effective access tier by capping the requested tier at the peer's authorized upper bound.
    pub fn resolve_effective_tier(&self, requested_tier: u8, authorized_tier: AccessTier) -> u8 {
        requested_tier.min(authorized_tier.as_u8())
    }
}
