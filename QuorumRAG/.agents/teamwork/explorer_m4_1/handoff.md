# Handoff Report: rust-libp2p 0.57.1 Swarm, Dual Transports & Network Controller (`crates/quorum-network`)

**Author**: `explorer_m4_1`  
**Milestone**: Milestone 4 (rust-libp2p Swarm, Coordinator, Fusion & GLM Selection)  
**Parent Conversation ID**: `e13403c5-b0e6-48a4-a1ec-a1f096f9f53e`  
**Date**: 2026-10-31  
**Status**: Complete Investigation & Implementation Blueprint  

---

## 1. Observation

Direct inspection of the workspace, rust-libp2p 0.57.1 crates, reference implementations in `d:\harfile`, and interface contracts establishes the following concrete evidence chains:

### 1.1 Dependency Verification in Local Cargo Registry (`libp2p 0.57.1`)
Inspection of `C:\Users\oyesanyf\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\libp2p-0.57.1\Cargo.toml` confirms available features and sub-crates:
- **`libp2p` 0.57.1** features:
  - `quic = ["dep:libp2p-quic"]` (line 122)
  - `tcp = ["dep:libp2p-tcp"]` (line 136)
  - `noise = ["dep:libp2p-noise"]` (line 114)
  - `yamux = ["dep:libp2p-yamux"]` (line 90)
  - `mdns = ["dep:libp2p-mdns"]` (line 108)
  - `kad = ["dep:libp2p-kad"]` (line 103)
  - `gossipsub = ["dep:libp2p-gossipsub"]` (line 93)
  - `request-response = ["dep:libp2p-request-response"]` (line 128)
  - `cbor = ["libp2p-request-response?/cbor"]` (line 44)
  - `identify = ["dep:libp2p-identify"]` (line 98)
  - `tokio = [...]` (line 138)
  - `macros = ["libp2p-swarm/macros"]` (line 107)
  - `ed25519 = ["libp2p-identity/ed25519"]` (line 51)
  - `dns = ["dep:libp2p-dns"]` (line 49)

### 1.2 Dual-Transport SwarmBuilder API in `libp2p 0.57.1`
Inspection of `C:\Users\oyesanyf\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\libp2p-0.57.1\src\builder.rs` (lines 40–64 and lines 174–187) confirms that TCP and QUIC can be cleanly chained on `SwarmBuilder`:
```rust
let swarm = SwarmBuilder::with_existing_identity(keypair)
    .with_tokio()
    .with_tcp(
        libp2p::tcp::Config::default().nodelay(true),
        libp2p::noise::Config::new,
        libp2p::yamux::Config::default,
    )?
    .with_quic()
    .with_dns()?
    .with_behaviour(|key| ...)?
    .with_swarm_config(|cfg| cfg)
    .build();
```
Both transports operate concurrently on the same Swarm:
- TCP Multiaddr format: `/ip4/0.0.0.0/tcp/{port}`
- QUIC Multiaddr format: `/ip4/0.0.0.0/udp/{port}/quic-v1`

### 1.3 CBOR Request-Response Behavior (`libp2p-request-response 0.30.0`)
Inspection of `C:\Users\oyesanyf\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\libp2p-request-response-0.30.0\src\cbor.rs` (lines 48–56) confirms:
```rust
pub type Behaviour<Req, Resp> = crate::Behaviour<codec::Codec<Req, Resp>>;
```
Instantiation pattern:
```rust
let protocols = [(
    libp2p::StreamProtocol::new("/quorum-rag/query/1.0.0"),
    libp2p::request_response::ProtocolSupport::Full,
)];
let rr_config = libp2p::request_response::Config::default()
    .with_request_timeout(Duration::from_millis(400));
let rr_behaviour = libp2p::request_response::cbor::Behaviour::<FederatedQueryRequest, FederatedQueryResponse>::new(
    protocols,
    rr_config,
);
```
Serialization is handled automatically via `cbor4ii::serde` for any type implementing `serde::Serialize + serde::de::DeserializeOwned`.

### 1.4 Existing Domain DTOs (`crates/quorum-types/src/lib.rs`)
Inspection of `crates/quorum-types/src/lib.rs` confirms:
1. `FederatedQueryRequest` (lines 358–542) implements `Debug, Clone, PartialEq, Serialize, Deserialize`:
   - `query_id: Uuid`
   - `sender_peer_id: String`
   - `sender_pubkey: [u8; 32]`
   - `query_text: String`
   - `embedding: Vec<f32>` (768-dim Gemma vector)
   - `top_k: usize`
   - `timestamp: i64`
   - `signature: [u8; 64]` (custom serde via `serde_sig64`)
   - Methods: `canonical_signing_bytes()`, `canonical_digest()`, `is_fresh(max_drift_secs)`.
2. `FederatedQueryResponse` (lines 544–555) implements `Debug, Clone, PartialEq, Serialize, Deserialize`:
   - `query_id: Uuid`
   - `responder_peer_id: String`
   - `chunks: Vec<CandidateChunkDto>`
3. `CandidateChunkDto` (lines 305–356) implements `Debug, Clone, PartialEq, Serialize, Deserialize, Zeroize`:
   - `chunk_id`, `file_path`, `line_start`, `line_end`, `content`, `score`, `access_tier`
   - `to_volatile_remote_chunk(peer_id)` and `into_volatile_remote_chunk(peer_id)`.
4. `VolatileRemoteChunk` (lines 557–616): Implements `Zeroize` and `ZeroizeOnDrop`.

### 1.5 Contract Handshake Verification (`crates/quorum-contract/src/lib.rs`)
Inspection of `crates/quorum-contract/src/lib.rs` confirms:
- `EmbeddingContract::canonical_gemma_300m()` (line 80)
- `derive_contract_hash(&self) -> [u8; 32]` (line 98): deterministic BLAKE3 digest over dimension (768), normalization ("L2"), metric ("Cosine"), pooling ("MeanPooling"), sequence length (512).
- `derive_contract_hash_hex(&self) -> String` (line 116)
- `verify_contract_handshake(local: &[u8; 32], remote: &[u8; 32]) -> Result<(), ContractError>` (line 130) using constant-time comparison `subtle::ConstantTimeEq`.
- `verify_contract_handshake_hex(local_hex: &str, remote_hex: &str) -> Result<(), ContractError>` (line 145).

### 1.6 Peer Keypair & Identity Derivation (`libp2p-identity 0.2.13`)
Inspection of `libp2p-identity-0.2.13\src\ed25519.rs` (lines 30–78) confirms:
- An Ed25519 keypair can be created directly from secret seed bytes `[u8; 32]`:
  ```rust
  let secret = libp2p::identity::ed25519::SecretKey::try_from_bytes(seed_bytes)?;
  let ed_kp = libp2p::identity::ed25519::Keypair::from(secret);
  let keypair = libp2p::identity::Keypair::from(ed_kp);
  let peer_id = keypair.public().to_peer_id();
  ```
- Public key bytes can be extracted for Ed25519 verification.

### 1.7 Absence of Arrow / LanceDB Types
Grep search confirmed `quorum-network` has zero dependencies on `arrow`, `arrow-array`, `arrow-schema`, `arrow-data`, or `lancedb`. The network layer operates solely on `quorum-types` domain DTOs.

---

## 2. Logic Chain

### 2.1 Swarm Architecture & Actor Pattern
The `libp2p::Swarm` type requires exclusive `&mut self` access to poll events and advance internal state machines. It cannot be placed directly behind an `Arc<Mutex<Swarm>>` because locking a mutex across async await points in user-facing query handlers causes deadlocks or severe thread contention.

Therefore, the actor architecture is necessary and optimal:
1. **`NetworkSwarmActor`**: Runs in a dedicated background `tokio::spawn` task, exclusively owning `Swarm<QuorumNetworkBehaviour>`.
2. **`NetworkSwarmHandle`**: Lightweight, cloneable (`Clone + Send + Sync`) controller struct held by `quorum-coordinator`, `quorum-daemon`, and API services.
3. **Communication**:
   - Commands (outbound queries, dialing, peer queries, shutdown) travel over a bounded `tokio::sync::mpsc` channel.
   - Request-response queries use internal `tokio::sync::oneshot` channels to return `FederatedQueryResponse` directly to the awaiting caller task.
   - Swarm lifecycle events (peer discovered, peer expired, peer verified, query received) are broadcast via a `tokio::sync::broadcast` channel for non-blocking multi-subscriber consumption.

### 2.2 Dual-Transport Resilience (QUIC + TCP/Noise/Yamux)
- **QUIC (`libp2p-quic`)**: Uses UDP, offers 0-RTT/1-RTT connection setup, built-in TLS 1.3 cryptographic security, and eliminates head-of-line blocking across multiplexed substreams.
- **TCP + Noise + Yamux (`libp2p-tcp`, `libp2p-noise`, `libp2p-yamux`)**: Reliable fallback transport when UDP traffic is filtered by corporate or host firewalls.
- Both transports listen on local network interfaces. Dials can be initiated using either `/ip4/.../udp/.../quic-v1` or `/ip4/.../tcp/...`.

### 2.3 Composite NetworkBehaviour Layout
Using `#[derive(NetworkBehaviour)]`:
```rust
#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct QuorumNetworkBehaviour {
    /// Zero-config LAN peer discovery
    pub mdns: libp2p::mdns::tokio::Behaviour,
    /// Distributed peer routing & DHT
    pub kademlia: libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore>,
    /// Broadcast query propagation with cryptographic signatures
    pub gossipsub: libp2p::gossipsub::Behaviour,
    /// Point-to-point chunk querying with CBOR binary serialization
    pub request_response: libp2p::request_response::cbor::Behaviour<FederatedQueryRequest, FederatedQueryResponse>,
    /// Protocol & contract hash handshake validation
    pub identify: libp2p::identify::Behaviour,
}
```

### 2.4 Multi-Tier Gemma Contract Handshake Verification
In a federated retrieval network, vector calculations and RRF scoring are invalid if peers use different embedding contracts. Verification is implemented at two distinct boundary layers:

1. **Layer 1: Identify Protocol Handshake Gating**:
   - During protocol handshake, nodes exchange `identify` metadata.
   - The local node embeds the canonical BLAKE3 contract hash in the protocol version:
     `protocol_version: "/quorum-rag/1.0.0/contract-<contract_hash_hex>"`
     and agent version: `"quorumrag/0.1.0/<contract_hash_hex>"`.
   - When `identify::Event::Received { peer_id, info }` is emitted:
     The actor parses the remote contract hash hex from `info.protocol_version` or `info.agent_version`.
     Calls `verify_contract_handshake_hex(&local_hash_hex, &remote_hash_hex)`.
     If verification fails:
     - Disconnects peer immediately: `swarm.disconnect_peer_id(peer_id)`.
     - Removes peer from Kademlia routing table: `swarm.behaviour_mut().kademlia.remove_peer(&peer_id)`.
     - Removes peer from GossipSub: `swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id)`.
     - Broadcasts `NetworkEvent::PeerRejected { peer_id, reason: "Gemma contract mismatch".into() }`.
2. **Layer 2: Inbound Query Validation**:
   - Inbound `FederatedQueryRequest` packets are validated for:
     - Embedding dimension == 768.
     - Timestamp freshness within 60s (`request.is_fresh(60)`).
     - Ed25519 signature validity over canonical payload.

### 2.5 Inbound & Outbound Query Lifecycle
1. **Outbound Point-to-Point Query**:
   - Caller invokes `handle.send_query(target_peer, request)`.
   - Generates a `oneshot::channel()`.
   - Sends `NetworkCommand::SendQuery { peer_id, request, reply_tx }` to actor.
   - Actor invokes `swarm.behaviour_mut().request_response.send_request(&peer_id, request)`.
   - Registers `pending_requests.insert(outbound_req_id, reply_tx)`.
   - When response arrives, actor fulfills `reply_tx.send(Ok(response))`.
2. **Inbound Query Delegation**:
   - Remote peer sends request via CBOR substream.
   - Actor receives `request_response::Event::Message { message: Message::Request { request, channel, .. }, peer, .. }`.
   - Actor validates signature, freshness, and dimension.
   - Delegates query execution to registered `InboundQueryHandler` trait implementation (backed by `quorum-coordinator` and `quorum-storage`).
   - Obtains candidate chunks, constructs `FederatedQueryResponse`, and sends reply via `swarm.behaviour_mut().request_response.send_response(channel, response)`.

---

## 3. Caveats

The following real-world networking constraints and edge cases must be strictly handled:

1. **Windows Defender Firewall & UDP QUIC Binding**:
   - Windows Firewall may silently drop inbound unsolicited UDP packets for QUIC on newly assigned dynamic ports unless an application exemption exists.
   - *Mitigation*: Dual transport ensures that if QUIC fails to connect, TCP with Noise encryption immediately establishes connectivity without breaking the query flow.
2. **mDNS Multi-Interface / Virtual Adapter Binding on Windows**:
   - Windows systems with Hyper-V (WSL2), Docker Desktop, Tailscale, or multiple network adapters can cause mDNS multicast packets (`224.0.0.251:5353`) to route through a disconnected or internal-only virtual adapter.
   - *Mitigation*:
     - Support explicit bootstrap peer multiaddresses in `NetworkConfig`.
     - Allow optional configuration of `mdns::Config`.
     - Expose manual `handle.dial(multiaddr)` to allow direct peer connections when mDNS is inactive.
3. **Dynamic Port Allocation (Port 0) in Multi-Node Tests**:
   - Hardcoding ports (e.g. 8000, 9000) causes `WSAEADDRINUSE` in concurrent test runs.
   - *Mitigation*: Both TCP and QUIC listen on port 0 (`/ip4/127.0.0.1/tcp/0` and `/ip4/127.0.0.1/udp/0/quic-v1`). The swarm retrieves actual bound ports via `SwarmEvent::NewListenAddr` and exposes them via `handle.listen_addresses()`.
4. **CBOR Message Size Bounds**:
   - `cbor::Codec` defaults: max request = 1 MiB, max response = 10 MiB.
   - A normal `FederatedQueryResponse` with 20 chunks of 2 KB each is ~40 KB, well within limits. However, oversized malicious payloads should be guarded against memory exhaustion.
5. **GossipSub Heartbeat & Mesh Churn**:
   - Default GossipSub heartbeat of 1 second is suitable for fast local convergence. For wide-area networks or low-power nodes, 5–15 seconds reduces background control messages.

---

## 4. Conclusion & Actionable File Blueprint

`crates/quorum-network` must be implemented with the following modular file structure:

```
crates/quorum-network/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public exports (NetworkSwarmHandle, NetworkConfig, NetworkEvent, InboundQueryHandler, etc.)
    ├── config.rs           # NetworkConfig (listening multiaddresses, bootnodes, topic, ed25519 seed)
    ├── error.rs            # NetworkError (enum with thiserror)
    ├── behaviour.rs        # QuorumNetworkBehaviour (composite NetworkBehaviour)
    ├── transport.rs        # Dual-transport SwarmBuilder constructor (QUIC + TCP/Noise/Yamux)
    ├── handshake.rs        # Gemma contract hash derivation & Identify verification
    ├── handle.rs           # Thread-safe NetworkSwarmHandle (commands, oneshot replies, broadcast events)
    └── actor.rs            # Background Swarm event loop & protocol dispatch
```

### 4.1 Crate Manifest: `crates/quorum-network/Cargo.toml`
```toml
[package]
name = "quorum-network"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "rust-libp2p 0.57.1 Swarm, dual transports (QUIC/TCP), mDNS, Kademlia, GossipSub, and CBOR Request-Response"

[dependencies]
# Pure domain types & contracts (ZERO Arrow/Lance leakage)
quorum-types = { workspace = true }
quorum-contract = { workspace = true }

# rust-libp2p 0.57.1 with exact features
libp2p = { version = "0.57.1", features = [
    "tokio",
    "quic",
    "tcp",
    "noise",
    "yamux",
    "mdns",
    "kad",
    "gossipsub",
    "request-response",
    "cbor",
    "identify",
    "macros",
    "ed25519",
    "dns",
] }

# Async & Networking
tokio = { workspace = true }
futures = { workspace = true }

# Cryptography & Security
ed25519-dalek = { workspace = true }
blake3 = { workspace = true }
subtle = { workspace = true }

# Serialization & Core
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }

# Error Handling & Tracing
thiserror = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

### 4.2 Error Handling: `crates/quorum-network/src/error.rs`
```rust
use libp2p::Multiaddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Failed to build libp2p Swarm: {0}")]
    SwarmBuildError(String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Dial error to {addr}: {source}")]
    DialError {
        addr: Multiaddr,
        source: libp2p::swarm::DialError,
    },

    #[error("Query request failed or timed out: {0}")]
    RequestFailed(String),

    #[error("Gossipsub publish error: {0}")]
    GossipsubError(String),

    #[error("Contract hash mismatch with peer {peer_id}: local={local_hex}, remote={remote_hex}")]
    ContractMismatch {
        peer_id: String,
        local_hex: String,
        remote_hex: String,
    },

    #[error("Peer not found or not connected: {0}")]
    PeerNotConnected(String),

    #[error("Network controller channel closed")]
    ChannelClosed,

    #[error("Keypair or identity derivation error: {0}")]
    KeypairError(String),

    #[error("Query signature or validation error: {0}")]
    ValidationError(String),
}
```

### 4.3 Network Configuration: `crates/quorum-network/src/config.rs`
```rust
use libp2p::Multiaddr;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Ed25519 seed for persistent identity (if None, random generated)
    pub ed25519_seed: Option<[u8; 32]>,
    /// TCP listen multiaddresses (e.g. `/ip4/0.0.0.0/tcp/0`)
    pub tcp_listen_addrs: Vec<Multiaddr>,
    /// QUIC listen multiaddresses (e.g. `/ip4/0.0.0.0/udp/0/quic-v1`)
    pub quic_listen_addrs: Vec<Multiaddr>,
    /// Explicit bootstrap peers to dial on startup
    pub bootstrap_peers: Vec<Multiaddr>,
    /// GossipSub query broadcast topic name
    pub gossip_topic: String,
    /// Enable mDNS LAN discovery
    pub enable_mdns: bool,
    /// Request-Response timeout (default: 400ms)
    pub request_timeout: Duration,
    /// Gemma embedding contract hash hex for handshake verification
    pub contract_hash_hex: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        let contract = quorum_contract::EmbeddingContract::canonical_gemma_300m();
        Self {
            ed25519_seed: None,
            tcp_listen_addrs: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
            quic_listen_addrs: vec!["/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap()],
            bootstrap_peers: Vec::new(),
            gossip_topic: "quorum-rag-query-broadcast".to_string(),
            enable_mdns: true,
            request_timeout: Duration::from_millis(400),
            contract_hash_hex: contract.derive_contract_hash_hex(),
        }
    }
}
```

### 4.4 Composite Behaviour: `crates/quorum-network/src/behaviour.rs`
```rust
use libp2p::swarm::NetworkBehaviour;
use quorum_types::{FederatedQueryRequest, FederatedQueryResponse};

#[derive(NetworkBehaviour)]
pub struct QuorumNetworkBehaviour {
    pub mdns: libp2p::mdns::tokio::Behaviour,
    pub kademlia: libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore>,
    pub gossipsub: libp2p::gossipsub::Behaviour,
    pub request_response: libp2p::request_response::cbor::Behaviour<FederatedQueryRequest, FederatedQueryResponse>,
    pub identify: libp2p::identify::Behaviour,
}
```

### 4.5 Dual Transports & Swarm Construction: `crates/quorum-network/src/transport.rs`
```rust
use crate::behaviour::QuorumNetworkBehaviour;
use crate::config::NetworkConfig;
use crate::error::NetworkError;
use libp2p::{
    gossipsub, identify, kad, mdns, noise, request_response, tcp, yamux,
    PeerId, StreamProtocol, Swarm, SwarmBuilder,
};
use std::time::Duration;

pub fn build_swarm(
    config: &NetworkConfig,
    keypair: libp2p::identity::Keypair,
) -> Result<Swarm<QuorumNetworkBehaviour>, NetworkError> {
    let local_peer_id = keypair.public().to_peer_id();

    let contract_hash_hex = config.contract_hash_hex.clone();
    let protocol_version = format!("/quorum-rag/1.0.0/contract-{}", contract_hash_hex);
    let agent_version = format!("quorumrag/0.1.0/{}", contract_hash_hex);

    let swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default().nodelay(true),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|e| NetworkError::TransportError(e.to_string()))?
        .with_quic()
        .with_dns()
        .map_err(|e| NetworkError::TransportError(e.to_string()))?
        .with_behaviour(|key| {
            // 1. mDNS
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)
                .map_err(|e| NetworkError::SwarmBuildError(e.to_string()))?;

            // 2. Kademlia
            let mut kademlia = kad::Behaviour::new(
                local_peer_id,
                kad::store::MemoryStore::new(local_peer_id),
            );
            kademlia.set_mode(Some(kad::Mode::Server));

            // 3. GossipSub
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(1))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(|msg| {
                    let digest = blake3::hash(&msg.data);
                    gossipsub::MessageId::from(digest.as_bytes().to_vec())
                })
                .build()
                .map_err(|e| NetworkError::SwarmBuildError(e.to_string()))?;

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )
            .map_err(|e| NetworkError::SwarmBuildError(e.to_string()))?;

            // 4. Request-Response (CBOR)
            let protocols = [(
                StreamProtocol::new("/quorum-rag/query/1.0.0"),
                request_response::ProtocolSupport::Full,
            )];
            let rr_config = request_response::Config::default()
                .with_request_timeout(config.request_timeout);
            let request_response = request_response::cbor::Behaviour::new(protocols, rr_config);

            // 5. Identify
            let id_config = identify::Config::new(protocol_version, key.public())
                .with_agent_version(agent_version);
            let identify = identify::Behaviour::new(id_config);

            Ok(QuorumNetworkBehaviour {
                mdns,
                kademlia,
                gossipsub,
                request_response,
                identify,
            })
        })
        .map_err(|e| NetworkError::SwarmBuildError(e.to_string()))?
        .with_swarm_config(|cfg| {
            cfg.with_idle_connection_timeout(Duration::from_secs(60))
        })
        .build();

    Ok(swarm)
}
```

### 4.6 Thread-Safe Controller Handle: `crates/quorum-network/src/handle.rs`
```rust
use crate::error::NetworkError;
use libp2p::{Multiaddr, PeerId};
use quorum_types::{FederatedQueryRequest, FederatedQueryResponse};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    PeerDiscovered { peer_id: PeerId, address: Multiaddr },
    PeerExpired { peer_id: PeerId },
    PeerConnected { peer_id: PeerId },
    PeerDisconnected { peer_id: PeerId },
    PeerHandshakeVerified { peer_id: PeerId },
    PeerRejected { peer_id: PeerId, reason: String },
    GossipQueryReceived { request: FederatedQueryRequest },
}

pub(crate) enum NetworkCommand {
    Dial {
        addr: Multiaddr,
        reply: oneshot::Sender<Result<(), NetworkError>>,
    },
    SendQuery {
        peer_id: PeerId,
        request: FederatedQueryRequest,
        reply: oneshot::Sender<Result<FederatedQueryResponse, NetworkError>>,
    },
    BroadcastQuery {
        request: FederatedQueryRequest,
        reply: oneshot::Sender<Result<(), NetworkError>>,
    },
    GetConnectedPeers {
        reply: oneshot::Sender<Vec<PeerId>>,
    },
    GetListenAddresses {
        reply: oneshot::Sender<Vec<Multiaddr>>,
    },
    Shutdown,
}

#[derive(Clone)]
pub struct NetworkSwarmHandle {
    pub(crate) local_peer_id: PeerId,
    pub(crate) command_tx: mpsc::Sender<NetworkCommand>,
    pub(crate) event_tx: broadcast::Sender<NetworkEvent>,
    pub(crate) listen_addrs: Arc<RwLock<Vec<Multiaddr>>>,
}

impl NetworkSwarmHandle {
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    pub async fn dial(&self, addr: Multiaddr) -> Result<(), NetworkError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(NetworkCommand::Dial { addr, reply: tx })
            .await
            .map_err(|_| NetworkError::ChannelClosed)?;
        rx.await.map_err(|_| NetworkError::ChannelClosed)?
    }

    pub async fn send_query(
        &self,
        peer_id: PeerId,
        request: FederatedQueryRequest,
    ) -> Result<FederatedQueryResponse, NetworkError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(NetworkCommand::SendQuery { peer_id, request, reply: tx })
            .await
            .map_err(|_| NetworkError::ChannelClosed)?;
        rx.await.map_err(|_| NetworkError::ChannelClosed)?
    }

    pub async fn broadcast_query(
        &self,
        request: FederatedQueryRequest,
    ) -> Result<(), NetworkError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(NetworkCommand::BroadcastQuery { request, reply: tx })
            .await
            .map_err(|_| NetworkError::ChannelClosed)?;
        rx.await.map_err(|_| NetworkError::ChannelClosed)?
    }

    pub async fn connected_peers(&self) -> Result<Vec<PeerId>, NetworkError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(NetworkCommand::GetConnectedPeers { reply: tx })
            .await
            .map_err(|_| NetworkError::ChannelClosed)?;
        Ok(rx.await.map_err(|_| NetworkError::ChannelClosed)?)
    }

    pub async fn listen_addresses(&self) -> Result<Vec<Multiaddr>, NetworkError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(NetworkCommand::GetListenAddresses { reply: tx })
            .await
            .map_err(|_| NetworkError::ChannelClosed)?;
        Ok(rx.await.map_err(|_| NetworkError::ChannelClosed)?)
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<NetworkEvent> {
        self.event_tx.subscribe()
    }
}
```

### 4.7 Inbound Query Handler Trait: `crates/quorum-network/src/lib.rs`
```rust
#[async_trait::async_trait]
pub trait InboundQueryHandler: Send + Sync {
    async fn handle_query(&self, req: FederatedQueryRequest) -> FederatedQueryResponse;
}
```

---

## 5. Verification Method

To independently verify the `quorum-network` implementation:

### 5.1 Compilation & Type Boundary Verification
```powershell
# 1. Check workspace members compile cleanly with 0 warnings
cargo check -p quorum-network

# 2. Assert zero Arrow/Lance types in dependency tree
cargo tree -p quorum-network | Select-String "arrow|lance"
# Invariant: Output must be empty (0 occurrences).
```

### 5.2 Unit & Multi-Node Integration Test Suite (`tests/network_tests.rs`)
Implement an end-to-end integration test verifying two nodes communicating over localhost:
```rust
#[tokio::test]
async fn test_dual_node_swarm_cbor_query_and_contract_verification() {
    let mut config_a = NetworkConfig::default();
    config_a.tcp_listen_addrs = vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()];
    config_a.quic_listen_addrs = vec!["/ip4/127.0.0.1/udp/0/quic-v1".parse().unwrap()];

    let mut config_b = NetworkConfig::default();
    config_b.tcp_listen_addrs = vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()];
    config_b.quic_listen_addrs = vec!["/ip4/127.0.0.1/udp/0/quic-v1".parse().unwrap()];

    let (node_a, handle_a) = start_network_node(config_a, None).await.unwrap();
    let (node_b, handle_b) = start_network_node(config_b, None).await.unwrap();

    let addrs_b = handle_b.listen_addresses().await.unwrap();
    assert!(!addrs_b.is_empty(), "Node B must bind to listening addresses");

    // Node A dials Node B via TCP or QUIC
    handle_a.dial(addrs_b[0].clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(150)).await;

    // Node A dispatches a point-to-point query to Node B
    let request = FederatedQueryRequest {
        query_id: uuid::Uuid::new_v4(),
        sender_peer_id: handle_a.local_peer_id().to_string(),
        sender_pubkey: [0u8; 32],
        query_text: "What is the consensus contract?".to_string(),
        embedding: vec![1.0; 768],
        top_k: 5,
        timestamp: chrono::Utc::now().timestamp(),
        signature: [0u8; 64],
    };

    let response = handle_a.send_query(handle_b.local_peer_id(), request).await.unwrap();
    assert_eq!(response.responder_peer_id, handle_b.local_peer_id().to_string());
}
```

### 5.3 Invalidation Conditions
The design is invalidated if:
1. `crates/quorum-network` imports any Arrow or LanceDB types.
2. `NetworkSwarmHandle` blocks Tokio async tasks during query send.
3. Contract hash verification fails to disconnect peers whose contract hash diverges.
4. Windows port binding to port 0 fails or conflicts.
