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
