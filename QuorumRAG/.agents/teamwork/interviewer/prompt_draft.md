Update QuorumRAG to extract user questions from chat input with Google LangExtract, rephrase them into declarative statements for RAG retrieval, orchestrate answers strictly grounded in retrieved documents with cited document URLs/line ranges and relevance scores, maintain the top logo while removing prompt chips, provide dynamic spellchecking in the chat box, and add response action buttons.

Working directory: d:\harfile\QuorumRAG
Integrity mode: [OPEN: Which of these should the team NOT be allowed to do?]

## Requirements

### R1. Question Extraction and Declarative Restatement
Extract questions from the chat input using Google LangExtract, restate each question as the opening of an answer (e.g., "Who is femi" becomes "Femi is ..."), and pass the extracted restatement to RAG retrieval [OPEN: Pass only restated text or both restated and original query to RAG?].

### R2. Grounded Answer Orchestration and Source Citation
Orchestrate answers using an LLM strictly grounded on documents retrieved by RAG, without generating ungrounded answers from model memory. Append sources at the end of each response, including document links, line ranges, and relevance scores.

### R3. UI Home Screen Adjustments
Retain the top page `🌐 QuorumRAG` logo visible on the page, and remove the sample prompt chips from the home screen.

### R4. Dynamic Spellcheck in Chat Box
Provide dynamic spelling correction in the chat box using an LLM.

### R5. Response Action Buttons
Provide action buttons at the end of responses [OPEN: Regenerate only, Regenerate and Copy, or let team decide?].

## Acceptance Criteria

### Extraction and Retrieval
- [ ] Questions submitted in the chat box are extracted with Google LangExtract and converted to declarative answer openers before querying RAG.
- [ ] RAG answers contain only information present in retrieved documents.
- [ ] Each response concludes with citations listing document links, line ranges, and relevance scores.

### User Interface
- [ ] The `🌐 QuorumRAG` logo remains displayed at the top of the UI.
- [ ] Sample prompt chips on the home screen are absent.
- [ ] Chat box dynamically corrects or highlights spelling errors.
- [ ] Action buttons (including Regenerate) appear at the end of generated responses.

### Verification
- [ ] [OPEN: Existing repository tests and browser verification, or custom verification resources?]
