# BRIEFING — 2026-10-31T20:25:00Z

## Mission
Investigate and design technical architecture for Two-Level BLAKE3 Dedup, Chunking Engine, and Image Indexing Engine for QuorumRAG Milestone 3.

## 🔒 My Identity
- Archetype: explorer
- Roles: teamwork_explorer
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_2\
- Original parent: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
- Milestone: Milestone 3 (Filesystem Ingestion, OS Watcher, Image & Reindex Engine)

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Do NOT write source code files to crates/
- Output must be in d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_2\
- No mocks, write actual code / realistic designs
- Adhere to user invariants (fail-closed, real types, zero mocks)

## Current Parent
- Conversation ID: e13403c5-b0e6-48a4-a1ec-a1f096f9f53e
- Updated: 2026-10-31T20:25:00Z

## Investigation State
- **Explored paths**:
  - `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md`
  - `d:\harfile\QuorumRAG\PROJECT.md`
  - `crates/quorum-types/src/lib.rs`
  - `crates/quorum-classifier/src/lib.rs`
  - `crates/quorum-contract/src/lib.rs`
  - `crates/quorum-storage/src/lib.rs`, `engine.rs`, `arrow_interop.rs`, `tantivy_fts.rs`
  - `tests/e2e/test_ingestion_dedup.rs` & `tests/e2e/src/lib.rs`
  - Peer explorer assignments (`explorer_m3_1` crawler/watcher, `explorer_m3_3` reindex/drift healer)
- **Key findings**:
  1. `dedup.rs`: Level-1 file catalog maps `(file_path, file_blake3, mtime, size)`. Dual indexing (`path -> hash` and `hash -> entry`) enables $O(1)$ fast-path unmodified checks (zero hashing), $O(1)$ move/rename detection (zero re-embeddings), and precise file deletion tracking.
  2. `dedup.rs`: Level-2 chunk cache maps `chunk_blake3 -> Vec<f32>` (Gemma 768d embedding) with bounded LRU memory retention, saving massive neural embedding costs across identical chunks in distinct files or single-line edits.
  3. `chunker.rs`: Robust line-aware chunker calculates newline prefix byte offsets for exact $O(\log L)$ line numbers (`line_start <= line_end`), enforces UTF-8 character boundary backtracking, and implements configurable sliding windows with overlap and natural boundary snapping (paragraph, line, sentence, word).
  4. `image_indexer.rs`: Prevents Magika-identified images (`image/jpeg`, `image/png`, `image/webp`, `image/gif`, `image/bmp`, `image/svg+xml`, etc.) from being discarded as generic binaries. Parses image dimensions, format tokens, EXIF/metadata (camera, lens, ISO, aperture, GPS), and embedded/SVG text into a rich searchable text representation and structured JSON payload, producing valid `DocumentChunk` records for LanceDB + Tantivy.
- **Unexplored areas**:
  - Exact implementation of OCR models or multimodal inference runtime (deferred to GLM-4V / vision integration per follow-up).
  - Background crawler and watcher concurrency controls (owned by M3-1).

## Key Decisions Made
- Level-1 Catalog uses dual HashMap indexing (`hash -> FileCatalogEntry` and `path -> file_blake3`) for instant rename/move detection.
- Level-2 Chunk Cache caches dense 768-dim `Vec<f32>` vectors in a thread-safe LRU cache, decoupled from storage row IDs.
- Chunker uses newline prefix indexing for guaranteed line number accuracy and char-boundary safe slicing.
- Image Indexer uses zero-dependency pure-Rust binary header & EXIF parsers for robust, lightning-fast extraction across PNG, JPEG, WebP, GIF, BMP, and SVG.

## Artifact Index
- `DISPATCH.md` — Task assignment
- `BRIEFING.md` — Working memory and status
- `progress.md` — Liveness heartbeat
- `handoff.md` — Final technical architecture report
