# Scope: Milestone M8.5 — Source Code & Temp File Exclusion & RAG Isolation

## Objective
Ensure QuorumRAG strictly indexes documents, text/data documents, and image assets while deterministically excluding all source code files (`FileCategory::Code` and source code extensions) and temporary/lock/autosave files (`~*`, `.tmp`, `.bak`) across crawlers, routers, watchers, and reindexing routines. Also ensure that stored or retrieved source code chunks are purged/skipped during reindexing and RAG retrieval so that RAG queries return only user documents and images. Preserve unit test coverage for `FileCategory::Code` in `quorum-classifier`.

## Owned Crates & Boundaries
- `crates/quorum-classifier/`: Preserve `FileTypeClassifier::classify_bytes` logic identifying `FileCategory::Code` so classifier tests pass.
- `crates/quorum-ingest/src/config.rs`: Add `exclude_source_code: bool` (default: `true`) to `IngestConfig`.
- `crates/quorum-ingest/src/classifier_router.rs`:
  - Deterministically reject source code files when `exclude_source_code` is enabled. Source code extensions: `.rs`, `.py`, `.js`, `.ts`, `.jsx`, `.tsx`, `.c`, `.cpp`, `.h`, `.hpp`, `.cc`, `.cs`, `.java`, `.go`, `.kt`, `.rb`, `.php`, `.swift`, `.scala`, `.sh`, `.bash`, `.ps1`, `.bat`, `.cmd`, `.sql`, `.css`, `.scss`, `.less`, `.vue`, `.svelte`, etc.
  - Deterministically reject Office/Word temporary lock & autosave files: `file_name.starts_with('~')`, `.tmp`, `.bak`.
- `crates/quorum-ingest/src/crawler.rs`: Exclude source code files and temporary files before parsing or chunking.
- `crates/quorum-ingest/src/reindex.rs`: Skip and/or purge any existing source code chunks or temporary file chunks during reindexing.
- `crates/quorum-storage/src/` or `crates/quorum-coordinator/src/agentic/retrieve.rs`: Filter out any `FileCategory::Code` / code extension / temp file chunks from retrieval results when `exclude_source_code` is active.

## Verification Criteria
1. `cargo test -p quorum-classifier` passes cleanly with all `FileCategory::Code` unit tests intact.
2. Ingestion pipeline skips `.rs`, `.py`, `.js`, etc., and skips `~$*.docx`, `*.tmp`, `*.bak` by default while indexing legitimate `.pdf`, `.docx`, `.txt`, `.md`, `.png`, `.jpg`, etc.
3. RAG retrieval queries only return document and image citations, never source code chunks or temp files.
