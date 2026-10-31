use crate::scatter_gather::ScatterGatherEngine;
use crate::types::AgenticGraphState;

pub struct RetrieveMeshNode;

impl RetrieveMeshNode {
    pub async fn execute(
        state: &mut AgenticGraphState,
        engine: &ScatterGatherEngine,
        vector: &[f32],
        access_tier: u8,
        top_k: usize,
    ) {
        let (chunks, volatile, _duration) = engine
            .execute_query(&state.current_query, vector, access_tier, top_k)
            .await;

        state.retrieved_chunks = chunks;
        state.volatile_chunks = volatile;
    }
}
