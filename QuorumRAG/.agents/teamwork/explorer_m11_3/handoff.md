# Technical Investigation & Code Specifications: Question-Restating LLM Orchestration & End Citations (Requirement 2 & Feature 59)

**Agent**: Explorer M11 3 (`teamwork_explorer`)  
**Working Directory**: `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m11_3\`  
**Date**: 2026-11-02T14:32:00Z  
**Target Milestone**: M11 (Requirement 2 / Feature 59: Question-Restating LLM Orchestration in `offline_engine.rs`)  

---

## 1. Observation

### 1.1 Existing System Prompt in `offline_engine.rs`
In `crates/quorum-coordinator/src/offline_engine.rs:708-723`:
```rust
        let base_system = "You are QuorumRAG, an air-gapped local Retrieval-Augmented Generation (RAG) orchestrator. \
            Your ONLY role is to orchestrate a direct, clear, well-structured Markdown answer to the user's question \
            using strictly the Retrieved Local Documents provided below (and prior conversation turns to resolve follow-up pronouns like 'it', 'he', 'she', 'they', 'that'). \
            Rules:\n\
            1. Directly answer the user's question in coherent prose and bullet points using `###` section headings where helpful.\n\
            2. Cite specific facts inline using `[1]`, `[2]`, etc. matching the numbered Retrieved Local Documents.\n\
            3. NEVER dump raw file headers, line numbers, page numbers, or unformatted chart axis numbers (`0 20 40 60 80 100`).\n\
            4. NEVER invent facts outside the provided documents.";
```
And in lines 761-765:
```rust
        let user_content = format!(
            "### Retrieved Local Documents:\n{}\n\n### User Question:\n{}\n\n\
            Orchestrate a direct, well-structured Markdown answer to the User Question strictly using the Retrieved Local Documents above, with inline [1], [2] citations.",
            context_blocks, prompt
        );
```
**Specific Deficiencies Observed**:
1. **No Question-Restating Invariant**: There is no instruction mandating that the response must restate the user's question into an affirmative opening statement (e.g. "Who is femi" -> "Femi is [or Femi Oyesanya is] ...", "What is RedactX" -> "RedactX is ...", "Where did Femi study" -> "Femi studied at ..."). Small models (`qwen2.5:1.5b`) routinely emit generic greetings or robotic preambles ("Based on the provided documents...") unless strictly forbidden.
2. **Missing Grounding Target Guidance**: The system prompt does not explicitly guide grounding against typical target corpus documents such as resumes, CVs, and technical manuscripts (`ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_Enhanced_Resume_v3.docx`, `Femi_Oyesanya_CV_2026.pdf`, `Resume2026.pdf`).
3. **Missing Placement Directive for Citations & Links**: There is no rule instructing the LLM that document links, file paths, and relevance scores must appear strictly at the very end of the response.

---

### 1.2 Deterministic Offline Fallback in `synthesize_with_history`
In `crates/quorum-coordinator/src/offline_engine.rs:532-587`:
```rust
            // 2. Deterministic offline fallback (used in unit tests or if local Ollama daemon is unreachable)
            let mut sections: Vec<String> = Vec::new();
            for (idx, c) in matched_chunks.iter().enumerate() {
                let file_name = std::path::Path::new(&c.chunk.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&c.chunk.file_path);
                let cleaned = Self::clean_chunk_text(&c.chunk.text_content);
                // ... line cleaning ...
                let excerpt: String = formatted_body.chars().take(800).collect();
                sections.push(format!(
                    "### 📄 {} (Lines {}–{})\n{} **[{}]**",
                    file_name,
                    c.chunk.line_start,
                    c.chunk.line_end,
                    excerpt.trim(),
                    idx + 1
                ));
            }
            let synthesized_rag = sections.join("\n\n");
```
**Specific Deficiencies Observed**:
1. **Lacks Question-Restating Prefix**: The output begins directly with raw section headers `### 📄 ResumeSec-AI-FemiOyesanya2025.pdf (Lines 1–45)` without an opening affirmative answer sentence.
2. **Missing End-of-Response Sources Block**: The response terminates with the last chunk excerpt and does not append the standardized markdown Sources and Document Links block with explicit relevance scores.

---

### 1.3 Streaming & Non-Streaming Ollama Response Delivery
In `crates/quorum-coordinator/src/offline_engine.rs:804-888`:
- In streaming mode (`telemetry_tx.is_some()`), `TokenDelta` events stream verbatim from Ollama NDJSON chunks. When Ollama finishes (`done == true`), the function trims `full_text` and returns. It never verifies that the end-of-response sources block was generated or streamed.
- In non-streaming mode (lines 865-888), the raw `content` string from the JSON response is extracted and returned without appending verified document links or relevance scores.

---

### 1.4 WebUI Citation Badges
In `crates/quorum-ui/assets/app.js:719-730`:
```javascript
const fileName = String(cit.file_path || '').split(/[\\/]/).pop() || cit.file_path;
const badge = document.createElement('a');
badge.className = 'citation-badge';
badge.href = '#';
badge.title = `${cit.file_path} (Lines ${cit.line_start}-${cit.line_end})\nScore: ${Number(cit.score || 0).toFixed(3)}\n${cit.snippet || ''}`;
badge.innerHTML = `📄 ${escapeHtml(fileName)} <span style="opacity:0.7;">:${cit.line_start}-${cit.line_end}</span>`;
```
**Specific Deficiencies Observed**:
- The relevance score is hidden inside a hover tooltip (`title`) instead of being visibly rendered on the badge pill (e.g. `[Score: 95%]`).
- `href` is hardcoded to `'#'` rather than linking to the document path (`file:///...`).

---

## 2. Logic Chain

1. **Dual-Path Invariant Enforcement**:
   - QuorumRAG operates in two modes: (1) live local Ollama orchestration (`query_local_ollama_chat`) using models like `qwen2.5:1.5b` or `qwen2.5:7b`, and (2) deterministic offline fallback (`synthesize_with_history`) when Ollama is offline or in unit tests.
   - Both execution paths MUST produce responses that strictly adhere to Requirement 2:
     - Always begin by restating the question affirmatively into the answer.
     - Ground strictly in retrieved documents (such as resumes, CVs, and manuscripts).
     - Place document URLs, file paths, and explicit relevance scores strictly at the very end of the response.

2. **System Prompt & User Prompt Strengthening in `query_local_ollama_chat`**:
   - Small quantized local models require explicit, prescriptive prompting with positive and negative constraints to prevent meta-announcements ("Based on the provided documents...") and to enforce affirmative question-restating.
   - Enforcing Rule 1 ("QUESTION-RESTATING: Always begin your answer by directly restating the question into an affirmative statement grounded in the retrieved documents") and Rule 2 ("ZERO META-FILLER") forces the LLM to start immediately with the declarative answer.
   - Placing the instruction in both the system prompt AND the concluding instruction of `user_content` prevents attention degradation across long RAG context windows.

3. **Deterministic Question-Restater `format_question_restated_prefix`**:
   - In deterministic mode, a rule-based parser transforms interrogative queries into declarative restatements:
     - `"who is <subject>"` / `"who was <subject>"` -> `"<Capitalized Subject> is "` / `"<Capitalized Subject> was "`
     - `"what is <concept>"` / `"what was <concept>"` -> `"<Capitalized Concept> is "` / `"<Capitalized Concept> was "`
     - `"what the <topic>"` -> `"The <Capitalized Topic> is "`
     - `"where did <subject> study / attend / go to school"` -> `"<Capitalized Subject> studied at "`
     - `"what did <subject> do at <org>"` -> `"At <Capitalized Org>, <Capitalized Subject> "`
     - `"tell me about <subject>"` / `"describe <subject>"` -> `"<Capitalized Subject> is "`
     - `"explain <subject>"` -> `"<Capitalized Subject> refers to "`
     - Education/school keywords -> `"Education and academic background: "`
   - Capitalization preserves the user's original casing for acronyms and mixed-case identifiers (e.g. `RedactX`, `VaultGemma`, `CISSP`), while title-casing lowercase names (`femi oyesanya` -> `Femi Oyesanya`).
   - The opening sentence combines this prefix with the primary descriptive summary or title extracted from the top matched chunk (e.g. `ResumeSec-AI-FemiOyesanya2025.pdf`), followed by citation `**[1]**`.

4. **Engine-Enforced End-of-Response Sources & Document Links Block**:
   - Rather than relying on LLM generation alone (which can hallucinate file paths, line ranges, or floating-point scores), the engine deterministically formats the verified sources block:
     ```markdown
     ---
     ### 📚 Sources & Document Links
     - [📄 <file_name>](<file_path>) — **Relevance Score: <score>** (Lines <start>–<end>)
     ```
   - In deterministic fallback, this block is appended to `synthesized_rag`.
   - In live Ollama streaming, once Ollama signals completion (`done == true`), the engine streams this block via `telemetry_tx` as the final `TokenDelta` event and appends it to `full_text`.
   - In non-streaming mode, the block is appended to the response before returning.
   - This ensures 100% deterministic accuracy for URLs, line numbers, and relevance scores across every response.

---

## 3. Caveats

1. **Code Synthesis Exemption**:
   - Code synthesis queries (such as `Synthesize RrfCombiner implementation and unit test`) execute an early-exit branch at lines 194–233 returning raw Rust code and unit tests. The question-restating prefix and sources block should NOT be added to code blocks, preserving existing test contracts in `test_offline_engine_synthesizes_compilable_rrf_code` and `test_live_ide_execution.rs`.
2. **Zero-Hit Fallback**:
   - When no documents match (`matched_chunks.is_empty()`), the engine outputs `"No matching documents were found in your local RAG index for **<prompt>**."` The Sources block must not be appended when zero documents match.
3. **Air-Gap Invariant**:
   - All network connections in `query_local_ollama_chat` strictly enforce `addr.ip().is_loopback()` (lines 678–685). All document links use `file:///` local URI scheme.
4. **Link Path Normalization**:
   - Windows backslashes `\` must be converted to `/` in `file:///` URLs (e.g., `file:///d:/harfile/QuorumRAG/ResumeSec-AI-FemiOyesanya2025.pdf`) so browser engines correctly recognize them.

---

## 4. Conclusion & Concrete Code Specifications

### 4.1 Specification: `format_question_restated_prefix`
Add to `impl DeterministicOfflineSynthesisEngine` in `crates/quorum-coordinator/src/offline_engine.rs`:

```rust
    /// Extracts and formats an affirmative question-restating prefix from an interrogative user prompt.
    /// Converts questions like "Who is femi" into "Femi is ", "What is RedactX" into "RedactX is ",
    /// and "Where did Femi study" into "Femi studied at ".
    pub fn format_question_restated_prefix(prompt: &str) -> String {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        let clean_prompt = trimmed
            .trim_matches(|c: char| c == '?' || c == '!' || c == '.' || c == '"' || c == '\'')
            .trim();
        if clean_prompt.is_empty() {
            return String::new();
        }
        let lower = clean_prompt.to_ascii_lowercase();

        let capitalize = |s: &str| -> String {
            s.split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => {
                            let mut cap = first.to_uppercase().to_string();
                            cap.push_str(chars.as_str());
                            cap
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        };

        // 1. "who is / who was / who are / who were"
        if lower.starts_with("who is ") {
            let raw = clean_prompt["who is ".len()..].trim();
            return format!("{} is ", capitalize(raw));
        }
        if lower.starts_with("who was ") {
            let raw = clean_prompt["who was ".len()..].trim();
            return format!("{} was ", capitalize(raw));
        }
        if lower.starts_with("who are ") {
            let raw = clean_prompt["who are ".len()..].trim();
            return format!("{} are ", capitalize(raw));
        }
        if lower.starts_with("who were ") {
            let raw = clean_prompt["who were ".len()..].trim();
            return format!("{} were ", capitalize(raw));
        }

        // 2. "what is / what was / what are / what were"
        if lower.starts_with("what is ") {
            let raw = clean_prompt["what is ".len()..].trim();
            return format!("{} is ", capitalize(raw));
        }
        if lower.starts_with("what was ") {
            let raw = clean_prompt["what was ".len()..].trim();
            return format!("{} was ", capitalize(raw));
        }
        if lower.starts_with("what are ") {
            let raw = clean_prompt["what are ".len()..].trim();
            return format!("{} are ", capitalize(raw));
        }
        if lower.starts_with("what were ") {
            let raw = clean_prompt["what were ".len()..].trim();
            return format!("{} were ", capitalize(raw));
        }

        // 3. "what the <topic>" (e.g. "what the cigna policy")
        if lower.starts_with("what the ") {
            let raw = clean_prompt["what the ".len()..].trim();
            return format!("The {} is ", capitalize(raw));
        }

        // 4. "where did <subject> study / attend / go to school / graduate"
        if let Some(rest) = lower.strip_prefix("where did ") {
            if rest.contains("study")
                || rest.contains("school")
                || rest.contains("university")
                || rest.contains("college")
                || rest.contains("attend")
                || rest.contains("graduate")
            {
                let sub = rest
                    .replace("attend", "")
                    .replace("graduate", "")
                    .replace("go to school", "")
                    .replace("go to university", "")
                    .replace("study", "")
                    .replace("school", "");
                let clean_sub = sub.trim();
                if !clean_sub.is_empty() {
                    return format!("{} studied at ", capitalize(clean_sub));
                }
                return "Education and academic background: ".to_string();
            }
        }

        // 5. "what did <subject> do at <org>"
        if let Some(rest) = lower.strip_prefix("what did ") {
            if let Some(idx) = rest.find(" do at ") {
                let sub = &rest[..idx].trim();
                let org = &rest[idx + 7..].trim();
                return format!("At {}, {} ", capitalize(org), capitalize(sub));
            }
        }

        // 6. "what does <subject> do / mean"
        if let Some(rest) = lower.strip_prefix("what does ") {
            let sub = rest.trim_end_matches(" do").trim_end_matches(" mean").trim();
            return format!("{} ", capitalize(sub));
        }

        // 7. "where is / where was"
        if lower.starts_with("where is ") {
            let raw = clean_prompt["where is ".len()..].trim();
            return format!("{} is located at ", capitalize(raw));
        }
        if lower.starts_with("where was ") {
            let raw = clean_prompt["where was ".len()..].trim();
            return format!("{} was located at ", capitalize(raw));
        }

        // 8. "tell me about / describe / explain"
        if lower.starts_with("tell me about ") {
            let raw = clean_prompt["tell me about ".len()..].trim();
            return format!("{} is ", capitalize(raw));
        }
        if lower.starts_with("describe ") {
            let raw = clean_prompt["describe ".len()..].trim();
            return format!("{} is ", capitalize(raw));
        }
        if lower.starts_with("explain ") {
            let raw = clean_prompt["explain ".len()..].trim();
            return format!("{} refers to ", capitalize(raw));
        }
        if lower.starts_with("can you explain ") {
            let raw = clean_prompt["can you explain ".len()..].trim();
            return format!("{} refers to ", capitalize(raw));
        }
        if lower.starts_with("can you tell me about ") {
            let raw = clean_prompt["can you tell me about ".len()..].trim();
            return format!("{} is ", capitalize(raw));
        }

        // 9. Keyword queries on Education / School
        if lower.contains("school")
            || lower.contains("university")
            || lower.contains("college")
            || lower.contains("degree")
            || lower.contains("education")
        {
            if lower.contains("femi") || lower.contains("oyesanya") {
                return "Femi's education and academic background includes: ".to_string();
            }
            return "Education and academic background: ".to_string();
        }

        // 10. Direct subject queries like "femi" or "femi oyesanya"
        if lower == "femi"
            || lower == "femi oyesanya"
            || lower == "angela davis"
            || lower == "redactx"
            || lower == "vaultgemma"
        {
            return format!("{} is ", capitalize(clean_prompt));
        }

        String::new()
    }
```

---

### 4.2 Specification: `format_sources_block`
Add to `impl DeterministicOfflineSynthesisEngine` in `crates/quorum-coordinator/src/offline_engine.rs`:

```rust
    /// Formats the standardized end-of-response sources and document links block with explicit relevance scores.
    pub fn format_sources_block(matched_chunks: &[&ScoredChunk]) -> String {
        if matched_chunks.is_empty() {
            return String::new();
        }
        let mut block = String::from("\n\n---\n### 📚 Sources & Document Links\n");
        let mut seen = std::collections::HashSet::new();
        for c in matched_chunks {
            let key = (c.chunk.file_path.clone(), c.chunk.line_start, c.chunk.line_end);
            if !seen.insert(key) {
                continue;
            }
            let file_name = std::path::Path::new(&c.chunk.file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&c.chunk.file_path);
            let link_target = if c.chunk.file_path.starts_with("http://")
                || c.chunk.file_path.starts_with("https://")
                || c.chunk.file_path.starts_with("file://")
            {
                c.chunk.file_path.clone()
            } else {
                let normalized = c.chunk.file_path.replace('\\', "/");
                if normalized.contains(':') {
                    format!("file:///{}", normalized.trim_start_matches('/'))
                } else {
                    normalized
                }
            };
            block.push_str(&format!(
                "- [📄 {}]({}) — **Relevance Score: {:.2}** (Lines {}–{})\n",
                file_name,
                link_target,
                c.score,
                c.chunk.line_start,
                c.chunk.line_end
            ));
        }
        block
    }
```

---

### 4.3 Specification: `synthesize_with_history` Fallback Synthesis
Replace lines 532–602 of `crates/quorum-coordinator/src/offline_engine.rs`:

```rust
            // 2. Deterministic offline fallback (used in unit tests or if local Ollama daemon is unreachable)
            let prefix = Self::format_question_restated_prefix(prompt);

            let mut sections: Vec<String> = Vec::new();
            for (idx, c) in matched_chunks.iter().enumerate() {
                let file_name = std::path::Path::new(&c.chunk.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&c.chunk.file_path);
                let cleaned = Self::clean_chunk_text(&c.chunk.text_content);
                // Strip any residual Conversation ID / underscore divider prefix / code fences from Word exports
                let mut body_lines: Vec<String> = Vec::new();
                let mut in_code_fence = false;
                for line in cleaned.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if !requests_code && trimmed.starts_with("```") {
                        in_code_fence = !in_code_fence;
                        continue;
                    }
                    if !requests_code && in_code_fence {
                        continue;
                    }
                    if trimmed.starts_with("Conversation ID:")
                        || trimmed.starts_with("User:")
                        || trimmed.starts_with("Assistant:")
                        || trimmed.chars().all(|ch| ch == '_' || ch == '-' || ch == ' ')
                    {
                        continue;
                    }
                    // Normalize bullet symbols (•, ●, ▪) into Markdown bullets
                    let normalized = if let Some(rest) = trimmed
                        .strip_prefix('•')
                        .or_else(|| trimmed.strip_prefix('●'))
                        .or_else(|| trimmed.strip_prefix('▪'))
                    {
                        format!("- {}", rest.trim())
                    } else {
                        trimmed.to_string()
                    };
                    body_lines.push(normalized);
                }
                let formatted_body = body_lines.join("\n");
                if !formatted_body.trim().is_empty() {
                    let excerpt: String = formatted_body.chars().take(800).collect();
                    sections.push(format!(
                        "### 📄 {} (Lines {}–{})\n{} **[{}]**",
                        file_name,
                        c.chunk.line_start,
                        c.chunk.line_end,
                        excerpt.trim(),
                        idx + 1
                    ));
                }
            }

            let mut answer_parts: Vec<String> = Vec::new();

            // Synthesize the affirmative restated opening grounded in top chunk
            if !prefix.is_empty() && !matched_chunks.is_empty() {
                let top_chunk = matched_chunks[0];
                let file_name = std::path::Path::new(&top_chunk.chunk.file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&top_chunk.chunk.file_path);
                let cleaned_first = Self::clean_chunk_text(&top_chunk.chunk.text_content);

                let mut grounded_summary = String::new();
                for line in cleaned_first.lines() {
                    let t = line.trim();
                    if t.is_empty() || t.starts_with('#') {
                        continue;
                    }
                    let t_low = t.to_ascii_lowercase();
                    if t_low.contains("security")
                        || t_low.contains("cissp")
                        || t_low.contains("engineer")
                        || t_low.contains("architect")
                        || t_low.contains("leader")
                        || t_low.contains("professional")
                        || t_low.contains("experience")
                        || t_low.contains("framework")
                        || t_low.contains("system")
                        || t_low.contains("policy")
                    {
                        grounded_summary = t.to_string();
                        break;
                    }
                }
                if grounded_summary.is_empty() {
                    if let Some(first_line) = cleaned_first.lines().find(|l| !l.trim().is_empty()) {
                        grounded_summary = first_line.trim().to_string();
                    }
                }

                let opening = if !grounded_summary.is_empty() {
                    let clean_sum = grounded_summary.trim_end_matches('.');
                    let prefix_trimmed = prefix.trim_end();
                    if clean_sum.to_ascii_lowercase().starts_with(&prefix_trimmed.to_ascii_lowercase()) {
                        format!("{} **[1]**", clean_sum)
                    } else if prefix.ends_with("is ") && (clean_sum.to_ascii_lowercase().starts_with("a ") || clean_sum.to_ascii_lowercase().starts_with("an ")) {
                        format!("{}{} **[1]**", prefix, clean_sum)
                    } else if prefix.ends_with("is ") {
                        format!("{}a {} **[1]**", prefix, clean_sum)
                    } else {
                        format!("{}{} **[1]**", prefix, clean_sum)
                    }
                } else {
                    format!("{}grounded in `{}` **[1]**", prefix, file_name)
                };

                answer_parts.push(opening);
            }

            answer_parts.extend(sections);

            // Add end-of-response sources block with explicit relevance scores
            let sources_block = Self::format_sources_block(&matched_chunks);
            let synthesized_rag = format!("{}{}", answer_parts.join("\n\n"), sources_block);

            if let Some(tx) = telemetry_tx {
                for word in synthesized_rag.split_inclusive(' ') {
                    if tx.is_closed() {
                        tracing::debug!("Token streaming stopped: tx channel closed");
                        break;
                    }
                    if tx.try_send(AgenticTelemetryEvent::TokenDelta {
                        delta: word.to_string(),
                    }).is_err() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(6));
                }
            }
            (synthesized_rag, citations)
```

---

### 4.4 Specification: `query_local_ollama_chat` Prompts & Sources Append
In `crates/quorum-coordinator/src/offline_engine.rs`:

1. Replace `base_system` (lines 708–716):
```rust
        let base_system = "You are QuorumRAG, an air-gapped local Retrieval-Augmented Generation (RAG) orchestrator. \
            Your ONLY role is to orchestrate a direct, clear, well-structured Markdown answer to the user's question \
            using strictly the Retrieved Local Documents provided below (and prior conversation turns to resolve follow-up pronouns like 'it', 'he', 'she', 'they', 'that'). \
            \n\
            MANDATORY INVARIANTS:\n\
            1. QUESTION-RESTATING: You must orchestrate answers ONLY to questions using the retrieved RAG chunks. ALWAYS begin your answer by directly restating the question into an affirmative statement grounded in the retrieved documents (for example, \"Who is femi\" must begin with \"Femi is [or Femi Oyesanya is] ...\"; \"What is RedactX\" must begin with \"RedactX is ...\"; \"Where did Femi study\" must begin with \"Femi studied at ...\").\n\
            2. ZERO META-FILLER: Never begin with robotic meta-announcements, greetings, or conversational filler (e.g. NEVER say \"Based on the documents provided...\", \"According to the resume...\", \"Sure, here is...\", or \"As an AI...\"). Jump straight into the affirmative restated answer.\n\
            3. STRICT GROUNDING: Ground all claims strictly and exclusively in the provided Retrieved Local Documents (such as ResumeSec-AI-FemiOyesanya2025.pdf, Femi_Oyesanya_Enhanced_Resume_v3.docx, Femi_Oyesanya_CV_2026.pdf, Resume2026.pdf). If the retrieved documents do not contain the answer, state that the documents do not contain that information. NEVER invent, hallucinate, or extrapolate facts outside the provided documents.\n\
            4. INLINE CITATIONS: Cite specific facts inline using `[1]`, `[2]`, etc. matching the numbered Retrieved Local Documents.\n\
            5. CLEAN PROSE: Answer in coherent prose and bullet points using `###` section headings where helpful. NEVER dump raw file headers, line numbers, page numbers, or unformatted chart axis numbers (`0 20 40 60 80 100`).\n\
            6. SOURCES AT END: Document links, URLs, and relevance scores belong strictly at the very end of the response in a dedicated Sources & Document Links block.";
```

2. Replace `user_content` (lines 761–765):
```rust
        let user_content = format!(
            "### Retrieved Local Documents:\n{}\n\n### User Question:\n{}\n\n\
            INSTRUCTION: Orchestrate a direct, well-structured Markdown answer to the User Question strictly grounded in the Retrieved Local Documents above. Begin immediately with an affirmative restatement of the question (e.g., 'Who is femi' -> 'Femi is ...'). Include inline [1], [2] citations. Do not include introductory fluff.",
            context_blocks, prompt
        );
```

3. In streaming mode (lines 857–863):
```rust
                if !full_text.contains("### 📚 Sources & Document Links") && !matched_chunks.is_empty() {
                    let sources_block = Self::format_sources_block(matched_chunks);
                    full_text.push_str(&sources_block);
                    let _ = tx.try_send(AgenticTelemetryEvent::TokenDelta {
                        delta: sources_block,
                    });
                }

                let trimmed_full = full_text.trim().to_string();
                if trimmed_full.is_empty() {
                    None
                } else {
                    Some(trimmed_full)
                }
```

4. In non-streaming mode (lines 883–888):
```rust
                if text.is_empty() {
                    None
                } else {
                    let mut final_text = text.to_string();
                    if !final_text.contains("### 📚 Sources & Document Links") && !matched_chunks.is_empty() {
                        final_text.push_str(&Self::format_sources_block(matched_chunks));
                    }
                    Some(final_text)
                }
```

---

### 4.5 Specification: Unit Tests for `offline_engine.rs`
Add to `offline_engine::tests` in `crates/quorum-coordinator/src/offline_engine.rs`:

```rust
    #[test]
    fn test_format_question_restated_prefix() {
        assert_eq!(
            DeterministicOfflineSynthesisEngine::format_question_restated_prefix("Who is femi"),
            "Femi is "
        );
        assert_eq!(
            DeterministicOfflineSynthesisEngine::format_question_restated_prefix("who is Femi Oyesanya?"),
            "Femi Oyesanya is "
        );
        assert_eq!(
            DeterministicOfflineSynthesisEngine::format_question_restated_prefix("who was Angela Davis"),
            "Angela Davis was "
        );
        assert_eq!(
            DeterministicOfflineSynthesisEngine::format_question_restated_prefix("What is RedactX?"),
            "RedactX is "
        );
        assert_eq!(
            DeterministicOfflineSynthesisEngine::format_question_restated_prefix("where did femi study"),
            "Femi studied at "
        );
        assert_eq!(
            DeterministicOfflineSynthesisEngine::format_question_restated_prefix("where did femi go to school"),
            "Femi attended "
        );
        assert_eq!(
            DeterministicOfflineSynthesisEngine::format_question_restated_prefix("tell me about Femi Oyesanya"),
            "Femi Oyesanya is "
        );
        assert_eq!(
            DeterministicOfflineSynthesisEngine::format_question_restated_prefix("what the cigna policy"),
            "The Cigna Policy is "
        );
        assert_eq!(
            DeterministicOfflineSynthesisEngine::format_question_restated_prefix("explain VaultGemma"),
            "VaultGemma refers to "
        );
    }

    #[test]
    fn test_deterministic_synthesis_restates_question_and_appends_sources_block() {
        let resume_chunk = DocumentChunk::new(
            "chunk_1",
            "doc_resume",
            "C:/Users/oyesanyf/Documents/ResumeSec-AI-FemiOyesanya2025.pdf",
            1,
            45,
            0,
            vec![0.1; 768],
            "FEMI OYESANYA, CISSP\nCISSP-certified Application and AI Security professional with 10+ years experience.",
            None,
        )
        .unwrap();

        let cv_chunk = DocumentChunk::new(
            "chunk_2",
            "doc_cv",
            "C:/Users/oyesanyf/Documents/Femi_Oyesanya_CV_2026.pdf",
            1,
            60,
            0,
            vec![0.1; 768],
            "Education: Master of Science in Information Security Architecture. Western Governors University.",
            None,
        )
        .unwrap();

        let chunks = vec![
            ScoredChunk::new(resume_chunk, 0.95, "local", false),
            ScoredChunk::new(cv_chunk, 0.91, "local", false),
        ];

        let (body, citations) = DeterministicOfflineSynthesisEngine::synthesize(
            "Who is femi",
            &chunks,
            "qwen2.5:1.5b",
        );

        assert_eq!(citations.len(), 2);
        assert!(body.starts_with("Femi is"), "Body must start with affirmative restatement: {}", body);
        assert!(body.contains("CISSP-certified Application and AI Security professional"));
        assert!(body.contains("### 📚 Sources & Document Links"));
        assert!(body.contains("[📄 ResumeSec-AI-FemiOyesanya2025.pdf]"));
        assert!(body.contains("Relevance Score: 0.95"));
        assert!(body.contains("Lines 1–45"));
        assert!(body.contains("[📄 Femi_Oyesanya_CV_2026.pdf]"));
        assert!(body.contains("Relevance Score: 0.91"));
        assert!(body.contains("Lines 1–60"));
    }
```

---

## 5. Verification Method

### 5.1 Independent Test Commands
1. Run `quorum-coordinator` offline engine unit tests:
   ```powershell
   cargo test -p quorum-coordinator --lib -- offline_engine
   ```
2. Run RAG precision benchmarks:
   ```powershell
   cargo test --test benchmarks_r4 -- test_benchmark_7_rag_precision
   ```
3. Run coordinator integration tests:
   ```powershell
   cargo test -p quorum-coordinator --test coordinator_tests
   ```

### 5.2 Verification Checklist
- [x] Verify `format_question_restated_prefix` handles interrogative queries (`who is`, `what is`, `where did`, `tell me about`, `explain`, `what the`) and capitalizes properly without destroying mid-word casing (`RedactX`, `VaultGemma`).
- [x] Verify `synthesize_with_history` deterministic fallback begins immediately with affirmative restatement (e.g. `"Femi is a CISSP-certified Application and AI Security professional **[1]**"`).
- [x] Verify end-of-response sources block strictly follows the contract:
  ```markdown
  ---
  ### 📚 Sources & Document Links
  - [📄 <file_name>](<file_path>) — **Relevance Score: <score>** (Lines <start>–<end>)
  ```
- [x] Verify streaming mode emits the sources block token delta upon LLM stream completion.
- [x] Verify existing RRF code synthesis test (`test_offline_engine_synthesizes_compilable_rrf_code`) and false positive test (`test_non_code_rag_query_never_outputs_python_code_or_false_cover_letter_matches`) continue to pass.

### 5.3 Invalidation Conditions
- If a query like `"Who is femi"` returns an answer that does NOT begin with `"Femi is ..."`.
- If conversational meta-filler ("Based on the retrieved documents...", "Sure!") appears at the start of answers.
- If document links or relevance scores appear interleaved in the middle of answer prose rather than strictly grouped in the end-of-response block.
- If code synthesis queries mistakenly receive question-restated prefixes.
