use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use quorum_classifier::FileTypeClassifier;
use quorum_contract::EmbeddingGenerator;
use quorum_coordinator::{QueryCoordinator, SynthesisModelFamily};
use quorum_ingest::{DriftHealer, IngestConfig, ReindexWorker, TwoLevelDedupEngine};
use quorum_network::NetworkSwarmHandle;
use quorum_storage::StorageEngine;

/// Global shared application state for Axum daemon routes.
pub struct DaemonState {
    pub start_time: Instant,
    pub storage: Arc<StorageEngine>,
    pub coordinator: Arc<QueryCoordinator>,
    pub reindex_worker: Arc<ReindexWorker>,
    pub drift_healer: Arc<DriftHealer>,
    pub classifier: Arc<FileTypeClassifier>,
    pub embedder: Arc<dyn EmbeddingGenerator>,
    pub dedup: Arc<TwoLevelDedupEngine>,
    pub swarm_handle: Option<NetworkSwarmHandle>,
    pub ingest_config: IngestConfig,
    pub default_model_family: SynthesisModelFamily,
    pub port: u16,
    pub host: String,
    pub data_dir: PathBuf,
    pub shutdown_notify: Arc<tokio::sync::Notify>,
}
