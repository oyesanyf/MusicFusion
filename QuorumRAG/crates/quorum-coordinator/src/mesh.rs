use crate::error::CoordinatorError;
use quorum_network::NetworkSwarmHandle;
use quorum_types::{CandidateChunkDto, FederatedQueryRequest, FederatedQueryResponse};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Clean interface abstraction separating mesh querying mechanisms.
#[async_trait::async_trait]
pub trait MeshQueryDispatcher: Send + Sync {
    async fn query_peers(
        &self,
        request: &FederatedQueryRequest,
        timeout: Duration,
    ) -> Result<Vec<FederatedQueryResponse>, CoordinatorError>;

    fn peer_count(&self) -> usize;
}

/// Simulated remote peer node with network latency and authorized access tier for testing.
#[derive(Debug, Clone)]
pub struct MockRemotePeer {
    pub peer_id: String,
    pub latency: Duration,
    pub max_access_tier: u8,
    pub candidate_chunks: Vec<CandidateChunkDto>,
}

impl MockRemotePeer {
    pub async fn handle_query(&self, requested_tier: u8) -> Vec<CandidateChunkDto> {
        tokio::time::sleep(self.latency).await;
        let effective_tier = requested_tier.min(self.max_access_tier);
        self.candidate_chunks
            .iter()
            .filter(|c| c.access_tier <= effective_tier)
            .cloned()
            .collect()
    }
}

/// In-memory mock dispatcher holding multiple mock peers for deterministic testing.
#[derive(Debug, Default, Clone)]
pub struct MockMeshDispatcher {
    pub peers: Arc<RwLock<Vec<MockRemotePeer>>>,
}

impl MockMeshDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn add_peer(&self, peer: MockRemotePeer) {
        let mut p = self.peers.write().await;
        p.push(peer);
    }
}

#[async_trait::async_trait]
impl MeshQueryDispatcher for MockMeshDispatcher {
    async fn query_peers(
        &self,
        request: &FederatedQueryRequest,
        timeout: Duration,
    ) -> Result<Vec<FederatedQueryResponse>, CoordinatorError> {
        let peers = self.peers.read().await.clone();
        if peers.is_empty() {
            return Ok(Vec::new());
        }

        let mut tasks = Vec::new();
        for peer in peers {
            let req = request.clone();
            tasks.push(tokio::spawn(async move {
                let chunks = peer.handle_query(0).await;
                FederatedQueryResponse {
                    query_id: req.query_id,
                    responder_peer_id: peer.peer_id,
                    chunks,
                }
            }));
        }

        let mut responses = Vec::new();
        let _ = tokio::time::timeout(timeout, async {
            for task in tasks {
                if let Ok(resp) = task.await {
                    responses.push(resp);
                }
            }
        })
        .await;

        Ok(responses)
    }

    fn peer_count(&self) -> usize {
        // Approximate count from blocking read or 0
        0
    }
}

/// Production mesh dispatcher bridging to the `NetworkSwarmHandle`.
#[derive(Clone)]
pub struct SwarmMeshDispatcher {
    pub handle: NetworkSwarmHandle,
}

impl SwarmMeshDispatcher {
    pub fn new(handle: NetworkSwarmHandle) -> Self {
        Self { handle }
    }
}

#[async_trait::async_trait]
impl MeshQueryDispatcher for SwarmMeshDispatcher {
    async fn query_peers(
        &self,
        request: &FederatedQueryRequest,
        timeout: Duration,
    ) -> Result<Vec<FederatedQueryResponse>, CoordinatorError> {
        let peers = self
            .handle
            .connected_peers()
            .await
            .map_err(|e| CoordinatorError::Network(e.to_string()))?;

        if peers.is_empty() {
            return Ok(Vec::new());
        }

        let mut tasks = Vec::new();
        for peer in peers {
            let handle = self.handle.clone();
            let req = request.clone();
            tasks.push(tokio::spawn(async move {
                handle.send_query(peer, req).await
            }));
        }

        let mut responses = Vec::new();
        let _ = tokio::time::timeout(timeout, async {
            for task in tasks {
                if let Ok(Ok(resp)) = task.await {
                    responses.push(resp);
                }
            }
        })
        .await;

        Ok(responses)
    }

    fn peer_count(&self) -> usize {
        1
    }
}

/// Dispatches only to local storage without any remote peers.
#[derive(Debug, Default, Clone)]
pub struct LocalOnlyDispatcher;

#[async_trait::async_trait]
impl MeshQueryDispatcher for LocalOnlyDispatcher {
    async fn query_peers(
        &self,
        _request: &FederatedQueryRequest,
        _timeout: Duration,
    ) -> Result<Vec<FederatedQueryResponse>, CoordinatorError> {
        Ok(Vec::new())
    }

    fn peer_count(&self) -> usize {
        0
    }
}
