use crate::offline_engine::DeterministicOfflineSynthesisEngine;
use crate::telemetry::AgenticTelemetryEvent;
use crate::types::AgenticGraphState;
use tokio::sync::mpsc::Sender;

pub struct SynthesizeWithCitationsNode;

impl SynthesizeWithCitationsNode {
    pub fn execute(
        state: &mut AgenticGraphState,
        telemetry_tx: Option<&Sender<AgenticTelemetryEvent>>,
    ) {
        let (resp, citations) = DeterministicOfflineSynthesisEngine::synthesize_with_history(
            &state.original_prompt,
            &state.current_query,
            &state.conversation_history,
            &state.retrieved_chunks,
            &state.selected_model_tag,
            telemetry_tx,
        );
        state.synthesized_response = Some(resp);
        state.citations = citations;
    }
}
