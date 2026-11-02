# Technical Architecture & Handoff Report: Agentic State Graph & Entity-Guided RAG Retrieval

**Agent**: Explorer M11 2 (`teamwork_explorer`)  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m11_2\`  
**Date**: 2026-11-02T14:32:00Z  
**Target Milestone**: Milestone 11 (Google LangExtract & Question-Restating LLM Orchestration — Requirements 1 & 2)  
**Assigned Scope**:
1. `crates/quorum-coordinator/src/types.rs`: Add `langextract_result`, `extracted_questions`, `extracted_entities` to `AgenticGraphState`.
2. `crates/quorum-coordinator/src/agentic/classify.rs`: Execute candidate extraction and 4-arm consensus, populating the state graph.
3. `crates/quorum-coordinator/src/agentic/retrieve.rs`: Steer RAG retrieval using extracted entities (boost document references +200, augment BM25 keyword search, align dense query vectors).
4. `crates/quorum-coordinator/src/agentic/mod.rs`: Verify and wire `QueryCache::insert` and cache-hit fast path with `graph_state.langextract_result`.

---

## 1. Observation

### 1.1 `AgenticGraphState` Definition and Instantiation
- **`crates/quorum-coordinator/src/types.rs:63-85`**:
  ```rust
  pub struct AgenticGraphState {
      pub session_id: Uuid,
      pub original_prompt: String,
      pub current_query: String,
      pub query_history: Vec<String>,
      pub conversation_history: Vec<ChatTurnMessage>,
      pub hop_count: usize,
      pub max_hops: usize, // default: 3
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
  `AgenticGraphState` currently lacks fields to store the verified LangExtract outputs: `langextract_result`, `extracted_questions`, and `extracted_entities`.
- **`crates/quorum-coordinator/src/types.rs:88-112`**:
  `AgenticGraphState::new(prompt: &str, model_family: SynthesisModelFamily, max_hops: usize) -> Self` initializes all default state fields. Every consumer and unit test across the codebase instantiates `AgenticGraphState` exclusively via `AgenticGraphState::new(...)`, guaranteeing that extending `Self` in `types.rs` maintains 100% backward compatibility.

### 1.2 `ClassifyIntentNode` Execution
- **`crates/quorum-coordinator/src/agentic/classify.rs:37-40`**:
  ```rust
  state.intent = Some(intent);
  state.query_metadata_filters =
      crate::langextract::extract_query_metadata_filters(&state.original_prompt);
  ```
  `ClassifyIntentNode::execute` only extracts scalar metadata filters (e.g. `filetype:pdf`). It does NOT run candidate extraction (`extract_chat_candidates`), verbatim span grounding (`ground_extraction`), or 4-arm consensus (`LlmFusionConsensus::evaluate_extractions`) on chat box queries. Consequently, downstream nodes receive an empty extraction state.

### 1.3 `RetrieveMeshNode` Steering & Scoring
- **`crates/quorum-coordinator/src/agentic/retrieve.rs:90-98`**:
  ```rust
  let (mut chunks, volatile, _duration, new_vector_hits) = engine
      .execute_query_with_cached_vector(
          &search_query,
          vector,
          cached_hits_opt,
          access_tier,
          candidate_k,
      )
      .await;
  ```
  `RetrieveMeshNode` accepts `vector: &[f32]` verbatim and does not align dense vector queries with clean, extracted question spans.
- **`crates/quorum-coordinator/src/agentic/retrieve.rs:115-137`**:
  ```rust
  let prelim_entities: Vec<String> = q_low
      .split(|c: char| !c.is_alphanumeric() && c != '_')
      .filter(|w| {
          w.len() >= 3
              && !STOPWORDS.contains(w)
              && !GENERIC_ATTRIBUTE_TERMS.contains(w)
      })
      .map(|w| w.to_string())
      .collect();
  if daemon_active && !prelim_entities.is_empty() {
      let entity_kw_query = prelim_entities.join(" ");
      let entity_hits = engine
          .local_storage
          .search_text(&entity_kw_query, access_tier, 512)
          .await;
  ...
  ```
  BM25 search keyword augmentation is gated on `daemon_active` and naive token splitting, missing verified entity objects (e.g., proper nouns, technical terms, and document reference targets).
- **`crates/quorum-coordinator/src/agentic/retrieve.rs:244-424` (`precompute_criteria`)**:
  Chunk scoring evaluates whole-word matches and profile terms, but lacks a dedicated scoring bonus for user-specified document reference targets (e.g., `ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_Enhanced_Resume_v3.docx`, `Femi_Oyesanya_CV_2026.pdf`, `Resume2026.pdf`). If a user asks a question about a specific file, that file's chunks do not receive an explicit +200 bonus and risk being pruned if general attribute keywords are missing.

### 1.4 `QueryCache` Disconnect in `AgenticStateGraphRunner`
- **`crates/quorum-coordinator/src/cache/query_cache.rs:27, 111`**:
  `CachedQueryEntry` ALREADY defines:
  ```rust
  pub langextract_result: Option<ConsensusExtractionResult>,
  ```
  And `QueryCache::insert` accepts `langextract_result: Option<ConsensusExtractionResult>` as argument 10.
- **`crates/quorum-coordinator/src/agentic/mod.rs:375-390`**:
  ```rust
  self.cache_manager.query_cache.insert(
      &cache_key_query,
      graph_state.search_mode,
      graph_state.model_family,
      access_tier,
      epoch,
      graph_state.retrieved_chunks.clone(),
      crate::offline_engine::DeterministicOfflineSynthesisEngine::build_synthesis_prompt(
          &graph_state.original_prompt,
          &graph_state.retrieved_chunks,
      ),
      graph_state.synthesized_response.clone(),
      graph_state.citations.clone(),
      None, // <--- Passing None here drops the LangExtract consensus result!
      graph_state.query_metadata_filters.clone(),
  );
  ```
  `None` was hardcoded during cache insertion. Furthermore, on cache hits (`agentic/mod.rs:153-160`), `langextract_result`, `extracted_questions`, and `extracted_entities` are not restored onto `graph_state`.

---

## 2. Logic Chain

### 2.1 State Graph Schema Extension (`types.rs`)
1. **Observation**: LangExtract extracts questions (`question`), document targets (`document_reference`), named entities (`named_entity`), and technical terms (`technical_term`), evaluated by `LlmFusionConsensus`.
2. **Deduction**: `AgenticGraphState` must expose:
   - `pub langextract_result: Option<crate::langextract::ConsensusExtractionResult>`: Full 4-arm consensus report, fused confidence, verified extractions, and metadata filters.
   - `pub extracted_questions: Vec<String>`: Isolated question spans for question-restating synthesis and dense embedding alignment.
   - `pub extracted_entities: Vec<crate::langextract::Extraction>`: Grounded entity and document reference objects for RAG retrieval steering.
3. **Safety & Zero-Cost Compatibility**: Initializing these fields to `None`, `Vec::new()`, `Vec::new()` in `AgenticGraphState::new` preserves all existing constructors across test suites and coordinator workflows without signature changes.

### 2.2 Intent Classification & Consensus Execution (`classify.rs`)
1. **Source Text Selection**:
   If multi-turn history expanded `state.current_query` (e.g. pronoun resolution `Where did he go to school?` -> `Where did he go to school? (Femi Oyesanya)`), candidate extraction and grounding must execute against `state.current_query` so prior entities are grounded verbatim. Otherwise, it executes against `state.original_prompt`.
2. **Consensus Pipeline**:
   - `let candidates = crate::langextract::extraction::extract_chat_candidates(source_text);`
   - `let consensus = crate::langextract::LlmFusionConsensus::evaluate_extractions(source_text, candidates, Some(state));`
   - Arm 1 grounds candidate spans; Arm 2 validates bracket and syntax balance; Arm 3 counterfactually rejects ungrounded claims; Arm 4 calibrates against active hop count and relevance score.
3. **Partitioning**:
   - Items with `extraction_class == "question"` populate `state.extracted_questions`.
   - All other items populate `state.extracted_entities`.
   - If no explicit question was verified, default `state.extracted_questions` to `vec![state.original_prompt.trim().to_string()]`.
4. **Metadata Filter Synthesis**:
   Merge scalar metadata filters (`extract_query_metadata_filters`) with `consensus.metadata_filters` so attributes like `file_name` and `filetype` guide retrieval.

### 2.3 Entity-Guided RAG Retrieval Steering (`retrieve.rs`)
1. **Target Document Boosting (+200)**:
   - Extract filenames from `state.extracted_entities` where `extraction_class == "document_reference"`.
   - In `precompute_criteria`, check if candidate chunk `file_name_l` or `path_l` contains any target filename (e.g. `ResumeSec-AI-FemiOyesanya2025.pdf`).
   - If matched:
     - Apply `intent_bonus += 200`.
     - Increase `weighted_hits += 40`.
     - Force `is_relevant = true` so chunks from the requested target file are never pruned by attribute token thresholds.
2. **Tantivy BM25 Keyword Search Augmentation**:
   - Collect terms from `prelim_entities` AND all `state.extracted_entities` (`named_entity`, `technical_term`, `document_reference`).
   - If non-empty, query Tantivy text index: `engine.local_storage.search_text(&entity_kw_query, access_tier, 512).await`.
   - Deduplicate chunk IDs and append to candidate pool. This guarantees exact lexical recall for proper nouns (`Femi`, `Oyesanya`, `Angela`, `Davis`, `RedactX`, `Presidio`) even when dense embeddings are diffused across generic tokens.
3. **Dense Vector Alignment**:
   - If `state.extracted_questions` has a verified question that is cleaner or more focused than `state.current_query`, generate an aligned dense embedding using `quorum_contract::DeterministicOfflineEngine::new().embed_text(primary_q)`.
   - Pass this aligned vector to `engine.execute_query_with_cached_vector`, ensuring LanceDB nearest-neighbor search is anchored strictly on the semantic question.

### 2.4 QueryCache Roundtrip (`agentic/mod.rs`)
1. **On Cache Insert (line 388)**:
   Pass `graph_state.langextract_result.clone()` instead of `None`.
2. **On Cache Hit (line 158)**:
   Restore `graph_state.langextract_result = cached.langextract_result.clone();` and reconstruct `graph_state.extracted_questions` and `graph_state.extracted_entities` from `res.verified_extractions`. This provides instant sub-millisecond retrieval while preserving full LangExtract telemetry.

---

## 3. Caveats

1. **Graceful Fallback When No Entities Are Extracted**:
   If a prompt contains no extractable named entities or document targets (e.g. `"explain scatter gather"`), `state.extracted_entities` is empty. `RetrieveMeshNode` must fall back cleanly to standard hybrid search without altering existing scoring.
2. **Subagent Read-Only Boundary**:
   As an explorer subagent, we do not edit source files directly. We provide exact line numbers, code snippets, and replacement blocks for the implementer (Worker).
3. **Ollama Offline Invariant**:
   All vector embeddings and spellcheckers must continue to work via deterministic offline engines (`quorum_contract::DeterministicOfflineEngine`) so test suites run 100% offline without external network or Ollama daemons.

---

## 4. Conclusion & Concrete Code Specifications

### Specification 1: `crates/quorum-coordinator/src/types.rs`

#### Line 63: Add fields to `AgenticGraphState`
```rust
// In crates/quorum-coordinator/src/types.rs

#[derive(Debug)]
pub struct AgenticGraphState {
    pub session_id: Uuid,
    pub original_prompt: String,
    pub current_query: String,
    pub query_history: Vec<String>,
    pub conversation_history: Vec<ChatTurnMessage>,
    pub hop_count: usize,
    pub max_hops: usize, // default: 3
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
    pub langextract_result: Option<crate::langextract::ConsensusExtractionResult>,
    pub extracted_questions: Vec<String>,
    pub extracted_entities: Vec<crate::langextract::Extraction>,
}
```

#### Line 88: Initialize fields in `AgenticGraphState::new`
```rust
impl AgenticGraphState {
    pub fn new(prompt: &str, model_family: SynthesisModelFamily, max_hops: usize) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            original_prompt: prompt.to_string(),
            current_query: prompt.to_string(),
            query_history: vec![prompt.to_string()],
            conversation_history: Vec::new(),
            hop_count: 0,
            max_hops,
            intent: None,
            search_mode: SearchMode::Hybrid,
            model_family,
            selected_model_tag: String::new(),
            routing_rationale: String::new(),
            retrieved_chunks: Vec::new(),
            volatile_chunks: Vec::new(),
            relevance_grade: None,
            relevance_score: 0.0,
            start_time: Instant::now(),
            synthesized_response: None,
            citations: Vec::new(),
            query_metadata_filters: None,
            cached_vector_hits: Vec::new(),
            langextract_result: None,
            extracted_questions: Vec::new(),
            extracted_entities: Vec::new(),
        }
    }
    // ...
}
```

---

### Specification 2: `crates/quorum-coordinator/src/agentic/classify.rs`

Replace `ClassifyIntentNode::execute` (lines 8–51):
```rust
use crate::hardware::HardwareTelemetry;
use crate::model_router::AutoFusionRouter;
use crate::types::{AgenticGraphState, QueryIntentType};

pub struct ClassifyIntentNode;

impl ClassifyIntentNode {
    pub fn execute(state: &mut AgenticGraphState, hw: &HardwareTelemetry) {
        let lower = state.original_prompt.to_lowercase();
        let is_multi_hop = lower.contains("compare")
            || lower.contains("difference")
            || lower.contains("versus")
            || lower.contains("vs ")
            || lower.contains("relationship");

        let is_mesh = lower.contains("mesh")
            || lower.contains("network")
            || lower.contains("federated")
            || lower.contains("all peers");

        let is_conversation = lower.starts_with("hello")
            || lower.starts_with("hi ")
            || lower == "hi"
            || lower.starts_with("who are you")
            || lower.starts_with("what can you do");

        let intent = if is_conversation {
            QueryIntentType::GeneralConversation
        } else if is_multi_hop {
            QueryIntentType::MultiHopReasoning
        } else if is_mesh {
            QueryIntentType::FederatedMeshSearch
        } else {
            QueryIntentType::LocalDocumentSearch
        };

        state.intent = Some(intent);

        // Feature 58: Google LangExtract Candidate Extraction & 4-Arm Consensus
        let text_for_extraction = if state.current_query != state.original_prompt {
            &state.current_query
        } else {
            &state.original_prompt
        };

        let candidates = crate::langextract::extraction::extract_chat_candidates(text_for_extraction);
        let consensus = crate::langextract::LlmFusionConsensus::evaluate_extractions(
            text_for_extraction,
            candidates,
            Some(state),
        );

        let mut verified_questions = Vec::new();
        let mut verified_entities = Vec::new();

        for ext in &consensus.verified_extractions {
            if ext.extraction_class == "question" {
                verified_questions.push(ext.extraction_text.clone());
            } else {
                verified_entities.push(ext.clone());
            }
        }

        if verified_questions.is_empty() && !state.original_prompt.trim().is_empty() {
            verified_questions.push(state.original_prompt.trim().to_string());
        }

        state.langextract_result = Some(consensus.clone());
        state.extracted_questions = verified_questions;
        state.extracted_entities = verified_entities;

        // Merge scalar metadata filters with LangExtract consensus attributes
        let mut combined_filters = crate::langextract::extract_query_metadata_filters(&state.original_prompt)
            .unwrap_or_default();
        for (k, v) in consensus.metadata_filters {
            combined_filters.insert(k, v);
        }
        if !combined_filters.is_empty() {
            state.query_metadata_filters = Some(combined_filters);
        }

        let (family, tag, rationale) = AutoFusionRouter::select_model(
            state.model_family,
            &state.original_prompt,
            &state.retrieved_chunks,
            hw,
        );
        state.model_family = family;
        state.selected_model_tag = tag;
        state.routing_rationale = rationale;
    }
}
```

---

### Specification 3: `crates/quorum-coordinator/src/agentic/retrieve.rs`

#### A. Dense Vector Alignment (replace lines 64–98):
```rust
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
```

#### B. Tantivy BM25 Keyword Augmentation (replace lines 112–137):
```rust
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
```

#### C. Target Document Boosting (+200) & Entity Weighting in `precompute_criteria` (around line 244 & lines 306–415):
```rust
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
```
Inside `precompute_criteria` (lines 306–420):
```rust
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

                // ... existing image and education and profile bonuses ...

                let is_relevant = (content_matched_terms >= required_query_hits
                    && entity_hits >= required_entity_hits)
                    || matches_target_doc;
```

---

### Specification 4: `crates/quorum-coordinator/src/agentic/mod.rs`

#### A. Cache Hit Restoration (lines 153–160):
```rust
                    if let Some(cached) = maybe_cached {
                        // FAST PATH: Short-circuit entire retrieval, grading, rewrite, and synthesis!
                        graph_state.retrieved_chunks = cached.chunks.clone();
                        graph_state.synthesized_response = cached.synthesized_response.clone();
                        graph_state.citations = cached.citations.clone();
                        graph_state.query_metadata_filters = cached.query_metadata_filters.clone();
                        graph_state.langextract_result = cached.langextract_result.clone();
                        if let Some(ref res) = graph_state.langextract_result {
                            graph_state.extracted_questions = res
                                .verified_extractions
                                .iter()
                                .filter(|e| e.extraction_class == "question")
                                .map(|e| e.extraction_text.clone())
                                .collect();
                            graph_state.extracted_entities = res
                                .verified_extractions
                                .iter()
                                .filter(|e| e.extraction_class != "question")
                                .cloned()
                                .collect();
                        }
```

#### B. Cache Insertion with Consensus Result (line 388):
```rust
                    // Insert into QueryCache
                    let epoch = engine.local_storage.index_epoch();
                    self.cache_manager.query_cache.insert(
                        &cache_key_query,
                        graph_state.search_mode,
                        graph_state.model_family,
                        access_tier,
                        epoch,
                        graph_state.retrieved_chunks.clone(),
                        crate::offline_engine::DeterministicOfflineSynthesisEngine::build_synthesis_prompt(
                            &graph_state.original_prompt,
                            &graph_state.retrieved_chunks,
                        ),
                        graph_state.synthesized_response.clone(),
                        graph_state.citations.clone(),
                        graph_state.langextract_result.clone(),
                        graph_state.query_metadata_filters.clone(),
                    );
```

---

## 5. Verification Method

### 5.1 Verification Commands
1. **Agentic State Graph and Router Tests**:
   ```powershell
   cargo test -p quorum-coordinator --test agentic_tests
   ```
2. **Retrieval Isolation and Document Targeting Tests**:
   ```powershell
   cargo test -p quorum-coordinator --test m8_5_rag_retrieval_isolation_tests
   cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests
   ```
3. **Query Cache & Adversarial Recache Tests**:
   ```powershell
   cargo test -p quorum-coordinator --test adversarial_cache_challenge_tests
   cargo test -p quorum-coordinator --test adversarial_cache_m5_5_2_stress_tests
   ```
4. **End-to-End Coordinator Test Suite**:
   ```powershell
   cargo test -p quorum-coordinator --test coordinator_tests
   ```

### 5.2 Assertions to Verify
1. For any query with a question (e.g. `"Who is femi"`, `"What did Femi do at PatientPoint?"`):
   - `state.extracted_questions` contains the interrogative clause.
   - `state.extracted_entities` contains `named_entity` extractions for `"femi"` / `"patientpoint"`.
   - `state.langextract_result.is_some() == true` with `consensus_passed == true`.
2. For any query targeting a document file (e.g. `"tell me about Angela Davis from ResumeSec-AI-FemiOyesanya2025.pdf"`):
   - `doc_targets` contains `"resumesec-ai-femioyesanya2025.pdf"`.
   - Chunks from `ResumeSec-AI-FemiOyesanya2025.pdf` receive `intent_bonus >= 200` and rank in `retrieved_chunks[0]`.
3. For query cache roundtrips:
   - On second query execution with identical prompt, `maybe_cached` hits.
   - `graph_state.langextract_result` is `Some(...)` matching the original verified consensus.
   - `graph_state.extracted_questions` and `graph_state.extracted_entities` are non-empty and match the original run.

### 5.3 Invalidation Conditions
- If `AgenticGraphState` fails to compile with existing test constructors (compile error on `AgenticGraphState::new`).
- If `doc_targets` matches are pruned by lexical stopwords or generic attribute filters.
- If `QueryCache::insert` passes `None` or drops `langextract_result`.
