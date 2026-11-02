## 2026-11-02T01:38:50Z

Make QuorumRAG add no noticeable I/O, CPU, or memory overhead to host systems or local storage operations while eliminating all indexing blind spots, filesystem race conditions, and exploitable peer-to-peer vulnerabilities across the entire distributed search lifecycle. The system must operate strictly offline with zero external internet search requests, preserving search recall, multimodal document and image retrieval accuracy, and quorum consensus scoring.

Integrity mode: development

## Requirements

### R1. Zero-Overhead Ingestion Pipeline, Deduplication, and Binary Filtering
Eliminate indexing blind spots, filesystem race conditions, and omitted files across physical drive discovery, selective document and image ingestion, and continuous real-time filesystem monitoring. Ensure deduplication state is hydrated on startup from existing storage chunks (such as LanceDB) and live indexer progress reporting accurately reflects total candidate files without slice undercounting. The ingestion pipeline must strictly index all target documents and image assets across available physical drives while deterministically filtering out compiled binaries, system libraries, and executables (including EXE and DLL files) prior to content parsing or embedding.

### R2. Low-Latency Query Execution, Consensus, and Peer-to-Peer Security Hardening
Eliminate noticeable CPU, memory, and I/O overhead during multi-hop RAG query execution chains, peer-to-peer mesh search routing, quorum consensus, and data verification handshakes. The system must operate strictly offline with zero external internet search requests. Hardening must eliminate exploitable peer-to-peer vulnerabilities while maintaining or improving distributed search recall, multimodal document and image retrieval accuracy, and quorum consensus scoring. Keep on-disk index storage schemas, physical drive mount interfaces, cryptographic chunk attestation and ACL payloads, lexical and vector ranking semantics, legitimate host disk throughput, and public Rust APIs unchanged.

### R3. User Interface Responsiveness and Indexer Controls
Provide an active stop button on the chat interface allowing immediate query cancellation and responsive chat interactions. Update live indexer progress display components to accurately reflect real-time chunk and file counts without misleading or stale status states.

### R4. Verification Resources, Benchmarks, and Solo Developer Process
Use QuorumRAG's existing automated test and benchmark suite, and back each performance optimization, file filtering rule, and mesh security hardening change with a deterministic benchmark or simulated Byzantine peer, spoofing, and drive fault regression test proving isolation and catching regressions. Follow the solo developer process: branch cleanly from main, commit atomic and well-documented changes, run the full local test and benchmark suite to verify zero regressions, open one pull request against main with before and after query latency, disk I/O, CPU, memory footprint, document and image index coverage, binary exclusion efficiency, and mesh consensus metrics in its description, perform a rigorous self-review, and merge it directly into main once all checks pass.

## Acceptance Criteria

### Performance and Host Overhead
- [ ] Ingestion, continuous filesystem monitoring, and query execution add no noticeable I/O, CPU, or memory overhead to host systems or local storage.
- [ ] Legitimate host disk throughput, lexical/vector ranking semantics, and public Rust APIs remain unchanged.
- [ ] Multi-hop RAG query execution chains execute strictly offline with zero external internet search requests.

### Ingestion Accuracy, Deduplication, and Binary Filtering
- [ ] Ingestion strictly indexes all target documents and image assets across available physical drives without file omission blind spots.
- [ ] Compiled binaries, system libraries, and executables (including EXE and DLL files) are deterministically filtered out prior to content parsing or embedding.
- [ ] Startup deduplication engine hydration and file estimation accurately reflect total files across folders without undercounting or race conditions.

### User Interface Controls and Latency
- [ ] The chat interface provides a working stop button that immediately cancels in-flight queries.
- [ ] Live indexer status and progress indicators accurately report chunk counts and file totals in real time.

### Mesh Security, Byzantine Resilience, and Consensus
- [ ] Simulated Byzantine peer, spoofing, and drive fault regression tests confirm peer-to-peer mesh resilience and isolation.
- [ ] Peer-to-peer mesh search routing, quorum consensus, and data verification handshakes eliminate exploitable vulnerabilities while keeping consensus scoring intact or improved.

### Solo PR Delivery and Verification
- [ ] All changes are developed on a clean branch from main with atomic, well-documented commits.
- [ ] The full local test and benchmark suite passes with zero regressions.
- [ ] A pull request is opened against main containing before and after metrics for query latency, disk I/O, CPU, memory footprint, document and image index coverage, binary exclusion efficiency, and mesh consensus, followed by self-review and clean merge into main.


## 2026-11-02T02:59:15Z

New user requirement update to incorporate into Milestone M9 (`crates/quorum-ui/assets/app.js`):
> "This randone words are not enough🤖 QuorumRAG (Agentic State Graph) ✨ Thinking..."

Please incorporate these exact specifications into Milestone M9 and its scope/dispatch documents:
1. Expand `THINKING_PHRASES` from a small 9-item list into a large pool (60+ varied, engaging status words/phrases plus dynamic randomized combinations like "Pondering...", "Sifting through local vectors...", "Connecting the dots...", "Mulling it over...", "Synthesizing insights...", "Traversing local knowledge graph...", "Weighing evidence...", "Distilling key facts...", "Evaluating context...", "Unpacking details...", "Formulating response...", "Drafting sections...", "Checking local citations...", "Refining narrative...", "Piecing together sources...", "Scanning local embeddings...", "Crunching context...", "Deliberating...", "Mapping relationships...", "Structuring paragraphs...", etc.).
2. Start on a random phrase (not always index 0 `"✨ Thinking..."`) and cycle rapidly (every ~650ms) picking a truly random next phrase (never repeating the same phrase twice in a row) until the first `TokenDelta` arrives.
3. Ensure `handleTelemetryEvent` (such as `StateTransition`, `HardwareProbed`, `IntentClassified`) never stops or freezes the thinking phrase interval before actual `TokenDelta` text arrives.

Recorded to `.agents/teamwork/ORIGINAL_REQUEST.md`.


## 2026-11-02T03:08:14Z

CRITICAL NEW USER REQUIREMENT:
> "the code must excude source code"

Please immediately incorporate these mandatory specifications into the active pipeline (`crates/quorum-ingest` / `ClassifierRouter` / `FilesystemCrawler`), `PROJECT.md`, and active milestones:
1. The ingestion pipeline strictly indexes **documents, text/data documents, and image assets** (e.g. `.pdf`, `.docx`, `.doc`, `.odt`, `.rtf`, `.epub`, `.xlsx`, `.xls`, `.ods`, `.pptx`, `.ppt`, `.odp`, `.txt`, `.md`, `.csv`, `.tsv`, `.json`, `.png`, `.jpg`, `.jpeg`, `.webp`, `.gif`, `.bmp`, `.svg`, `.tiff`, etc.) and **EXCLUDES source code files** (`FileCategory::Code` / source code extensions such as `.rs`, `.py`, `.js`, `.ts`, `.jsx`, `.tsx`, `.c`, `.cpp`, `.h`, `.hpp`, `.cc`, `.cs`, `.java`, `.go`, `.kt`, `.rb`, `.php`, `.swift`, `.scala`, `.sh`, `.bash`, `.ps1`, `.bat`, `.cmd`, `.sql`, `.css`, `.scss`, `.less`, `.vue`, `.svelte`, etc.) in addition to excluding compiled binaries, system libraries, and executables (`EXE`, `DLL`, `SYS`, `PDB`, `OBJ`, `LIB`, etc.).
2. Preserve any existing unit tests in `quorum-classifier` (so `FileTypeClassifier::classify_bytes` still identifies `FileCategory::Code` for classifier unit tests), while filtering out `FileCategory::Code` / source code extensions in `quorum-ingest` (`ClassifierRouter` / `FilesystemCrawler`) or making source code exclusion the default in `IngestConfig` (`exclude_source_code: true`) unless a test explicitly enables source code indexing.
3. Also purge/skip any indexed source code chunks during reindexing or retrieval so RAG searches only return user documents and images!

Recorded to `.agents/teamwork/ORIGINAL_REQUEST.md`.


## 2026-11-02T03:16:16Z

CRITICAL USER FEEDBACK & REQUIREMENTS:

1. Relevance & Precision Guard (Prevent Accidental Single-Word Matching):
   - When a multi-word query is asked (e.g. "what is universal mineral income"), reject spurious hits on single common words like "income".
   - In `crates/quorum-coordinator/src/offline_engine.rs` (and `retrieve.rs`):
     a) Expand `GENERIC_ATTRIBUTE_TERMS` to include generic financial/common nouns (`income`, `tax`, `taxes`, `statement`, `plan`, `payment`, `credit`, `account`, `amount`, `rate`, `form`, `section`, `page`, `total`, `year`, `day`, `heart`, `soul`, `life`, `time`, `work`, `world`, `reading`, `book`, `universal`, etc. when not paired).
     b) Multi-term entity matching: when `entity_terms.len() >= 2`, a chunk must match at least `(entity_terms.len() + 1) / 2` distinct `entity_terms` (for 2 or 3 terms, at least 2 distinct terms must appear, and at least one non-generic primary subject term must match). Single-word accidental matches on multi-word queries must be rejected (`is_relevant = false`).
     c) Never output `Retrieved Grounded Evidence (N matching chunks)`!

2. Sources Listed After All Response ("source must be listed after all response"):
   - Every response must list `Sources` (`citations`) strictly at the bottom AFTER the full response text has completed.
   - Stream/emit all `TokenDelta` chunks first, and emit `CitationEmitted` events only after response text is complete.
   - For general knowledge queries with 0 matching local files, emit clean bottom source attribution citation (`Local Neural Model (Air-Gapped Ollama)`) so `SOURCES` is always present at the bottom of every response.

3. Streaming Prompt Optimization & Latency ("tell me about Femi Oyesanya"):
   - Prevent stalling on "⚡ Synthesizing comprehensive response...":
   - Stream tokens immediately via SSE (`TokenDelta`), use fast compact prompt (top 4 deduplicated chunks, max 900 chars each, `num_predict: 600`, `num_ctx: 2048`) so tokens stream within 1-2 seconds.

Recorded to `.agents/teamwork/ORIGINAL_REQUEST.md`.


## 2026-11-02T03:18:08Z

EXACT ROOT-CAUSE & ARCHITECTURAL REMEDIATION:

1. Streaming Freeze & Inverted Citations Root Cause:
   - In `crates/quorum-coordinator/src/agentic/synthesize.rs`: `SynthesizeWithCitationsNode::execute` passed `None` for `telemetry_tx` to `synthesize_with_history(...)`. This forced `query_local_ollama_chat` into synchronous blocking mode (`use_streaming = false`), freezing the UI on "⚡ Synthesizing comprehensive response..." for 15-30s until all 600 tokens completed.
   - In `crates/quorum-coordinator/src/agentic/mod.rs`: `CitationEmitted` and `ExecutionFinished` were emitted before `query_agentic_with_history` returned, and before tokens were ever sent.
   - In `crates/quorum-daemon/src/handlers/chat.rs`: `chat.rs` tried to simulate streaming with a post-hoc `for word in words` loop after the query returned.

2. Architecture Fix:
   - `synthesize.rs`: Update `SynthesizeWithCitationsNode::execute(state, telemetry_tx: Option<&Sender<AgenticTelemetryEvent>>)` to forward `telemetry_tx` to `synthesize_with_history(...)`.
   - `offline_engine.rs`: Stream `TokenDelta` live from `query_local_ollama_chat`. If fallback branch, emit `TokenDelta` with body.
   - `offline_engine.rs` (Word Boundary Entity Matching): Tokenize chunk text into alphanumeric words rather than naive substring `.contains()`. Multi-term queries require at least `(entity_terms.len() + 1) / 2` distinct terms.
   - `agentic/mod.rs`: Ensure `TokenDelta` is streamed first. Only after synthesis completes, emit `CitationEmitted` (with fallback attribution `Local Neural Engine (Ollama)` if empty), followed by `ExecutionFinished`.
   - `chat.rs`: Remove post-hoc word split loop; let live SSE events pass through cleanly.

Recorded to `.agents/teamwork/ORIGINAL_REQUEST.md`.


## 2026-11-02T04:08:43Z

Live verification succeeded across all 3 test queries:
1. "what is universal mineral income" -> 0 false-positive tax PDFs, clean Markdown prose, Sources Cited: Local Neural Model (Air-Gapped Ollama).
2. "tell me about Femi Oyesanya" -> Grounded Markdown prose citing real local resumes.
3. "where did he go to school?" -> Completed in 7.7s citing transcripts and resumes.

Please incorporate this polish directly into Milestone M8.5 (Source Code & Temp File Exclusion):
- In `offline_engine.rs`, `retrieve.rs`, and `quorum-ingest` (`ClassifierRouter` / `FilesystemCrawler`), also filter out Office/Word temporary lock & autosave files (`file_name.starts_with('~')` or `file_name.ends_with(".tmp")` or `file_name.ends_with(".bak")`) in addition to excluding source code files (`FileCategory::Code`) and compiled binaries (`EXE`, `DLL`, etc.).

Recorded to `.agents/teamwork/ORIGINAL_REQUEST.md`.


## 2026-11-02T04:56:25Z
CRITICAL INVARIANTS APPLIED TO WORKSPACE & MILESTONE ROADMAP:

1. **Strict RAG-Only Synthesis (`"I dont think I want responses from the model just from RAG"`)**:
   - In `crates/quorum-coordinator/src/offline_engine.rs` and `crates/quorum-coordinator/src/agentic/mod.rs`:
     - Removed `default_model_citation` (`Local Neural Model` / `Local Neural Engine`) and removed ungrounded LLM synthesis (`query_local_ollama_chat`).
     - Responses are now synthesized 100% deterministically from retrieved RAG chunks (`matched_chunks`) with `### 📄 <filename>` headers and real local file `Sources` at the bottom.
     - When `chunks.is_empty()` or `matched_chunks.is_empty()`, returns `"No matching documents were found in your local RAG index for **<prompt>**."` with `Vec::new()` citations.
   - In `crates/quorum-coordinator/src/agentic/retrieve.rs`:
     - Applied the multi-term precision fix (`content_matched_terms >= required_query_hits && entity_hits >= required_entity_hits`) and daemon-active source-code/dump filtering.
2. **Source Code & Dump Exclusion (M8.5)**:
   - Added `exclude_source_code: bool` to `IngestConfig` (`default: false` for unit test compatibility, set to `true` in `quorum-daemon/src/main.rs`).
   - Updated `ClassifierRouter` and `FilesystemCrawler` to exclude `FileCategory::Code`, source/config/dump extensions, and Office temporary lock files (`~$*`, `.~lock*`, `*.tmp`, `*.bak`, `*.wbk`).
3. **UI Welcome Banner Removal (`"REMOVE THESE"`)**:
   - Removed the welcome banner and quick-prompt chips from `crates/quorum-ui/assets/index.html`, `app.js`, and `styles.css`.

Please ensure these invariants are strictly preserved across M8, M8.5, M9, and M10! Recorded to `ORIGINAL_REQUEST.md`.
