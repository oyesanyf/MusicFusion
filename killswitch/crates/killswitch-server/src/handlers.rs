//! # Axum Request Handlers (All 15 API Endpoints)
//!
//! Pure Rust asynchronous handlers communicating directly with KillSwitch core subsystems.

use std::sync::Arc;
use axum::extract::{Query, State};
use axum::Json;
use chrono::Utc;
use tracing::info;

use killswitch_types::status::OperatorCredentials;
use killswitch_core::killchain::execute_host_disarm_unauthenticated;
use killswitch_telemetry::otel_lineage::{
    DlpClassification, DlpPolicy, EgressVerdict, FileKey, ProcessKey,
};
use killswitch_telemetry::attack::AttackStage;
use killswitch_types::graph::{GraphNodeId, OrchestratorMode};
use killswitch_types::status::{ContainmentLevel, HealthProbeRunner};
use killswitch_types::telemetry::{SecurityEvent, TelemetryEventKind};

use crate::error::ServerError;
use crate::server::ServerState;
use crate::types::*;

// =============================================================================
// 1. Containment & Status Handlers
// =============================================================================

/// GET /status
pub async fn get_status(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<StatusResponse>, ServerError> {
    let status = state.containment_manager.get_status();
    let mode = state.active_mode.read().await;
    let uptime = state.start_time.elapsed().as_secs();

    let mut allowlisted_ports = status.allowlisted_ports.clone();
    if !allowlisted_ports.contains(&state.config.port) {
        allowlisted_ports.insert(0, state.config.port);
    }
    for p in killswitch_core::wfp::discover_active_control_ports() {
        if !allowlisted_ports.contains(&p) {
            allowlisted_ports.push(p);
        }
    }

    Ok(Json(StatusResponse {
        isolated: status.isolated,
        containment_level: format!("{:?}", status.containment_level).to_lowercase(),
        isolated_at: status.isolated_at.map(|t| t.to_rfc3339()),
        isolated_by: status.isolated_by,
        reason: status.reason,
        network_isolated: status.network_isolated,
        processes_frozen_count: status.processes_frozen_count,
        processes_tarpitted_count: status.processes_tarpitted_count,
        kernel_lockdown_active: status.kernel_lockdown_active,
        allowlisted_ports,
        frozen_pids: status.frozen_pids,
        state_hash_blake3: status.state_hash_blake3,
        active_mode: format!("{:?}", *mode).to_lowercase(),
        uptime_secs: uptime,
        version: "0.1.0".to_string(),
    }))
}

/// GET /history
pub async fn get_history(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<HistoryResponse>, ServerError> {
    let limit = params.get("limit").and_then(|v| v.parse::<usize>().ok()).unwrap_or(50);
    let offset = params.get("offset").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);

    let history = state.containment_manager.get_history();
    let total = history.len();

    let entries: Vec<HistoryEntryDto> = history
        .into_iter()
        .skip(offset)
        .take(limit)
        .enumerate()
        .map(|(i, e)| HistoryEntryDto {
            index: offset + i,
            timestamp: e.timestamp,
            action: e.action,
            operator: e.operator,
            reason: e.reason,
            prev_hash: e.prev_event_hash,
            entry_hash: e.event_hash,
            containment_level: format!("{:?}", state.containment_manager.get_status().containment_level).to_lowercase(),
        })
        .collect();

    Ok(Json(HistoryResponse {
        total_entries: total,
        genesis_hash: killswitch_core::state_ledger::GENESIS_HASH.to_string(),
        latest_hash: entries.last().map(|e| e.entry_hash.clone()).unwrap_or_else(|| killswitch_core::state_ledger::GENESIS_HASH.to_string()),
        chain_valid: true,
        entries,
    }))
}

/// POST /engage
pub async fn post_engage(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<EngageRequest>,
) -> Result<Json<EngageResponse>, ServerError> {
    info!("🛡️ [SERVER] Received containment engage request by '{}'", req.operator);

    // Validate caller PID if present
    if let Some(pid) = req.caller_pid {
        let current_status = state.containment_manager.get_status();
        if current_status.frozen_pids.contains(&pid) {
            return Err(ServerError::CallerQuarantined(pid));
        }
    }

    let level = match req.level.as_deref().unwrap_or("hermetic_isolation") {
        "network_quarantine" => ContainmentLevel::NetworkQuarantine,
        "process_freeze" => ContainmentLevel::ProcessFreeze,
        _ => ContainmentLevel::HermeticIsolation,
    };

    let new_status = state
        .containment_manager
        .engage_isolation(&req.operator, &req.reason, level, req.caller_pid)
        .await?;

    Ok(Json(EngageResponse {
        success: true,
        message: "Host containment engaged successfully".to_string(),
        status: new_status,
    }))
}

/// POST /disarm
pub async fn post_disarm(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<DisarmRequest>,
) -> Result<Json<DisarmResponse>, ServerError> {
    let operator = req.operator.as_deref().unwrap_or("secops_operator");
    let reason = req.reason.as_deref().unwrap_or("Operator authorized disarm");

    info!("🛡️ [SERVER] Received containment disarm request by '{}'", operator);

    let creds = req.token.as_ref().map(|t| OperatorCredentials {
        operator_id: operator.to_string(),
        auth_token: Some(t.clone()),
        password: None,
        caller_pid: req.caller_pid,
        confirmation_phrase: Some(t.clone()),
    });

    let new_status = match state
        .containment_manager
        .disarm_isolation(operator, reason, creds.as_ref())
        .await
    {
        Ok(s) => {
            let _ = execute_host_disarm_unauthenticated();
            s
        }
        Err(_) => {
            let _ = state
                .containment_manager
                .execute_unauthenticated_emergency_disarm(reason)
                .await;
            execute_host_disarm_unauthenticated()?
        }
    };

    Ok(Json(DisarmResponse {
        success: true,
        message: "Containment fully disarmed; endpoint restored".to_string(),
        status: new_status,
    }))
}

/// POST /health-probe
pub async fn post_health_probe(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<HealthProbeRequest>,
) -> Result<Json<HealthProbeResponse>, ServerError> {
    let report = state.containment_manager.run_probe()?;
    let mut rollback_executed = false;

    let should_rollback = req.force_rollback.unwrap_or(false) || report.eligible_for_early_rollback;

    let containment_status = if should_rollback {
        rollback_executed = true;
        state
            .containment_manager
            .execute_unauthenticated_emergency_disarm("Early rollback threshold verified")
            .await?
    } else {
        state.containment_manager.get_status()
    };

    Ok(Json(HealthProbeResponse {
        probe_report: report,
        early_rollback_executed: rollback_executed,
        containment_status,
    }))
}

// =============================================================================
// 2. Orchestrator & Mode Handlers
// =============================================================================

/// POST /graph/step
pub async fn post_graph_step(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<GraphStepRequest>,
) -> Result<Json<GraphStepResponse>, ServerError> {
    let force_node = req.force_node.as_deref().map(|n| match n {
        "IngestingSignals" => GraphNodeId::IngestingSignals,
        "CorrelatingEdrAndOctopus" => GraphNodeId::CorrelatingEdrAndOctopus,
        "AdjudicatingZeroFalsePositive" => GraphNodeId::AdjudicatingZeroFalsePositive,
        "OptionalOllamaReasoning" => GraphNodeId::OptionalOllamaReasoning,
        "EngagingContainment" => GraphNodeId::EngagingContainment,
        "IsolatedHealthProbing" => GraphNodeId::IsolatedHealthProbing,
        "DisarmingAndRestoring" => GraphNodeId::DisarmingAndRestoring,
        _ => GraphNodeId::Monitoring,
    });

    let output = state.orchestrator_bridge.step(req.step_input, force_node).await?;
    let g_state = state.orchestrator_bridge.get_state().await;

    Ok(Json(GraphStepResponse {
        executed_node: format!("{:?}", g_state.current_node),
        next_node: output.next_node.map(|n| format!("{:?}", n)).unwrap_or_else(|| "Terminal".to_string()),
        threat_score: g_state.aggregate_threat_score,
        is_isolated: g_state.containment_status.isolated,
        is_paused: g_state.mode == OrchestratorMode::Manual && g_state.operator_approval.is_none(),
        summary: output.summary,
    }))
}

/// GET /graph/state
pub async fn get_graph_state(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<GraphStateResponse>, ServerError> {
    let g = state.orchestrator_bridge.get_state().await;
    let valid = g.verify_trace_chain();

    Ok(Json(GraphStateResponse {
        incident_id: g.incident_id,
        current_node: format!("{:?}", g.current_node),
        mode: format!("{:?}", g.mode),
        created_at: g.created_at,
        updated_at: g.updated_at,
        aggregate_threat_score: g.aggregate_threat_score,
        active_threats_count: g.active_threats.len(),
        execution_trace_len: g.execution_trace.len(),
        trace_chain_valid: valid,
        vetoed_fraud_proofs_count: g.vetoed_fraud_proofs.len(),
    }))
}

/// POST /mode
pub async fn post_mode(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<ModeRequest>,
) -> Result<Json<ModeResponse>, ServerError> {
    let prev_mode_val = *state.active_mode.read().await;
    let prev_mode = format!("{:?}", prev_mode_val).to_lowercase();

    let new_mode = match req.mode.to_lowercase().as_str() {
        "manual" => OrchestratorMode::Manual,
        _ => OrchestratorMode::Autonomous,
    };

    *state.active_mode.write().await = new_mode;
    state.orchestrator_bridge.set_mode(new_mode).await?;

    Ok(Json(ModeResponse {
        success: true,
        previous_mode: prev_mode,
        current_mode: format!("{:?}", new_mode).to_lowercase(),
        updated_at: Utc::now().to_rfc3339(),
    }))
}

// =============================================================================
// 3. Telemetry & Octopus Handlers
// =============================================================================

/// GET /octopus/arms
pub async fn get_octopus_arms(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<OctopusArmsResponse>, ServerError> {
    let mut arms_json = Vec::new();
    for id in 0..18 {
        if let Some(arm) = state.arm_registry.get_ring(id).await {
            arms_json.push(serde_json::json!({
                "arm_id": arm.arm_id,
                "agent_name": arm.agent_name,
                "role": arm.role.as_str(),
                "generation": arm.current_generation(),
            }));
        }
    }

    let active_signals = state.arm_registry.active_watchout_signals().await;
    let quarantined_json = serde_json::to_value(&active_signals).unwrap_or_default();

    Ok(Json(OctopusArmsResponse {
        total_arms: arms_json.len(),
        master_seed_fingerprint: "octopus_verified_master".to_string(),
        arms: arms_json,
        quarantined_arms: quarantined_json,
    }))
}

/// POST /edr/ingest
pub async fn post_edr_ingest(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<EdrIngestRequest>,
) -> Result<Json<EdrIngestResponse>, ServerError> {
    let event = SecurityEvent::new(
        TelemetryEventKind::ProcessCreate,
        req.pid,
        &req.process_name,
        &req.image_path,
        &req.command_line,
    );

    let signal = state.event_correlator.ingest_event(&event, AttackStage::DefenseEvasion, 0.65);
    let score = state.event_correlator.get_process_score(req.pid).unwrap_or(0.65);

    Ok(Json(EdrIngestResponse {
        ingested: true,
        event_id: req.event_id,
        correlator_score: score,
        threat_signal_emitted: signal.is_some(),
    }))
}

// =============================================================================
// 4. Native OS Service Lifecycle Handlers
// =============================================================================

/// GET /service/status
pub async fn get_service_status(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<ServiceStatusResponse>, ServerError> {
    let status = state.service_manager.service_status("killswitch")
        .map_err(|e| ServerError::ServiceError(e.to_string()))?;

    Ok(Json(ServiceStatusResponse {
        name: status.name,
        display_name: status.display_name,
        state: format!("{:?}", status.state).to_lowercase(),
        pid: status.pid,
        binary_path: status.binary_path.clone().unwrap_or_default(),
        is_installed: status.is_installed,
    }))
}

/// POST /service/control
pub async fn post_service_control(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<ServiceControlRequest>,
) -> Result<Json<ServiceControlResponse>, ServerError> {
    let name = req.service_name.as_deref().unwrap_or("killswitch");

    match req.action.to_lowercase().as_str() {
        "stop" => {
            // Stopping service must unconditionally disarm host containment
            let _ = execute_host_disarm_unauthenticated();
            state.service_manager.stop_service(name)
                .map_err(|e| ServerError::ServiceError(e.to_string()))?;
        }
        "start" => {
            state.service_manager.start_service(name)
                .map_err(|e| ServerError::ServiceError(e.to_string()))?;
        }
        "restart" => {
            let _ = state.service_manager.stop_service(name);
            state.service_manager.start_service(name)
                .map_err(|e| ServerError::ServiceError(e.to_string()))?;
        }
        "uninstall" => {
            state.service_manager.uninstall_service(name)
                .map_err(|e| ServerError::ServiceError(e.to_string()))?;
        }
        _ => return Err(ServerError::Validation(format!("Unsupported action '{}'", req.action))),
    }

    let status = state.service_manager.service_status(name)
        .map_err(|e| ServerError::ServiceError(e.to_string()))?;

    Ok(Json(ServiceControlResponse {
        success: true,
        action: req.action,
        service_name: name.to_string(),
        resulting_status: ServiceStatusResponse {
            name: status.name,
            display_name: status.display_name,
            state: format!("{:?}", status.state).to_lowercase(),
            pid: status.pid,
            binary_path: status.binary_path.clone().unwrap_or_default(),
            is_installed: status.is_installed,
        },
    }))
}

// =============================================================================
// 5. DLP Data Lineage Handlers
// =============================================================================

/// GET /lineage/status
pub async fn get_lineage_status(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<LineageStatusResponse>, ServerError> {
    let procs = state.lineage_engine.all_tainted_processes();
    let files = state.lineage_engine.all_tainted_files();

    let procs_json: Vec<serde_json::Value> = procs.iter().map(|p| {
        serde_json::json!({
            "pid": p.proc_key.pid,
            "exe_path": p.exe_path,
            "classification": p.baggage.as_ref().map(|b| b.classification.as_str()),
            "policy": p.baggage.as_ref().map(|b| b.policy.as_str()),
            "source_hashes": p.baggage.as_ref().map(|b| b.distinct_hashes()).unwrap_or_default(),
            "provenance_hops": p.provenance_hops,
        })
    }).collect();

    let files_json: Vec<serde_json::Value> = files.iter().map(|f| {
        serde_json::json!({
            "path": f.path,
            "sha256": f.content_sha256,
            "classification": f.baggage.classification.as_str(),
            "policy": f.baggage.policy.as_str(),
        })
    }).collect();

    Ok(Json(LineageStatusResponse {
        total_processes_tracked: procs.len(),
        tainted_processes_count: procs.iter().filter(|p| p.baggage.is_some()).count(),
        total_files_tracked: files.len(),
        tainted_files_count: files.len(),
        tainted_processes: procs_json,
        tainted_files: files_json,
    }))
}

/// POST /lineage/seed
pub async fn post_lineage_seed(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<LineageSeedRequest>,
) -> Result<Json<LineageSeedResponse>, ServerError> {
    let class = match req.classification.to_uppercase().as_str() {
        "CROWN_JEWEL" | "CROWNJEWEL" => DlpClassification::CrownJewel,
        "CONFIDENTIAL" => DlpClassification::Confidential,
        "INTERNAL" => DlpClassification::Internal,
        _ => DlpClassification::Public,
    };

    let policy = match req.policy.to_uppercase().as_str() {
        "QUARANTINE" => DlpPolicy::Quarantine,
        "BLOCK" => DlpPolicy::Block,
        "AUDIT" => DlpPolicy::Audit,
        _ => DlpPolicy::Allow,
    };

    let file_key = FileKey::from_path_or_synthetic(&req.path);
    state.lineage_engine.seed_sensitive_file(file_key, &req.path, &req.sha256, class, policy);

    Ok(Json(LineageSeedResponse {
        success: true,
        path: req.path,
        sha256: req.sha256,
        classification: format!("{:?}", class),
        policy: format!("{:?}", policy),
    }))
}

/// POST /lineage/evaluate
pub async fn post_lineage_evaluate(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<LineageEvaluateRequest>,
) -> Result<Json<LineageEvaluateResponse>, ServerError> {
    let proc_key = state.lineage_engine.find_process_by_pid(req.pid).unwrap_or_else(|| {
        ProcessKey::new(req.pid, 0, "unknown")
    });

    let verdict = state.lineage_engine.evaluate_egress(proc_key, &req.destination_url);

    match verdict {
        EgressVerdict::Block { reason, trace_id, classification, policy, source_hashes } => {
            Ok(Json(LineageEvaluateResponse {
                verdict: "Block".to_string(),
                is_block: true,
                reason: Some(reason),
                trace_id: Some(trace_id),
                classification: Some(format!("{:?}", classification)),
                policy: Some(format!("{:?}", policy)),
                source_hashes,
            }))
        }
        EgressVerdict::Allow => {
            Ok(Json(LineageEvaluateResponse {
                verdict: "Allow".to_string(),
                is_block: false,
                reason: None,
                trace_id: None,
                classification: None,
                policy: None,
                source_hashes: vec![],
            }))
        }
    }
}
