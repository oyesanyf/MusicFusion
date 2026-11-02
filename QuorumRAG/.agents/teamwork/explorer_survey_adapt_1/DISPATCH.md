# Task Assignment: Survey Explorer 1 (Google LangExtract & Question-Restating LLM Orchestration)

## Working Directory
d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_1\

## Mission & Scope
Map and investigate how to implement Requirements 1 & 2 from ORIGINAL_REQUEST.md:
1. Google LangExtract:
   - Use Google LangExtract to extract questions and grounded entities from the chat box.
   - Pass what is extracted to RAG retrieval.
2. Question-Restating LLM Orchestration:
   - The LLM must orchestrate answers only to questions using the retrieved RAG chunks (e.g. "Who is femi" becomes "Femi is ...").
   - Restate the answer to the question grounded in the retrieved content (e.g., ResumeSec-AI-FemiOyesanya2025.pdf, Femi_Oyesanya_Enhanced_Resume_v3.docx, Femi_Oyesanya_CV_2026.pdf, Resume2026.pdf).
   - URL and doc link must be strictly at the end of the response with relevance scores added.

Investigate:
- How Google LangExtract is currently configured and implemented in `crates/quorum-coordinator/src/langextract/` and `crates/quorum-types`.
- How the chat box / query flow currently runs (`ClassifyIntent` -> `RetrieveMesh` -> `GradeRelevance` -> `SynthesizeWithCitations` or streaming in `crates/quorum-coordinator/src/synthesis.rs`, `query.rs`, `retrieve.rs`, `crates/quorum-ui/src/routes/chat.rs`).
- How to extract questions and grounded entities from the chat box input via LangExtract and pass them into RAG retrieval (`RetrieveMesh` scalar/keyword/vector filters).
- How the prompt template or LLM synthesis logic should be formatted to enforce question-restating ("Who is X" -> "X is ...") and grounding strictly in retrieved content (especially resume/profile files like `ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_Enhanced_Resume_v3.docx`, etc.).
- How document and URL citations are formatted and placed strictly at the end of the response, including relevance scores (e.g. `[Score: 0.95] [ResumeSec-AI-FemiOyesanya2025.pdf]`).
- Identify all affected files, modules, function signatures, data structures, and edge cases.
- Produce a detailed technical handoff report in `d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_1\handoff.md`.

## 2026-11-02T14:10:05Z
From: 6e43b521-b448-4762-842a-0bc8d66e2854
Priority: MESSAGE_PRIORITY_HIGH
Content: You are Survey Explorer 1 (teamwork_explorer).
Your working directory is: d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_1\
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md and d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_1\DISPATCH.md immediately.

Investigate Requirements 1 & 2:
1. Google LangExtract:
   - Use Google LangExtract to extract questions and grounded entities from the chat box.
   - Pass what is extracted to RAG retrieval.
2. Question-Restating LLM Orchestration:
   - The LLM must orchestrate answers only to questions using the retrieved RAG chunks (e.g. "Who is femi" becomes "Femi is ...").
   - Restate the answer to the question grounded in the retrieved content (e.g., ResumeSec-AI-FemiOyesanya2025.pdf, Femi_Oyesanya_Enhanced_Resume_v3.docx, Femi_Oyesanya_CV_2026.pdf, Resume2026.pdf).
   - URL and doc link must be strictly at the end of the response with relevance scores added.

Examine existing files in `crates/quorum-coordinator/src/langextract/`, `crates/quorum-coordinator/src/synthesis.rs`, `query.rs`, `retrieve.rs`, `crates/quorum-types/`, and `crates/quorum-ui/src/routes/chat.rs`.
Identify what data structures, methods, prompt templates, and streaming handlers need to change.
Write your complete, structured analysis and findings to d:\harfile\QuorumRAG\.agents\teamwork\explorer_survey_adapt_1\handoff.md.
When finished, send a completion message via send_message to your parent orchestrator.
