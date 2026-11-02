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
        let candidate_k = top_k.max(48);
        let (mut chunks, volatile, _duration) = engine
            .execute_query(&state.current_query, vector, access_tier, candidate_k)
            .await;

        if let Some(ref filters) = state.query_metadata_filters {
            if let Some(ext) = filters.get("filetype") {
                let target_ext = format!(".{}", ext.trim_start_matches('.'));
                chunks.retain(|c| c.chunk.file_path.ends_with(&target_ext));
            }
        }

        // Extract meaningful query content terms from current_query (which includes expanded follow-up context)
        const STOPWORDS: &[&str] = &[
            "what", "does", "do", "did", "where", "when", "who", "how", "why",
            "is", "are", "was", "were", "the", "and", "for", "with", "from",
            "into", "about", "using", "have", "has", "had", "will", "would",
            "should", "could", "can", "you", "tell", "show", "went", "attend", "attended",
            "disambiguation", "hop", "more", "else", "also", "his", "her", "their", "there",
        ];

        let query_lower = state.current_query.to_ascii_lowercase();
        let query_terms: Vec<String> = query_lower
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() >= 3 && !STOPWORDS.contains(w))
            .fold(Vec::new(), |mut acc, w| {
                let s = w.to_string();
                if !acc.contains(&s) {
                    acc.push(s);
                }
                acc
            });

        if !query_terms.is_empty() && !chunks.is_empty() {
            let asks_education = query_lower.contains("university")
                || query_lower.contains("college")
                || query_lower.contains("school")
                || query_lower.contains("degree")
                || query_lower.contains("education")
                || query_lower.contains("transcript")
                || query_lower.contains("study")
                || query_lower.contains("studied")
                || query_lower.contains("gpa");

            let asks_profile_or_role = query_lower.contains("what does")
                || query_lower.contains("who is")
                || query_lower.contains("resume")
                || query_lower.contains("work")
                || query_lower.contains("job")
                || query_lower.contains("role")
                || query_lower.contains("skill")
                || query_lower.contains("profession")
                || query_lower.contains("experience")
                || query_lower.contains("career")
                || query_lower.contains("background")
                || query_lower.contains("about");

            let asks_image = query_lower.contains("image")
                || query_lower.contains("photo")
                || query_lower.contains("picture")
                || query_lower.contains("screenshot");

            let score_chunk = |c: &quorum_types::ScoredChunk| -> (usize, i32) {
                let text_l = c.chunk.text_content.to_ascii_lowercase();
                let path_l = c.chunk.file_path.to_ascii_lowercase();
                let file_name_l = std::path::Path::new(&c.chunk.file_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();

                // Direct matches in document text or filename count 12x more than parent OS directory matches
                let mut weighted_hits = 0usize;
                let mut content_matched_terms = 0usize;
                for t in &query_terms {
                    if text_l.contains(t.as_str()) || file_name_l.contains(t.as_str()) {
                        weighted_hits += 12;
                        content_matched_terms += 1;
                    } else if path_l.contains(t.as_str()) {
                        weighted_hits += 1;
                    }
                }
                if content_matched_terms >= 2 {
                    weighted_hits += content_matched_terms * 8;
                }

                let mut intent_bonus = 0i32;

                // Deprioritize raw image metadata stubs unless the user asked about images
                if !asks_image
                    && (text_l.starts_with("[image document:")
                        || file_name_l.ends_with(".gif")
                        || file_name_l.ends_with(".jpg")
                        || file_name_l.ends_with(".jpeg")
                        || file_name_l.ends_with(".png")
                        || file_name_l.ends_with(".heic"))
                {
                    intent_bonus -= 150;
                }

                // Deprioritize noisy tax/form/scratch files unless explicitly queried
                if file_name_l == "stuff.txt" || file_name_l.contains("1095-c") {
                    intent_bonus -= 120;
                }

                // Domain-agnostic academic structure signal when education/school is queried
                if asks_education {
                    if text_l.contains("university")
                        || text_l.contains("college")
                        || text_l.contains("transcript")
                        || text_l.contains("master of")
                        || text_l.contains("bachelor of")
                        || text_l.contains("education")
                    {
                        intent_bonus += 90;
                    }
                    if text_l.contains("gpa") || text_l.contains("degree") || text_l.contains("curriculum") {
                        intent_bonus += 40;
                    }
                }

                // Domain-agnostic profile/resume structure signal when biography/experience is queried
                if asks_profile_or_role {
                    if file_name_l.contains("resume") || path_l.contains("resume") || file_name_l.contains("cv") {
                        intent_bonus += 70;
                    }
                    if text_l.contains("summary")
                        || text_l.contains("experience")
                        || text_l.contains("competencies")
                        || text_l.contains("skills")
                        || text_l.contains("certification")
                        || text_l.contains("education")
                    {
                        intent_bonus += 45;
                    }
                }

                // Prefer substantive text chunks over 1-line headers
                if c.chunk.text_content.trim().len() >= 180 {
                    intent_bonus += 20;
                }

                (weighted_hits, intent_bonus)
            };

            let any_lexical_match = chunks.iter().any(|c| score_chunk(c).0 > 0);
            if any_lexical_match {
                chunks.retain(|c| {
                    let (hits, bonus) = score_chunk(c);
                    hits > 0 && bonus > -100
                });
                chunks.sort_by(|a, b| {
                    let (hits_a, bonus_a) = score_chunk(a);
                    let (hits_b, bonus_b) = score_chunk(b);
                    let total_a = (hits_a as i32) * 5 + bonus_a;
                    let total_b = (hits_b as i32) * 5 + bonus_b;
                    total_b
                        .cmp(&total_a)
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
