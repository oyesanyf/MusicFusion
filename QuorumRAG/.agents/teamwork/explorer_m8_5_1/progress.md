# Progress — Explorer M8.5 1 (Ingest & Router)

- Last visited: 2026-11-02T06:26:00Z
- Status: Completed. Ingest test suite verified (`cargo test -p quorum-ingest -- --test-threads=2` passed: 51 passed, 0 failed). Full handoff report generated.

## Completed Investigation Items:
1. `exclude_source_code` default:
   - In `IngestConfig::default()`, `exclude_source_code` is `false`. This is intentional and necessary for unit tests (`ingest_tests.rs`, `m7_integration_tests.rs`, `m7_adversarial_stress_tests.rs`) which use `main.rs` as test fixtures.
   - In `quorum-daemon/src/main.rs`, `run_daemon_and_ui` sets `ingest_config.exclude_source_code = true`.
   - Identified gap: `run_standalone_reindex` (lines 411-417) and `run_standalone_fix_drift` (lines 464-477) initialize `IngestConfig::default()` without setting `exclude_source_code = true`.
2. Temporary / lock / binary filtering:
   - Office lock/temp files (`~$*`, `.~lock*`, `*.tmp`, `*.bak`, `*.wbk`) and package lock files (`cargo.lock`, `package-lock.json`, etc.) are filtered in both `ClassifierRouter` (`FilterDecision::SkipHidden`) and `FilesystemCrawler` (`filter_entry -> false`).
   - Binary/executable files are filtered both on extension fast-path (`quorum_classifier::is_binary_executable_extension`) and content Magika classification (`FilterDecision::SkipBinary`).
3. Reindex worker inspection (`reindex.rs`):
   - `ReindexWorker` crawls files using `FilesystemCrawler` with the passed `IngestConfig`. When `exclude_source_code` is true, source code and temp files are not crawled/indexed.
   - However, `ReindexWorker` does not automatically purge previously indexed code or temp chunks that may already reside in LanceDB/Tantivy.
4. Test execution:
   - Ran `cargo test -p quorum-ingest -- --test-threads=2`: All 7 test targets passed (51 passed, 0 failed).
