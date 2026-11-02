# Scope: QuorumRAG Adaptation & Hardening

## Architecture
QuorumRAG adaptation adds Google LangExtract candidate extraction and 4-arm consensus to chat box queries, question-restating LLM synthesis over retrieved RAG chunks (e.g. resume documents) with citations and scores strictly at the end, dynamic LLM spellcheck in the chat box, response action buttons (Copy, Regenerate, Read Aloud, Thumbs Up/Down), clean UI (prompt chips removed, top logo preserved), OshoosiClaw Mesh Configuration UI and TPM 2.0 hardware identity & attestation, and port 3031 live hosting.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 58 | Google LangExtract Chat Extraction | Extract question spans, document references, named entities & technical terms from chat box queries; verify char intervals; run 4-arm consensus | M11 | ORIGINAL_REQUEST §1 |
| 59 | Question-Restating LLM Orchestration & End Citations | Orchestrate answers only to questions using retrieved RAG chunks, restating questions affirmatively ("Who is femi" -> "Femi is ..."), grounding in resume/document chunks, placing URLs/doc links strictly at the end with relevance scores | M11 | ORIGINAL_REQUEST §2 |
| 60 | Clean WebUI & Logo Retention | Remove home screen prompt chips while keeping top page 🌐 QuorumRAG logo visible across all views | M12 | ORIGINAL_REQUEST §3 |
| 61 | Dynamic Real-Time Chat Box Spellcheck | Debounced real-time LLM-assisted spellcheck in chat box via local Ollama with deterministic dictionary fallback, preserving cursor position | M12 | ORIGINAL_REQUEST §3 |
| 62 | Response Action Buttons & server.rs Fix | Browser-style response action buttons (Copy, Regenerate, Read Aloud via Web Speech API, Feedback Thumbs) below citations; fix `server.rs:115` `bypass_cache: false` | M12 | ORIGINAL_REQUEST §3, §5 |
| 63 | TPM 2.0 Hardware Identity & Attestation | Adapt OshoosiClaw TPM 2.0 inspection (<1ms Windows WMI registry + Linux sysfs + simulated fallback), PCR 0/7/16 composite digest, and X.509 EK DER parsing | M13 | ORIGINAL_REQUEST §4 |
| 64 | Attestation-Based Access Tier Gating | Dynamically authorize verified TPM 2.0 peers for higher tiers in `PeerAclManager`, software fallbacks to Tier 1, unverified to Tier 0 | M13 | ORIGINAL_REQUEST §4 |
| 65 | OshoosiClaw Mesh Configuration UI & Daemon Endpoints | Adapt OshoosiClaw Mesh UI into Settings ("Mesh & Shared Folders"): TPM RoT badge, connected peers with attestation badges, Pending Joins HITL approval queue, Quarantined Peers panel, and WAN multiaddr/DuckDNS settings; wire daemon REST endpoints | M13 | ORIGINAL_REQUEST §4 |
| 66 | Final Workspace Build, Port 3031 Live Server & Solo PR Ship | Verify `$env:CARGO_INCREMENTAL = "0"`, all tests pass, keep `quorumrag.exe` running on `0.0.0.0:3031` accessible at `http://10.0.0.3:3031` and `http://127.0.0.1:3031`, solo PR ship to main | M14 | ORIGINAL_REQUEST §5 |
| 67 | Action Buttons Bar: Continue, Save File & Clear | Add ▶️ Continue, 💾 Save File (Blob download), and 🗑️ Clear buttons to `.response-actions-bar` in `app.js` and `styles.css` | M13.5 | ORIGINAL_REQUEST §Follow-up 2026-11-02T17:22:00Z |
| 68 | Q&A Opening Restatement, Typo Map & Grounding Calibration | Enforce Q&A opening topic restatement for all prompts, expand typo map & seed words, apply `file_hits * 160` filename title bonus, calibrate grounding accuracy scores (78%–99%), clean chunk boundary artifacts in `offline_engine.rs` | M13.5 | ORIGINAL_REQUEST §Follow-up 2026-11-02T17:22:00Z |
| 69 | Port 3031 Live Server Hot-Restart | Stop PID 80036 (quorumrag_live.exe), recompile, and launch live daemon on `0.0.0.0:3031` in M14 | M14 | ORIGINAL_REQUEST §Follow-up 2026-11-02T17:22:00Z |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M11 | Google LangExtract & Question-Restating LLM Orchestration | Features 58, 59: Candidate extraction in `langextract/extraction.rs`, state graph propagation, entity boosting in `retrieve.rs`, question-restating prompts & deterministic synthesis in `offline_engine.rs`, end-of-response sources block with accuracy scores | none | DONE |
| M12 | WebUI Clean Interface, Dynamic Spellcheck & Action Buttons | Features 60, 61, 62: Clean UI (suppress prompt chips, keep logo), dynamic debounced chat spellcheck, response action buttons (Copy, Regenerate, Read Aloud, Thumbs), `server.rs:115` `bypass_cache: false`, zero line numbers | none | DONE |
| M13 | OshoosiClaw Mesh Configuration UI & TPM Hardware Identity | Features 63, 64, 65: TPM 2.0 hardware inspection, attestation-tier gating in `PeerAclManager`, daemon REST endpoints, Mesh UI in Settings modal | M11, M12 | DONE |
| M13.5 | Live Testing Feedback: Action Buttons (Continue, Save File, Clear) & Q&A Grounding Calibration | Features 67, 68: Add Continue, Save File, Clear to `.response-actions-bar`, Q&A opening topic restatement, typo map & seed words, filename title match bonus, calibrated grounding accuracy (78%–99%), chunk boundary cleanup | M13 | DONE |
| M14 | Final Verification, Port 3031 Hot-Restart Live Daemon & Solo PR Ship | Features 66, 69: Clean build with `$env:CARGO_INCREMENTAL = "0"`, all tests pass, hot-restart PID 80036 on port 3031, solo PR ship & merge to main | M11, M12, M13, M13.5 | IN_PROGRESS |

## Interface Contracts

### `quorum-coordinator::langextract` ↔ `AgenticGraphState`
- `extract_chat_candidates(prompt: &str) -> Vec<Extraction>`
- `AgenticGraphState`:
  - `pub langextract_result: Option<ConsensusExtractionResult>`
  - `pub extracted_questions: Vec<String>`
  - `pub extracted_entities: Vec<Extraction>`

### `quorum-coordinator::offline_engine` ↔ Synthesis Output
- Prompt contract: Must begin with affirmative question-restating prefix ("Who is femi" -> "Femi is ...") grounded in retrieved chunks (e.g. `ResumeSec-AI-FemiOyesanya2025.pdf`, `Femi_Oyesanya_CV_2026.pdf`).
- NEVER output raw file headers (`### 📄 filename (Lines X–Y)`) or line numbers anywhere in the response body.
- End-of-response Sources block: Deduplicated by file name, displaying only source document name/link and accuracy percentage with ZERO line numbers:
  ```markdown
  ---
  ### 📚 Sources & Document Links
  - [📄 <file_name>](<file_path>) — **<accuracy_pct>% accuracy**
  ```

### `quorum-coordinator::tpm` ↔ `quorum-daemon` ↔ `quorum-ui`
- `TpmHardwareInspection`:
  - `available: bool`, `tpm_version: String`, `manufacturer_id: u32`, `vendor: TpmOemVendor`, `hardware_backed: bool`, `pcr_values: BTreeMap<u32, String>`
- Endpoints:
  - `GET /api/mesh/tpm/status` -> `TpmHardwareInspection`
  - `GET /api/mesh/topology` -> `{ local_node, peers: Vec<PeerNodeDto> }`
  - `GET /api/mesh/pending-joins`, `POST /api/mesh/pending-joins/:peer_id/allow`, `POST /api/mesh/pending-joins/:peer_id/deny`
  - `GET /api/mesh/quarantined-peers`, `POST /api/mesh/quarantined-peers/:peer_id/release`
  - `GET /api/mesh/bootstrap-peers`, `POST /api/mesh/bootstrap-peers`

### `quorum-ui` ↔ Chat Box & Actions
- `POST /api/chat/spellcheck` -> `{ original, corrected, changed, engine }`
- Assistant response message structure:
  1. `.message-header`
  2. `.message-body`
  3. `.citation-container` (Sources: deduplicated by file name, displaying only source document link and accuracy percentage e.g. `📄 Femi_Oyesanya_CV_2026.pdf · 96%`, with ZERO line numbers)
  4. `.response-actions-bar` (Copy, Regenerate, Read Aloud, Thumbs Up, Thumbs Down)
