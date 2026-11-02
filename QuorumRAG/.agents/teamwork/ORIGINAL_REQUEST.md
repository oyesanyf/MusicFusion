# Original User Request

## Initial Approved Task Prompt — 2026-11-02T14:06:00Z

Adapt QuorumRAG with Google LangExtract, LLM answer orchestration over RAG chunks, chat box dynamic spellcheck, action buttons, and Mesh UI & TPM hardware identity adaptation from OshoosiClaw.

Requirements:
1. Google LangExtract:
   - Use Google LangExtract to extract questions and grounded entities from the chat box.
   - Pass what is extracted to RAG retrieval.
2. Question-Restating LLM Orchestration:
   - The LLM must orchestrate answers only to questions using the retrieved RAG chunks (e.g. "Who is femi" becomes "Femi is ...").
   - Restate the answer to the question grounded in the retrieved content (e.g., ResumeSec-AI-FemiOyesanya2025.pdf, Femi_Oyesanya_Enhanced_Resume_v3.docx, Femi_Oyesanya_CV_2026.pdf, Resume2026.pdf).
   - URL and doc link must be strictly at the end of the response with relevance scores added.
3. WebUI Chat Box & Clean Interface:
   - Dynamic real-time spellchecking in the chat box using LLM.
   - Remove the sample prompt chips on the home screen ("Explain scatter-gather retrieval", "Show RrfCombiner & write unit test", etc.), but KEEP the top page 🌐 QuorumRAG logo visible.
   - Add browser-style response action buttons at the end of each assistant response: Copy, Regenerate, Read Aloud, and feedback thumbs.
4. Adapt Mesh Configuration UI and TPM from OshoosiClaw:
   - Inspect `d:\harfile\OshoosiClaw` for its Mesh Configuration UI and TPM (Trusted Platform Module / hardware-backed identity & attestation) implementation.
   - Fully adapt OshoosiClaw's Mesh Config UI and TPM hardware identity/attestation into QuorumRAG (`crates/quorum-network`, `crates/quorum-coordinator`, `crates/quorum-daemon`, and `crates/quorum-ui`).
5. Build and Server Health:
   - In `crates/quorum-daemon/src/server.rs:115`, ensure `chat::ChatRequest` constructs with `bypass_cache: false` in tests.
   - Set `$env:CARGO_INCREMENTAL = "0"` (or `cargo clean -p quorum-daemon`) when building `quorum-daemon` to avoid rustc incremental ICE.
   - Ensure the QuorumRAG binary builds and keep `target\debug\quorumrag.exe --host 0.0.0.0 --port 3031 --data-dir "d:\harfile\QuorumRAG\.quorumrag-rag-index" --no-browser` running live, accessible on `http://10.0.0.3:3031` and `http://127.0.0.1:3031`.
   - Ensure all tests pass.


## Follow-up — 2026-11-02T14:35:45Z

URGENT USER REQUIREMENT UPDATE for Milestone M11 & M12:

User message:
"all these Line numbers cam be exclkuded  source onely and accuuracy at the bottom of tesponse  📄 Complete Assignments__.docx (Lines 35–39) ... Sources [📄 Complete Assignments__.docx:35-39](http://127.0.0.1:3031/#)..."

Exact requirements to enforce in both `crates/quorum-coordinator/src/offline_engine.rs` and `crates/quorum-ui/assets/app.js`:
1. NO Line Numbers Anywhere:
   - Remove `(Lines X–Y)` and `:line_start-line_end` completely from both the synthesized response body AND the bottom `Sources` citation badges in `app.js` (`appendCitation`).
2. NO Raw File Headers in the Response Body:
   - Even in the deterministic fallback path of `offline_engine.rs` (and in LLM orchestration), NEVER output `### 📄 filename (Lines X–Y)` headers or raw ChatGPT export artifacts (`fileciteturn0file0`, `sandbox:/mnt/data/...`). Synthesize a single cohesive, affirmative prose answer restating the question (e.g. `"Femi Oyesanya is a CISSP-certified Application and AI Security professional..."`).
3. Source Only + Accuracy Score at the Bottom of Response:
   - At the bottom of the response (`Sources`), display only the source document name/link and its accuracy/relevance score (e.g. `📄 Femi_Oyesanya_CV_2026.pdf · 96%` or `📄 Resume2026.pdf (98% accuracy)`), with zero line numbers!
   - Deduplicate `Sources` by file name so the same file (e.g. `Complete Assignments__.docx` or `Case Discrepancies Review.docx`) never appears multiple times in the `Sources` bar.
4. Filter Out ChatGPT Export Artifacts:
   - Exclude chunks containing `fileciteturn0file0` or `sandbox:/mnt/data/` (or strip those tokens) and ensure profile queries (`"Who is Femi"`, `"Femi Oyesanya"`) strongly prioritize CV/Resume documents (`Resume2026.pdf`, `Femi_Oyesanya_CV_2026.pdf`, `Femi_Oyesanya_Enhanced_Resume_v3.docx`).


## Follow-up — 2026-11-02T14:36:03Z

User message:
"all these Line numbers cam be exclkuded  source onely and accuuracy at the bottom of tesponse  📄 Complete Assignments__.docx (Lines 35–39) ... Sources [📄 Complete Assignments__.docx:35-39](http://127.0.0.1:3031/#)..."

Requirements:
1. Exclude Line Numbers completely: No `(Lines X–Y)` or `:line_start-line_end` in response text or bottom citations.
2. No Raw File Headers in Response Body: Never output `### 📄 filename (Lines X–Y)` headers in the synthesized response. Output a cohesive prose answer restating the question affirmatively.
3. Source Only + Accuracy Score at Bottom of Response: Only source document name/link and accuracy/relevance percentage (e.g. `📄 Femi_Oyesanya_CV_2026.pdf · 96%`), deduplicated by file name.
4. Filter Out ChatGPT Export Artifacts: Filter `fileciteturn0file0` or `sandbox:/mnt/data/` and strongly prioritize resume/CV documents (`Resume2026.pdf`, `Femi_Oyesanya_CV_2026.pdf`, etc.) for profile queries.


## Follow-up — 2026-11-02T17:22:00Z

URGENT USER FEEDBACK FROM LIVE TESTING ON PORT 3031 — MUST BE INCLUDED AND VERIFIED BEFORE COMPLETION:

The user just tested "tell me all about african church and gang violance" on http://127.0.0.1:3031 and uploaded a screenshot of the .response-actions-bar below SOURCES. Incorporate and verify all 3 of the following requirements across crates/quorum-coordinator/src/offline_engine.rs and crates/quorum-ui/assets/app.js (and styles.css):

1. Action Buttons Bar (.response-actions-bar in crates/quorum-ui/assets/app.js & styles.css) — Add Continue, Save File, and Clear Buttons:
   - Add three interactive buttons to .response-actions-bar in app.js:
     - ▶️ Continue (continue-btn): Sends a continuation query expanding the answer for that turn's prompt with deeper details from the RAG index.
     - 💾 Save File (save-file-btn): Triggers a real browser file download (Blob + <a download="quorumrag-response-....md">) saving the response text and its sources to disk.
     - 🗑️ Clear (clear-btn): Removes that response turn (and preceding user prompt bubble, or resets to welcome banner if no turns remain) and persists the updated chat session.

2. Every Response Must Repeat the Question/Topic at the Start like a Q&A ("The response must repeat the question For example African Church and gang violence ....... it must answer like a QA"):
   - When the user asked "tell me all about african church and gang violance":
     - Add "violance" -> "violence", "churh" -> "church", "continuew" -> "continue", "lile" -> "like" to EXACT_TYPO_MAP and "African", "Church", "Churches", "gang", "violence" to SEED_WORDS in crates/quorum-coordinator/src/offline_engine.rs.
     - In synthesize_with_history, run let corrected_prompt = Self::correct_query_spelling(prompt, false).0; so typos like "violance" are corrected to "violence" for both query-term matching and the Q&A opening restatement.
     - In format_question_restated_prefix:
       - Support "tell me all about <X>", "tell me everything about <X>", "tell me about <X>", "what do you know about <X>", "give me details on <X>", etc.
       - NEVER return an empty string "" for any non-empty prompt! If a prompt does not match a specific "who is"/"what is"/"where did" template, strip conversational command prefixes and return format!("{} — ", capitalize(topic)) or format!("{} is ", capitalize(topic)) so EVERY response begins by repeating the question/topic in Q&A style.
       - Also ensure that in query_local_ollama_chat, if the LLM output does not already start with the subject/topic words, prepend the Q&A restatement prefix automatically.
     - Clean up mid-word chunk boundary artifacts in clean_chunk_text / body_paragraphs:
       - Strip leading broken lowercase word fragments at the start of a chunk and trim trailing mid-word cutoffs to the last complete sentence or word boundary.
       - Skip body chunks that are only a bare document title (< 35 chars) when longer body chunks are present.

3. Filename Title Match Priority & Calibrated Grounding Accuracy Scores (75%–99% instead of raw RRF 2%):
   - In synthesize_with_history (offline_engine.rs):
     - Compute file_hits (how many query/entity terms match tokens in file_l, including singular/plural "church" <-> "churches" and 1-edit-distance typo tolerance "violance" <-> "violence"). Give a strong filename title match bonus (doc_bonus += (file_hits as i32) * 160) so African Churches and Gang Violence.docx ranks #1.
     - Normalize cit.score on each returned Citation (and in format_sources_block) from raw RRF (0.01..0.20) into a calibrated grounding accuracy score (0.78..0.99, i.e., 78%–99%) based on query term coverage, filename match, and rank order (e.g., top full-match source gets 96%–98%, subsequent sources get 84%–94%), so citation badges never display raw RRF 2%.

4. Hot-Restart quorumrag_live.exe / quorumrag.exe on Port 3031 in M14:
   - In Milestone M14, stop PID 80036 (quorumrag_live.exe), rebuild the fresh binary, and launch it on 0.0.0.0:3031 so http://127.0.0.1:3031 serves all new M11–M14 code live.


## Follow-up — 2026-11-02T17:28:00Z

USER ARCHITECTURAL BLUEPRINT FOR MILESTONE M13.5 & M14:

Incorporate and test all 5 of these architectural upgrades across crates/quorum-coordinator, crates/quorum-ingest, crates/quorum-storage, and crates/quorum-ui:

1. Long-Form Grounded XML Synthesis Prompt & Ollama Runtime Parameters (crates/quorum-coordinator/src/offline_engine.rs):
   - Format retrieved documents inside structured <context><document index="{}" source="{}">\n{}\n</document></context> XML blocks.
   - Pass up to 8–10 merged/expanded parent passages (up to 2,500 chars per passage) for full multi-paragraph context.
   - Configure Ollama /api/chat options: num_ctx: 16384, num_predict: 4096, temperature: 0.15, top_p: 0.9, repeat_penalty: 1.15, num_thread: 16.
   - System & user instructions must enforce:
     1. Q&A Question-Repeating Opening: Always start by repeating/restating user's question in Q&A form (e.g., "African Churches and Gang Violence — ...", "Angela Davis is ...").
     2. Exhaustive Long-Form Analysis: Write comprehensive multi-paragraph analysis covering every historical event, date, name, quote, argument in <context>.
     3. Strict Grounding & Honest Missing-Info Reporting: Never invent facts; if excerpts lack information (e.g. "Eastern University"), state clearly that info is not found in local files.
     4. Inline Citations: Cite with [1], [2], etc., matching <document index="...">, with deduplicated sources and calibrated accuracy scores at bottom (zero line numbers).
   - Deterministic synthesis fallback must perform Two-Pass Evidence Extraction + Long-Form Multi-Paragraph Q&A Synthesis.

2. Boundary-Aware Word/Sentence/Paragraph Chunking & Document Title Header Injection (crates/quorum-ingest/src/chunker.rs & offline_engine.rs):
   - In chunker.rs: boundary-aware word/sentence splitting with ~15% overlap, max_chars ~1200, overlap_chars ~200, prepending Document Title: <clean_title>.
   - In offline_engine.rs (clean_chunk_text): automatically strip leading broken mid-word lowercase fragments (e.g. "rs in gang infested areas", "gela Davis", "ecus is the earliest") and trim trailing mid-word cutoffs ("...propo") back to last complete sentence or word boundary.
   - Parent Document / Sequential Chunk Merging: sort and merge adjacent/overlapping chunks into continuous unbroken parent passages before synthesis.

3. Strict Minimum Similarity Cutoff, Phrase/Named-Entity Co-Occurrence Guard, & Title Match Priority (offline_engine.rs):
   - For multi-word queries like "eastern university", require non-stopword query terms to co-occur in same chunk/doc (and adjacent phrase or same sentence).
   - Strict minimum similarity cutoff: discard candidates below calibrated similarity threshold (0.35 / 35%). If empty (e.g. "Eastern University"), return: "No matching documents were found in your local RAG index for **Eastern University**." with zero bogus citations.
   - Title match priority bonus (+160 per matched term in filename) so African Churches and Gang Violence.docx ranks #1 with 92%–98% calibrated accuracy.

4. Conversational Follow-Up Router for "continue" / "tell me more" & Offset Pagination (classify.rs, retrieve.rs, offline_engine.rs):
   - Detect continuation intent ("continue", "tell me more", "go on", "what else", ▶️ Continue button).
   - Rewrite search query using last non-continuation user question from history.
   - Exclude chunks or snippet text already presented in preceding assistant message so "continue" fetches next unseen chunks (pagination offset).

5. WebUI Action Buttons (▶️ Continue, 💾 Save File, 🗑️ Clear) + Live Port 3031 Restart:
   - Ensure .response-actions-bar in app.js includes ▶️ Continue, 💾 Save File, 🗑️ Clear alongside Copy, Regenerate, Read Aloud, and Thumbs.
   - In Milestone M14, terminate old quorumrag_live.exe (PID 80036), rebuild fresh quorumrag.exe, start on 0.0.0.0:3031, and verify all 4 live queries ("tell me all about african church and gang violance", "tell me about Angela davis", "continue", "tell me about eastern university") against http://127.0.0.1:3031/api/query.


## Follow-up — 2026-11-02T17:28:23Z

USER ARCHITECTURAL BLUEPRINT FOR MILESTONE M13.5 & M14:

Incorporate and test all 5 of these architectural upgrades across crates/quorum-coordinator, crates/quorum-ingest, crates/quorum-storage, and crates/quorum-ui:

1. Long-Form Grounded XML Synthesis Prompt & Ollama Runtime Parameters (offline_engine.rs):
   - XML context: <context><document index="{}" source="{}">\n{}\n</document></context>
   - Pass 8–10 merged parent passages (up to 2,500 chars per passage).
   - Configure Ollama chat options: num_ctx: 16384, num_predict: 4096, temperature: 0.15, top_p: 0.9, repeat_penalty: 1.15, num_thread: 16.
   - Enforce: Q&A Question-Repeating Opening; Exhaustive Long-Form Analysis; Honest Missing-Info Reporting (e.g. Eastern University -> no local match); Inline Citations [1], [2] with deduplicated scores and zero line numbers.
   - Deterministic synthesis fallback: Two-Pass Evidence Extraction + Long-Form Multi-Paragraph Q&A Synthesis.

2. Boundary-Aware Word/Sentence/Paragraph Chunking & Document Title Header Injection (chunker.rs & offline_engine.rs):
   - In chunker.rs: boundary-aware splitting, max_chars ~1200, overlap_chars ~200, prepending Document Title: <clean_title>.
   - In offline_engine.rs (clean_chunk_text): strip leading broken mid-word fragments ("rs in gang...", "gela Davis") and trim trailing mid-word cutoffs ("...propo") to complete sentence/word boundary.
   - Parent Document / Sequential Chunk Merging: sort and merge adjacent/overlapping chunks into continuous unbroken parent passages.

3. Strict Minimum Similarity Cutoff, Phrase/Named-Entity Co-Occurrence Guard, & Title Match Priority (offline_engine.rs):
   - Multi-word queries like "eastern university": require non-stopword query terms to co-occur in same chunk/doc.
   - Discard candidates below calibrated threshold (0.35 / 35%). If empty, return "No matching documents were found in your local RAG index for **Eastern University**." with zero bogus citations.
   - Title match priority bonus (+160 per matched term in filename) so African Churches and Gang Violence.docx ranks #1 with 92%–98% calibrated accuracy.

4. Conversational Follow-Up Router for "continue" / "tell me more" & Offset Pagination (classify.rs, retrieve.rs, offline_engine.rs):
   - Detect continuation intent ("continue", "tell me more", ▶️ Continue button).
   - Rewrite query using last non-continuation user question from history.
   - Exclude chunks or snippet text already presented in preceding assistant message so "continue" fetches next unseen chunks (pagination offset).

5. WebUI Action Buttons (▶️ Continue, 💾 Save File, 🗑️ Clear) + Live Port 3031 Restart:
   - Ensure .response-actions-bar in app.js includes ▶️ Continue, 💾 Save File, 🗑️ Clear.
   - In Milestone M14, terminate PID 80036 (quorumrag_live.exe), rebuild fresh quorumrag.exe, start on 0.0.0.0:3031, and verify all 4 live queries ("tell me all about african church and gang violance", "tell me about Angela davis", "continue", "tell me about eastern university") against http://127.0.0.1:3031/api/query.
