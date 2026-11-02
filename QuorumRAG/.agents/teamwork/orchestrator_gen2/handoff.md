# Soft Handoff — Orchestrator Gen 2 to Orchestrator Gen 3

## 1. Milestone State
- **Milestone M7 (Features 44-47)**: **DONE & GATED PASS**
  - Dedup hydration from LanceDB on startup (`hydrate_from_storage`).
  - Physical drive discovery for fixed NTFS/ReFS/exFAT volumes without dot-dir blind spots.
  - Fast-path deterministic binary extension exclusion prior to I/O.
  - Watcher directory rename recursion and Windows sharing violation (Error 32) retries.
  - Passed all reviews, stress challenges, and forensic audit cleanly.
- **Milestone M8 (Features 48-51)**: **GATED FAIL — READY FOR REMEDIATION**
  - Features 48-51 implemented by Worker M8 (`b4bde733`): stack buffer reuse, Tantivy seq 0 sync, RFC1918 dial restrictions, mesh Ed25519 anti-replay, BLAKE3 RRF anti-stuffing, tier clamping, cached vector reuse.
  - Multi-term precision fix applied to workspace by Sentinel/Sentinel-dispatch:
    - `offline_engine.rs` & `agentic/mod.rs` strict RAG-only synthesis (no ungrounded model fallback, sources strictly at bottom, empty RAG message on no hits).
    - `retrieve.rs` multi-term precision fix: `content_matched_terms >= required_query_hits && entity_hits >= required_entity_hits`.
    - `IngestConfig` `exclude_source_code: bool` flag and code/dump/temp exclusion in `ClassifierRouter` & `FilesystemCrawler`.
  - Next step: Run verification suite (`adversarial_challenge_m8_2_tests`), confirm 6/6 pass, dispatch review/challenge/audit, and gate M8 to PASS.
- **Milestone M8.5 (Feature 57)**: **IN_PROGRESS / VERIFICATION**
  - Mandatory Source Code & Temp File Ingestion Exclusion & RAG Isolation. Verify integration across `quorum-ingest`, `quorum-daemon`, and reindexing.
- **Milestone M9 (Features 52-54)**: **PLANNED**
  - UI stop button (`AbortController`, `POST /api/chat/cancel`, `tx.is_closed()` break).
  - Real-time monotonic indexer progress without 25% fake fallback.
  - Dynamic thinking phrases engine: 60+ phrase pool, ~650ms cycle, non-repeating, non-freezing on telemetry before `TokenDelta`.
  - UI clean-up: welcome banner and quick chips removed per user request.
- **Milestone M10 (Features 55-56)**: **PLANNED**
  - Deterministic 7-metric benchmark suite (`tests/benchmarks_r4.rs`).
  - Byzantine peer & fault regression suite (`tests/adversarial_mesh_and_faults.rs`).
  - Solo PR ship script (`scripts/solo_pr_ship.ps1`) to branch cleanly, atomic commit, PR with 7-metric table, automated review, and squash-merge to `main`.

---

## 2. Active Subagents
- **None**: All 18 subagents from Gen 2 have delivered their handoff reports and are retired.

---

## 3. Pending Decisions & Forensic Root Cause for M8
- **Forensic Audit & Challenger Failure Analysis**:
  - In `crates/quorum-coordinator/src/agentic/retrieve.rs`:
    - `"universal"` and `"income"` are both classified in `GENERIC_ATTRIBUTE_TERMS`.
    - For the 3-term query `"what is universal mineral income"`, `entity_terms` is reduced to `["mineral"]` (length 1).
    - `entity_terms.len() >= 2` evaluates to `false`, collapsing `required_entity_hits` to 1.
    - Accidental single-word hits bypassed rejection.
  - Sentinel applied the multi-term precision fix to `retrieve.rs` and `offline_engine.rs`.
  - Orchestrator Gen 3 must verify this remediation with workers/challengers/reviewers/auditor.

---

## 4. Remaining Work (Concrete Next Steps for Successor Gen 3)
1. **Initialize in `.agents/teamwork/orchestrator_gen3`**:
   - Start 10-minute heartbeat cron.
2. **Execute M8 Remediation Verification & Gate**:
   - Spawn Worker M8 Remediation to run `cargo test -p quorum-coordinator --test adversarial_challenge_m8_2_tests -- --test-threads=2` and workspace check.
   - Spawn Reviewers, Challengers, and Forensic Auditor.
   - Record verdicts in `GATE_STATUS.md` and gate M8 to **PASS**. Update `PROJECT.md`.
3. **Milestone M8.5 (Source Code & Temp File Ingestion Exclusion & RAG Isolation)**:
   - Verify `exclude_source_code` in `IngestConfig`, `ClassifierRouter`, and `FilesystemCrawler`.
   - Gate M8.5 to **PASS**.
4. **Milestone M9 (UI Stop Button, Dynamic Thinking Phrases & Indexer Progress)**:
   - Features 52-54 implementation, review, challenge, audit, and gate.
5. **Milestone M10 (Benchmarks, Regressions & Solo PR Shipping)**:
   - Features 55-56: 7-metric benchmark suite, Byzantine fault regression tests, solo PR shipping via `scripts/solo_pr_ship.ps1`.
6. **Victory Notification**:
   - When all milestones pass and PR merges cleanly into `main`, send final victory report to Sentinel (`dc0b066f-390f-46a5-bd85-361145ac6afd`).

---

## 5. Key Artifacts
- `.agents/teamwork/ORIGINAL_REQUEST.md`: Authoritative user requests and invariants.
- `PROJECT.md`: Global architecture, feature inventory, milestones, contracts, layout.
- `.agents/teamwork/orchestrator_gen2/GATE_STATUS.md`: Full gate history for M7 and M8.
- `.agents/teamwork/auditor_m8_1/handoff.md`: Full forensic audit evidence for M8.
- `.agents/teamwork/challenger_m8_2/handoff.md`: Adversarial challenge failure evidence.
- `.agents/teamwork/reviewer_m8_2/handoff.md`: Reviewer request changes evidence.
