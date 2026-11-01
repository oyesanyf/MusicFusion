# Dispatch: Explorer M3-2 (Two-Level Dedup, Chunker & Image Indexing)

## 2026-10-31T20:00:00Z
- **Role**: teamwork_explorer
- **Milestone**: Milestone 3 (Filesystem Ingestion, OS Watcher, Image & Reindex Engine)
- **Angle**: Two-Level BLAKE3 Dedup, Chunking Engine & Image Indexing Engine
- **Working Directory**: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_2\
- **Mandatory References**:
  - `d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md` (Read in full!)
  - `d:\harfile\QuorumRAG\PROJECT.md`
  - `d:\harfile\QuorumRAG\crates\quorum-types\`
  - `d:\harfile\QuorumRAG\crates\quorum-classifier\`
  - `d:\harfile\QuorumRAG\crates\quorum-contract\`
  - `d:\harfile\QuorumRAG\crates\quorum-storage\`
  - `d:\harfile\QuorumRAG\tests\e2e\test_ingestion_dedup.rs`
- **Objectives**:
  1. Design `crates/quorum-ingest/src/dedup.rs`:
     - Level 1: Full-file BLAKE3 hash catalog mapping `(file_path, file_blake3, mtime, size)`. Detects renames/moves on disk and skips unchanged files entirely without chunking or embedding calls.
     - Level 2: Chunk-level BLAKE3 hash cache mapping `chunk_blake3 -> Vec<f32>` ensuring identical text chunks across distinct files or edits reuse existing embeddings.
  2. Design `crates/quorum-ingest/src/chunker.rs`: line-aware and character-boundary-safe chunking with configurable window and overlap, producing `DocumentChunk` records with correct line numbers and BLAKE3 hashes.
  3. Design `crates/quorum-ingest/src/image_indexer.rs`: Handle Magika-identified image MIME types (`image/jpeg`, `image/png`, `image/webp`, `image/gif`, `image/bmp`, `image/svg+xml`, etc.) without skipping them! Extract dimensions, format tokens, EXIF/metadata, visual descriptions/captions, and index rich searchable text into LanceDB + Tantivy.
  4. Write a comprehensive technical report to `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m3_2\handoff.md`.
