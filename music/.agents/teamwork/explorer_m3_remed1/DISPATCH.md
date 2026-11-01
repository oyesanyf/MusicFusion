# DISPATCH: explorer_m3_remed1 (Milestone 3 Remediation Explorer 1 — Path Security & Windows Reserved Device Names)

## Role & Focus
Technical investigation and fix strategy for the Forensic Audit Failure (Integrity Violation) in `crates/music_pipeline/src/checkpointer.rs`:
1. CheckpointManager `sanitize_path_component` currently allows Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1` through `COM9`, `LPT1` through `LPT9` in uppercase, lowercase, and mixed-case, both standalone and with extensions or trailing dots/spaces) to bypass validation.
2. In `crates/music_pipeline/tests/challenger_stress_tests.rs`, `stress_test_checkpoint_path_injection_and_traversal_attacks` fails with 27 bypassed device names.
3. Formulate the exact, mathematical, robust implementation of `sanitize_path_component` in `crates/music_pipeline/src/checkpointer.rs` that:
   - Rejects empty strings, slashes, colons, null bytes, `..`, percent-encoded traversal (`%2e`, `%2f`, `%5c`, `%00`).
   - Checks if component ends with '.' or ' ' (trailing dots/spaces).
   - Extracts base stem (before any extension or dot) and checks case-insensitively against all DOS device names: `con`, `prn`, `aux`, `nul`, `com1`..`com9`, `lpt1`..`lpt9`.
   - Also checks if `p.components()` matches only `Component::Normal`.
   - Returns `Err(PipelineError::SecurityError(...))` on any violation.
4. Formulate test additions for `crates/music_pipeline/tests/graph_tests.rs` in `test_checkpoint_manager_path_traversal_defense` to comprehensively verify all device names.
5. Provide full report in `.agents/teamwork/explorer_m3_remed1/handoff.md`.

## 2026-11-01T03:50:29Z
You are explorer_m3_remed1, the Milestone 3 Remediation Explorer 1 (Path Security & Windows Reserved Device Names).
Your working directory is `d:\harfile\music\.agents\teamwork\explorer_m3_remed1\`.

CRITICAL CONSTRAINTS:
- Read-only investigation: NEVER edit, modify, or create production source code files.
- You MUST read `d:\harfile\music\ORIGINAL_REQUEST.md` (mandatory).
- You MUST read the full Forensic Audit Evidence report at `d:\harfile\music\.agents\teamwork\auditor_m3\handoff.md`.
- Read reviewer and challenger reports at:
  - `d:\harfile\music\.agents\teamwork\reviewer_m3_1\handoff.md`
  - `d:\harfile\music\.agents\teamwork\reviewer_m3_2\handoff.md`
  - `d:\harfile\music\.agents\teamwork\challenger_m3_1\handoff.md`
  - `d:\harfile\music\.agents\teamwork\challenger_m3_2\handoff.md`
- Read `d:\harfile\music\.agents\teamwork\explorer_m3_remed1\DISPATCH.md`.
- Read target source files:
  - `d:\harfile\music\crates\music_pipeline\src\checkpointer.rs`
  - `d:\harfile\music\crates\music_pipeline\tests\challenger_stress_tests.rs`
  - `d:\harfile\music\crates\music_pipeline\tests\graph_tests.rs`

INVESTIGATION OBJECTIVES:
1. Address the Forensic Audit Integrity Violation in `crates/music_pipeline/src/checkpointer.rs`: `sanitize_path_component` allowed Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`, `LPT1`..`LPT9` in all cases, alone or with extensions) to bypass validation, causing `cargo test -p music_pipeline --test challenger_stress_tests` to fail on `stress_test_checkpoint_path_injection_and_traversal_attacks`.
2. Formulate the exact, mathematically robust implementation of `CheckpointManager::sanitize_path_component(component: &str) -> Result<String>` that:
   - Rejects empty components, slashes (`/`, `\`), colons (`:`), null bytes (`\0`), traversal (`..`), percent-encoded traversals (`%2e`, `%2f`, `%5c`, `%00`).
   - Rejects components ending in `.` or space ` ` (Windows filesystem quirks).
   - Extracts base file stem (splitting on `.` and taking the first segment, lowercasing) and checks if it matches any reserved DOS device names: `con`, `prn`, `aux`, `nul`, `com1` through `com9`, `lpt1` through `lpt9`.
   - Checks `p.components()` matches only `Component::Normal`.
   - Returns `Err(PipelineError::SecurityError(format!("Windows reserved device name rejected: '{}'", component)))`.
3. Formulate the exact changes to `test_checkpoint_manager_path_traversal_defense` in `crates/music_pipeline/tests/graph_tests.rs` to include all Windows reserved device names.
4. Verify that this fix will make `stress_test_checkpoint_path_injection_and_traversal_attacks` pass cleanly.
5. Write your complete analysis and recommended code diff to `d:\harfile\music\.agents\teamwork\explorer_m3_remed1\handoff.md`.
6. Send completion message to parent orchestrator.
