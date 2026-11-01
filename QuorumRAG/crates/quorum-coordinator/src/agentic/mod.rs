pub mod classify;
pub mod grade;
pub mod retrieve;
pub mod rewrite;
pub mod synthesize;

use crate::hardware::HardwareTelemetry;
use crate::scatter_gather::ScatterGatherEngine;
use crate::telemetry::AgenticTelemetryEvent;
use crate::types::*;
use classify::ClassifyIntentNode;
use grade::GradeRelevanceNode;
use retrieve::RetrieveMeshNode;
use rewrite::RewriteQueryNode;
use synthesize::SynthesizeWithCitationsNode;
use tokio::sync::mpsc::Sender;
use std::sync::Arc;
use crate::cache::CacheManager;

pub struct AgenticStateGraphRunner {
    pub max_hops: usize,
    pub relevance_threshold: f32,
    pub cache_manager: Arc<CacheManager>,
}

impl Default for AgenticStateGraphRunner {
    fn default() -> Self {
        Self {
            max_hops: 3,
            relevance_threshold: 0.60,
            cache_manager: Arc::new(CacheManager::default()),
        }
    }
}

impl AgenticStateGraphRunner {
    pub fn new(max_hops: usize, relevance_threshold: f32) -> Self {
        Self {
            max_hops,
            relevance_threshold,
            cache_manager: Arc::new(CacheManager::default()),
        }
    }

    pub fn with_cache_manager(mut self, cache_manager: Arc<CacheManager>) -> Self {
        self.cache_manager = cache_manager;
        self
    }

    /// Executes the LangGraph-style state graph with live SSE telemetry and mathematical termination guarantee.
    #[allow(clippy::too_many_arguments)]
    pub async fn run(
        &self,
        prompt: &str,
        requested_family: SynthesisModelFamily,
        engine: &ScatterGatherEngine,
        vector: &[f32],
        access_tier: u8,
        top_k: usize,
        telemetry_tx: Option<Sender<AgenticTelemetryEvent>>,
    ) -> Result<AgenticGraphState, String> {
        let mut state_name = AgenticStateName::ClassifyIntent;
        let mut graph_state = AgenticGraphState::new(prompt, requested_family, self.max_hops);

        let hw = HardwareTelemetry::probe();
        if let Some(ref tx) = telemetry_tx {
            let _ = tx
                .send(AgenticTelemetryEvent::HardwareProbed {
                    free_ram_gb: hw.free_ram_gb,
                    free_vram_mb: hw.free_vram_mb,
                    gpu_name: hw.gpu_name.clone(),
                    selected_tier: format!(
                        "RAM: {:.1}GB free, VRAM: {}MB",
                        hw.free_ram_gb, hw.free_vram_mb
                    ),
                })
                .await;
        }

        let grader = GradeRelevanceNode {
            relevance_threshold: self.relevance_threshold,
        };

        loop {
            match state_name {
                AgenticStateName::ClassifyIntent => {
                    ClassifyIntentNode::execute(&mut graph_state, &hw);

                    let epoch = engine.local_storage.index_epoch();
                    let maybe_cached = self.cache_manager.query_cache.get(
                        &graph_state.original_prompt,
                        graph_state.search_mode,
                        graph_state.model_family,
                        access_tier,
                        epoch,
                    );

                    let cache_hit = maybe_cached.is_some();
                    let telem = self.cache_manager.telemetry(cache_hit);

                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx
                            .send(AgenticTelemetryEvent::CacheTelemetry {
                                cache_hit,
                                cache_epoch: telem.cache_epoch,
                                cached_queries: telem.cached_queries,
                                cached_embeddings: telem.cached_embeddings,
                                hits: telem.hits,
                                misses: telem.misses,
                                hit_rate: telem.hit_rate,
                            })
                            .await;
                    }

                    if let Some(cached) = maybe_cached {
                        // FAST PATH: Short-circuit entire retrieval, grading, rewrite, and synthesis!
                        graph_state.retrieved_chunks = cached.chunks.clone();
                        graph_state.synthesized_response = cached.synthesized_response.clone();
                        graph_state.citations = cached.citations.clone();
                        graph_state.query_metadata_filters = cached.query_metadata_filters.clone();

                        if let Some(ref tx) = telemetry_tx {
                            for cit in &graph_state.citations {
                                let _ = tx.send(AgenticTelemetryEvent::CitationEmitted { citation: cit.clone() }).await;
                            }

                            let _ = tx
                                .send(AgenticTelemetryEvent::StateTransition {
                                    from_state: AgenticStateName::ClassifyIntent,
                                    to_state: AgenticStateName::Complete,
                                    hop: 0,
                                    timestamp_ms: graph_state.start_time.elapsed().as_millis() as u64,
                                })
                                .await;

                            let _ = tx
                                .send(AgenticTelemetryEvent::ExecutionFinished {
                                    total_hops: 0,
                                    total_duration_ms: graph_state.start_time.elapsed().as_millis() as u64,
                                    total_tokens: graph_state
                                        .synthesized_response
                                        .as_ref()
                                        .map(|s| s.len() / 4)
                                        .unwrap_or(0),
                                    final_state: AgenticStateName::Complete,
                                    model_name: graph_state.selected_model_tag.clone(),
                                })
                                .await;
                        }

                        return Ok(graph_state);
                    }

                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx
                            .send(AgenticTelemetryEvent::IntentClassified {
                                intent: graph_state.intent.unwrap_or(QueryIntentType::LocalDocumentSearch),
                                search_mode: graph_state.search_mode,
                                model_family: graph_state.model_family,
                                model_name: graph_state.selected_model_tag.clone(),
                                routing_rationale: graph_state.routing_rationale.clone(),
                                is_retrieval_required: true,
                            })
                            .await;

                        let _ = tx
                            .send(AgenticTelemetryEvent::StateTransition {
                                from_state: AgenticStateName::ClassifyIntent,
                                to_state: AgenticStateName::RetrieveMesh,
                                hop: graph_state.hop_count,
                                timestamp_ms: graph_state.start_time.elapsed().as_millis() as u64,
                            })
                            .await;
                    }
                    state_name = AgenticStateName::RetrieveMesh;
                }

                AgenticStateName::RetrieveMesh => {
                    let start = std::time::Instant::now();
                    RetrieveMeshNode::execute(
                        &mut graph_state,
                        engine,
                        vector,
                        access_tier,
                        top_k,
                    )
                    .await;
                    let latency = start.elapsed().as_millis() as u64;

                    if let Some(ref tx) = telemetry_tx {
                        let local_count = graph_state.retrieved_chunks.iter().filter(|c| !c.is_remote).count();
                        let remote_count = graph_state.retrieved_chunks.iter().filter(|c| c.is_remote).count();

                        let _ = tx
                            .send(AgenticTelemetryEvent::RetrievalCompleted {
                                hop: graph_state.hop_count,
                                query: graph_state.current_query.clone(),
                                search_mode: graph_state.search_mode,
                                retrieved_count: graph_state.retrieved_chunks.len(),
                                local_count,
                                remote_count,
                                latency_ms: latency,
                            })
                            .await;

                        let _ = tx
                            .send(AgenticTelemetryEvent::StateTransition {
                                from_state: AgenticStateName::RetrieveMesh,
                                to_state: AgenticStateName::GradeRelevance,
                                hop: graph_state.hop_count,
                                timestamp_ms: graph_state.start_time.elapsed().as_millis() as u64,
                            })
                            .await;
                    }
                    state_name = AgenticStateName::GradeRelevance;
                }

                AgenticStateName::GradeRelevance => {
                    let (grade, will_rewrite) = grader.execute(&mut graph_state);

                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx
                            .send(AgenticTelemetryEvent::RelevanceGraded {
                                hop: graph_state.hop_count,
                                grade: grade.clone(),
                                score: grade.score,
                                reasoning: grade.reasoning,
                                will_rewrite,
                            })
                            .await;
                    }

                    if will_rewrite {
                        if let Some(ref tx) = telemetry_tx {
                            let _ = tx
                                .send(AgenticTelemetryEvent::StateTransition {
                                    from_state: AgenticStateName::GradeRelevance,
                                    to_state: AgenticStateName::RewriteQuery,
                                    hop: graph_state.hop_count,
                                    timestamp_ms: graph_state.start_time.elapsed().as_millis() as u64,
                                })
                                .await;
                        }
                        state_name = AgenticStateName::RewriteQuery;
                    } else {
                        if let Some(ref tx) = telemetry_tx {
                            let _ = tx
                                .send(AgenticTelemetryEvent::StateTransition {
                                    from_state: AgenticStateName::GradeRelevance,
                                    to_state: AgenticStateName::SynthesizeWithCitations,
                                    hop: graph_state.hop_count,
                                    timestamp_ms: graph_state.start_time.elapsed().as_millis() as u64,
                                })
                                .await;
                        }
                        state_name = AgenticStateName::SynthesizeWithCitations;
                    }
                }

                AgenticStateName::RewriteQuery => {
                    match RewriteQueryNode::execute(&mut graph_state) {
                        Ok(rewritten) => {
                            if let Some(ref tx) = telemetry_tx {
                                let _ = tx
                                    .send(AgenticTelemetryEvent::QueryRewritten {
                                        hop: graph_state.hop_count,
                                        original_query: graph_state.original_prompt.clone(),
                                        rewritten_query: rewritten,
                                        rewrite_strategy: "Context disambiguation hop".to_string(),
                                    })
                                    .await;

                                let _ = tx
                                    .send(AgenticTelemetryEvent::StateTransition {
                                        from_state: AgenticStateName::RewriteQuery,
                                        to_state: AgenticStateName::RetrieveMesh,
                                        hop: graph_state.hop_count,
                                        timestamp_ms: graph_state.start_time.elapsed().as_millis() as u64,
                                    })
                                    .await;
                            }
                            state_name = AgenticStateName::RetrieveMesh;
                        }
                        Err(_) => {
                            // Cycle detected - break out to synthesis
                            state_name = AgenticStateName::SynthesizeWithCitations;
                        }
                    }
                }

                AgenticStateName::SynthesizeWithCitations => {
                    if let Some(ref tx) = telemetry_tx {
                        let _ = tx
                            .send(AgenticTelemetryEvent::SynthesisStarted {
                                model_name: graph_state.selected_model_tag.clone(),
                                model_family: graph_state.model_family,
                                context_chunks_count: graph_state.retrieved_chunks.len(),
                                estimated_prompt_tokens: 300,
                            })
                            .await;
                    }

                    SynthesizeWithCitationsNode::execute(&mut graph_state);

                    // Insert into QueryCache
                    let epoch = engine.local_storage.index_epoch();
                    self.cache_manager.query_cache.insert(
                        &graph_state.original_prompt,
                        graph_state.search_mode,
                        graph_state.model_family,
                        access_tier,
                        epoch,
                        graph_state.retrieved_chunks.clone(),
                        crate::offline_engine::DeterministicOfflineSynthesisEngine::build_synthesis_prompt(
                            &graph_state.original_prompt,
                            &graph_state.retrieved_chunks,
                        ),
                        graph_state.synthesized_response.clone(),
                        graph_state.citations.clone(),
                        None,
                        graph_state.query_metadata_filters.clone(),
                    );

                    if let Some(ref tx) = telemetry_tx {
                        for cit in &graph_state.citations {
                            let _ = tx
                                .send(AgenticTelemetryEvent::CitationEmitted {
                                    citation: cit.clone(),
                                })
                                .await;
                        }

                        let _ = tx
                            .send(AgenticTelemetryEvent::ExecutionFinished {
                                total_hops: graph_state.hop_count,
                                total_duration_ms: graph_state.start_time.elapsed().as_millis() as u64,
                                total_tokens: graph_state
                                    .synthesized_response
                                    .as_ref()
                                    .map(|s| s.len() / 4)
                                    .unwrap_or(0),
                                final_state: AgenticStateName::Complete,
                                model_name: graph_state.selected_model_tag.clone(),
                            })
                            .await;
                    }

                    return Ok(graph_state);
                }

                AgenticStateName::Complete | AgenticStateName::Failed => {
                    return Ok(graph_state);
                }
            }
        }
    }
}
