use crate::error::CoordinatorError;
use crate::types::AgenticGraphState;

pub struct RewriteQueryNode;

impl RewriteQueryNode {
    pub fn execute(state: &mut AgenticGraphState) -> Result<String, CoordinatorError> {
        state.hop_count += 1;

        let rewritten = format!(
            "{} (disambiguation hop {})",
            state.original_prompt, state.hop_count
        );

        if state.query_history.contains(&rewritten) {
            return Err(CoordinatorError::GraphError(format!(
                "Anti-cycle triggered: query '{}' already evaluated",
                rewritten
            )));
        }

        state.query_history.push(rewritten.clone());
        state.current_query = rewritten.clone();
        Ok(rewritten)
    }
}
