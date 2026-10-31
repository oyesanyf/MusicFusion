use crate::offline_engine::DeterministicOfflineSynthesisEngine;
use crate::types::AgenticGraphState;

pub struct SynthesizeWithCitationsNode;

impl SynthesizeWithCitationsNode {
    pub fn execute(state: &mut AgenticGraphState) {
        let (resp, citations) = DeterministicOfflineSynthesisEngine::synthesize(
            &state.original_prompt,
            &state.retrieved_chunks,
            &state.selected_model_tag,
        );
        state.synthesized_response = Some(resp);
        state.citations = citations;
    }
}
