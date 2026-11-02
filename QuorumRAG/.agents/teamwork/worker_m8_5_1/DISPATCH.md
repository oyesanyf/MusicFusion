## 2026-11-02T06:17:05Z
You are Worker M8.5 1 (Implementation Worker for Milestone M8.5: Source Code & Temp File Exclusion & RAG Isolation, Feature 57).
Working directory: d:\harfile\QuorumRAG\.agents\teamwork\worker_m8_5_1

MANDATORY FIRST STEP:
Read d:\harfile\QuorumRAG\.agents\teamwork\ORIGINAL_REQUEST.md, d:\harfile\QuorumRAG\PROJECT.md, and d:\harfile\QuorumRAG\.agents\teamwork\sub_orch_m8_5_code_exclusion\SCOPE.md before doing anything else.
Also read the Explorer handoff reports:
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_5_1\handoff.md
- d:\harfile\QuorumRAG\.agents\teamwork\explorer_m8_5_3\handoff.md

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

FILE OWNERSHIP (EXCLUSIVE):
You exclusively own and may edit:
- `crates/quorum-daemon/src/main.rs`
- `crates/quorum-ingest/src/classifier_router.rs`
- `crates/quorum-ingest/src/crawler.rs`
- `crates/quorum-coordinator/src/config.rs`
- `crates/quorum-coordinator/src/agentic/retrieve.rs`
- `crates/quorum-coordinator/tests/adversarial_tier5_coordinator_daemon_stress.rs`
- `crates/quorum-ingest/tests/m8_5_exclusion_tests.rs` (new)
- `crates/quorum-coordinator/tests/m8_5_rag_retrieval_isolation_tests.rs` (new)

TASKS:
1. **CLI Commands Exclusion Invariant (`crates/quorum-daemon/src/main.rs`)**:
   - In `run_standalone_reindex` (line 414) and `run_standalone_fix_drift` (line 467): Ensure `ingest_cfg.exclude_source_code = true` so CLI commands enforce source code and temp file exclusion consistently with the background daemon.
2. **Router & Crawler Parity (`crates/quorum-ingest/src/classifier_router.rs` and `crawler.rs`)**:
   - In `classifier_router.rs` (line 110) and `crawler.rs` (line 264), update `starts_with("~$")` to `starts_with('~')` (or check both `"~$"` and `'~'`) so all tilde-prefixed Office temporary/autosave files (`~*`) are rejected, aligning with `retrieve.rs`.
3. **Coordinator Config & Retrieval Isolation (`crates/quorum-coordinator/src/config.rs` and `retrieve.rs`)**:
   - In `CoordinatorConfig`, add `pub exclude_source_code: bool` defaulting to `true`.
   - In `retrieve.rs` (line 155), check `(daemon_active || self.config.exclude_source_code) && !requests_code` so RAG retrieval strictly isolates documents and images from source code and temp chunks across both live daemon and unit tests.
4. **Adversarial Tier 5 Test Fixture Fix (`crates/quorum-coordinator/tests/adversarial_tier5_coordinator_daemon_stress.rs`)**:
   - In `challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops` (line 657), change the mock chunk path from `"crypto.rs"` to `"crypto_spec.md"` so this architecture prose specification is not excluded as source code by Feature 57.
5. **Acceptance Test Suites**:
   - Create `crates/quorum-ingest/tests/m8_5_exclusion_tests.rs` implementing:
     a) `test_classifier_router_excludes_all_source_code_extensions_when_active`
     b) `test_classifier_router_excludes_office_temporary_lock_and_autosave_files`
     c) `test_crawler_directory_pruning_and_file_filtering_under_source_exclusion`
     d) `test_magika_content_inspection_rejects_extensionless_scripts`
   - Create `crates/quorum-coordinator/tests/m8_5_rag_retrieval_isolation_tests.rs` implementing:
     e) `test_m8_5_rag_retrieval_isolation_never_returns_code_or_temp_chunks`
6. **Verification Requirements**:
   - Run `cargo test -p quorum-classifier -- --test-threads=2` (must pass 34/34).
   - Run `cargo test -p quorum-ingest --test m8_5_exclusion_tests -- --test-threads=2`.
   - Run `cargo test -p quorum-coordinator --test m8_5_rag_retrieval_isolation_tests -- --test-threads=2`.
   - Run `cargo test -p quorum-coordinator --test adversarial_tier5_coordinator_daemon_stress -- challenge_tier5_agentic_state_graph_loop_rewrite_and_max_hops --test-threads=2`.
   - Run `cargo check --workspace`.

Deliver your handoff report to `d:\harfile\QuorumRAG\.agents\teamwork\worker_m8_5_1\handoff.md` and send a message back with your verification commands and test results.


## 2026-11-02T06:22:17Z
[Message] sender=facfe77d-94d3-44be-95df-eec2b9dfe368 priority=MESSAGE_PRIORITY_HIGH
**Context**: Milestone M8.5 Implementation & Acceptance Testing
**Content**: Checking in on progress. Have you completed edits in `main.rs`, `classifier_router.rs`, `crawler.rs`, `config.rs`, `retrieve.rs`, and the acceptance test suites?
**Action**: Please report your current step or deliver `handoff.md` when tests finish.


## 2026-11-02T06:28:44Z
[Message] sender=facfe77d-94d3-44be-95df-eec2b9dfe368 priority=MESSAGE_PRIORITY_HIGH
**Context**: Milestone M8.5 Verification Status Check
**Content**: Checking in on test execution. Which test suites have completed so far?
**Action**: Please reply with your current test status and write `handoff.md` once all verification suites pass.
