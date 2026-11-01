use std::sync::Arc;
use tracing::{error, info, warn};
use crate::state::DaemonState;

/// Installs OS signal listeners (Ctrl+C / SIGINT on all platforms, SIGTERM on unix)
/// and listens for internal shutdown notification.
pub async fn shutdown_signal(notify: Arc<tokio::sync::Notify>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C / SIGINT. Initiating graceful shutdown...");
        },
        _ = terminate => {
            info!("Received SIGTERM. Initiating graceful shutdown...");
        },
        _ = notify.notified() => {
            info!("Received internal shutdown notification (/api/shutdown). Initiating graceful shutdown...");
        },
    }
}

/// Executes orderly cleanup and database flush when shutdown signal is triggered.
pub async fn execute_graceful_shutdown(state: Arc<DaemonState>) {
    info!("Executing QuorumRAG graceful shutdown sequence...");

    // 1. Cooperative cancellation of active reindexer
    let cancelled = state.reindex_worker.cancel_reindex();
    if cancelled {
        info!("Active reindex worker cancelled successfully.");
    }

    // 2. Shut down libp2p swarm actor
    if let Some(ref swarm) = state.swarm_handle {
        match swarm.shutdown().await {
            Ok(_) => info!("Libp2p network swarm actor shut down cleanly."),
            Err(e) => warn!("Libp2p network swarm shutdown error: {}", e),
        }
    }

    // 3. Compact and commit LanceDB and Tantivy indexes
    info!("Flushing and compacting LanceDB & Tantivy storage indexes...");
    match state.storage.compact().await {
        Ok(stats) => {
            info!(
                "Storage indexes cleanly committed: {} fragments removed, {} added, Tantivy segments merged: {}",
                stats.fragments_removed, stats.fragments_added, stats.tantivy_segments_merged
            );
        }
        Err(e) => {
            error!("Error flushing storage during shutdown: {}", e);
        }
    }

    info!("QuorumRAG shutdown sequence complete. Goodbye!");
}
