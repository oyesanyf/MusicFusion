# Technical Architecture & Handoff Report: Google LangExtract Chat Candidate Extraction & 4-Arm Consensus

**Agent**: Explorer M11 1 (`teamwork_explorer`)  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m11_1\`  
**Date**: 2026-11-02T14:35:00Z  
**Target Milestone**: Milestone 11 (Google LangExtract Chat Extraction & 4-Arm Consensus — Feature 58 & Requirement 1)  
**Assigned Scope**:
1. `crates/quorum-coordinator/src/langextract/extraction.rs`:
   - Design and verify the exact implementation of `extract_chat_candidates(prompt: &str) -> Vec<Extraction>` for extracting questions, document references (e.g. `ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_Enhanced_Resume_v3.docx`, `Femi_Oyesanya_CV_2026.pdf`, `Resume2026.pdf`), and named entities/technical terms.
   - Verify character interval grounding (`ground_extraction`) and 4-arm consensus in `consensus.rs`.
2. Unit tests in `crates/quorum-coordinator/` to verify extraction precision.

---

## 1. Observation

### 1.1 Existing Subsystem in `crates/quorum-coordinator/src/langextract/`
1. **`crates/quorum-coordinator/src/langextract/extraction.rs:5-49`**:
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
   Provides builder methods `Extraction::new`, `with_source`, `with_attribute`, and `with_interval`.
2. **`crates/quorum-coordinator/src/langextract/extraction.rs:57-83`**:
   Implements verbatim text grounding `ground_extraction(extraction: &mut Extraction, source_text: &str) -> bool`:
   ```rust
   pub fn ground_extraction(extraction: &mut Extraction, source_text: &str) -> bool {
       if extraction.extraction_text.is_empty() {
           extraction.char_interval = None;
           return false;
       }

       // 1. Verbatim exact byte match
       if let Some(start) = source_text.find(&extraction.extraction_text) {
           let end = start + extraction.extraction_text.len();
           extraction.char_interval = Some((start, end));
           return true;
       }

       // 2. Case-insensitive substring fallback
       let source_lower = source_text.to_lowercase();
       let target_lower = extraction.extraction_text.to_lowercase();
       if let Some(start) = source_lower.find(&target_lower) {
           let end = start + target_lower.len();
           if source_text.is_char_boundary(start) && source_text.is_char_boundary(end) {
               extraction.char_interval = Some((start, end));
               return true;
           }
       }

       extraction.char_interval = None;
       false
   }
   ```
   Lines 86–91 define `filter_grounded(extractions: Vec<Extraction>) -> Vec<Extraction>`.
   **Critical Defect**: There is currently NO function to extract candidate questions or grounded entities from user chat box queries.
3. **`crates/quorum-coordinator/src/langextract/consensus.rs:34-185`**:
   Implements `LlmFusionConsensus::evaluate_extractions(source_text: &str, mut candidates: Vec<Extraction>, graph_state: Option<&AgenticGraphState>) -> ConsensusExtractionResult`:
   - **Arm 1: `SemanticSpanExtractor`** (lines 43–66): Verbatim text grounding via `ground_extraction(ext, source_text)`. Evaluates `arm1_passed = total_candidates > 0 && arm1_grounded_count == total_candidates`.
   - **Arm 2: `SyntacticAstVerifier`** (lines 69–95): Schema validation checking `!ext.extraction_class.trim().is_empty() && !ext.extraction_text.trim().is_empty()` and `is_balanced_syntax(&ext.extraction_text)`. Evaluates `arm2_passed = total_candidates > 0 && arm2_valid_count == total_candidates`.
   - **Arm 3: `CounterfactualJudge`** (lines 98–126): Adversarial check verifying that `ext.char_interval == Some((start, end))` satisfies `start <= end && end <= source_text.len()` and `source_text[start..end].eq_ignore_ascii_case(&ext.extraction_text)`. Evaluates `arm3_passed = total_candidates > 0 && arm3_passed_items.len() == total_candidates`.
   - **Arm 4: `StateCalibration`** (lines 129–158): Calibrates confidence with `AgenticGraphState` (verifying `relevance_score >= 0.0` and `hop_count <= max_hops`). Standalone queries pass with 9500 bp (100%).
   - **Consensus Gate** (lines 160–167):
     ```rust
     let fused_confidence_bp = ((arm1_confidence as u32
         + arm2_confidence as u32
         + arm3_confidence as u32
         + arm4_confidence as u32)
         / 4) as u16;
     let consensus_passed =
         arm1_passed && arm2_passed && arm3_passed && arm4_passed && fused_confidence_bp >= 8500;
     ```
   - **Arm Invariant**: If even a single candidate fails grounding in `source_text` or fails syntax/counterfactual verification, `arm1_passed` or `arm3_passed` drops to `false`, causing `consensus_passed` to be `false`! Every candidate produced by `extract_chat_candidates` MUST be an exact verbatim substring of `prompt`.
4. **`crates/quorum-coordinator/src/langextract/mod.rs:13-49`**:
   Only contains `extract_query_metadata_filters(query: &str) -> Option<BTreeMap<String, String>>` parsing `filetype:`, `author:`, `date:`, `tier:`, `tag:`. It does not export `extract_chat_candidates`.
5. **Existing Tests in `crates/quorum-coordinator/tests/coordinator_tests.rs:433-471`**:
   `test_langextract_4arm_consensus_and_adversarial_rejection` verifies genuine extractions and confirms adversarial rejection of fake extractions (`QuantumEncryption128`).
   Command run: `cargo test -p quorum-coordinator --test coordinator_tests` passes (12 tests passed, 0 failed in 0.41s).
   Command run: `cargo test -p quorum-coordinator --test agentic_tests` passes (4 tests passed, 0 failed in 0.33s).

---

## 2. Logic Chain

### 2.1 The Verbatim Grounding Invariant
- **Observation**: Arm 1 invokes `ground_extraction(ext, source_text)`. Arm 3 checks `source_text[start..end].eq_ignore_ascii_case(&ext.extraction_text)`.
- **Deduction**: If candidate extraction invents or hallucinates any token not present verbatim in `prompt`, Arm 1 and Arm 3 will fail, and `consensus_passed` becomes `false`.
- **Engineering Principle**: Every candidate extraction's `extraction_text` MUST be extracted directly as a slice/substring of `prompt`. Normalizations (e.g. lowercase canonical forms) must be stored in `attributes` (e.g. `attributes.insert("canonical", lower)`), NEVER in `extraction_text`.

### 2.2 Clause Splitting Without Breaking Filenames or Decimals
- **Problem**: Splitting naively on `.` breaks filenames like `ResumeSec-AI-FemiOyesanya2025.pdf` into `ResumeSec-AI-FemiOyesanya2025` and `pdf`, and breaks numbers like `v3.0`.
- **Resolution**: A period `.` is a sentence delimiter ONLY if it is followed by whitespace, newline, or end-of-string, AND is not part of a recognized file extension (`.pdf`, `.docx`, etc.). Delimiters `?`, `\n`, `;`, and `!` are always sentence boundaries.
- **Trace**:
  `"Tell me about Femi from ResumeSec-AI-FemiOyesanya2025.pdf. What did he do at Cigna?"`
  - In `ResumeSec-AI-FemiOyesanya2025.pdf`, the dot before `pdf` is followed by `'p'`, so it is not a boundary.
  - The dot after `pdf` is followed by `' '`, so it is a boundary.
  - Produces two clean clauses: `"Tell me about Femi from ResumeSec-AI-FemiOyesanya2025.pdf"` and `"What did he do at Cigna"`. Both are verbatim substrings of `prompt`.

### 2.3 Question Extraction
- **Rules**:
  1. Any clause starting with an interrogative prefix (`who `, `what `, `where `, `when `, `why `, `how `, `tell me `, `explain `, `describe `, `show me `, `list `, `can you `, `could you `, `is `, `are `, `was `, `were `, `does `, `did `, `do `, `which `) is extracted with `extraction_class = "question"` and attribute `type = "interrogative_clause"`.
  2. Any clause that ended with `?` in the prompt is extracted as a question.
  3. If no clause matches either rule, the full trimmed prompt is extracted with attribute `type = "full_prompt_question"`.
- This guarantees that every prompt yields at least one verified question.

### 2.4 Document Reference Extraction
- **Rules**:
  1. Scan whitespace-delimited tokens in `prompt`.
  2. Trim enclosing boundary punctuation (`(`, `)`, `'`, `"`, `[`, `]`, `{`, `}`, `<`, `>`, `,`, `;`).
  3. Check against supported document extensions: `.pdf`, `.docx`, `.doc`, `.txt`, `.rs`, `.md`, `.csv`, `.json`, `.toml`.
  4. Enforce `clean_tok.len() > ext.len()` and `!clean_tok.starts_with('.')` so raw extensions like `".pdf"` are ignored.
  5. Ignore tokens containing `:` (which are query metadata filters like `filetype:pdf`).
  6. Assert `prompt.contains(clean_tok)` before emitting `Extraction::new("document_reference", clean_tok)` with attributes:
     - `"file_name"`: `clean_tok`
     - `"extension"`: extension without dot (e.g. `"pdf"`, `"docx"`)
     - `"canonical"`: `clean_tok.to_ascii_lowercase()`

### 2.5 Named Entity & Technical Term Extraction
- **Rules**:
  1. Filter out question words and common stopwords (`who`, `what`, `where`, `is`, `are`, `the`, `and`, `tell`, `explain`, etc.) so that capitalized sentence-starters like `"Who"` in `"Who is Femi?"` are NEVER mistakenly extracted as named entities.
  2. Strip possessives: `clean_word.strip_suffix("'s").or_else(|| clean_word.strip_suffix('\''))` so `"Davis's"` yields `"Davis"` and `"Femi's"` yields `"Femi"`.
  3. Check known entities (case-insensitively matched): `"femi"`, `"oyesanya"`, `"redactx"`, `"vaultgemma"`, `"presidio"`, `"patientpoint"`, `"cigna"`, `"angela"`, `"davis"`, `"rrfcombiner"`, `"lancedb"`, `"tantivy"`, `"blake3"`, `"ed25519"`.
  4. Check capitalized words (`len >= 2`) not in stopwords.
  5. Classify kind:
     - `technical_term`: `RedactX`, `VaultGemma`, `Presidio`, `RrfCombiner`, `LanceDB`, `Tantivy`, `BLAKE3`, `Ed25519`
     - `organization`: `PatientPoint`, `Cigna`
     - `named_entity`: `Femi`, `Oyesanya`, `Angela`, `Davis`, etc.
  6. Multi-word entity detection: Detect consecutive capitalized entity words (e.g. `"Femi Oyesanya"`, `"Angela Davis"`) and emit the full phrase as `kind = "full_name"` in addition to individual tokens.

### 2.6 Deduplication & Ordering
- Deduplicate candidates using `(extraction_class, extraction_text)`.
- Order: Questions first, Document References second, Named Entities third.

---

## 3. Caveats

1. **Read-Only Explorer Boundary**:
   As Explorer M11 1, we do not modify source code directly. This report provides exact implementations and unit tests for the Worker agent to insert into `crates/quorum-coordinator/src/langextract/extraction.rs` and `tests/coordinator_tests.rs`.
2. **Case Handling**:
   `extraction_text` retains the exact casing found in `prompt` so `str::find` succeeds on step 1 with zero allocations. Casing normalization is placed strictly in the `"canonical"` attribute.
3. **Punctuation Boundaries**:
   Punctuation around tokens (`'Femi'`, `(Resume2026.pdf)`) is stripped from the entity/filename, but the trimmed slice is verified to exist in `prompt` before emitting.

---

## 4. Conclusion & Concrete Implementation Specifications

### 4.1 Specification: `crates/quorum-coordinator/src/langextract/extraction.rs`

Add the following complete implementation to `crates/quorum-coordinator/src/langextract/extraction.rs`:

```rust
// ============================================================================
// Google LangExtract Chat Box Candidate Extraction & Invariant Constants
// ============================================================================

const DOC_EXTENSIONS: &[&str] = &[
    ".pdf", ".docx", ".doc", ".txt", ".rs", ".md", ".csv", ".json", ".toml",
];

const KNOWN_ENTITIES: &[&str] = &[
    "femi",
    "oyesanya",
    "redactx",
    "vaultgemma",
    "presidio",
    "patientpoint",
    "cigna",
    "angela",
    "davis",
    "rrfcombiner",
    "lancedb",
    "tantivy",
    "blake3",
    "ed25519",
];

const ENTITY_STOPWORDS: &[&str] = &[
    "who", "what", "where", "when", "why", "how", "which",
    "is", "are", "was", "were", "be", "been", "being",
    "do", "does", "did", "have", "has", "had",
    "the", "this", "that", "these", "those",
    "and", "but", "or", "nor", "for", "yet", "so",
    "with", "from", "into", "about", "using", "under", "over",
    "tell", "show", "explain", "describe", "list", "find", "check",
    "can", "could", "would", "will", "shall", "should", "may", "might", "must",
    "you", "your", "yours", "his", "her", "hers", "their", "theirs", "its", "our", "ours",
    "file", "files", "document", "documents", "doc", "docs", "docx", "pdf", "txt",
    "school", "university", "college", "degree", "education", "resume", "experience",
];

const INTERROGATIVE_PREFIXES: &[&str] = &[
    "who ", "what ", "where ", "when ", "why ", "how ",
    "tell me ", "explain ", "describe ", "show me ", "list ",
    "can you ", "could you ", "would you ", "which ",
    "is ", "are ", "was ", "were ", "does ", "did ", "do ",
];

/// Splits prompt text into sentence and interrogative clauses without breaking file extensions or decimals.
fn split_prompt_clauses(prompt: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    let mut current_start = 0;
    let bytes = prompt.as_bytes();
    let len = bytes.len();

    let mut i = 0;
    while i < len {
        let b = bytes[i];
        let is_boundary = if b == b'?' || b == b'\n' || b == b';' {
            true
        } else if b == b'.' || b == b'!' {
            // Only split on '.' or '!' if followed by whitespace or at the very end of prompt
            i + 1 == len || bytes[i + 1].is_ascii_whitespace()
        } else {
            false
        };

        if is_boundary {
            let slice = &prompt[current_start..i];
            let trimmed = slice.trim();
            if !trimmed.is_empty() {
                clauses.push(trimmed);
            }
            current_start = i + 1;
        }
        i += 1;
    }

    if current_start < len {
        let slice = &prompt[current_start..];
        let trimmed = slice.trim();
        if !trimmed.is_empty() {
            clauses.push(trimmed);
        }
    }

    clauses
}

/// Extracts candidate questions, document references, and named entities/technical terms
/// from a user chat box query.
///
/// Guaranteed Invariant: Every candidate extraction_text is an exact verbatim substring
/// of `prompt`, guaranteeing 100% pass rate in `ground_extraction` (Arm 1) and
/// `CounterfactualJudge` (Arm 3) under `LlmFusionConsensus`.
pub fn extract_chat_candidates(prompt: &str) -> Vec<Extraction> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut all_candidates = Vec::new();

    // ------------------------------------------------------------------------
    // 1. Extract Questions
    // ------------------------------------------------------------------------
    let clauses = split_prompt_clauses(trimmed);
    let mut found_question = false;

    for clause in &clauses {
        let lower = clause.to_ascii_lowercase();
        let is_interrogative = INTERROGATIVE_PREFIXES.iter().any(|prefix| lower.starts_with(prefix));
        let ends_with_qmark = prompt.contains(&format!("{}?", clause));

        if is_interrogative || ends_with_qmark {
            if prompt.contains(clause) {
                all_candidates.push(
                    Extraction::new("question", *clause)
                        .with_attribute("type", "interrogative_clause")
                );
                found_question = true;
            }
        }
    }

    if !found_question {
        if prompt.contains(trimmed) {
            all_candidates.push(
                Extraction::new("question", trimmed)
                    .with_attribute("type", "full_prompt_question")
            );
        }
    }

    // ------------------------------------------------------------------------
    // 2. Extract Document References (e.g. .pdf, .docx, .doc, .txt, .rs, .md)
    // ------------------------------------------------------------------------
    for token in trimmed.split_whitespace() {
        if token.contains(':') {
            continue; // Skip metadata query filters (e.g. filetype:pdf)
        }
        let clean_tok = token.trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '.' && c != '_' && c != '-'
        });
        let lower_tok = clean_tok.to_ascii_lowercase();

        for ext in DOC_EXTENSIONS {
            if lower_tok.ends_with(ext) && clean_tok.len() > ext.len() && !clean_tok.starts_with('.') {
                if prompt.contains(clean_tok) {
                    let ext_name = ext.trim_start_matches('.');
                    all_candidates.push(
                        Extraction::new("document_reference", clean_tok)
                            .with_attribute("file_name", clean_tok)
                            .with_attribute("extension", ext_name)
                            .with_attribute("canonical", &lower_tok)
                    );
                }
                break;
            }
        }
    }

    // ------------------------------------------------------------------------
    // 3. Extract Named Entities & Technical Terms
    // ------------------------------------------------------------------------
    let mut detected_entity_words = Vec::new();

    for token in trimmed.split_whitespace() {
        if token.contains(':') {
            continue;
        }
        let is_doc = DOC_EXTENSIONS.iter().any(|ext| token.to_ascii_lowercase().ends_with(ext));
        if is_doc {
            continue;
        }

        let raw_clean = token
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '-')
            .trim_matches('-');
        if raw_clean.len() < 2 {
            continue;
        }

        // Strip possessive 's or trailing apostrophe ('s / ')
        let clean = raw_clean
            .strip_suffix("'s")
            .or_else(|| raw_clean.strip_suffix('\''))
            .unwrap_or(raw_clean);
        if clean.len() < 2 {
            continue;
        }

        let lower = clean.to_ascii_lowercase();
        if ENTITY_STOPWORDS.contains(&lower.as_str()) {
            continue;
        }

        let is_capitalized = clean.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
        let is_known = KNOWN_ENTITIES.contains(&lower.as_str());

        if is_capitalized || is_known {
            if prompt.contains(clean) {
                let is_tech = lower == "redactx"
                    || lower == "vaultgemma"
                    || lower == "presidio"
                    || lower == "rrfcombiner"
                    || lower == "lancedb"
                    || lower == "tantivy"
                    || lower == "blake3"
                    || lower == "ed25519";

                let kind = if is_tech {
                    "technical_term"
                } else if lower == "cigna" || lower == "patientpoint" {
                    "organization"
                } else {
                    "named_entity"
                };

                all_candidates.push(
                    Extraction::new("named_entity", clean)
                        .with_attribute("canonical", &lower)
                        .with_attribute("kind", kind)
                );
                detected_entity_words.push(clean);
            }
        }
    }

    // ------------------------------------------------------------------------
    // 4. Extract Multi-Word Proper Nouns (e.g. "Femi Oyesanya", "Angela Davis")
    // ------------------------------------------------------------------------
    if detected_entity_words.len() >= 2 {
        for window in detected_entity_words.windows(2) {
            let phrase = format!("{} {}", window[0], window[1]);
            if prompt.contains(&phrase) {
                let lower_phrase = phrase.to_ascii_lowercase();
                all_candidates.push(
                    Extraction::new("named_entity", phrase)
                        .with_attribute("canonical", lower_phrase)
                        .with_attribute("kind", "full_name")
                );
            }
        }
    }

    // ------------------------------------------------------------------------
    // 5. Deduplicate Candidates by (class, text)
    // ------------------------------------------------------------------------
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for cand in all_candidates {
        let key = (cand.extraction_class.clone(), cand.extraction_text.clone());
        if seen.insert(key) {
            result.push(cand);
        }
    }

    result
}
```

### 4.2 Specification: Export in `crates/quorum-coordinator/src/langextract/mod.rs`
Update line 7:
```rust
pub use extraction::{extract_chat_candidates, filter_grounded, ground_extraction, Extraction};
```

---

## 5. Verification Method

### 5.1 Unit Tests Specification (`tests/coordinator_tests.rs`)

Add these 8 unit tests to `crates/quorum-coordinator/tests/coordinator_tests.rs`:

```rust
#[test]
fn test_extract_chat_candidates_questions_interrogative() {
    let prompt = "Who is femi? Where did he study?";
    let candidates = extract_chat_candidates(prompt);

    let questions: Vec<&str> = candidates
        .iter()
        .filter(|e| e.extraction_class == "question")
        .map(|e| e.extraction_text.as_str())
        .collect();

    assert_eq!(questions.len(), 2);
    assert_eq!(questions[0], "Who is femi");
    assert_eq!(questions[1], "Where did he study");

    for q in &questions {
        assert!(prompt.contains(q), "Question must be verbatim in prompt: {}", q);
    }
}

#[test]
fn test_extract_chat_candidates_document_references() {
    let prompt = "Tell me about Femi from ResumeSec-AI-FemiOyesanya2025.pdf and (Femi_Oyesanya_Enhanced_Resume_v3.docx). Also check Femi_Oyesanya_CV_2026.pdf and Resume2026.pdf.";
    let candidates = extract_chat_candidates(prompt);

    let doc_refs: Vec<&str> = candidates
        .iter()
        .filter(|e| e.extraction_class == "document_reference")
        .map(|e| e.extraction_text.as_str())
        .collect();

    assert_eq!(doc_refs.len(), 4);
    assert!(doc_refs.contains(&"ResumeSec-AI-FemiOyesanya2025.pdf"));
    assert!(doc_refs.contains(&"Femi_Oyesanya_Enhanced_Resume_v3.docx"));
    assert!(doc_refs.contains(&"Femi_Oyesanya_CV_2026.pdf"));
    assert!(doc_refs.contains(&"Resume2026.pdf"));

    for d in &doc_refs {
        assert!(prompt.contains(d), "Document reference must be verbatim in prompt: {}", d);
    }
}

#[test]
fn test_extract_chat_candidates_named_entities_and_technical_terms() {
    let prompt = "What did Femi do at PatientPoint and Cigna with RedactX, VaultGemma, and Presidio?";
    let candidates = extract_chat_candidates(prompt);

    let entities: Vec<(&str, Option<&str>)> = candidates
        .iter()
        .filter(|e| e.extraction_class == "named_entity")
        .map(|e| (e.extraction_text.as_str(), e.attributes.get("kind").map(String::as_str)))
        .collect();

    let texts: Vec<&str> = entities.iter().map(|(t, _)| *t).collect();
    assert!(texts.contains(&"Femi"));
    assert!(texts.contains(&"PatientPoint"));
    assert!(texts.contains(&"Cigna"));
    assert!(texts.contains(&"RedactX"));
    assert!(texts.contains(&"VaultGemma"));
    assert!(texts.contains(&"Presidio"));

    let redactx_ext = candidates.iter().find(|e| e.extraction_text == "RedactX").unwrap();
    assert_eq!(redactx_ext.attributes.get("kind").map(String::as_str), Some("technical_term"));

    let cigna_ext = candidates.iter().find(|e| e.extraction_text == "Cigna").unwrap();
    assert_eq!(cigna_ext.attributes.get("kind").map(String::as_str), Some("organization"));
}

#[test]
fn test_extract_chat_candidates_stopwords_never_extracted_as_entities() {
    let prompt = "Who is Femi and What does he do? Where did he study?";
    let candidates = extract_chat_candidates(prompt);

    let entity_texts: Vec<&str> = candidates
        .iter()
        .filter(|e| e.extraction_class == "named_entity")
        .map(|e| e.extraction_text.as_str())
        .collect();

    // Invariant: Sentence-starting question words must NEVER be extracted as named entities
    assert!(!entity_texts.contains(&"Who"));
    assert!(!entity_texts.contains(&"What"));
    assert!(!entity_texts.contains(&"Where"));
    assert!(entity_texts.contains(&"Femi"));
}

#[test]
fn test_extract_chat_candidates_possessive_stripping() {
    let prompt = "Tell me about Angela Davis's school from ResumeSec-AI-FemiOyesanya2025.pdf";
    let candidates = extract_chat_candidates(prompt);

    let entity_texts: Vec<&str> = candidates
        .iter()
        .filter(|e| e.extraction_class == "named_entity")
        .map(|e| e.extraction_text.as_str())
        .collect();

    assert!(entity_texts.contains(&"Angela"));
    assert!(entity_texts.contains(&"Davis")); // Stripped from "Davis's"
    assert!(entity_texts.contains(&"Angela Davis")); // Multi-word phrase
    assert!(!entity_texts.contains(&"Davis's"));
}

#[test]
fn test_extract_chat_candidates_4arm_consensus_100_percent() {
    let prompt = "Who is Femi Oyesanya from ResumeSec-AI-FemiOyesanya2025.pdf? What did he do at PatientPoint?";
    let candidates = extract_chat_candidates(prompt);
    assert!(!candidates.is_empty());

    let result = LlmFusionConsensus::evaluate_extractions(prompt, candidates, None);

    assert!(result.consensus_passed, "Consensus must pass for chat candidates");
    assert!(result.fused_confidence_bp >= 8500, "Confidence must exceed 8500 bp, got {}", result.fused_confidence_bp);
    assert_eq!(result.arm_reports.len(), 4);

    for arm in &result.arm_reports {
        assert!(arm.passed, "Arm '{}' failed: {}", arm.arm_name, arm.details);
        assert_eq!(arm.score_pct, 100.0, "Arm '{}' score must be 100%", arm.arm_name);
    }
}

#[test]
fn test_extract_chat_candidates_empty_and_whitespace() {
    assert!(extract_chat_candidates("").is_empty());
    assert!(extract_chat_candidates("   \n\t  ").is_empty());
}

#[test]
fn test_adversarial_hallucination_injection_rejected() {
    let prompt = "Who is Femi from ResumeSec-AI-FemiOyesanya2025.pdf?";
    let mut candidates = extract_chat_candidates(prompt);

    // Inject hallucinated ungrounded candidate
    candidates.push(Extraction::new("FakeEntity", "QuantumBreakoutSuperweapon99"));

    let result = LlmFusionConsensus::evaluate_extractions(prompt, candidates, None);

    assert!(!result.consensus_passed, "Hallucinated extraction must cause consensus to fail");
    assert!(!result.arm_reports[0].passed, "Arm 1 (SemanticSpanExtractor) must fail on hallucination");
    assert!(!result.arm_reports[2].passed, "Arm 3 (CounterfactualJudge) must fail on hallucination");

    // Only genuine grounded items must survive
    assert!(!result.verified_extractions.iter().any(|e| e.extraction_text == "QuantumBreakoutSuperweapon99"));
}
```

### 5.2 Verification Commands
Run tests in `crates/quorum-coordinator`:
```powershell
cargo test -p quorum-coordinator --test coordinator_tests -- test_extract_chat_candidates
cargo test -p quorum-coordinator --test coordinator_tests
```

### 5.3 Invalidation Conditions
- If any extraction has `char_interval == None` after `ground_extraction`.
- If `LlmFusionConsensus::evaluate_extractions` returns `consensus_passed == false` on genuine chat box queries.
- If sentence-initial question words (`Who`, `What`, `Where`) are extracted as `named_entity`.
- If document references with extensions are truncated or fail to extract.
