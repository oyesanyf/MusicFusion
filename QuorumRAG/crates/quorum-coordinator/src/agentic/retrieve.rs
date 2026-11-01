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
        let candidate_k = top_k.max(24);
        let (mut chunks, volatile, _duration) = engine
            .execute_query(&state.current_query, vector, access_tier, candidate_k)
            .await;

        if let Some(ref filters) = state.query_metadata_filters {
            if let Some(ext) = filters.get("filetype") {
                let target_ext = format!(".{}", ext.trim_start_matches('.'));
                chunks.retain(|c| c.chunk.file_path.ends_with(&target_ext));
            }
        }

        // Extract meaningful query content terms (excluding conversational question stop-words)
        const STOPWORDS: &[&str] = &[
            "what", "does", "do", "did", "where", "when", "who", "how", "why",
            "is", "are", "was", "were", "the", "and", "for", "with", "from",
            "into", "about", "using", "have", "has", "had", "will", "would",
            "should", "could", "can", "you", "tell", "show", "went", "disambiguation", "hop",
        ];

        let prompt_lower = state.original_prompt.to_ascii_lowercase();
        let query_terms: Vec<String> = prompt_lower
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() >= 3 && !STOPWORDS.contains(w))
            .map(|w| w.to_string())
            .collect();

        if !query_terms.is_empty() && !chunks.is_empty() {
            let asks_university = prompt_lower.contains("university")
                || prompt_lower.contains("college")
                || prompt_lower.contains("school")
                || prompt_lower.contains("degree")
                || prompt_lower.contains("education")
                || prompt_lower.contains("transcript")
                || prompt_lower.contains("gpa");

            let asks_profile_or_role = prompt_lower.contains("what does")
                || prompt_lower.contains("who is")
                || prompt_lower.contains("resume")
                || prompt_lower.contains("work")
                || prompt_lower.contains("job")
                || prompt_lower.contains("role")
                || prompt_lower.contains("skill")
                || prompt_lower.contains("profession")
                || prompt_lower.contains("experience")
                || prompt_lower.contains("career");

            let score_chunk = |c: &quorum_types::ScoredChunk| -> (usize, i32) {
                let text_l = c.chunk.text_content.to_ascii_lowercase();
                let path_l = c.chunk.file_path.to_ascii_lowercase();
                let term_hits = query_terms
                    .iter()
                    .filter(|t| text_l.contains(t.as_str()) || path_l.contains(t.as_str()))
                    .count();

                let mut intent_bonus = 0i32;
                if asks_university {
                    if text_l.contains("university") || text_l.contains("college") {
                        intent_bonus += 50;
                    }
                    if path_l.contains("transcript") || path_l.contains("transctipt") || path_l.contains("school") || path_l.contains("0754749") {
                        intent_bonus += 30;
                    }
                    if text_l.contains("gpa") || text_l.contains("master") || text_l.contains("bachelor") {
                        intent_bonus += 20;
                    }
                }
                if asks_profile_or_role {
                    if path_l.contains("resume") || path_l.contains("cv") {
                        intent_bonus += 50;
                    }
                    if text_l.contains("professional summary")
                        || text_l.contains("summary")
                        || text_l.contains("core competencies")
                        || text_l.contains("cissp")
                        || text_l.contains("security professional")
                        || text_l.contains("security engineer")
                    {
                        intent_bonus += 40;
                    }
                }

                // Prefer clean recent docs (`Resume2026`, `ResumeSecAI2025`, `Enhanced_Resume`) over raw lock/binary artifacts
                if path_l.contains("2026") || path_l.contains("2025") || path_l.contains("enhanced_resume") {
                    intent_bonus += 15;
                }

                (term_hits, intent_bonus)
            };

            let any_lexical_match = chunks.iter().any(|c| score_chunk(c).0 > 0);
            if any_lexical_match {
                chunks.retain(|c| score_chunk(c).0 > 0);
                chunks.sort_by(|a, b| {
                    let (hits_a, bonus_a) = score_chunk(a);
                    let (hits_b, bonus_b) = score_chunk(b);
                    hits_b
                        .cmp(&hits_a)
                        .then_with(|| bonus_b.cmp(&bonus_a))
                        .then_with(|| {
                            b.score
                                .partial_cmp(&a.score)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                });
            }
        }

        chunks.truncate(top_k);
        state.retrieved_chunks = chunks;
        state.volatile_chunks = volatile;
    }
}
