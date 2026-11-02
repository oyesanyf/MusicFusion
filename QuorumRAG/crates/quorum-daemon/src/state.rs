use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::task::AbortHandle;

use quorum_classifier::FileTypeClassifier;
use quorum_contract::EmbeddingGenerator;
use quorum_coordinator::{QueryCoordinator, SynthesisModelFamily};
use quorum_ingest::{DriftHealer, IngestConfig, ReindexWorker, TwoLevelDedupEngine};
use quorum_network::NetworkSwarmHandle;
use quorum_storage::StorageEngine;

#[derive(Clone, Default)]
pub struct InFlightCancelToken(Arc<AtomicBool>);

impl InFlightCancelToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Clone)]
pub struct InFlightEntry {
    pub token: InFlightCancelToken,
    pub abort_handle: Arc<std::sync::Mutex<Option<AbortHandle>>>,
}

impl InFlightEntry {
    pub fn set_abort_handle(&self, handle: AbortHandle) {
        if let Ok(mut lock) = self.abort_handle.lock() {
            *lock = Some(handle);
        }
    }
}

pub struct InFlightGuard {
    pub query_id: String,
    pub registry: Arc<RwLock<HashMap<String, InFlightEntry>>>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        if let Ok(mut map) = self.registry.write() {
            map.remove(&self.query_id);
        }
    }
}

#[derive(Clone, Default)]
pub struct InFlightQueryRegistry {
    queries: Arc<RwLock<HashMap<String, InFlightEntry>>>,
}

impl InFlightQueryRegistry {
    pub fn new() -> Self {
        Self {
            queries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, query_id: String) -> (InFlightEntry, InFlightGuard) {
        let entry = InFlightEntry {
            token: InFlightCancelToken::new(),
            abort_handle: Arc::new(std::sync::Mutex::new(None)),
        };
        if let Ok(mut map) = self.queries.write() {
            map.insert(query_id.clone(), entry.clone());
        }
        let guard = InFlightGuard {
            query_id,
            registry: Arc::clone(&self.queries),
        };
        (entry, guard)
    }

    pub fn cancel(&self, query_id: &str) -> bool {
        if let Ok(map) = self.queries.read() {
            if let Some(entry) = map.get(query_id) {
                entry.token.cancel();
                if let Ok(mut lock) = entry.abort_handle.lock() {
                    if let Some(handle) = lock.take() {
                        handle.abort();
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn cancel_all(&self) -> usize {
        let mut count = 0;
        if let Ok(map) = self.queries.read() {
            for entry in map.values() {
                entry.token.cancel();
                if let Ok(mut lock) = entry.abort_handle.lock() {
                    if let Some(handle) = lock.take() {
                        handle.abort();
                    }
                }
                count += 1;
            }
        }
        count
    }
}

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
    pub in_flight_queries: Arc<InFlightQueryRegistry>,
}
