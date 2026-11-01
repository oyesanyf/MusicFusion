pub mod actor;
pub mod behaviour;
pub mod config;
pub mod error;
pub mod handle;
pub mod handshake;
pub mod transport;

pub use actor::NetworkSwarmActor;
pub use config::NetworkConfig;
pub use error::NetworkError;
pub use handle::{NetworkCommand, NetworkEvent, NetworkSwarmHandle};
pub use handshake::{
    extract_contract_hash, make_agent_version, make_protocol_version, verify_peer_contract,
};
pub use libp2p::Multiaddr;

use quorum_types::{FederatedQueryRequest, FederatedQueryResponse};
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait InboundQueryHandler: Send + Sync {
    async fn handle_query(&self, req: FederatedQueryRequest) -> FederatedQueryResponse;
}

#[derive(Debug, Default, Clone)]
pub struct DefaultInboundQueryHandler;

#[async_trait::async_trait]
impl InboundQueryHandler for DefaultInboundQueryHandler {
    async fn handle_query(&self, req: FederatedQueryRequest) -> FederatedQueryResponse {
        FederatedQueryResponse {
            query_id: req.query_id,
            responder_peer_id: String::new(),
            chunks: Vec::new(),
        }
    }
}

pub async fn start_network_node(
    config: NetworkConfig,
    inbound_handler: Option<Arc<dyn InboundQueryHandler>>,
) -> Result<(tokio::task::JoinHandle<()>, NetworkSwarmHandle), NetworkError> {
    let keypair = match config.ed25519_seed {
        Some(seed) => {
            let secret = libp2p::identity::ed25519::SecretKey::try_from_bytes(seed)
                .map_err(|e| NetworkError::KeypairError(e.to_string()))?;
            let ed_kp = libp2p::identity::ed25519::Keypair::from(secret);
            libp2p::identity::Keypair::from(ed_kp)
        }
        None => libp2p::identity::Keypair::generate_ed25519(),
    };

    let local_peer_id = keypair.public().to_peer_id();
    let mut swarm = transport::build_swarm(&config, keypair)?;

    for addr in &config.tcp_listen_addrs {
        swarm
            .listen_on(addr.clone())
            .map_err(|e| NetworkError::TransportError(e.to_string()))?;
    }
    for addr in &config.quic_listen_addrs {
        swarm
            .listen_on(addr.clone())
            .map_err(|e| NetworkError::TransportError(e.to_string()))?;
    }

    for peer in &config.bootstrap_peers {
        let _ = swarm.dial(peer.clone());
    }

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(128);
    let (evt_tx, _) = tokio::sync::broadcast::channel(256);
    let listen_addrs = Arc::new(tokio::sync::RwLock::new(Vec::new()));

    let handle = NetworkSwarmHandle {
        local_peer_id,
        command_tx: cmd_tx.clone(),
        event_tx: evt_tx.clone(),
        listen_addrs: Arc::clone(&listen_addrs),
    };

    let handler = inbound_handler.unwrap_or_else(|| Arc::new(DefaultInboundQueryHandler));

    let actor = NetworkSwarmActor {
        swarm,
        command_rx: cmd_rx,
        command_tx: cmd_tx,
        event_tx: evt_tx,
        listen_addrs,
        inbound_handler: handler,
        pending_requests: HashMap::new(),
        config,
    };

    let join_handle = tokio::spawn(async move {
        actor.run().await;
    });

    // Wait briefly (up to 200ms) for initial listen addresses to register
    for _ in 0..20 {
        if let Ok(addrs) = handle.listen_addresses().await {
            if !addrs.is_empty() {
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    Ok((join_handle, handle))
}
