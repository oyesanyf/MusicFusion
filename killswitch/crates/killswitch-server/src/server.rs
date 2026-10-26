//! # Server Lifecycle, Configuration & Loopback Binding (Port 3030)
//!
//! Enforces loopback and control port 3030 preservation during full network containment.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::info;

use killswitch_adjudication::ZeroFalsePositiveAdjudicator;
use killswitch_core::killchain::HostContainmentManager;
use killswitch_core::service::{default_service_manager, NativeServiceManager};
use killswitch_telemetry::correlator::EventCorrelator;
use killswitch_telemetry::octopus::ArmRingRegistry;
use killswitch_telemetry::otel_lineage::EndpointLineageEngine;
use killswitch_telemetry::phaselock::PhaseLockManager;
use killswitch_types::graph::OrchestratorMode;

use crate::error::ServerError;
use crate::orchestrator_bridge::{InMemoryOrchestratorBridge, OrchestratorBridge};
use crate::routes::build_router;

/// Configuration for local Axum HTTP control server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_body_bytes: usize,
    pub request_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3030,
            max_body_bytes: 2 * 1024 * 1024, // 2 MB
            request_timeout_secs: 30,
        }
    }
}

/// Shared application state injected across all HTTP handlers.
#[derive(Clone)]
pub struct ServerState {
    pub containment_manager: Arc<HostContainmentManager>,
    pub adjudicator: Arc<ZeroFalsePositiveAdjudicator>,
    pub lineage_engine: Arc<EndpointLineageEngine>,
    pub arm_registry: Arc<ArmRingRegistry>,
    pub event_correlator: Arc<EventCorrelator>,
    pub phaselock_manager: Arc<PhaseLockManager>,
    pub service_manager: Arc<dyn NativeServiceManager>,
    pub orchestrator_bridge: Arc<dyn OrchestratorBridge>,
    pub start_time: Instant,
    pub active_mode: Arc<RwLock<OrchestratorMode>>,
    pub config: ServerConfig,
}

pub type AppState = ServerState;

impl ServerState {
    pub fn new(
        containment_manager: Arc<HostContainmentManager>,
        adjudicator: Arc<ZeroFalsePositiveAdjudicator>,
        lineage_engine: Arc<EndpointLineageEngine>,
        orchestrator_bridge: Arc<dyn OrchestratorBridge>,
        service_manager: Arc<dyn NativeServiceManager>,
    ) -> Self {
        Self {
            containment_manager,
            adjudicator,
            lineage_engine,
            arm_registry: Arc::new(ArmRingRegistry::new(b"killswitch_master_octopus_seed_00")),
            event_correlator: Arc::new(EventCorrelator::default()),
            phaselock_manager: Arc::new(PhaseLockManager::default()),
            service_manager,
            orchestrator_bridge,
            start_time: Instant::now(),
            active_mode: Arc::new(RwLock::new(OrchestratorMode::Autonomous)),
            config: ServerConfig::default(),
        }
    }

    pub fn with_defaults(containment_manager: Arc<HostContainmentManager>) -> Self {
        let adjudicator = Arc::new(ZeroFalsePositiveAdjudicator::new());
        let lineage_engine = Arc::new(EndpointLineageEngine::new_in_memory());
        let orchestrator_bridge = Arc::new(InMemoryOrchestratorBridge::default());
        let service_manager = default_service_manager();
        Self::new(containment_manager, adjudicator, lineage_engine, orchestrator_bridge, service_manager)
    }
}

/// HTTP Server wrapper.
pub struct KillSwitchServer {
    state: Arc<ServerState>,
}

impl KillSwitchServer {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }

    /// Binds to configured IP and port (default 127.0.0.1:3030) and starts the Axum server.
    pub async fn run_with_shutdown(
        &self,
        shutdown_signal: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), ServerError> {
        let addr: SocketAddr = format!("{}:{}", self.state.config.host, self.state.config.port)
            .parse()
            .map_err(|e| ServerError::Validation(format!("Invalid socket address: {}", e)))?;

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                ServerError::PortInUse(self.state.config.port)
            } else {
                ServerError::Internal(format!("Failed to bind to {}: {}", addr, e))
            }
        })?;

        info!("🛡️ [KILLSWITCH-SERVER] Local Axum HTTP API listening on http://{}", addr);

        let app = build_router(Arc::clone(&self.state));

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| ServerError::Internal(format!("Server error: {}", e)))?;

        info!("🛡️ [KILLSWITCH-SERVER] Server stopped cleanly.");
        Ok(())
    }
}

/// Convenience runner function used by Unified Loader.
pub async fn run_server(config: ServerConfig, mut state: ServerState) -> Result<(), ServerError> {
    state.config = config;
    let server = KillSwitchServer::new(Arc::new(state));
    server.run_with_shutdown(async {
        // Wait for shutdown or let caller manage
        tokio::signal::ctrl_c().await.ok();
    }).await
}
