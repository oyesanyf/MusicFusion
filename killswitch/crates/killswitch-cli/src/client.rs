//! # Resilient HTTP & Direct In-Process Client
//!
//! Communicates with the local Axum API on `http://127.0.0.1:3030`.
//! Features automatic emergency fallback to direct native unauthenticated host disarm
//! when the HTTP server daemon is unreachable or port 3030 is stopped.

use std::time::Duration;
use anyhow::{Context, Result};
use tracing::{info, warn};

use killswitch_core::killchain::execute_host_disarm_unauthenticated;
use killswitch_server::types::*;

pub struct KillSwitchClient {
    base_url: String,
    client: reqwest::Client,
}

impl KillSwitchClient {
    pub fn new(host: &str, port: u16) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            base_url: format!("http://{}:{}", host, port),
            client,
        }
    }

    pub fn default_local() -> Self {
        Self::new("127.0.0.1", 3030)
    }

    pub async fn get_status(&self) -> Result<StatusResponse> {
        let url = format!("{}/status", self.base_url);
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to connect to KillSwitch server at {}", url))?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Status query failed: {}", err_text);
        }

        let body = resp.json::<StatusResponse>().await
            .context("Failed to parse status response JSON")?;
        Ok(body)
    }

    pub async fn get_history(&self, limit: Option<usize>, offset: Option<usize>) -> Result<HistoryResponse> {
        let mut url = format!("{}/history", self.base_url);
        let mut query = Vec::new();
        if let Some(l) = limit {
            query.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            query.push(format!("offset={}", o));
        }
        if !query.is_empty() {
            url.push('?');
            url.push_str(&query.join("&"));
        }

        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to query history at {}", url))?;

        let body = resp.json::<HistoryResponse>().await
            .context("Failed to parse history response JSON")?;
        Ok(body)
    }

    pub async fn engage(
        &self,
        operator: &str,
        reason: &str,
        level: Option<&str>,
        token: Option<&str>,
    ) -> Result<EngageResponse> {
        let url = format!("{}/engage", self.base_url);
        let req = EngageRequest {
            operator: operator.to_string(),
            reason: reason.to_string(),
            token: token.map(|t| t.to_string()),
            level: level.map(|l| l.to_string()),
            caller_pid: Some(std::process::id()),
        };

        let resp = self.client.post(&url).json(&req).send().await
            .with_context(|| format!("Failed to send engage request to {}", url))?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Engage failed: {}", err_text);
        }

        let body = resp.json::<EngageResponse>().await
            .context("Failed to parse engage response JSON")?;
        Ok(body)
    }

    pub async fn disarm(
        &self,
        operator: &str,
        reason: &str,
        token: Option<&str>,
    ) -> Result<DisarmResponse> {
        let url = format!("{}/disarm", self.base_url);
        let req = DisarmRequest {
            operator: Some(operator.to_string()),
            reason: Some(reason.to_string()),
            token: token.map(|t| t.to_string()),
            caller_pid: Some(std::process::id()),
        };

        match self.client.post(&url).json(&req).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let err_text = resp.text().await.unwrap_or_default();
                    anyhow::bail!("Disarm failed: {}", err_text);
                }
                let body = resp.json::<DisarmResponse>().await
                    .context("Failed to parse disarm response JSON")?;
                Ok(body)
            }
            Err(e) => {
                warn!("⚠️ HTTP server at {} unreachable: {}. Invoking direct in-process disarm fallback...", url, e);
                let fallback_status = execute_host_disarm_unauthenticated()
                    .context("Emergency direct in-process disarm failed")?;

                info!("✅ Emergency host disarm executed successfully in process.");
                Ok(DisarmResponse {
                    success: true,
                    message: "Direct in-process emergency restore executed (HTTP offline)".to_string(),
                    status: fallback_status,
                })
            }
        }
    }

    pub async fn get_graph_state(&self) -> Result<GraphStateResponse> {
        let url = format!("{}/graph/state", self.base_url);
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to query graph state at {}", url))?;

        let body = resp.json::<GraphStateResponse>().await
            .context("Failed to parse graph state response JSON")?;
        Ok(body)
    }

    pub async fn step_graph(&self, force_node: Option<&str>) -> Result<GraphStepResponse> {
        let url = format!("{}/graph/step", self.base_url);
        let req = GraphStepRequest {
            step_input: None,
            force_node: force_node.map(|s| s.to_string()),
        };

        let resp = self.client.post(&url).json(&req).send().await
            .with_context(|| format!("Failed to step graph at {}", url))?;

        let body = resp.json::<GraphStepResponse>().await
            .context("Failed to parse graph step response JSON")?;
        Ok(body)
    }

    pub async fn set_mode(&self, mode: &str, operator: Option<&str>) -> Result<ModeResponse> {
        let url = format!("{}/mode", self.base_url);
        let req = ModeRequest {
            mode: mode.to_string(),
            operator: operator.map(|s| s.to_string()),
            token: None,
        };

        let resp = self.client.post(&url).json(&req).send().await
            .with_context(|| format!("Failed to set mode at {}", url))?;

        let body = resp.json::<ModeResponse>().await
            .context("Failed to parse mode response JSON")?;
        Ok(body)
    }

    pub async fn get_service_status(&self) -> Result<ServiceStatusResponse> {
        let url = format!("{}/service/status", self.base_url);
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to query service status at {}", url))?;

        let body = resp.json::<ServiceStatusResponse>().await
            .context("Failed to parse service status JSON")?;
        Ok(body)
    }

    pub async fn control_service(&self, action: &str) -> Result<ServiceControlResponse> {
        let url = format!("{}/service/control", self.base_url);
        let req = ServiceControlRequest {
            action: action.to_string(),
            service_name: None,
        };

        let resp = self.client.post(&url).json(&req).send().await
            .with_context(|| format!("Failed to control service at {}", url))?;

        let body = resp.json::<ServiceControlResponse>().await
            .context("Failed to parse service control JSON")?;
        Ok(body)
    }

    pub async fn get_lineage_status(&self) -> Result<LineageStatusResponse> {
        let url = format!("{}/lineage/status", self.base_url);
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to query lineage status at {}", url))?;

        let body = resp.json::<LineageStatusResponse>().await
            .context("Failed to parse lineage status JSON")?;
        Ok(body)
    }

    pub async fn seed_lineage(&self, path: &str, sha256: &str, class: &str, policy: &str) -> Result<LineageSeedResponse> {
        let url = format!("{}/lineage/seed", self.base_url);
        let req = LineageSeedRequest {
            path: path.to_string(),
            sha256: sha256.to_string(),
            classification: class.to_string(),
            policy: policy.to_string(),
            inode_or_id: None,
        };

        let resp = self.client.post(&url).json(&req).send().await
            .with_context(|| format!("Failed to seed lineage at {}", url))?;

        let body = resp.json::<LineageSeedResponse>().await
            .context("Failed to parse lineage seed JSON")?;
        Ok(body)
    }

    pub async fn evaluate_lineage(&self, pid: u32, destination_url: &str) -> Result<LineageEvaluateResponse> {
        let url = format!("{}/lineage/evaluate", self.base_url);
        let req = LineageEvaluateRequest {
            pid,
            destination_url: destination_url.to_string(),
        };

        let resp = self.client.post(&url).json(&req).send().await
            .with_context(|| format!("Failed to evaluate lineage at {}", url))?;

        let body = resp.json::<LineageEvaluateResponse>().await
            .context("Failed to parse lineage evaluate JSON")?;
        Ok(body)
    }

    pub async fn get_octopus_arms(&self) -> Result<OctopusArmsResponse> {
        let url = format!("{}/octopus/arms", self.base_url);
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to query octopus arms at {}", url))?;

        let body = resp.json::<OctopusArmsResponse>().await
            .context("Failed to parse octopus arms JSON")?;
        Ok(body)
    }

    pub async fn trigger_health_probe(&self, force_rollback: bool) -> Result<HealthProbeResponse> {
        let url = format!("{}/health-probe", self.base_url);
        let req = HealthProbeRequest {
            operator: Some("operator".to_string()),
            force_rollback: Some(force_rollback),
        };

        let resp = self.client.post(&url).json(&req).send().await
            .with_context(|| format!("Failed to trigger health probe at {}", url))?;

        let body = resp.json::<HealthProbeResponse>().await
            .context("Failed to parse health probe JSON")?;
        Ok(body)
    }
}
