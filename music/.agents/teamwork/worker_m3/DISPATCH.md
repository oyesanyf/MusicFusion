# DISPATCH: Milestone 3 Implementation Worker (worker_m3)

## Task Description
Implement Milestone 3 (`crates/music_pipeline`) in `d:\harfile\music`:
Native Rust `StateGraph<MusicProductionState>` async execution engine, `CheckpointManager` with path traversal defense, and the 6 sequential fusion nodes (`node1_producer_panel`, `node2_groove_yue`, `node3_vocal_timbre`, `node4_stem_inpainting`, `node5_dsp_judge_gate` with conditional retry edge, `node6_analog_mastering`) with deterministic VRAM process isolation (`tokio::process::Command` + `CREATE_NO_WINDOW`) and pure-Rust DSP fallback.

## Authoritative Inputs
Subagent MUST read these files:
- `d:\harfile\music\ORIGINAL_REQUEST.md` (Mandatory authoritative requirements)
- `d:\harfile\music\PROJECT.md` (Global architecture & Milestone 3 scope)
- `d:\harfile\music\TEST_INFRA.md` (E2E testing architecture)
- Explorer reports & proposed implementations:
  - `d:\harfile\music\.agents\teamwork\explorer_m3_1\handoff.md` (StateGraph, Checkpointer, Runner, Node trait, Error)
  - `d:\harfile\music\.agents\teamwork\explorer_m3_2\handoff.md` and proposed files:
    - `d:\harfile\music\.agents\teamwork\explorer_m3_2\proposed_node1_producer_panel.rs`
    - `d:\harfile\music\.agents\teamwork\explorer_m3_2\proposed_node2_groove_yue.rs`
    - `d:\harfile\music\.agents\teamwork\explorer_m3_2\proposed_node3_vocal_timbre.rs`
    - `d:\harfile\music\.agents\teamwork\explorer_m3_2\proposed_nodes_mod.rs`
    - `d:\harfile\music\.agents\teamwork\explorer_m3_2\proposed_node_tests.rs`
  - `d:\harfile\music\.agents\teamwork\explorer_m3_3\handoff.md` (Nodes 4, 5, 6, retry logic, mastering profiles)

## Exclusive File Ownership
You have exclusive write ownership of:
- `crates/music_pipeline/Cargo.toml`
- `crates/music_pipeline/src/**`
- `crates/music_pipeline/tests/**`

## Verification Requirements
1. `cargo check --workspace`
2. `cargo clippy -p music_pipeline -- -D warnings`
3. `cargo test -p music_pipeline` (all unit and integration tests must pass)
4. `cargo test --workspace` (all 150+ workspace tests must pass)
5. `cargo test --manifest-path tests/Cargo.toml` (all 191 E2E tests must pass)
6. Git add, commit with clean message ("feat(pipeline): implement native Rust StateGraph, CheckpointManager, and 6 fusion nodes")
7. Write `d:\harfile\music\.agents\teamwork\worker_m3\handoff.md` and send completion message to parent orchestrator.

## Mandatory Integrity Warning
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## 2026-11-01T03:03:58Z
You are worker_m3, the Milestone 3 Pipeline & Nodes Implementation Worker.
Your working directory is `d:\harfile\music\.agents\teamwork\worker_m3\`.
Follow instructions in dispatch: Milestone 3 StateGraph, CheckpointManager, and 6 fusion nodes.
Exclusive write ownership:
- `crates/music_pipeline/Cargo.toml`
- `crates/music_pipeline/src/**`
- `crates/music_pipeline/tests/**`
