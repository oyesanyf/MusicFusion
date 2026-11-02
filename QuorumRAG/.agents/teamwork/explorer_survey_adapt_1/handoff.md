# Technical Architecture & Handoff Report: Google LangExtract & Question-Restating LLM Orchestration (Requirements 1 & 2)

**Agent**: Survey Explorer 1 (`teamwork_explorer`)  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_1\`  
**Date**: 2026-11-02T14:18:00Z  
**Target Milestone**: Requirements 1 & 2 Survey and Architectural Specifications  

---

## 1. Observation

### 1.1 Existing Google LangExtract Subsystem
- **`crates/quorum-coordinator/src/langextract/extraction.rs:5-13`**:
  Defines `Extraction`:
  ```rust
  pub struct Extraction {
      pub extraction_class: String,
      pub extraction_text: String,
      pub char_interval: Option<(usize, usize)>,
      pub attributes: BTreeMap<String, String>,
      pub source_chunk_id: Option<String>,
      pub source_file_path: Option<String>,
  }
  ```
  And verbatim text grounding function `ground_extraction(extraction: &mut Extraction, source_text: &str) -> bool` (lines 57–83) which checks exact substring positions and verifies UTF-8 character boundaries (`source_text.is_char_boundary(start)`).
- **`crates/quorum-coordinator/src/langextract/consensus.rs:27-185`**:
  Implements the 4-Arm `LlmFusionConsensus::evaluate_extractions(source_text: &str, candidates: Vec<Extraction>, graph_state: Option<&AgenticGraphState>) -> ConsensusExtractionResult`:
  - **Arm 1: `SemanticSpanExtractor`**: Verbatim text grounding via `ground_extraction`.
  - **Arm 2: `SyntacticAstVerifier`**: Validates non-empty classes and balanced bracket syntax (`is_balanced_syntax`).
  - **Arm 3: `CounterfactualJudge`**: Adversarial check strictly verifying substring identity against source text (0 hallucinations).
  - **Arm 4: `StateCalibration`**: Calibrates extraction confidence against active `AgenticGraphState` (hop count and relevance score).
- **`crates/quorum-coordinator/src/langextract/mod.rs:13-49`**:
  Only contains `extract_query_metadata_filters(query: &str) -> Option<BTreeMap<String, String>>` parsing scalar filters (`filetype:`, `author:`, `date:`, `tier:`, `tag:`).
  **Missing**: There is currently NO function to extract candidate questions or grounded entities from user chat box queries.
- **`crates/quorum-daemon/src/handlers/langextract.rs:109-177`**:
  Defines a private helper `generate_candidate_extractions(text: &str, file_path: Option<&str>) -> Vec<Extraction>`, but it is scoped strictly to extracting code symbols (`fn`, `struct`) and security advisories (`CVE-`, `RFC-`), not conversational questions or user/document entities.

### 1.2 Agentic State Graph & Query Pipeline
- **`crates/quorum-coordinator/src/types.rs:63-85`**:
  `AgenticGraphState` currently lacks fields to hold LangExtract verified outputs:
  ```rust
  pub struct AgenticGraphState {
      pub session_id: Uuid,
      pub original_prompt: String,
      pub current_query: String,
      pub query_history: Vec<String>,
      pub conversation_history: Vec<ChatTurnMessage>,
      pub hop_count: usize,
      pub max_hops: usize,
      pub intent: Option<QueryIntentType>,
      pub search_mode: SearchMode,
      pub model_family: SynthesisModelFamily,
      pub selected_model_tag: String,
      pub routing_rationale: String,
      pub retrieved_chunks: Vec<ScoredChunk>,
      pub volatile_chunks: Vec<VolatileRemoteChunk>,
      pub relevance_grade: Option<RelevanceGrade>,
      pub relevance_score: f32,
      pub start_time: Instant,
      pub synthesized_response: Option<String>,
      pub citations: Vec<Citation>,
      pub query_metadata_filters: Option<std::collections::BTreeMap<String, String>>,
      pub cached_vector_hits: Vec<ScoredChunk>,
  }
  ```
  Notice: `langextract_result`, `extracted_questions`, and `extracted_entities` are completely missing from `AgenticGraphState`.
- **`crates/quorum-coordinator/src/cache/query_cache.rs:21-33`**:
  In contrast, `CachedQueryEntry` ALREADY has `pub langextract_result: Option<ConsensusExtractionResult>`, and `QueryCache::insert` accepts it at line 111! In `crates/quorum-coordinator/src/agentic/mod.rs:388`, `None` was being passed into `insert` because `AgenticGraphState` did not retain it.
- **`crates/quorum-coordinator/src/agentic/classify.rs:38-39`**:
  `ClassifyIntentNode::execute` only invokes:
  ```rust
  state.query_metadata_filters =
      crate::langextract::extract_query_metadata_filters(&state.original_prompt);
  ```
  It does NOT run candidate extraction, span grounding, or 4-arm consensus over the chat prompt.
- **`crates/quorum-coordinator/src/agentic/retrieve.rs:115-137, 197-213`**:
  `RetrieveMeshNode` relies on naive whitespace/punctuation token splitting (`q_low.split(|c| !c.is_alphanumeric())`) and heuristic `STOPWORDS` / `GENERIC_ATTRIBUTE_TERMS` filtering to guess entity terms. It does not receive verified entity objects, question spans, or document target references from Google LangExtract.

### 1.3 LLM Orchestration & Prompt Formatting
- **`crates/quorum-coordinator/src/offline_engine.rs:708-765` (`query_local_ollama_chat`)**:
  The system prompt currently instructs:
  ```rust
  let base_system = "You are QuorumRAG, an air-gapped local Retrieval-Augmented Generation (RAG) orchestrator. \
      Your ONLY role is to orchestrate a direct, clear, well-structured Markdown answer to the user's question \
      using strictly the Retrieved Local Documents provided below... \
      Rules:\n\
      1. Directly answer the user's question in coherent prose and bullet points using `###` section headings where helpful.\n\
      2. Cite specific facts inline using `[1]`, `[2]`, etc. matching the numbered Retrieved Local Documents.\n\
      3. NEVER dump raw file headers, line numbers, page numbers, or unformatted chart axis numbers (`0 20 40 60 80 100`).\n\
      4. NEVER invent facts outside the provided documents.";
  ```
  **Missing**: There is NO instruction mandating that the response must restate the question (e.g., "Who is femi" -> "Femi is ...").
- **`crates/quorum-coordinator/src/offline_engine.rs:533-586` (`synthesize_with_history` deterministic fallback)**:
  When Ollama is not running (e.g., unit tests or offline environments), the deterministic fallback outputs raw document chunks in sections formatted as:
  ```rust
  sections.push(format!(
      "### 📄 {} (Lines {}–{})\n{} **[{}]**",
      file_name, c.chunk.line_start, c.chunk.line_end, excerpt.trim(), idx + 1
  ));
  ```
  It does NOT restate the user's question, nor does it synthesize a coherent answer prefix.
- **Citations and Links Placement**:
  - In `offline_engine.rs`, the synthesized text body does not include a standardized end-of-response document link block with explicit relevance scores.
  - In `crates/quorum-ui/assets/app.js:720-730`, citation badges are rendered with `href="#"` and only embed the relevance score inside a hidden hover tooltip (`title="... Score: 0.95 ..."`), rather than displaying the score visibly on the badge or link.

---

## 2. Logic Chain

### 2.1 Implementing Requirement 1: Google LangExtract on Chat Inputs
1. **Candidate Extraction for Chat Prompts**:
   A user entering text into the chat box sends natural language queries (e.g. *"Who is femi"*, *"What did Femi Oyesanya do at PatientPoint?"*, *"Tell me about Angela Davis's school from ResumeSec-AI-FemiOyesanya2025.pdf filetype:pdf"*).
   - Candidate questions can be extracted by identifying:
     - Sentences/clauses ending in `?`.
     - Clauses starting with interrogative pronouns/verbs: `who is`, `who was`, `what is`, `what does`, `what are`, `where did`, `where is`, `when did`, `why is`, `how does`, `tell me about`, `explain`, `describe`, `show me`, `list`.
     - If no explicit interrogative marker exists, the primary prompt text is classified as a candidate question.
   - Candidate entities can be extracted by:
     - **`document_reference`**: File paths or filenames with extensions (`.pdf`, `.docx`, `.doc`, `.txt`, `.rs`, `.md`, e.g., `ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_Enhanced_Resume_v3.docx`, `Femi_Oyesanya_CV_2026.pdf`, `Resume2026.pdf`).
     - **`named_entity`**: Proper noun spans (capitalized tokens or multi-token capitalized phrases like `Femi Oyesanya`, `Angela Davis`, `Western Governors University`).
     - **`technical_term`**: Domain terms (`RedactX`, `VaultGemma`, `Presidio`, `RrfCombiner`, `LanceDB`, `Tantivy`, `BLAKE3`, `Ed25519`).
     - **`attribute`**: Predicate concepts (`school`, `education`, `degree`, `resume`, `experience`, `certification`, `policy`).
2. **Grounding & 4-Arm Consensus**:
   - Each candidate extraction is passed through `ground_extraction(&mut ext, prompt)` to guarantee exact character intervals (`char_interval: Some((start, end))`).
   - The extractions are evaluated through `LlmFusionConsensus::evaluate_extractions(prompt, candidates, Some(&state))`.
   - The resulting `ConsensusExtractionResult` contains `verified_extractions`, `arm_reports`, and `metadata_filters`.
3. **Propagation to State & Cache**:
   - Add `langextract_result`, `extracted_questions`, and `extracted_entities` to `AgenticGraphState`.
   - In `ClassifyIntentNode`, run chat candidate extraction and consensus evaluation, populating the state.
   - In `AgenticStateGraphRunner`, pass `graph_state.langextract_result` to `query_cache.insert` and restore it on cache hits.
4. **Passing Extractions to RAG Retrieval (`RetrieveMeshNode`)**:
   - **Target Document Filtering & Boosting**: If an extraction has `extraction_class == "document_reference"`, match chunks against that specific filename and apply an intent bonus (+200).
   - **BM25 Keyword Augmentation**: Pass verified entity terms directly into `local_storage.search_text` to guarantee exact lexical hits for proper nouns.
   - **Dense Vector Query Focus**: If questions were extracted, use the primary extracted question for Gemma 768d embedding generation.
   - **Scoring & RRF Ranking**: Ensure chunks containing verified entities receive top scoring priority over generic distractor files.

### 2.2 Implementing Requirement 2: Question-Restating LLM Orchestration
1. **Enforcing Question-Restatement in Local Ollama Prompt**:
   - In `crates/quorum-coordinator/src/offline_engine.rs:708-765`, the system prompt and user prompt must explicitly mandate:
     ```text
     MANDATORY QUESTION-RESTATING RULE:
     1. You must orchestrate answers ONLY to questions using the provided Retrieved Local Documents.
     2. ALWAYS begin your answer by directly restating the question into an affirmative statement grounded in the retrieved documents (e.g., "Who is femi" must begin with "Femi is [or Femi Oyesanya is] ..."; "What is RedactX" must begin with "RedactX is ..."; "Where did Femi study" must begin with "Femi studied at ...").
     3. NEVER begin with conversational filler, meta-announcements, or greetings (e.g. do NOT say "Based on the documents...", "Sure!", "According to the resume..."). Immediately begin with the restated answer.
     4. Ground all claims strictly in the retrieved documents (e.g. ResumeSec-AI-FemiOyesanya2025.pdf, Femi_Oyesanya_Enhanced_Resume_v3.docx, Femi_Oyesanya_CV_2026.pdf, Resume2026.pdf).
     5. All document links, URLs, and relevance scores must be placed strictly at the end of the response.
     ```
2. **Deterministic Offline Synthesis Question-Restater**:
   - For offline test runs or when Ollama is offline, add a deterministic formatter `format_question_restated_answer(prompt: &str, chunks: &[ScoredChunk]) -> String`.
   - Parses the question prefix:
     - `"who is <entity>"` -> `"<Entity> is <role/title/summary from chunk 1>."`
     - `"what is <concept>"` -> `"<Concept> is <definition from chunk 1>."`
     - `"where did <entity> study / attend school"` -> `"<Entity> attended <school from chunk 1>."`
     - `"what does <entity> do"` -> `"<Entity> is <role from chunk 1>."`
   - Incorporates verbatim grounded excerpts from top RAG chunks (`ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_CV_2026.pdf`, etc.).
3. **Strict End-of-Response Document Links & Relevance Scores**:
   - At the bottom of the synthesized response text, append:
     ```markdown
     ---
     ### 📚 Sources & Document Links
     - [📄 ResumeSec-AI-FemiOyesanya2025.pdf](file:///d:/harfile/QuorumRAG/ResumeSec-AI-FemiOyesanya2025.pdf) — **Relevance Score: 0.95** (Lines 1–45)
     - [📄 Femi_Oyesanya_CV_2026.pdf](file:///d:/harfile/QuorumRAG/Femi_Oyesanya_CV_2026.pdf) — **Relevance Score: 0.91** (Lines 1–60)
     ```
   - In `crates/quorum-ui/assets/app.js`:
     - Update `appendCitation(citationsEl, cit)` to visibly display the relevance score:
       `📄 ${fileName} [Score: ${Math.round((cit.score || 0) * 100)}%] :${cit.line_start}-${cit.line_end}`
     - Ensure the badge links to a document viewer or file path.
     - Citations container renders strictly below the response body.

---

## 3. Caveats

1. **Air-Gapped Loopback Invariant**:
   All synthesis requests to Ollama dial strictly `127.0.0.1:11434` (enforcing loopback containment). No external API calls may be made.
2. **Deterministic Offline Fallback**:
   In environments where Ollama is not installed or offline, deterministic offline synthesis must produce structurally identical question-restated answers and end-of-response citation blocks so unit tests and benchmarks pass with 100% precision.
3. **Coreference and Multi-turn Context**:
   When a user asks follow-up questions (e.g. *"Where did he go to school?"*), `AgenticGraphState::expand_followup_query` resolves pronouns by appending prior subject terms. Candidate extraction in LangExtract must be run on the expanded query so entities like *"Femi"* are successfully grounded and restated in the answer.

---

## 4. Conclusion & Concrete Implementation Specifications

### Proposed Code Modifications

#### 1. `crates/quorum-coordinator/src/langextract/extraction.rs`
Add `extract_chat_candidates`:
```rust
/// Extracts candidate questions and source-grounded entities from a chat box query string.
pub fn extract_chat_candidates(prompt: &str) -> Vec<Extraction> {
    let mut candidates = Vec::new();
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return candidates;
    }

    // 1. Extract Questions
    let mut found_question = false;
    for segment in trimmed.split(|c| c == '?' || c == '.' || c == '\n') {
        let seg_trim = segment.trim();
        if seg_trim.is_empty() {
            continue;
        }
        let lower = seg_trim.to_ascii_lowercase();
        let is_q = lower.starts_with("who ")
            || lower.starts_with("what ")
            || lower.starts_with("where ")
            || lower.starts_with("when ")
            || lower.starts_with("why ")
            || lower.starts_with("how ")
            || lower.starts_with("tell me ")
            || lower.starts_with("explain ")
            || lower.starts_with("describe ")
            || lower.starts_with("list ")
            || lower.starts_with("can you ");
        if is_q {
            candidates.push(
                Extraction::new("question", seg_trim)
                    .with_attribute("type", "interrogative_clause")
            );
            found_question = true;
        }
    }
    if !found_question {
        candidates.push(
            Extraction::new("question", trimmed)
                .with_attribute("type", "full_prompt_question")
        );
    }

    // 2. Extract Document References (e.g. .pdf, .docx, .doc, .txt, .rs)
    for token in trimmed.split_whitespace() {
        let clean_tok = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '-');
        let lower_tok = clean_tok.to_ascii_lowercase();
        if lower_tok.ends_with(".pdf")
            || lower_tok.ends_with(".docx")
            || lower_tok.ends_with(".doc")
            || lower_tok.ends_with(".txt")
            || lower_tok.ends_with(".rs")
            || lower_tok.ends_with(".md")
        {
            candidates.push(
                Extraction::new("document_reference", clean_tok)
                    .with_attribute("file_name", clean_tok)
            );
        }
    }

    // 3. Extract Named Entities and Proper Nouns
    // Capitalized terms or known profile keywords
    for word in trimmed.split_whitespace() {
        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
        if clean_word.len() >= 3 {
            let is_capitalized = clean_word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
            let lower_word = clean_word.to_ascii_lowercase();
            let is_known_entity = lower_word == "femi"
                || lower_word == "oyesanya"
                || lower_word == "redactx"
                || lower_word == "vaultgemma"
                || lower_word == "presidio"
                || lower_word == "patientpoint"
                || lower_word == "cigna";

            if is_capitalized || is_known_entity {
                candidates.push(
                    Extraction::new("named_entity", clean_word)
                        .with_attribute("canonical", &lower_word)
                );
            }
        }
    }

    candidates
}
```

#### 2. `crates/quorum-coordinator/src/types.rs`
Add fields to `AgenticGraphState`:
```rust
pub struct AgenticGraphState {
    // ... existing fields ...
    pub langextract_result: Option<crate::langextract::ConsensusExtractionResult>,
    pub extracted_questions: Vec<String>,
    pub extracted_entities: Vec<crate::langextract::Extraction>,
}
```
And initialize in `AgenticGraphState::new`:
```rust
    langextract_result: None,
    extracted_questions: Vec::new(),
    extracted_entities: Vec::new(),
```

#### 3. `crates/quorum-coordinator/src/agentic/classify.rs`
Update `ClassifyIntentNode::execute`:
```rust
    // Run Google LangExtract on the prompt
    let mut candidates = crate::langextract::extraction::extract_chat_candidates(&state.original_prompt);
    let mut verified_questions = Vec::new();
    let mut verified_entities = Vec::new();

    let consensus = crate::langextract::LlmFusionConsensus::evaluate_extractions(
        &state.original_prompt,
        candidates,
        Some(state),
    );

    for ext in &consensus.verified_extractions {
        if ext.extraction_class == "question" {
            verified_questions.push(ext.extraction_text.clone());
        } else {
            verified_entities.push(ext.clone());
        }
    }

    state.langextract_result = Some(consensus.clone());
    state.extracted_questions = verified_questions;
    state.extracted_entities = verified_entities;

    // Merge scalar metadata filters
    let mut combined_filters = crate::langextract::extract_query_metadata_filters(&state.original_prompt)
        .unwrap_or_default();
    for (k, v) in consensus.metadata_filters {
        combined_filters.insert(k, v);
    }
    if !combined_filters.is_empty() {
        state.query_metadata_filters = Some(combined_filters);
    }
```

#### 4. `crates/quorum-coordinator/src/agentic/retrieve.rs`
In `RetrieveMeshNode::execute`:
- Check `state.extracted_entities` for `document_reference`:
  ```rust
  let doc_refs: Vec<String> = state.extracted_entities.iter()
      .filter(|e| e.extraction_class == "document_reference")
      .map(|e| e.extraction_text.to_ascii_lowercase())
      .collect();
  ```
- If `doc_refs` is non-empty, boost chunks from these files by +200 in `ChunkScoreCriteria`.
- For BM25 keyword retrieval, pass entity terms extracted by LangExtract directly to `engine.local_storage.search_text`.

#### 5. `crates/quorum-coordinator/src/offline_engine.rs`
1. **Ollama Chat Prompt**:
   In `query_local_ollama_chat`, update `base_system` and `user_content`:
   ```rust
   let base_system = "You are QuorumRAG, an air-gapped local Retrieval-Augmented Generation (RAG) orchestrator. \
       Your ONLY role is to orchestrate a direct, clear, well-structured Markdown answer to the user's question \
       using strictly the Retrieved Local Documents provided below. \
       MANDATORY RULES:\n\
       1. QUESTION-RESTATING: Always begin your answer by directly restating the question into an affirmative statement grounded in the retrieved documents (e.g., 'Who is femi' must begin with 'Femi is [or Femi Oyesanya is] ...'; 'What is X' must begin with 'X is ...'; 'Where did X attend school' must begin with 'X attended ...'). Never begin with meta greetings or robotic filler.\n\
       2. Ground all claims strictly in the retrieved documents (e.g., ResumeSec-AI-FemiOyesanya2025.pdf, Femi_Oyesanya_Enhanced_Resume_v3.docx, Femi_Oyesanya_CV_2026.pdf, Resume2026.pdf).\n\
       3. Cite specific facts inline using [1], [2], etc. matching the numbered Retrieved Local Documents.\n\
       4. All document links, URLs, and relevance scores must be placed strictly at the end of the response in a dedicated '### Sources & Document Links' section.\n\
       5. NEVER invent facts outside the provided documents.";
   ```
2. **Deterministic Offline Synthesis**:
   In `synthesize_with_history`, implement question-restating opening logic:
   ```rust
   pub fn format_question_restated_prefix(prompt: &str) -> String {
       let lower = prompt.trim().to_ascii_lowercase();
       if let Some(subject) = lower.strip_prefix("who is ") {
           let clean_sub = capitalize_words(subject.trim_end_matches('?'));
           format!("{} is ", clean_sub)
       } else if let Some(subject) = lower.strip_prefix("what is ") {
           let clean_sub = capitalize_words(subject.trim_end_matches('?'));
           format!("{} is ", clean_sub)
       } else if let Some(subject) = lower.strip_prefix("what does ") {
           let clean_sub = capitalize_words(subject.trim_end_matches('?').split_whitespace().next().unwrap_or(""));
           format!("{} ", clean_sub)
       } else if lower.contains("school") || lower.contains("education") || lower.contains("university") {
           "Education and academic background: ".to_string()
       } else {
           String::new()
       }
   }
   ```
   And append the formatted document links and relevance scores block at the end:
   ```rust
   let mut doc_links_block = String::from("\n\n---\n### 📚 Sources & Document Links\n");
   for c in &matched_chunks {
       let file_name = std::path::Path::new(&c.chunk.file_path)
           .file_name()
           .and_then(|n| n.to_str())
           .unwrap_or(&c.chunk.file_path);
       doc_links_block.push_str(&format!(
           "- [📄 {}]({}) — **Relevance Score: {:.2}** (Lines {}–{})\n",
           file_name,
           c.chunk.file_path,
           c.score,
           c.chunk.line_start,
           c.chunk.line_end
       ));
   }
   ```

#### 6. `crates/quorum-ui/assets/app.js`
In `appendCitation(citationsEl, cit)`:
```javascript
const fileName = String(cit.file_path || '').split(/[\\/]/).pop() || cit.file_path;
const scorePct = Math.round((Number(cit.score) || 0) * 100);
const badge = document.createElement('a');
badge.className = 'citation-badge';
badge.href = cit.file_path ? `file:///${cit.file_path.replace(/\\/g, '/')}` : '#';
badge.title = `${cit.file_path} (Lines ${cit.line_start}-${cit.line_end})\nRelevance Score: ${(Number(cit.score) || 0).toFixed(3)}\n${cit.snippet || ''}`;
badge.innerHTML = `📄 ${escapeHtml(fileName)} <span class="score-pill" style="margin-left:4px;padding:1px 5px;border-radius:4px;background:rgba(16,163,127,0.2);font-weight:600;font-size:0.85em;">${scorePct}%</span> <span style="opacity:0.7;">:${cit.line_start}-${cit.line_end}</span>`;
```

---

## 5. Verification Method

### Concrete Verification Steps
1. **Unit & Integration Tests**:
   - `cargo test -p quorum-coordinator --test agentic_tests`
   - `cargo test -p quorum-coordinator --test coordinator_tests`
   - `cargo test -p quorum-daemon --test daemon_tests`
   - `cargo test --test benchmarks_r4 -- test_benchmark_7_rag_precision`
2. **Code Inspection**:
   - Verify `extract_chat_candidates` is present in `crates/quorum-coordinator/src/langextract/extraction.rs`.
   - Verify `AgenticGraphState` in `crates/quorum-coordinator/src/types.rs` contains `langextract_result`, `extracted_questions`, and `extracted_entities`.
   - Verify `ClassifyIntentNode::execute` invokes candidate extraction and consensus evaluation.
   - Verify `RetrieveMeshNode::execute` consumes the verified entities.
   - Verify `DeterministicOfflineSynthesisEngine::synthesize_with_history` and `query_local_ollama_chat` format question-restated answers and place document links and relevance scores strictly at the bottom.
3. **Invalidation Conditions**:
   - If a prompt like `"Who is femi"` returns an answer that does NOT begin with `"Femi is ..."` or conversational meta-filler is outputted.
   - If document citations and relevance scores appear interleaved in the middle of answer text instead of strictly grouped at the end.
   - If candidate extractions are generated without verbatim character intervals or fail 4-arm consensus.
