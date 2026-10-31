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
        let max_score = state
            .retrieved_chunks
            .iter()
            .map(|c| c.score)
            .fold(0.0f32, f32::max);

        state.relevance_score = max_score;

        let satisfactory =
            max_score >= self.relevance_threshold || state.retrieved_chunks.is_empty();
        let will_rewrite = !satisfactory && state.hop_count < state.max_hops;

        let grade = RelevanceGrade {
            is_relevant: satisfactory,
            score: max_score,
            reasoning: if satisfactory {
                format!(
                    "Top chunk score {:.3} meets threshold {:.3}",
                    max_score, self.relevance_threshold
                )
            } else {
                format!(
                    "Top chunk score {:.3} below threshold {:.3}",
                    max_score, self.relevance_threshold
                )
            },
            satisfactory,
        };

        state.relevance_grade = Some(grade.clone());
        (grade, will_rewrite)
    }
}
