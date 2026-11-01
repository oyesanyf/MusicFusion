# BRIEFING — 2026-11-01T01:45:00Z

## Mission
Plan exact step-by-step remediation guide for Worker 2 regarding music_fusion test flakiness/race condition.

## 🔒 My Identity
- Archetype: explorer
- Roles: investigation, synthesis
- Working directory: d:\harfile\music\.agents\teamwork\explorer_m1_fix3
- Original parent: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Milestone: Milestone 1 Iteration 2 (Remediation)

## 🔒 Key Constraints
- Read-only investigation — do NOT implement
- Plan exact step-by-step remediation guide for Worker 2
- Provide exact file diff recommendations for `crates/music_fusion/tests/fusion_tests.rs` (and `crates/music_fusion/src/memory.rs` if needed)
- Formulate verification commands under high parallelism and stress

## Current Parent
- Conversation ID: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Updated: 2026-11-01T01:40:18Z

## Investigation State
- **Explored paths**:
  - `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md`
  - `d:\harfile\music\PROJECT.md`
  - `d:\harfile\music\.agents\teamwork\reviewer_m1_1\handoff.md`
  - `crates/music_fusion/tests/fusion_tests.rs`
  - `crates/music_fusion/tests/adversarial_fusion_stress_tests.rs`
  - `crates/music_fusion/src/memory.rs`
  - Workspace test executions under parallel multi-threaded conditions
- **Key findings**:
  1. `fusion_tests.rs:74` mutates global process environment via `std::env::set_var("MUSIC_FORCE_DSP", "1")`, causing `test_stage_routing_gpu_vs_dsp` to panic intermittently on thread contention.
  2. `adversarial_fusion_stress_tests.rs:274` contains the exact same vulnerability at `test_stress_pure_rust_and_env_force_overrides`, which empirically reproduced and failed at Run 4 under 16 threads (`test_stress_execution_routing_all_stages_under_auto_mode` panicked).
  3. Decoupling environment checks into `SystemMemory::detect_live()` and adding `force_dsp: bool` / `with_force_dsp(bool)` builder methods to `SystemMemory` completely eliminates the need for `std::env::set_var` in all test files, guaranteeing 100% thread safety and deterministic test runs.
- **Unexplored areas**: None for M1 remediation.

## Key Decisions Made
- Recommend the Architectural Fix: add `force_dsp` / `force_hf` to `SystemMemory` (initialized via `detect_live()` from env, default `false` in `simulated()`, with builder helpers `with_force_dsp` / `with_force_hf`).
- Update both `fusion_tests.rs` and `adversarial_fusion_stress_tests.rs` to eliminate `std::env::set_var` / `std::env::remove_var` calls completely.
- Formulate a 10-iteration PowerShell verification command with `--test-threads=16`.

## Artifact Index
- DISPATCH.md — incoming instructions
- BRIEFING.md — persistent situational awareness
- progress.md — liveness heartbeat
- handoff.md — comprehensive remediation plan and diff guide for Worker 2
