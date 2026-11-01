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
        let (mut chunks, volatile, _duration) = engine
            .execute_query(&state.current_query, vector, access_tier, top_k)
            .await;

        if let Some(ref filters) = state.query_metadata_filters {
            if let Some(ext) = filters.get("filetype") {
                let target_ext = format!(".{}", ext.trim_start_matches('.'));
                chunks.retain(|c| c.chunk.file_path.ends_with(&target_ext));
            }
        }

        state.retrieved_chunks = chunks;
        state.volatile_chunks = volatile;
    }
}
