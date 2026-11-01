use clap::Parser;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};

pub mod browser;
pub mod cli;
pub mod handlers;
pub mod server;
pub mod shutdown;
pub mod state;

use cli::{Cli, Commands};
use state::DaemonState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "quorumrag=info,quorum_daemon=info,quorum_coordinator=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Reindex {
            path,
            full,
            force_reembed,
            batch_size,
        }) => {
            run_standalone_reindex(cli.clone(), path.clone(), *full, *force_reembed, *batch_size).await?;
        }
        Some(Commands::FixDrift { dry_run, drift_type }) => {
            run_standalone_fix_drift(cli.clone(), *dry_run, drift_type.clone()).await?;
        }
        None => {
            run_daemon_and_ui(cli).await?;
        }
    }

    Ok(())
}

/// Starts the QuorumRAG Daemon and embedded Web UI.
pub async fn run_daemon_and_ui(cli: Cli) -> anyhow::Result<()> {
    info!(
        port = cli.port,
        host = %cli.host,
        data_dir = ?cli.data_dir,
        no_browser = cli.no_browser,
        "Starting QuorumRAG Daemon & ChatGPT Web Server..."
    );

    // 1. Ensure storage directories exist
    let lance_path = cli.data_dir.join("lance");
    let tantivy_path = cli.data_dir.join("tantivy");
    tokio::fs::create_dir_all(&lance_path).await?;
    tokio::fs::create_dir_all(&tantivy_path).await?;

    let storage_config = quorum_storage::StorageConfig::new(cli.data_dir.clone());

    let storage = Arc::new(quorum_storage::StorageEngine::open(storage_config).await?);
    let classifier = Arc::new(quorum_classifier::FileTypeClassifier::new());
    let embedder: Arc<dyn quorum_contract::EmbeddingGenerator> = Arc::new(
        quorum_contract::DeterministicOfflineEngine::new()
    );
    let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());

    // 2. Ingest Config & Workers
    let mut ingest_config = quorum_ingest::IngestConfig::default();
    if let Some(ref watch_path) = cli.watch_dir {
        ingest_config.watch_paths.push(watch_path.clone());
    }

    let reindex_worker = Arc::new(quorum_ingest::ReindexWorker::new(
        ingest_config.clone(),
        Arc::clone(&storage),
        Arc::clone(&classifier),
        Arc::clone(&embedder),
        Arc::clone(&dedup),
    ));

    let drift_healer = Arc::new(quorum_ingest::DriftHealer::new(
        ingest_config.clone(),
        Arc::clone(&storage),
        Arc::clone(&classifier),
        Arc::clone(&embedder),
        Arc::clone(&dedup),
    ));

    // 3. Network Swarm & Query Coordinator
    let (mesh_dispatcher, swarm_handle): (Arc<dyn quorum_coordinator::MeshQueryDispatcher>, Option<quorum_network::NetworkSwarmHandle>) = if cli.local_only {
        info!("Running in local-only mode (P2P swarm networking disabled).");
        (Arc::new(quorum_coordinator::LocalOnlyDispatcher), None)
    } else {
        let net_config = quorum_network::NetworkConfig {
            tcp_listen_addrs: vec![format!("/ip4/{}/tcp/{}", cli.host, cli.p2p_port).parse()?],
            quic_listen_addrs: vec![format!("/ip4/{}/udp/{}/quic-v1", cli.host, cli.p2p_port).parse()?],
            ..Default::default()
        };
        match quorum_network::start_network_node(net_config, None).await {
            Ok((_join, handle)) => {
                let disp = Arc::new(quorum_coordinator::SwarmMeshDispatcher {
                    handle: handle.clone(),
                });
                (disp, Some(handle))
            }
            Err(e) => {
                error!("Failed to start libp2p network node: {}. Falling back to local-only.", e);
                (Arc::new(quorum_coordinator::LocalOnlyDispatcher), None)
            }
        }
    };

    let coordinator_engine = Arc::new(quorum_coordinator::ScatterGatherEngine::new(
        Arc::clone(&storage) as Arc<dyn quorum_coordinator::StorageSearcher>,
        mesh_dispatcher,
        Arc::new(tokio::sync::RwLock::new(quorum_coordinator::PeerAclManager::default())),
        quorum_coordinator::CoordinatorConfig::default(),
        [0u8; 32],
        "local_daemon".to_string(),
    ));

    let coordinator = Arc::new(quorum_coordinator::QueryCoordinator::new(
        coordinator_engine,
        quorum_coordinator::CoordinatorConfig::default(),
    ));

    let default_model_family = match cli.model_family.as_str() {
        "Glm4Agentic" => quorum_coordinator::SynthesisModelFamily::Glm4Agentic,
        "Qwen25Code" => quorum_coordinator::SynthesisModelFamily::Qwen25Code,
        "GemmaProse" => quorum_coordinator::SynthesisModelFamily::GemmaProse,
        _ => quorum_coordinator::SynthesisModelFamily::AutoFusion,
    };

    let shutdown_notify = Arc::new(tokio::sync::Notify::new());

    let app_state = Arc::new(DaemonState {
        start_time: Instant::now(),
        storage,
        coordinator,
        reindex_worker,
        drift_healer,
        classifier,
        embedder,
        dedup,
        swarm_handle,
        ingest_config,
        default_model_family,
        port: cli.port,
        host: cli.host.clone(),
        data_dir: cli.data_dir.clone(),
        shutdown_notify: Arc::clone(&shutdown_notify),
    });

    let app = server::build_router(Arc::clone(&app_state));

    let bind_addr = format!("{}:{}", cli.host, cli.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("QuorumRAG Web UI listening on http://{}", bind_addr);

    // 4. Auto-launch browser if not disabled
    if !cli.no_browser {
        let url = format!("http://{}:{}", cli.host, cli.port);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            browser::launch_browser(&url);
        });
    }

    // 5. Axum 0.7 server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::shutdown_signal(shutdown_notify))
        .await?;

    // 6. Execute graceful shutdown cleanup
    shutdown::execute_graceful_shutdown(app_state).await;

    Ok(())
}

/// Executes standalone reindex CLI command.
pub async fn run_standalone_reindex(
    cli: Cli,
    path: Option<std::path::PathBuf>,
    full: bool,
    force_reembed: bool,
    batch_size: usize,
) -> anyhow::Result<()> {
    info!("Starting standalone reindex CLI worker...");
    let storage_config = quorum_storage::StorageConfig::new(cli.data_dir.clone());
    let storage = Arc::new(quorum_storage::StorageEngine::open(storage_config).await?);
    let classifier = Arc::new(quorum_classifier::FileTypeClassifier::new());
    let embedder: Arc<dyn quorum_contract::EmbeddingGenerator> = Arc::new(
        quorum_contract::DeterministicOfflineEngine::new()
    );
    let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());

    let target_paths = path.map(|p| vec![p]).unwrap_or_else(|| {
        cli.watch_dir
            .map(|w| vec![w])
            .unwrap_or_else(|| vec![std::env::current_dir().unwrap_or_default()])
    });

    let options = quorum_ingest::ReindexOptions {
        target_paths,
        full_rebuild: full,
        force_reembed,
        batch_size,
        num_threads: 0,
        dry_run: false,
    };

    let worker = quorum_ingest::ReindexWorker::new(
        quorum_ingest::IngestConfig::default(),
        storage,
        classifier,
        embedder,
        dedup,
    );

    let task_id = worker.start_reindex(options).await?;
    info!(task_id = %task_id, "Reindex task started. Streaming progress...");

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        if let Some(prog) = worker.get_progress().await {
            info!(
                phase = ?prog.phase,
                files_scanned = prog.files_scanned,
                files_indexed = prog.files_indexed,
                chunks_new = prog.chunks_newly_embedded,
                chunks_reused = prog.chunks_reused_from_cache,
                "Progress"
            );
            if matches!(
                prog.phase,
                quorum_ingest::ReindexPhase::Completed
                    | quorum_ingest::ReindexPhase::Failed
                    | quorum_ingest::ReindexPhase::Cancelled
            ) {
                break;
            }
        }
    }

    info!("Standalone reindex finished.");
    Ok(())
}

/// Executes standalone drift healing CLI command.
pub async fn run_standalone_fix_drift(
    cli: Cli,
    dry_run: bool,
    drift_type: Option<String>,
) -> anyhow::Result<()> {
    info!("Starting standalone drift healing CLI worker...");
    let storage_config = quorum_storage::StorageConfig::new(cli.data_dir.clone());
    let storage = Arc::new(quorum_storage::StorageEngine::open(storage_config).await?);
    let classifier = Arc::new(quorum_classifier::FileTypeClassifier::new());
    let embedder: Arc<dyn quorum_contract::EmbeddingGenerator> = Arc::new(
        quorum_contract::DeterministicOfflineEngine::new()
    );
    let dedup = Arc::new(quorum_ingest::TwoLevelDedupEngine::new());

    let healer = quorum_ingest::DriftHealer::new(
        quorum_ingest::IngestConfig::default(),
        storage,
        classifier,
        embedder,
        dedup,
    );

    let mut targets = Vec::new();
    if let Some(t) = drift_type {
        match t.to_lowercase().as_str() {
            "merkle_state" => targets.push(quorum_ingest::DriftType::MerkleState),
            "hybrid_index_desync" => targets.push(quorum_ingest::DriftType::HybridIndexDesync),
            "vector_contract" => targets.push(quorum_ingest::DriftType::VectorContract),
            "acl_policy" => targets.push(quorum_ingest::DriftType::AclPolicy),
            "storage_compaction" => targets.push(quorum_ingest::DriftType::StorageCompaction),
            _ => {}
        }
    }

    let report = healer
        .fix_all_drifts(quorum_ingest::DriftFixOptions {
            target_drifts: targets,
            dry_run,
            access_tier_rules: Vec::new(),
        })
        .await?;

    println!("\n=== QuorumRAG 5-Drift Remediation Report ===");
    println!("Execution Duration: {}ms", report.duration_ms);
    println!("Total Anomalies Detected: {}", report.total_anomalies_detected);
    println!("Total Anomalies Healed:   {}", report.total_anomalies_healed);
    println!("--------------------------------------------");
    println!("1. Merkle State (FS <-> DB):");
    println!("   - Orphaned DB Files Purged: {}", report.orphaned_db_files_purged);
    println!("   - Missing FS Files Indexed: {}", report.missing_fs_files_indexed);
    println!("2. Hybrid Index Desync (LanceDB <-> Tantivy):");
    println!("   - Lance chunks missing in Tantivy: {}", report.lance_chunks_missing_in_tantivy);
    println!("   - Tantivy chunks missing in Lance: {}", report.tantivy_chunks_missing_in_lance);
    println!("   - Parity Restored: {}", report.hybrid_parity_restored);
    println!("3. Vector Contract Drift:");
    println!("   - Drifted chunks re-embedded: {}", report.contract_drifted_chunks_reembedded);
    println!("4. ACL Policy Drift:");
    println!("   - Chunks updated: {}", report.acl_drifted_chunks_updated);
    println!("5. Storage Compaction:");
    println!("   - Fragments consolidated: {}", report.fragments_consolidated);
    println!("   - Tantivy segments merged: {}", report.tantivy_segments_merged);
    println!("============================================\n");

    Ok(())
}
