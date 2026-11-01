# Dead Ends Tracking — QuorumRAG

This file logs failed approaches and dead ends across milestone gating iterations to prevent oscillation.
On retries, Explorers MUST NOT recommend approaches listed in this file.

| Milestone | Iteration | Approach Tried | Why It Failed | Files Touched |
|-----------|-----------|---------------|---------------|---------------|
| Milestone 1 | 1 | Naive hex nibble parsing & CJK text rejection | Panic on out-of-bounds nibbles; control character false positives on CJK characters | `crates/quorum-types/src/sig64.rs` |
| Milestone 2 | 1 | Unbounded `TopDocs::with_limit(usize::MAX)` & single-field Tantivy schema | Memory exhaustion / capacity overflow in TopDocs; delete query silently matching 0 documents in tokenized field | `crates/quorum-storage/src/tantivy_fts.rs`, `engine.rs` |
| Milestone 4 | 1 | Sequential async loop in scatter-gather & missing consensus boost | Head-of-line blocking stalled fast peers when one peer hung; missing 1.25x multi-peer consensus boost | `crates/quorum-coordinator/src/scatter_gather.rs`, `fusion.rs` |
| Milestone 5 | 1 | Non-terminating `$ErrorActionPreference = "Continue"` in `verify_live_ide_execution.ps1` with unhandled Tantivy index writer thread kill & offline engine returning bullet point snippets instead of compilable ````rust ... ```` code blocks | Script printed pass strings despite failed citation assertions; reindexing crashed with index writer thread panic; offline synthesis failed to output a compilable code block | `scripts/verify_live_ide_execution.ps1`, `crates/quorum-storage/src/tantivy_fts.rs`, `crates/quorum-coordinator/src/offline_engine.rs` |
