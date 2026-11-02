use crate::scatter_gather::ScatterGatherEngine;
use crate::types::AgenticGraphState;
use quorum_contract::EmbeddingGenerator;
use quorum_types::ScoredChunk;

/// Pre-computed scoring criteria evaluated once per chunk before sorting,
/// eliminating repetitive lowercase allocations and substring scans inside the comparator.
#[derive(Debug, Clone, Copy)]
pub struct ChunkScoreCriteria {
    pub weighted_hits: usize,
    pub content_matched_terms: usize,
    pub entity_hits: usize,
    pub intent_bonus: i32,
    pub total_score: i32,
    pub is_relevant: bool,
}

pub const GENERIC_ATTRIBUTE_TERMS: &[&str] = &[
    "school", "schools", "university", "universities", "college", "colleges",
    "degree", "degrees", "education", "academic", "transcript", "transcripts",
    "gpa", "study", "studied", "major", "course", "courses", "work", "worked",
    "job", "jobs", "role", "roles", "skill", "skills", "profession", "professional",
    "experience", "experiences", "career", "background", "profile", "resume",
    "resumes", "summary", "summaries", "history", "company", "companies",
    "project", "projects", "certification", "certifications", "went", "attend",
    "attended", "graduated", "year", "years", "detail", "details", "information",
    "income", "tax", "taxes", "statement", "statements", "plan", "plans",
    "payment", "payments", "credit", "credits", "account", "accounts",
    "amount", "amounts", "rate", "rates", "form", "forms", "section",
    "sections", "page", "pages", "total", "day", "days", "heart", "soul",
    "life", "time", "world", "reading", "book", "books", "universal", "system", "systems",
    "model", "models", "data", "report", "reports", "policy", "policies",
];

const STOPWORDS: &[&str] = &[
    "what", "does", "do", "did", "where", "when", "who", "how", "why",
    "is", "are", "was", "were", "the", "and", "for", "with", "from",
    "into", "about", "using", "have", "has", "had", "will", "would",
    "should", "could", "can", "you", "tell", "show", "went", "attend", "attended",
    "disambiguation", "hop", "more", "else", "also", "his", "her", "their", "there",
    "file", "files", "document", "documents", "doc", "docs", "docx", "pdf", "txt",
    "folder", "folders", "content", "contents", "text", "exist", "exists", "existing",
    "find", "check", "list", "located", "location",
];

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

        // Feature 48: Reuse cached vector hits across rewrite hops to avoid redundant LanceDB scans
        let cached_hits_opt = if state.hop_count > 0 && !state.cached_vector_hits.is_empty() {
            Some(state.cached_vector_hits.as_slice())
        } else {
            None
        };

        let q_low = state.current_query.to_ascii_lowercase();
        let search_query = if q_low.contains("school")
            || q_low.contains("college")
            || q_low.contains("university")
            || q_low.contains("education")
            || q_low.contains("degree")
            || q_low.contains("graduated")
        {
            format!(
                "{} education university college master bachelor degree certifications",
                state.current_query
            )
        } else if q_low.contains("about ")
            || q_low.contains("who is")
            || q_low.contains("work")
            || q_low.contains("experience")
            || q_low.contains("career")
        {
            format!(
                "{} professional summary experience competencies publications",
                state.current_query
            )
        } else {
            state.current_query.clone()
        };

        // Align dense vector with primary extracted question if cleaner than noisy raw prompt
        let aligned_vec_storage;
        let effective_vector = if let Some(primary_q) = state.extracted_questions.first() {
            if primary_q != &state.current_query && !primary_q.trim().is_empty() {
                let embedder = quorum_contract::DeterministicOfflineEngine::new();
                if let Ok(fresh) = embedder.embed_text(primary_q) {
                    aligned_vec_storage = fresh;
                    aligned_vec_storage.as_slice()
                } else {
                    vector
                }
            } else {
                vector
            }
        } else {
            vector
        };

        let (mut chunks, volatile, _duration, new_vector_hits) = engine
            .execute_query_with_cached_vector(
                &search_query,
                effective_vector,
                cached_hits_opt,
                access_tier,
                candidate_k,
            )
            .await;

        if !new_vector_hits.is_empty() && state.cached_vector_hits.is_empty() {
            state.cached_vector_hits = new_vector_hits;
        }

        // When running in the live daemon (or when non-code user documents exist), filter out source code,
        // lock files, raw .dat blobs, and monolithic chat dumps so they never crowd out user PDF/DOCX documents.
        let daemon_active = crate::dlp_mesh::global_mesh_dlp_controller()
            .settings_file_path
            .read()
            .map(|p| p.is_some())
            .unwrap_or(false);

        // Supplement candidate pool with direct entity-term keyword hits so large corpora
        // never drown out specific entity documents (e.g. "Femi_Oyesanya_CV_2026.pdf") when generic
        // education/profile expansion terms ("school", "university", "experience") are present.
        let mut prelim_entities: Vec<String> = q_low
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| {
                w.len() >= 3
                    && !STOPWORDS.contains(w)
                    && !GENERIC_ATTRIBUTE_TERMS.contains(w)
            })
            .map(|w| w.to_string())
            .collect();

        // Augment with LangExtract verified entities and document targets
        for ext in &state.extracted_entities {
            let t = ext.extraction_text.trim();
            if !t.is_empty() && !prelim_entities.iter().any(|e| e.eq_ignore_ascii_case(t)) {
                prelim_entities.push(t.to_string());
            }
        }

        if (daemon_active || !state.extracted_entities.is_empty()) && !prelim_entities.is_empty() {
            let entity_kw_query = prelim_entities.join(" ");
            let entity_hits = engine
                .local_storage
                .search_text(&entity_kw_query, access_tier, 512)
                .await;
            let mut seen_ids: std::collections::HashSet<String> =
                chunks.iter().map(|c| c.chunk.chunk_id.clone()).collect();
            for hit in entity_hits {
                if seen_ids.insert(hit.chunk.chunk_id.clone()) {
                    chunks.push(hit);
                }
            }
        }

        if let Some(ref filters) = state.query_metadata_filters {
            if let Some(ext) = filters.get("filetype") {
                let target_ext = format!(".{}", ext.trim_start_matches('.'));
                chunks.retain(|c| c.chunk.file_path.ends_with(&target_ext));
            }
        }

        // Extract meaningful query content terms from current_query
        let query_lower = state.current_query.to_ascii_lowercase();
        let requests_code = query_lower.contains("code")
            || query_lower.contains("unit test")
            || query_lower.contains("implement")
            || query_lower.contains("rrfcombiner")
            || query_lower.contains("struct ")
            || query_lower.contains("fn ");

        if (daemon_active || engine.config.exclude_source_code) && !requests_code {
            const EXCLUDED_EXTS: &[&str] = &[
                ".rs", ".py", ".js", ".ts", ".jsx", ".tsx", ".c", ".cpp", ".h", ".hpp",
                ".cc", ".cs", ".java", ".go", ".kt", ".rb", ".php", ".swift", ".scala",
                ".sh", ".bash", ".ps1", ".bat", ".cmd", ".sql", ".css", ".scss", ".less",
                ".vue", ".svelte", ".toml", ".yaml", ".yml", ".lock", ".dat", ".log",
                ".tmp", ".bak", ".wbk",
            ];
            chunks.retain(|c| {
                let path_l = c.chunk.file_path.to_ascii_lowercase();
                let text_l = c.chunk.text_content.to_ascii_lowercase();
                let file_l = std::path::Path::new(&c.chunk.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if file_l.starts_with('~')
                    || file_l.starts_with(".~lock")
                    || file_l == "chat.html"
                    || (file_l.starts_with("conversations-") && file_l.ends_with(".json"))
                    || path_l.contains("\\harfile\\")
                    || path_l.contains("/harfile/")
                {
                    return false;
                }
                // Exclude coding tutorials / Google Colab notebooks / fenced code dumps when user did not ask for code
                if text_l.contains("```python")
                    || text_l.contains("```javascript")
                    || text_l.contains("```js")
                    || text_l.contains("```rust")
                    || text_l.contains("```bash")
                    || text_l.contains("```cpp")
                    || text_l.contains("from google.colab import")
                    || text_l.contains("drive.mount(")
                    || text_l.contains("np.loadtxt(")
                    || text_l.contains("import numpy as np")
                {
                    return false;
                }
                !EXCLUDED_EXTS.iter().any(|ext| file_l.ends_with(ext))
            });
        }
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

        let mut entity_terms: Vec<String> = query_terms
            .iter()
            .filter(|t| !GENERIC_ATTRIBUTE_TERMS.contains(&t.as_str()))
            .cloned()
            .collect();

        // Extract document targets from LangExtract verified entities
        let doc_targets: Vec<String> = state
            .extracted_entities
            .iter()
            .filter(|e| e.extraction_class == "document_reference")
            .map(|e| e.extraction_text.to_ascii_lowercase())
            .collect();

        // Incorporate LangExtract named entities into entity_terms
        for ext in &state.extracted_entities {
            if ext.extraction_class == "named_entity" || ext.extraction_class == "technical_term" {
                let lower_term = ext.extraction_text.to_ascii_lowercase();
                for sub in lower_term.split(|c: char| !c.is_alphanumeric()) {
                    let s = sub.trim();
                    if s.len() >= 3 && !entity_terms.iter().any(|e| e == s) {
                        entity_terms.push(s.to_string());
                    }
                }
            }
        }

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

            // Compute score criteria once upfront per chunk
            let precompute_criteria = |c: &ScoredChunk| -> ChunkScoreCriteria {
                let text_l = c.chunk.text_content.to_ascii_lowercase();
                let path_l = c.chunk.file_path.to_ascii_lowercase();
                let file_name_l = std::path::Path::new(&c.chunk.file_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();

                let doc_tokens: std::collections::HashSet<&str> = text_l
                    .split(|ch: char| !ch.is_alphanumeric())
                    .chain(file_name_l.split(|ch: char| !ch.is_alphanumeric()))
                    .filter(|w| !w.is_empty())
                    .collect();

                let matches_whole_word = |term: &str| -> bool {
                    if doc_tokens.contains(term) {
                        return true;
                    }
                    // Allow simple plural/singular inflection ("letter" <-> "letters", "school" <-> "schools"),
                    // never arbitrary prefix matches like "cover" matching "coverage" or "discover".
                    if term.len() >= 4 {
                        let plural_s = format!("{}s", term);
                        let plural_es = format!("{}es", term);
                        if doc_tokens.contains(plural_s.as_str())
                            || doc_tokens.contains(plural_es.as_str())
                        {
                            return true;
                        }
                        if let Some(stem) = term
                            .strip_suffix("es")
                            .or_else(|| term.strip_suffix('s'))
                        {
                            if stem.len() >= 4 && doc_tokens.contains(stem) {
                                return true;
                            }
                        }
                    }
                    false
                };

                let mut weighted_hits = 0usize;
                let mut content_matched_terms = 0usize;
                let mut entity_hits = 0usize;

                for t in &query_terms {
                    let is_entity = entity_terms.contains(t);
                    let matches_text_or_name = matches_whole_word(t.as_str());

                    if matches_text_or_name {
                        weighted_hits += 12;
                        content_matched_terms += 1;
                        if is_entity {
                            entity_hits += 1;
                        }
                    } else if path_l.contains(t.as_str()) {
                        weighted_hits += 1;
                    }
                }
                if content_matched_terms >= 2 {
                    weighted_hits += content_matched_terms * 8;
                }

                let mut intent_bonus = 0i32;

                // Google LangExtract: Boost document reference targets +200
                let matches_target_doc = !doc_targets.is_empty()
                    && doc_targets.iter().any(|target| {
                        file_name_l.contains(target.as_str()) || path_l.contains(target.as_str())
                    });

                if matches_target_doc {
                    intent_bonus += 200;
                    weighted_hits += 40;
                    content_matched_terms += 1;
                }

                // Filter out ChatGPT export artifacts from dominating profile queries
                if text_l.contains("fileciteturn0file0") || text_l.contains("sandbox:/mnt/data") {
                    intent_bonus -= 300;
                }

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

                if file_name_l == "stuff.txt" || file_name_l.contains("1095-c") {
                    intent_bonus -= 120;
                }

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
                    if text_l.contains("gpa")
                        || text_l.contains("degree")
                        || text_l.contains("curriculum")
                    {
                        intent_bonus += 40;
                    }
                }

                if asks_profile_or_role
                    || entity_terms
                        .iter()
                        .any(|e| e == "femi" || e == "oyesanya")
                {
                    if file_name_l.contains("resume")
                        || path_l.contains("resume")
                        || file_name_l.contains("cv")
                    {
                        intent_bonus += 160;
                    }
                    if text_l.contains("professional summary")
                        || text_l.contains("summary")
                        || text_l.contains("experience")
                        || text_l.contains("competencies")
                        || text_l.contains("skills")
                        || text_l.contains("certification")
                        || text_l.contains("education")
                    {
                        intent_bonus += 55;
                    }
                }

                // Boost opening / Abstract / Introduction / Overview sections for definitional and entity queries,
                // and deprioritize raw chart axis chunks ("0 20 40 60 80 100") so answers are orchestrated from core prose.
                if c.chunk.line_start <= 45
                    || text_l.contains("abstract")
                    || text_l.contains("introduction")
                    || text_l.contains("professional summary")
                    || text_l.contains("executive summary")
                    || text_l.contains("overview")
                {
                    intent_bonus += 95;
                }
                if text_l.contains("0 20 40 60 80 100") {
                    intent_bonus -= 75;
                }

                if c.chunk.text_content.trim().len() >= 180 {
                    intent_bonus += 20;
                }
                if file_name_l.ends_with(".docx")
                    || file_name_l.ends_with(".pdf")
                    || file_name_l.ends_with(".doc")
                {
                    intent_bonus += 35;
                }
                if !file_name_l.starts_with("recovered_file_") && !file_name_l.starts_with("file_") {
                    intent_bonus += 15;
                }

                // Strictly enforce multi-term entity matching:
                // When 2 specific entity terms are present (e.g. "cover letter", "universal mineral"), require both.
                // When only 1 entity term is paired with a generic attribute term (e.g. "cigna policy"),
                // require the entity term and rank higher if the attribute term also matches.
                let required_entity_hits = if entity_terms.len() == 2 {
                    2
                } else if entity_terms.len() > 2 {
                    ((entity_terms.len() + 1) / 2).max(2).min(entity_terms.len())
                } else if entity_terms.len() == 1 {
                    1
                } else {
                    0
                };
                let required_query_hits = if query_terms.len() >= 2 {
                    ((query_terms.len() + 1) / 2).max(2).min(query_terms.len())
                } else {
                    1
                };

                let is_relevant = (content_matched_terms >= required_query_hits
                    && entity_hits >= required_entity_hits)
                    || matches_target_doc;

                let total_score = (weighted_hits as i32) * 5 + intent_bonus;

                ChunkScoreCriteria {
                    weighted_hits,
                    content_matched_terms,
                    entity_hits,
                    intent_bonus,
                    total_score,
                    is_relevant,
                }
            };

            // Precompute criteria upfront into static structs for all candidates
            let mut paired: Vec<(ScoredChunk, ChunkScoreCriteria)> = chunks
                .into_iter()
                .map(|c| {
                    let crit = precompute_criteria(&c);
                    (c, crit)
                })
                .collect();

            let any_lexical_match = paired.iter().any(|(_, crit)| crit.weighted_hits > 0);
            if any_lexical_match {
                paired.retain(|(_, crit)| crit.is_relevant && crit.weighted_hits > 0 && crit.intent_bonus > -100);

                // Zero string allocation comparator: sorts using precomputed total_score
                paired.sort_by(|(chunk_a, crit_a), (chunk_b, crit_b)| {
                    crit_b
                        .total_score
                        .cmp(&crit_a.total_score)
                        .then_with(|| {
                            chunk_b
                                .score
                                .partial_cmp(&chunk_a.score)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                });
            }

            chunks = paired.into_iter().map(|(c, _)| c).collect();
        }

        chunks.truncate(top_k);
        state.retrieved_chunks = chunks;
        state.volatile_chunks = volatile;
    }
}
