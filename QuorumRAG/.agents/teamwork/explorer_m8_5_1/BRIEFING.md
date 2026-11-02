# BRIEFING — 2026-11-02T06:26:00Z

## Mission
Investigate `crates/quorum-ingest` (`src/config.rs`, `src/classifier_router.rs`, `src/crawler.rs`, `src/watcher.rs`, `src/reindex.rs`) regarding `exclude_source_code` and Office temp/lock file filtering (`~$*`, `.~lock*`, `*.tmp`, `*.bak`, `*.wbk`).

## 🔒 My Identity
- Archetype: explorer
- Roles: [investigator, synthesizer]
- Working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_5_1
- Original parent: facfe77d-94d3-44be-95df-eec2b9dfe368
- Milestone: M8.5 (Source Code & Temp File Exclusion)

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Process-scoped environment only
- Production code standards: no mock signatures, no lazy path bypasses
- Write only to working directory: d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_5_1

## Current Parent
- Conversation ID: facfe77d-94d3-44be-95df-eec2b9dfe368
- Updated: 2026-11-02T06:06:21Z

## Investigation State
- **Explored paths**:
  - `crates/quorum-ingest/src/config.rs`
  - `crates/quorum-ingest/src/classifier_router.rs`
  - `crates/quorum-ingest/src/crawler.rs`
  - `crates/quorum-ingest/src/watcher.rs`
  - `crates/quorum-ingest/src/reindex.rs`
  - `crates/quorum-ingest/src/engine.rs`
  - `crates/quorum-ingest/src/drift_healer.rs`
  - `crates/quorum-daemon/src/main.rs`
  - `crates/quorum-classifier/src/lib.rs`
  - `crates/quorum-coordinator/src/agentic/retrieve.rs`
  - `crates/quorum-ingest/tests/{ingest_tests.rs, m7_integration_tests.rs, m7_adversarial_stress_tests.rs, adversarial_challenge_tests.rs, adversarial_reindex_and_drift.rs, m7_challenger_stress_tests.rs}`
- **Key findings**:
  1. `exclude_source_code` default is `false` in `IngestConfig::default()`. This is critical for backwards compatibility with existing unit tests in `quorum-ingest` (`ingest_tests.rs:212`, `m7_integration_tests.rs:335`, etc.) which explicitly create and assert on `main.rs`.
  2. In `quorum-daemon/src/main.rs:84`, `run_daemon_and_ui` sets `ingest_config.exclude_source_code = true`.
  3. CLI gap: `run_standalone_reindex` (main.rs:412) and `run_standalone_fix_drift` (main.rs:464) use `IngestConfig::default()` directly and do NOT set `exclude_source_code = true`.
  4. Office lock/temp files (`~$*`, `.~lock*`, `*.tmp`, `*.bak`, `*.wbk`) and package lock files (`cargo.lock`, `package-lock.json`, etc.) are rejected in both `ClassifierRouter` (`FilterDecision::SkipHidden`) and `FilesystemCrawler` (`filter_entry -> false`).
  5. `ReindexWorker` in `reindex.rs` respects `exclude_source_code` when crawling (skips code/temp files), but does NOT purge existing code/temp chunks from LanceDB/Tantivy.
  6. `quorum-coordinator/src/agentic/retrieve.rs` acts as a second defense line by actively filtering out source code and temp chunks during retrieval when `daemon_active && !requests_code`.
  7. Verification completed: `cargo test -p quorum-ingest -- --test-threads=2` passed cleanly with 51 tests passed, 0 failed.
- **Unexplored areas**: None within the scope of this investigation.

## Key Decisions Made
- Confirmed dual-mode design: `IngestConfig::default()` keeps `exclude_source_code: false` for unit test compatibility while production daemon & CLI workers should enforce `exclude_source_code: true`.

## Artifact Index
- DISPATCH.md — incoming dispatch instructions
- BRIEFING.md — persistent working memory
- progress.md — liveness heartbeat
- handoff.md — 5-component handoff report
