# DISPATCH — 2026-11-01T08:55:00Z

## Mission
Apply the Milestone 6 Iteration 3 storage remediation to `crates/quorum-storage/src/tantivy_fts.rs` and verify 100% deterministic test passes across 5 consecutive runs of `--test-threads=2` without a single flake or panic.

## Mandates & Context
- Read the authoritative user request at `d:\harfile\QuorumRAG\ORIGINAL_REQUEST.md`.
- Read the dead ends log at `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\DEAD_ENDS.md`.
- Read the reviewer report at `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m6_remediation_1\handoff.md`.
- Read the explorer report at `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_2\handoff.md`.
- Use the drop-in replacement file at `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_2\proposed_tantivy_fts.rs` or patch at `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_2\proposed_tantivy_fts.patch`.

## Exclusive Write Ownership
You have exclusive write ownership of:
- `crates/quorum-storage/src/tantivy_fts.rs`
- `crates/quorum-storage/src/engine.rs` (if any adjustments are needed)

DO NOT modify any other files in the workspace without explicit permission.

## MANDATORY INTEGRITY WARNING
> DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Detailed Tasks
1. Initialize your workspace in `.agents/teamwork/worker_m6_remediation_2/` with `BRIEFING.md` and `progress.md`.
2. Apply `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m6_remediation_2\proposed_tantivy_fts.rs` to `crates/quorum-storage/src/tantivy_fts.rs`.
3. Verify that all 4 root causes are resolved:
   - Monotonic generation tracking (`commit_generation: AtomicU64`, `synced_generation: AtomicU64`) prevents premature sync.
   - Initial cycle 0/1 works deterministically (`target_gen == 0 || meta.opstamp >= min_opstamp`).
   - Loop fails closed on attempt exhaustion with `Err(StorageError::Tantivy(...))`.
   - Replaces blocking millisecond `std::thread::sleep` with cooperative `std::thread::yield_now()` and sub-millisecond delay to prevent Tokio thread starvation.
   - `compact()` queries `self.index.searchable_segment_ids()?` directly from authoritative metadata, retries `merge()`, and drops writer lock before reloading readers.
   - `scan_all_chunks()` bounds limit to `num_docs.max(10_000).min(500_000)`.
4. Run compilation and clippy:
   ```powershell
   cargo clippy -p quorum-storage --tests -- -D warnings
   ```
   Must pass with 0 warnings and 0 errors.
5. Run 5 consecutive runs of `adversarial_tier5_storage_ingest_stress` under `--test-threads=2`:
   ```powershell
   powershell -Command "for ($i=1; $i -le 5; $i++) { Write-Host \"=== RUN $i ===\"; cargo test -p quorum-storage --test adversarial_tier5_storage_ingest_stress -- --test-threads=2; if ($LASTEXITCODE -ne 0) { exit 1 } }"
   ```
   Every single run (5/5) MUST pass 8/8 tests with 100% determinism.
6. Run `cargo test -p quorum-storage --test adversarial_stress_tests -- --nocapture` (all 13 tests must pass).
7. Run multi-crate test suite:
   ```powershell
   cargo test -p quorum-storage -p quorum-ingest -p quorum-classifier -p quorum-contract -p quorum-types -- --test-threads=2
   ```
   Must pass with exit code 0.
8. Stage and commit verified changes with git:
   ```powershell
   git add crates/quorum-storage/src/tantivy_fts.rs
   git commit -m "fix(quorum-storage): resolve Tantivy reader reload race, Tokio starvation, and compaction segment lock contention"
   ```
9. Write `handoff.md` with:
   - Observation
   - Logic Chain
   - Caveats
   - Conclusion
   - Verification Commands & Output
10. Send completion message back to orchestrator.
