# Deep Research & Iterative Section RAG Pipeline Investigation Report
**Subagent**: Survey Explorer 3 (`explorer_survey_gen4_3`)  
**Mission**: Exhaustive Investigation of Agentic Chaining, Map-Reduce Fact Extraction, Iterative Section Generation, and Synthesis Telemetry  
**Target Codebase**: `d:\harfile\QuorumRAG`  
**Date**: 2026-11-02  

---

## Executive Summary

An exhaustive audit of the QuorumRAG synthesis and agentic execution pipeline across `crates/quorum-coordinator` (`offline_engine.rs`, `agentic/`, `dlp_mesh.rs`), `crates/quorum-daemon` (`handlers/chat.rs`), and `crates/quorum-ui` (`app.js`, `styles.css`) was conducted. 

Key findings:
1. **Critical Hardcoded Token Truncation in `offline_engine.rs:1154`**: `let num_predict = 1600usize;` is hardcoded in `query_local_ollama_chat`. Despite user settings and UI displaying `max_output_tokens: 262144` and `agentic_loop_enabled: true`, the synthesis engine completely ignores `ui.max_output_tokens`, `ui.agentic_loop_enabled`, `ui.agentic_chunk_size`, and `ui.agentic_strategy`.
2. **Telemetry Loss Hazard (`tx.try_send` vs `tx.blocking_send`)**: In `offline_engine.rs:1309`, streaming tokens are emitted via `tx.try_send(...)`. When high-throughput streaming (up to 262k tokens) fills the channel buffer, tokens are silently dropped. Because synthesis executes inside `tokio::task::block_in_place`, converting to `tx.blocking_send(...)` guarantees zero dropped tokens.
3. **Missing `done_reason == "length"` Handling & Recursive Turn Chaining**: In `offline_engine.rs:1314`, only `val.get("done")` is checked; `val.get("done_reason")` is ignored. When the model hits `num_predict`, it stops abruptly with `done_reason == "length"` rather than recursively chaining subsequent hops up to `max_output_tokens`.
4. **Absence of Sentence-Boundary Completion Guards**: Truncated generation can cut off mid-word or mid-sentence (e.g., *"Sacred math has been"*). A completion guard is needed to either prompt for a micro-budget sentence completion or cleanly trim to the last valid terminal punctuation boundary (`.`, `!`, `?`).
5. **Architectural Blueprints Developed**: Concrete designs and data structures are specified for **Iterative Section Generation** (`DocumentPlan` and `SectionPlan` with ~1500-char `rolling_summary`), **Map-Reduce Fact Sheet Extraction** (clustering 3-4 chunks into dense fact sheets), and **Parent Document Retrieval** (stitching contiguous chunks by `chunk_index` / `line_start..line_end`).
6. **Defects Verified in `offline_engine.rs` Retrieval**: Line 499 incorrectly calls `matches_whole_word` on body text inside `file_hits`, and line 505 applies a broken `scored.score < 0.35` threshold that filters out valid RRF candidates (typical RRF scores are 0.01–0.03).

---

## 1. Observations

### 1.1 Hardcoded `num_predict` and Ignored UI Settings
- **Location**: `crates/quorum-coordinator/src/offline_engine.rs:1153-1156`
```rust
1153:         let temp = ui.sampling_temperature.clamp(0.05, 0.20);
1154:         let num_predict = 1600usize;
1155:         let num_ctx = 16384usize;
```
- **Context**: In lines 1110-1114 of `offline_engine.rs`:
```rust
1110:         let ui = controller
1111:             .settings
1112:             .read()
1113:             .map(|g| g.ui.clone())
1114:             .unwrap_or_default();
```
`ui` contains the full user configuration, including `ui.max_output_tokens`, `ui.agentic_loop_enabled`, `ui.agentic_chunk_size`, and `ui.agentic_strategy`.
- **Existing Config Definitions** in `crates/quorum-coordinator/src/dlp_mesh.rs:248-251` & `316-319`:
```rust
248:     pub max_output_tokens: usize,
249:     pub agentic_loop_enabled: bool,
250:     pub agentic_chunk_size: usize,
251:     pub agentic_strategy: String,
...
316:             max_output_tokens: 262144,
317:             agentic_loop_enabled: true,
318:             agentic_chunk_size: 4096,
319:             agentic_strategy: "auto_continuation".to_string(),
```
- **Finding**: Line 1154 hardcodes `num_predict = 1600usize`, completely ignoring `ui.max_output_tokens` (which defaults to 262,144) and `ui.agentic_chunk_size` (which defaults to 4,096).

### 1.2 Telemetry Streaming and Token Dropping via `try_send`
- **Location**: `crates/quorum-coordinator/src/offline_engine.rs:1302-1317`
```rust
1302:                         if let Some(tok) = val
1303:                             .get("message")
1304:                             .and_then(|m| m.get("content"))
1305:                             .and_then(|c| c.as_str())
1306:                         {
1307:                             if !tok.is_empty() {
1308:                                 full_text.push_str(tok);
1309:                                 let _ = tx.try_send(AgenticTelemetryEvent::TokenDelta {
1310:                                     delta: tok.to_string(),
1311:                                 });
1312:                             }
1313:                         }
1314:                         if val.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
1315:                             break;
1316:                         }
```
- **Context**: In `crates/quorum-coordinator/src/agentic/mod.rs:394-399`:
```rust
394:                     tokio::task::block_in_place(|| {
395:                         SynthesizeWithCitationsNode::execute(
396:                             &mut graph_state,
397:                             telemetry_tx.as_ref(),
398:                         );
399:                     });
```
- **Finding**: Because `SynthesizeWithCitationsNode::execute` runs synchronously inside `tokio::task::block_in_place`, the thread is already in Tokio's designated blocking context. Calling `tx.try_send(...)` on an `mpsc::Sender` drops tokens as soon as the channel fills up (returning `TrySendError::Full`). In contrast, `tx.blocking_send(...)` blocks safely until channel space is available, guaranteeing that 0 tokens are dropped.

### 1.3 Ignored `done_reason` and Missing Recursive Turn Chaining
- **Location**: `crates/quorum-coordinator/src/offline_engine.rs:1314-1316`
```rust
1314:                         if val.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
1315:                             break;
1316:                         }
```
- **Finding**: Ollama emits `done_reason: "length"` when the token limit is reached, or `done_reason: "stop"` when generation concludes naturally. Current code only tests `val.get("done")`. If `done_reason == "length"`, the generation stopped prematurely because of `num_predict`. No recursive loop exists to initiate the next turn or continuation hop.

### 1.4 Hardcoded Chunk Limit (8 Chunks)
- **Location**: `crates/quorum-coordinator/src/offline_engine.rs:610-615`
```rust
610:         // Keep top 8 deduplicated RAG chunks for comprehensive multi-paragraph synthesis
611:         let matched_chunks: Vec<&ScoredChunk> = scored_matches
612:             .into_iter()
613:             .take(8)
614:             .map(|(c, _, _, _)| c)
615:             .collect();
```
- **Finding**: Only 8 chunks are forwarded to synthesis, preventing the engine from building comprehensive, multi-section research reports spanning 20–50 chunks.

### 1.5 Retrieval Defects in `offline_engine.rs`: `file_hits` and `scored.score < 0.35`
- **Location**: `crates/quorum-coordinator/src/offline_engine.rs:499-507`
```rust
499:                 let file_hits = query_terms
500:                     .iter()
501:                     .filter(|t| matches_whole_word(t.as_str()) || file_l.contains(t.as_str()))
502:                     .count();
503: 
504:                 // Strict minimum similarity cutoff: discard candidates below calibrated similarity threshold (0.35 / 35%)
505:                 if scored.score < 0.35 && scored.score > 0.0 && file_hits == 0 {
506:                     continue;
507:                 }
```
- **Finding**:
  1. `matches_whole_word(t.as_str())` tests whether `t` appears in `doc_tokens` (the chunk *body* text), not `file_l` (the *filename*). Thus `file_hits` counts body hits rather than filename hits.
  2. RRF scores range from `1 / (60 + rank)` = `0.01` to `0.05`. Filtering by `scored.score < 0.35` with `file_hits == 0` discards valid RRF candidates whose filenames do not contain the query terms.

### 1.6 WebUI Response Actions Bar & Settings State
- **Location**: `crates/quorum-ui/assets/app.js:852-1029` (`renderResponseActionsBar`) and `styles.css:589-642`:
  - `continue-btn` (▶️ Continue)
  - `btn-copy-response` (📋 Copy)
  - `btn-regenerate-response` (🔄 Regenerate)
  - `btn-speak-response` (🔊 Read Aloud)
  - `save-file-btn` (💾 Save File)
  - `clear-btn` (🗑️ Clear)
  - `btn-feedback-up` (👍)
  - `btn-feedback-down` (👎)
- **Settings UI Controls** in `crates/quorum-ui/assets/app.js:2354-2404`:
  - `setting-max-tokens` dropdown (256, 512, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144)
  - `setting-agentic-loop-enable` checkbox
  - `setting-agentic-chunk-size` select (2048, 4096, 8192)
  - `setting-agentic-strategy` select (`auto_continuation`, `multi_part_assembly`, `target_budget`)
- **Finding**: The WebUI settings controls already exist in the frontend and persist to backend `ChatGptUiSettings`, but the backend synthesis engine in `offline_engine.rs` has not been reading or honoring them.

---

## 2. Logic Chain

```
[Observation 1.1: offline_engine.rs:1154 has hardcoded num_predict = 1600usize]
       │
       ▼
[Finding: ui.max_output_tokens, ui.agentic_loop_enabled, ui.agentic_chunk_size are ignored]
       │
       ▼
[Observation 1.3: Ollama returns done_reason == "length", but code breaks out on done == true]
       │
       ▼
[Root Cause: Model stops generation after 1600 tokens without continuing, resulting in truncation e.g. "Sacred math has been"]
       │
       ▼
[Observation 1.2: telemetry_tx.try_send drops tokens under consumer backpressure]
       │
       ▼
[Fix Required: Use tx.blocking_send in block_in_place + recursive chaining loop up to max_output_tokens]
```

1. **Step 1**: The user selects `max_output_tokens = 262144` and `agentic_loop_enabled = true` in the WebUI.
2. **Step 2**: The settings persist via `/api/settings` into `ChatGptUiSettings`.
3. **Step 3**: During query execution, `query_local_ollama_chat` reads `ui`, but line 1154 hardcodes `num_predict = 1600`.
4. **Step 4**: When the model generates 1600 tokens, Ollama sets `done: true` and `done_reason: "length"`.
5. **Step 5**: Because line 1314 checks only `val.get("done")`, the loop immediately exits.
6. **Step 6**: The response terminates abruptly mid-sentence (e.g., *"Sacred math has been"*).
7. **Step 7**: Furthermore, if any consumer lag occurs during streaming, `tx.try_send` drops tokens silently.

---

## 3. Architectural Blueprints & Implementation Designs

### 3.1 Recursive Agentic Loop Chaining & Sentence-Boundary Guard

#### 3.1.1 Per-Hop Parameter Sizing
- If `!ui.agentic_loop_enabled` or `ui.max_output_tokens <= 4096`:
  - `hop_budget = ui.max_output_tokens.clamp(256, 8192)`
  - Single turn execution with sentence boundary guard.
- If `ui.agentic_loop_enabled` and `ui.max_output_tokens > 4096`:
  - `hop_budget = ui.agentic_chunk_size.clamp(2048, 8192)` (default: 4096)
  - `max_hops = (ui.max_output_tokens + hop_budget - 1) / hop_budget` (capped up to 64 hops for 262k tokens).

#### 3.1.2 Recursive Hop Execution Loop
```rust
let mut accumulated_tokens = 0usize;
let mut hop_count = 0usize;
let mut current_messages = messages.clone();

while accumulated_tokens < ui.max_output_tokens {
    hop_count += 1;
    let (hop_text, done_reason, eval_count) = execute_single_hop_stream(
        &addr,
        &host_port,
        target_model,
        &current_messages,
        hop_budget,
        temp,
        num_ctx,
        telemetry_tx,
    )?;

    full_text.push_str(&hop_text);
    accumulated_tokens += eval_count.max(hop_text.len() / 4);

    // If finished naturally on complete sentence, stop chaining
    if done_reason == "stop" && is_complete_sentence(&full_text) {
        break;
    }

    if done_reason == "length" && accumulated_tokens < ui.max_output_tokens {
        // Feed generated output back into context for seamless auto-continuation
        current_messages.push(serde_json::json!({
            "role": "assistant",
            "content": hop_text
        }));
        current_messages.push(serde_json::json!({
            "role": "user",
            "content": "Continue immediately from where you stopped. Maintain the exact same tone and formatting, do not repeat earlier text, and seamlessly complete the analysis."
        }));
    } else {
        break;
    }
}
```

#### 3.1.3 Zero-Drop Telemetry Streaming
In `execute_single_hop_stream`:
```rust
if let Some(tx) = telemetry_tx {
    if !tok.is_empty() {
        hop_text.push_str(tok);
        // Use blocking_send inside block_in_place so not a single token is dropped
        if tx.blocking_send(AgenticTelemetryEvent::TokenDelta {
            delta: tok.to_string(),
        }).is_err() {
            // Client disconnected / stopped
            break;
        }
    }
}
```

#### 3.1.4 Sentence-Boundary Completion Guard
```rust
fn enforce_sentence_boundary_guard(
    text: &mut String,
    addr: &SocketAddr,
    host_port: &str,
    target_model: &str,
    telemetry_tx: Option<&Sender<AgenticTelemetryEvent>>,
) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    // Valid terminators: period, exclamation, question, closing quote, markdown header/divider
    let ends_cleanly = trimmed.ends_with('.')
        || trimmed.ends_with('!')
        || trimmed.ends_with('?')
        || trimmed.ends_with(".\"")
        || trimmed.ends_with("!\"")
        || trimmed.ends_with("?\"")
        || trimmed.ends_with("```")
        || trimmed.ends_with("---");

    if !ends_cleanly {
        // Option A: Micro-budget completion prompt (128 tokens)
        let finish_prompt = [
            serde_json::json!({
                "role": "user",
                "content": format!("Complete the following unfinished sentence cleanly in one concluding phrase ending with a period:\n\"{}\"", trimmed)
            })
        ];
        if let Some(conclusion) = query_quick_completion(addr, host_port, target_model, &finish_prompt, 128) {
            let clean_concl = conclusion.trim();
            if !clean_concl.is_empty() {
                if !clean_concl.starts_with(char::is_whitespace) {
                    text.push(' ');
                }
                text.push_str(clean_concl);
                if let Some(tx) = telemetry_tx {
                    let _ = tx.blocking_send(AgenticTelemetryEvent::TokenDelta {
                        delta: format!(" {}", clean_concl),
                    });
                }
                return;
            }
        }
        // Option B: Fallback boundary snap (trim trailing partial word or clause)
        if let Some(last_boundary) = trimmed.rfind(['.', '!', '?']) {
            if last_boundary > trimmed.len() * 3 / 4 {
                *text = trimmed[..=last_boundary].to_string();
            }
        }
    }
}
```

---

### 3.2 Iterative Section Generation (`DocumentPlan` and `SectionPlan`)

For complex multi-page synthesis, broad retrieval is decomposed into structured chapters:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionPlan {
    pub section_index: usize,
    pub title: String,
    pub targeted_query: String,
    pub focus_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPlan {
    pub document_title: String,
    pub question_restatement: String,
    pub sections: Vec<SectionPlan>,
}
```

#### Execution Pipeline
1. **Outline Generation (`DocumentPlan`)**:
   From broad retrieval (20–50 chunks), synthesize an outline of 3–6 distinct sections based on subtopics.
   *Example for "tell me about sacred maths"*:
   - Section 1: *Foundations & Cosmological Origins* (`"sacred maths cosmological foundations geometry"`)
   - Section 2: *Geometric Proportions: Golden Ratio, Fibonacci & Platonic Solids* (`"golden ratio fibonacci platonic solids sacred geometry"`)
   - Section 3: *Theological, Temple & Architectural Manifestations* (`"temple architecture sacred mathematics symbolism"`)
   - Section 4: *Modern Mathematical Interpretations & Synthesis* (`"modern physics sacred geometry synthesis"`)
2. **Targeted Section Retrieval**:
   For each section, execute a targeted vector + BM25 query using `section.targeted_query` to gather dedicated chunks.
3. **Chapter Writing with Rolling Continuity (`rolling_summary`)**:
   Pass the tail (~1,500 characters) of the previous section as `rolling_summary`:
   ```markdown
   <context>
   [Dedicated chunks for this section]
   </context>
   Previous Section Tail:
   """
   {rolling_summary}
   """
   Instruction: Write Chapter {section_index}: "{title}".
   Begin directly with `## {title}` and build upon the previous section seamlessly without repeating facts.
   ```
4. **Streaming**:
   Each chapter's tokens are streamed directly to `telemetry_tx`, followed by the unified `### 📚 Sources & Document Links` block.

---

### 3.3 Map-Reduce Fact Sheet Extraction (R4)

To prevent context dilution across 50 chunks, a Map-Reduce fact extraction stage distills evidence into high-density fact sheets:

```
[Raw Retrieved Chunks: 20–50 Chunks]
             │
             ▼
   [Map / Chunk Grouping]
   Group into clusters of 3–4 chunks
             │
             ▼
   [Parallel Nugget Extraction]
   Extract: Verbatim Quotes, Dates, Names, Metrics, Theorems
             │
             ▼
   [Reduce / Deduplication]
   Consolidate overlapping facts into dense FactSheet
             │
             ▼
   [High-Density Context for Section Synthesis]
```

#### Fact Sheet Schema
```markdown
<fact_sheet source="{file_name}" index="{doc_index}">
- [Core Definition]: ...
- [Quantitative Metric / Formula]: ...
- [Historical Date / Citation]: ...
- [Verbatim Quote]: "..."
</fact_sheet>
```
Providing distilled fact sheets enables the synthesis model to generate high-density, rigorously factual chapters without hallucinations or repetition.

---

### 3.4 Parent Document Retrieval & Sequential Chunk Merging (R2)

In `crates/quorum-ingest/src/chunker.rs:178-184`:
- Chunks have `chunk_id: "{file_blake3}:{chunk_index}"`
- Metadata contains `char_start`, `char_end`, and `chunk_index`
- Source file is identified by `document_id` (`file_blake3`) and `file_path`

#### Merging Algorithm
When matched chunks share `file_path` and adjacent `chunk_index` values (`idx` and `idx + 1`):
1. Sort candidate chunks by `(file_path, line_start, chunk_index)`.
2. Stitch overlapping or contiguous chunks:
   ```rust
   if current.file_path == prev.file_path && current.line_start <= prev.line_end + 3 {
       // Merge text and update line_end
       prev.text_content.push_str("\n");
       prev.text_content.push_str(&current.text_content);
       prev.line_end = current.line_end;
   }
   ```
3. Expand single isolated chunks by querying adjacent sibling chunks `k-1` and `k+1` from local storage when `parent_document_retrieval` is enabled.

---

### 3.5 Offline Engine Retrieval Fixes (R5)

#### 1. Fix `file_hits` (line 499)
- **Before**:
  ```rust
  let file_hits = query_terms
      .iter()
      .filter(|t| matches_whole_word(t.as_str()) || file_l.contains(t.as_str()))
      .count();
  ```
- **After**:
  ```rust
  let file_hits = query_terms
      .iter()
      .filter(|t| {
          let term = t.as_str();
          file_l.contains(term)
              || (term.ends_with('s') && file_l.contains(&term[..term.len() - 1]))
              || file_tokens.contains(&term)
      })
      .count();
  ```

#### 2. Remove broken RRF similarity cutoff (line 505)
- **Before**:
  ```rust
  if scored.score < 0.35 && scored.score > 0.0 && file_hits == 0 {
      continue;
  }
  ```
- **After**:
  Remove this filter entirely. RRF scores range from 0.01 to 0.05, so `< 0.35` erroneously eliminated valid candidate chunks.

---

## 4. Response Formatting & Actions Bar (R5)

- **Question-Restating Opening**: `format_question_restated_prefix` converts queries like `"tell me about sacred maths"` into `"Sacred Maths — "` or `"Sacred Maths is "`. In `query_local_ollama_chat`, if the LLM output does not start with the topic, the prefix is prepended automatically.
- **Inline Bracketed Citations**: Citations appear as `[1]`, `[2]`, matching `<document index="...">`.
- **Deduplicated Sources Block**: `format_sources_block` outputs `### 📚 Sources & Document Links` deduplicated by filename with calibrated accuracy scores (78%–99%) and zero line numbers.
- **WebUI Response Actions Bar**: `.response-actions-bar` in `app.js` and `styles.css` is verified to render:
  - ▶️ `Continue` (`continue-btn`)
  - 📋 `Copy` (`btn-copy-response`)
  - 🔄 `Regenerate` (`btn-regenerate-response`)
  - 🔊 `Read Aloud` (`btn-speak-response`)
  - 💾 `Save File` (`save-file-btn`)
  - 🗑️ `Clear` (`clear-btn`)
  - 👍 `Thumbs Up` (`btn-feedback-up`)
  - 👎 `Thumbs Down` (`btn-feedback-down`)

---

## 5. Caveats

1. **Air-Gap Invariant**: Ollama connectivity in `query_local_ollama_chat` strictly dials local loopback (`127.0.0.1:11434`) via raw TCP streams with zero third-party cloud egress.
2. **Model Availability**: The local engine relies on pinned local models (`qwen2.5:1.5b`, `qwen2.5:7b`, `gemma3:4b`). When Ollama is offline, the deterministic offline synthesis engine fallback executes.
3. **Hardware Budget**: Chaining up to 262k tokens on large local models requires adequate RAM/VRAM. The per-hop chunk size (4096 tokens) keeps KV cache memory within bounds.

---

## 6. Conclusion

The root causes of token truncation, mid-sentence cutoffs, and dropped streaming tokens have been definitively isolated to:
1. Hardcoded `let num_predict = 1600usize;` in `crates/quorum-coordinator/src/offline_engine.rs:1154`.
2. Unhandled `done_reason == "length"` in `offline_engine.rs:1314`.
3. Use of `tx.try_send` instead of `tx.blocking_send` in `offline_engine.rs:1309`.
4. Hardcoded `take(8)` chunk limit in `offline_engine.rs:613`.
5. Retrieval bugs in `offline_engine.rs:499` (`file_hits`) and `505` (`scored.score < 0.35`).

Complete blueprints for **Recursive Turn Chaining**, **Iterative Section Generation** (`DocumentPlan`), **Map-Reduce Fact Sheet Extraction**, and **Parent Document Retrieval** have been developed to enable QuorumRAG to produce exhaustive, multi-page, cited research reports without truncation.

---

## 7. Verification Method

1. **Verify Line 1154**:
   Inspect `crates/quorum-coordinator/src/offline_engine.rs` line 1154 to confirm `let num_predict = 1600usize;`.
2. **Verify Line 1309**:
   Inspect `offline_engine.rs` line 1309 to confirm `tx.try_send(AgenticTelemetryEvent::TokenDelta ...)`.
3. **Verify Line 499 & 505**:
   Inspect `offline_engine.rs` lines 499-507 to confirm the body text check in `file_hits` and `scored.score < 0.35` cutoff.
4. **Verify WebUI Settings & Actions Bar**:
   Inspect `crates/quorum-ui/assets/app.js` lines 852-1029 and 2354-2404 to confirm the presence of all 8 action buttons and the 256–262144 token dropdown options.
5. **Project Test Suite**:
   Run `cargo test -p quorum-coordinator -p quorum-daemon` to ensure all existing and newly specified invariant tests pass cleanly.
