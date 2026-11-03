# Project: QuorumRAG Deep Research & Iterative Section RAG

## Architecture
QuorumRAG is a decentralized, air-gapped retrieval-augmented generation (RAG) platform.
The architecture comprises:
- `crates/quorum-coordinator`: The core coordinator library managing settings persistence (`dlp_mesh.rs`), P2P gossip, local offline synthesis engine (`offline_engine.rs`), and agentic workflow orchestration (`agentic/`).
- `crates/quorum-daemon`: The server binary exposing REST and SSE streaming endpoints (`/api/chat`, `/api/settings`, `/api/preferences`), telemetry channels, and lifecycle management.
- `crates/quorum-ui`: Embedded WebUI assets (`app.js`, `index.html`, `styles.css`) for chat interaction, live progress telemetry, dynamic settings, and action bars.
- `crates/quorum-ingest`: Physical drive discovery, filesystem crawler, and document chunker with BLAKE3 chunk IDs and metadata offsets.

Data Flow for Deep Research & Iterative Section RAG:
1. User configures preferences in Settings UI (`app.js`) -> persisted via `/api/settings` into `PersistedDaemonSettings` (`dlp_mesh.rs`).
2. User submits query (e.g. `"tell me about sacred maths"`) -> `chat_sse_handler` in `chat.rs` -> `query_agentic_with_history` in `offline_engine.rs`.
3. Broad Hybrid Retrieval: BM25 lexical + vector cosine search retrieves 20-50 candidate chunks without broken score cutoffs.
4. Parent Document Retrieval: Adjacent chunks sharing `file_path` and `chunk_index` are stitched into broad contiguous context.
5. Map-Reduce Fact Extraction: Chunk clusters (3-4 chunks) are distilled into dense fact sheets (quotes, dates, formulas, definitions).
6. Iterative Section Generation (`DocumentPlan` & `SectionPlan`):
   - Broad outline synthesizes subtopics and targeted queries.
   - Sequential chapter synthesis queries targeted chunks and passes `rolling_summary` (~1,500 chars) for continuity.
7. Recursive Turn Chaining & Sentence-Boundary Guard:
   - Dynamic `num_predict` derived from `ui.max_output_tokens` and `ui.agentic_chunk_size`.
   - On `done_reason == "length"`, automatically chain turns up to `max_output_tokens`.
   - Terminal punctuation guard ensures responses never terminate mid-sentence.
8. Telemetry & Streaming: `tx.blocking_send` inside `block_in_place` streams tokens without dropping, concluding with `### 📚 Sources & Document Links` and interactive `.response-actions-bar`.

---

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| F1 | Settings Models | Add `UiPreferencesConfig` alias, typed fields for `deep_research_iterative_rag`, `parent_document_retrieval`, `map_reduce_fact_extraction`, and serde aliases in `dlp_mesh.rs` | M11 | Survey E1 |
| F2 | Settings WebUI Controls | Render 10-option max token dropdown, interactive toggles for deep research, parent doc, map-reduce, and recursive chaining in `app.js` | M11 | Survey E1 |
| F3 | Daemon Preferences Route | Add `/api/preferences` route alias in `server.rs` and verify settings round-trip | M11 | Survey E1 |
| F4 | Filename Hit Bugfix | Fix `file_hits` in `offline_engine.rs:499` to check filename tokens (`file_l`) rather than chunk body text | M12 | Survey E1/E3 |
| F5 | Remove RRF Cutoff | Remove invalid `scored.score < 0.35` RRF filter in `offline_engine.rs:505` | M12 | Survey E1/E3 |
| F6 | Conversational Context | Fix follow-up queries ("tell me more", "continue") to retrieve context from history without failing on empty chunks | M12 | Survey E1/E3 |
| F7 | Parent Document Retrieval | Stitch contiguous adjacent chunks by `file_path` and `chunk_index` into cohesive parent context | M13 | Survey E2/E3 |
| F8 | Retrieval Breadth Expansion | Expand retrieval `top_k` candidate pool from 8/12 up to 20..50 chunks | M13 | Survey E2/E3 |
| F9 | Map-Reduce Fact Extraction | Implement 3-4 chunk grouping and fact sheet distillation prior to section writing | M13 | Survey E3 |
| F10 | Dynamic Output Sizing | Replace hardcoded `let num_predict = 1600usize;` at line 1154 with dynamic sizing respecting `ui.max_output_tokens` and `ui.agentic_chunk_size` | M14 | Survey E1/E3 |
| F11 | Zero-Drop Telemetry | Convert `tx.try_send` in `offline_engine.rs:1309` to `tx.blocking_send` inside `block_in_place` | M14 | Survey E1/E3 |
| F12 | Recursive Agentic Chaining | Handle `done_reason == "length"` with recursive continuation hops up to 262k tokens | M14 | Survey E3 |
| F13 | Sentence-Boundary Guard | Ensure response ends on clean terminal punctuation boundary (`.`, `!`, `?`) | M14 | Survey E3 |
| F14 | Iterative Section Generation | Implement `DocumentPlan` and `SectionPlan` chapter generation with `rolling_summary` continuity | M14 | Survey E3 |
| F15 | Response Formatting & Actions Bar | Ensure question restatement prefix, bracketed `[1]` citations, `### 📚 Sources & Document Links`, and `.response-actions-bar` render | M14 | Survey E3 |
| F16 | Workspace Test Suite | Verify all unit and integration tests across crates pass (`cargo test`) | M15 | R6 |
| F17 | Binary Build & PR Shipping | Kill running daemons, rebuild `quorumrag.exe`, ship via `scripts/solo_pr_ship.ps1` | M15 | R6 |
| F18 | Live SSE Verification | Launch `quorumrag.exe` on `0.0.0.0:3031` and verify live SSE responses for `"tell me about sacred maths"` and `"tell me more"` | M15 | R6 |

---

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M11 | Settings & Preferences UI | F1, F2, F3 (`dlp_mesh.rs`, `app.js`, `server.rs`) | none | DONE |
| M12 | Retrieval Engine Bugfixes | F4, F5, F6 (`offline_engine.rs:499, 505`, follow-ups) | none | DONE |
| M13 | Parent Doc Retrieval & Map-Reduce | F7, F8, F9 (chunk stitching, top_k 20-50, fact extraction) | M12 | DONE |
| M14 | Recursive Chaining & Section RAG | F10, F11, F12, F13, F14, F15 (num_predict, blocking_send, section plans, sentence guard) | M11, M13 | DONE |
| M15 | Verification, PR Shipping & Live SSE | F16, F17, F18 (cargo test, solo_pr_ship, live test on 3031) | M14 | IN_PROGRESS |

---

## Interface Contracts

### M11: Settings Contract (`crates/quorum-coordinator/src/dlp_mesh.rs`)
```rust
pub type UiPreferencesConfig = ChatGptUiSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatGptUiSettings {
    // ...
    pub max_output_tokens: usize,
    #[serde(alias = "agentic_loop_chaining")]
    pub agentic_loop_enabled: bool,
    #[serde(alias = "agentic_loop_chunk_size")]
    pub agentic_chunk_size: usize,
    pub agentic_strategy: String,
    pub deep_research_iterative_rag: bool,
    pub parent_document_retrieval: bool,
    pub map_reduce_fact_extraction: bool,
    // ...
}
```

### M13: Parent Document Stitching Contract (`offline_engine.rs` / `agentic/`)
```rust
pub fn stitch_parent_document_chunks(chunks: Vec<ScoredChunk>) -> Vec<ScoredChunk>;
pub fn extract_map_reduce_fact_sheet(chunks: &[ScoredChunk]) -> String;
```

### M14: Recursive Chaining Contract (`offline_engine.rs`)
```rust
pub struct DocumentPlan {
    pub document_title: String,
    pub question_restatement: String,
    pub sections: Vec<SectionPlan>,
}

pub struct SectionPlan {
    pub section_index: usize,
    pub title: String,
    pub targeted_query: String,
    pub focus_description: String,
}
```

---

## Code Layout
- `crates/quorum-coordinator/src/dlp_mesh.rs`: Settings data models, serialization, persistence.
- `crates/quorum-coordinator/src/offline_engine.rs`: Hybrid retrieval, RRF scoring, parent chunk stitching, map-reduce, Ollama TCP chat streaming, recursive chaining, formatting.
- `crates/quorum-coordinator/src/agentic/`: Agentic workflow state graph, nodes, telemetry events.
- `crates/quorum-daemon/src/handlers/chat.rs`: SSE chat streaming handler, channel capacity, request parsing.
- `crates/quorum-daemon/src/server.rs`: Route registration.
- `crates/quorum-ui/assets/app.js`: WebUI event handling, settings rendering, action buttons.
- `tests/`: Integration tests and regression benchmarks.
