use crate::types::{AgenticGraphState, RelevanceGrade};

pub struct GradeRelevanceNode {
    pub relevance_threshold: f32,
}

impl Default for GradeRelevanceNode {
    fn default() -> Self {
        Self {
            relevance_threshold: 0.60,
        }
    }
}

impl GradeRelevanceNode {
    pub fn execute(&self, state: &mut AgenticGraphState) -> (RelevanceGrade, bool) {
        let is_empty = state.retrieved_chunks.is_empty();
        let max_score = if is_empty {
            0.0f32
        } else {
            state
                .retrieved_chunks
                .iter()
                .map(|c| c.score)
                .fold(0.0f32, f32::max)
        };

        state.relevance_score = max_score;

        let effective_threshold = if max_score > 0.0 && max_score <= 0.1 {
            self.relevance_threshold * (1.0 / 60.0)
        } else {
            self.relevance_threshold
        };

        let satisfactory = !is_empty && max_score >= effective_threshold;
        let will_rewrite = !satisfactory && state.hop_count < state.max_hops;

        let reasoning = if is_empty {
            "Empty chunks retrieved. Scheduling query rewrite or fallback.".to_string()
        } else if satisfactory {
            format!(
                "Top chunk score {:.3} meets threshold {:.3}",
                max_score, effective_threshold
            )
        } else {
            format!(
                "Top chunk score {:.3} below threshold {:.3}",
                max_score, effective_threshold
            )
        };

        let grade = RelevanceGrade {
            is_relevant: satisfactory,
            score: max_score,
            reasoning,
            satisfactory,
        };

        state.relevance_grade = Some(grade.clone());
        (grade, will_rewrite)
    }
}
