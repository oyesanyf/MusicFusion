use crate::hardware::HardwareTelemetry;
use crate::model_router::AutoFusionRouter;
use crate::types::{AgenticGraphState, QueryIntentType};

pub struct ClassifyIntentNode;

impl ClassifyIntentNode {
    pub fn execute(state: &mut AgenticGraphState, hw: &HardwareTelemetry) {
        let lower = state.original_prompt.to_lowercase();
        let is_multi_hop = lower.contains("compare")
            || lower.contains("difference")
            || lower.contains("versus")
            || lower.contains("vs ")
            || lower.contains("relationship");

        let is_mesh = lower.contains("mesh")
            || lower.contains("network")
            || lower.contains("federated")
            || lower.contains("all peers");

        let is_conversation = lower.starts_with("hello")
            || lower.starts_with("hi ")
            || lower == "hi"
            || lower.starts_with("who are you")
            || lower.starts_with("what can you do");

        let intent = if is_conversation {
            QueryIntentType::GeneralConversation
        } else if is_multi_hop {
            QueryIntentType::MultiHopReasoning
        } else if is_mesh {
            QueryIntentType::FederatedMeshSearch
        } else {
            QueryIntentType::LocalDocumentSearch
        };

        state.intent = Some(intent);
        state.query_metadata_filters =
            crate::langextract::extract_query_metadata_filters(&state.original_prompt);

        let (family, tag, rationale) = AutoFusionRouter::select_model(
            state.model_family,
            &state.original_prompt,
            &state.retrieved_chunks,
            hw,
        );
        state.model_family = family;
        state.selected_model_tag = tag;
        state.routing_rationale = rationale;
    }
}
