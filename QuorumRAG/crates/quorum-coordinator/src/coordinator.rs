use crate::agentic::AgenticStateGraphRunner;
use crate::config::CoordinatorConfig;
use crate::error::CoordinatorError;
use crate::offline_engine::DeterministicOfflineSynthesisEngine;
use crate::scatter_gather::ScatterGatherEngine;
use crate::telemetry::AgenticTelemetryEvent;
use crate::types::SynthesisModelFamily;
use quorum_types::{Citation, QueryResult};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub struct QueryCoordinator {
    pub engine: Arc<ScatterGatherEngine>,
    pub config: CoordinatorConfig,
    pub agentic_runner: AgenticStateGraphRunner,
}

impl QueryCoordinator {
    pub fn new(engine: Arc<ScatterGatherEngine>, config: CoordinatorConfig) -> Self {
        Self {
            engine,
            config,
            agentic_runner: AgenticStateGraphRunner::default(),
        }
    }

    /// Fast single-pass scatter-gather retrieval across local storage and federated peers.
    pub async fn query(&self, prompt: &str, top_k: usize) -> Result<QueryResult, CoordinatorError> {
        let start = std::time::Instant::now();
        let engine = quorum_contract::DeterministicOfflineEngine::new();
        let vector = quorum_contract::EmbeddingGenerator::embed_text(&engine, prompt)
            .unwrap_or_else(|_| vec![0.0; 768]);

        let (chunks, _volatile, _) = self
            .engine
            .execute_query(
                prompt,
                &vector,
                self.config.default_peer_access_tier,
                top_k,
            )
            .await;

        let synthesis_prompt =
            DeterministicOfflineSynthesisEngine::build_synthesis_prompt(prompt, &chunks);
        let sources_cited: Vec<Citation> = chunks.iter().map(|c| c.to_citation()).collect();
        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            chunks,
            synthesis_prompt,
            execution_time_ms,
            sources_cited,
        })
    }

    /// Executes iterative multi-hop retrieval and synthesis via LangGraph-style Agentic State Graph.
    pub async fn query_agentic(
        &self,
        prompt: &str,
        family: SynthesisModelFamily,
        top_k: usize,
        telemetry_tx: Option<Sender<AgenticTelemetryEvent>>,
    ) -> Result<QueryResult, CoordinatorError> {
        let start = std::time::Instant::now();
        let engine = quorum_contract::DeterministicOfflineEngine::new();
        let vector = quorum_contract::EmbeddingGenerator::embed_text(&engine, prompt)
            .unwrap_or_else(|_| vec![0.0; 768]);

        let final_state = self
            .agentic_runner
            .run(
                prompt,
                family,
                &self.engine,
                &vector,
                self.config.default_peer_access_tier,
                top_k,
                telemetry_tx,
            )
            .await
            .map_err(CoordinatorError::GraphError)?;

        let synthesis_prompt = final_state
            .synthesized_response
            .unwrap_or_else(|| DeterministicOfflineSynthesisEngine::build_synthesis_prompt(prompt, &final_state.retrieved_chunks));

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            chunks: final_state.retrieved_chunks,
            synthesis_prompt,
            execution_time_ms,
            sources_cited: final_state.citations,
        })
    }
}
