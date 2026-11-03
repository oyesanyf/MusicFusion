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
    pub prompt_entity_hits: usize,
    pub intent_bonus: i32,
    pub total_score: i32,
    pub is_relevant: bool,
    /// True when every user-capitalised proper-noun anchor (or all but one when 3+) appears in the chunk.
    pub proper_ok: bool,
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
    "pay", "pays", "paid", "paying", "cost", "costs", "price", "prices", "fee", "fees",
    "start", "started", "starts", "begin", "began", "end", "ended", "ends", "date", "dates",
    "important", "importance", "result", "results", "respond", "responds", "response",
];

const STOPWORDS: &[&str] = &[
    "what", "does", "do", "did", "where", "when", "who", "how", "why",
    "is", "are", "was", "were", "the", "and", "for", "with", "from",
    "into", "about", "using", "have", "has", "had", "will", "would",
    "should", "could", "can", "you", "tell", "show", "went", "attend", "attended",
    "disambiguation", "hop", "more", "else", "also", "his", "her", "hers", "him",
    "she", "they", "them", "their", "theirs", "there", "that", "this", "these", "those",
    "file", "files", "document", "documents", "doc", "docs", "docx", "pdf", "txt",
    "folder", "folders", "content", "contents", "text", "exist", "exists", "existing",
    "find", "check", "list", "located", "location", "explain", "describe", "give", "please",
    "know", "knows", "knew", "known", "think", "thinks", "thought", "mean", "means", "meant",
    "say", "says", "said", "talk", "talks", "talked", "need", "needs", "want", "wants",
    "like", "likes", "make", "makes", "made", "take", "takes", "took", "come", "comes",
    "came", "look", "looks", "looked", "summarize", "detail", "details", "info", "information",
    "anything", "everything", "something", "nothing", "someone", "anyone", "all", "any", "some",
    "much", "many", "often", "long", "kind", "type", "sort", "way", "get", "gets", "got", "use",
    "uses", "used", "which", "whose", "whom", "its", "our", "your", "my", "me", "one", "ones",
    "not", "than", "then", "very", "just", "only", "each", "every", "both", "either", "other",
    "others", "own", "same", "such", "been", "being", "off", "out", "over", "under", "again",
    "here", "now", "still", "yet", "ever", "even",
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

        let (corrected_q, _) = crate::offline_engine::DeterministicOfflineSynthesisEngine::correct_query_spelling(
            &state.current_query,
            false,
        );
        if !corrected_q.trim().is_empty() {
            state.current_query = corrected_q;
        }
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
        } else if q_low.contains("who is")
            || q_low.contains("resume")
            || q_low.contains("cv")
            || q_low.contains("femi")
            || q_low.contains("oyesanya")
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

        tracing::info!(query = %search_query, "Calling execute_query_with_cached_vector");
        let (mut chunks, volatile, _duration, new_vector_hits) = engine
            .execute_query_with_cached_vector(
                &search_query,
                effective_vector,
                cached_hits_opt,
                access_tier,
                candidate_k,
            )
            .await;
        tracing::info!(chunks = chunks.len(), volatile = volatile.len(), "Finished execute_query_with_cached_vector");

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

        let orig_low = state.original_prompt.to_ascii_lowercase();
        let prompt_only_entities: Vec<String> = orig_low
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| {
                w.len() >= 3
                    && !STOPWORDS.contains(w)
                    && !GENERIC_ATTRIBUTE_TERMS.contains(w)
            })
            .fold(Vec::new(), |mut acc, w| {
                let s = w.to_string();
                if !acc.contains(&s) {
                    acc.push(s);
                }
                acc
            });

        // Proper-noun anchors: words the user capitalised mid-sentence ("how much rent does Casey Ringgold pay").
        // When present, a chunk must contain them, so generic words ("rent", "pay") can never outvote the named subject.
        let proper_terms: Vec<String> = state
            .original_prompt
            .split_whitespace()
            .enumerate()
            .filter_map(|(i, raw)| {
                if i == 0 {
                    return None;
                }
                let w = raw
                    .trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                    .trim_end_matches("'s")
                    .trim_end_matches("\u{2019}s")
                    .trim_matches(|c: char| !c.is_alphanumeric());
                let first_upper = w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
                let lw = w.to_ascii_lowercase();
                if first_upper
                    && lw.len() >= 3
                    && !STOPWORDS.contains(&lw.as_str())
                    && !GENERIC_ATTRIBUTE_TERMS.contains(&lw.as_str())
                {
                    Some(lw)
                } else {
                    None
                }
            })
            .fold(Vec::new(), |mut acc, w| {
                if !acc.contains(&w) {
                    acc.push(w);
                }
                acc
            });

        if (daemon_active || !state.extracted_entities.is_empty()) && !prelim_entities.is_empty() {
            let mut seen_ids: std::collections::HashSet<String> =
                chunks.iter().map(|c| c.chunk.chunk_id.clone()).collect();

            let entity_kw_query = prelim_entities.join(" ");
            let entity_hits = engine
                .local_storage
                .search_text(&entity_kw_query, access_tier, 96)
                .await;
            for hit in entity_hits {
                if seen_ids.insert(hit.chunk.chunk_id.clone()) {
                    chunks.push(hit);
                }
            }

            // When a follow-up query introduces its own specific entity term (e.g. "does he have cissp"),
            // run an explicit search for that term so documents containing it are never drowned out by prior-turn terms.
            if !prompt_only_entities.is_empty() && prompt_only_entities != prelim_entities {
                let prompt_kw_query = prompt_only_entities.join(" ");
                let prompt_hits = engine
                    .local_storage
                    .search_text(&prompt_kw_query, access_tier, 64)
                    .await;
                for hit in prompt_hits {
                    if seen_ids.insert(hit.chunk.chunk_id.clone()) {
                        chunks.push(hit);
                    }
                }
            }

            // Answer-type targeted search: "where does X live" -> X + address/street/phone, "how much ... X" -> X + rent/amount,
            // so the excerpt holding the requested kind of fact is in the candidate pool, not just excerpts naming X.
            let cue_terms = crate::offline_engine::DeterministicOfflineSynthesisEngine::answer_type_cue_terms(&state.original_prompt);
            if !cue_terms.is_empty() && !prompt_only_entities.is_empty() {
                let cue_query = format!("{} {}", prompt_only_entities.join(" "), cue_terms.join(" "));
                let cue_hits = engine.local_storage.search_text(&cue_query, access_tier, 48).await;
                for hit in cue_hits {
                    if seen_ids.insert(hit.chunk.chunk_id.clone()) {
                        chunks.push(hit);
                    }
                }
            }

            // When querying about Femi Oyesanya, also pull resume/CV summary chunks across all resume documents
            if q_low.contains("femi") || q_low.contains("oyesanya") {
                let resume_hits = engine
                    .local_storage
                    .search_text(
                        "Femi Oyesanya CISSP PROFESSIONAL SUMMARY Resume CV",
                        access_tier,
                        64,
                    )
                    .await;
                for hit in resume_hits {
                    if seen_ids.insert(hit.chunk.chunk_id.clone()) {
                        chunks.push(hit);
                    }
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
                if crate::offline_engine::DeterministicOfflineSynthesisEngine::is_corrupt_or_binary_chunk(
                    &c.chunk.text_content,
                ) {
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
                let mut prompt_entity_hits = 0usize;

                for t in &query_terms {
                    let is_entity = entity_terms.contains(t);
                    let is_prompt_entity = prompt_only_entities.contains(t);
                    let matches_text_or_name = matches_whole_word(t.as_str());

                    if matches_text_or_name {
                        weighted_hits += 12;
                        content_matched_terms += 1;
                        if is_entity {
                            entity_hits += 1;
                        }
                        if is_prompt_entity {
                            prompt_entity_hits += 1;
                        }
                    } else if path_l.contains(t.as_str()) {
                        weighted_hits += 1;
                    }
                }
                if content_matched_terms >= 2 {
                    weighted_hits += content_matched_terms * 8;
                }

                let proper_hits = proper_terms
                    .iter()
                    .filter(|t| matches_whole_word(t.as_str()))
                    .count();

                let mut intent_bonus = (prompt_entity_hits as i32) * 320;

                // Boost chunks whose filename directly matches query terms (e.g. "African Churches and Gang Violence.docx")
                let file_tokens: std::collections::HashSet<&str> = file_name_l
                    .split(|ch: char| !ch.is_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .collect();
                let file_term_hits = query_terms
                    .iter()
                    .filter(|t| {
                        file_tokens.contains(t.as_str())
                            || (t.len() >= 4
                                && (file_tokens.contains(format!("{}s", t).as_str())
                                    || file_tokens.contains(format!("{}es", t).as_str())))
                    })
                    .count();
                if file_term_hits > 0 {
                    intent_bonus += (file_term_hits as i32) * 180;
                    weighted_hits += file_term_hits * 25;
                }

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
                        intent_bonus += 260;
                    }
                    if text_l.contains("professional summary")
                        || text_l.contains("cissp-certified")
                        || text_l.contains("summary")
                        || text_l.contains("experience")
                        || text_l.contains("competencies")
                        || text_l.contains("skills")
                        || text_l.contains("certification")
                        || text_l.contains("education")
                    {
                        intent_bonus += 90;
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

                let required_proper_hits = if proper_terms.len() > 2 {
                    proper_terms.len() - 1
                } else {
                    proper_terms.len()
                };
                let proper_ok = proper_hits >= required_proper_hits;
                let is_relevant = ((content_matched_terms >= required_query_hits
                    && entity_hits >= required_entity_hits)
                    || (proper_ok && !proper_terms.is_empty() && content_matched_terms >= proper_terms.len()))
                    || matches_target_doc;

                intent_bonus += crate::offline_engine::DeterministicOfflineSynthesisEngine::answer_type_cue_score(
                    &state.original_prompt,
                    &text_l,
                );
                let total_score = (weighted_hits as i32) * 5 + intent_bonus;

                ChunkScoreCriteria {
                    weighted_hits,
                    content_matched_terms,
                    entity_hits,
                    prompt_entity_hits,
                    intent_bonus,
                    total_score,
                    is_relevant,
                    proper_ok,
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
            let any_proper_match = !proper_terms.is_empty()
                && paired.iter().any(|(_, crit)| crit.is_relevant && crit.proper_ok);
            let required_prompt_hits = if prompt_only_entities.len() == 2 {
                2
            } else if prompt_only_entities.len() > 2 {
                ((prompt_only_entities.len() + 1) / 2).max(2)
            } else {
                prompt_only_entities.len()
            };
            let any_prompt_entity_match = !prompt_only_entities.is_empty()
                && paired
                    .iter()
                    .any(|(_, crit)| crit.is_relevant && crit.prompt_entity_hits >= required_prompt_hits);
            if any_lexical_match {
                paired.retain(|(_, crit)| {
                    crit.is_relevant
                        && crit.weighted_hits > 0
                        && crit.intent_bonus > -100
                        && (!any_prompt_entity_match || crit.prompt_entity_hits >= required_prompt_hits)
                        && (!any_proper_match || crit.proper_ok)
                });

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

        let ui = crate::dlp_mesh::global_mesh_dlp_controller()
            .settings
            .read()
            .map(|g| g.ui.clone())
            .unwrap_or_default();

        if ui.parent_document_retrieval {
            chunks = crate::offline_engine::DeterministicOfflineSynthesisEngine::stitch_parent_document_chunks(chunks);
        }

        chunks.truncate(top_k);
        state.retrieved_chunks = chunks;
        state.volatile_chunks = volatile;
    }
}
