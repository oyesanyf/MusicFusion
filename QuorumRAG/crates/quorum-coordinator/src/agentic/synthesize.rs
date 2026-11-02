use crate::offline_engine::DeterministicOfflineSynthesisEngine;
use crate::types::AgenticGraphState;

pub struct SynthesizeWithCitationsNode;

impl SynthesizeWithCitationsNode {
    pub fn execute(state: &mut AgenticGraphState) {
        let (resp, citations) = DeterministicOfflineSynthesisEngine::synthesize_with_history(
            &state.original_prompt,
            &state.current_query,
            &state.conversation_history,
            &state.retrieved_chunks,
            &state.selected_model_tag,
        );
        state.synthesized_response = Some(resp);
        state.citations = citations;
    }
}
