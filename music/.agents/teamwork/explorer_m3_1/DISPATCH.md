# Dispatch: Explorer M3-1 (Native Rust StateGraph Engine & Checkpointer)

## Objective
Design and specify the pure-Rust `StateGraph<MusicProductionState>` async DAG / Pregel execution engine and checkpointer:
1. `StateGraph` builder and `CompiledGraph` execution engine:
   - Zero Python LangGraph, zero runtime ModelFusion link.
   - Node registration: `add_node(name: &str, node: impl GraphNode<MusicProductionState>)`.
   - Edge registration: `add_edge(from: &str, to: &str)`.
   - Conditional edge registration: `add_conditional_edges(from: &str, condition_fn, branches: HashMap<String, String>)`.
   - `execute(&self, initial_state: MusicProductionState) -> Result<MusicProductionState, PipelineError>`.
2. Checkpoint manager:
   - Saves state to `./storage/songs/<song_id>/v<version>/state.json`.
   - Persists intermediate WAV stems to `./storage/songs/<song_id>/v<version>/stems/`.
   - Supports version increments and state recovery.
3. Strongly-typed `PipelineError` in `crates/music_pipeline/src/error.rs`.

## Inputs
- Read `ORIGINAL_REQUEST.md`, `PROJECT.md`, `TEST_INFRA.md`.
- Inspect `crates/music_core/src/state.rs`, `schema.rs`.
- Inspect `crates/music_fusion/src/lib.rs` and `crates/music_dsp/src/lib.rs`.

## Output
Deliver `d:\harfile\music\.agents\teamwork\explorer_m3_1\handoff.md` with complete architectural design, types, traits, method signatures, and unit test plan.


## 2026-11-01T02:54:53Z
Sender: 4caf3f77-b3ac-4b67-8bda-934a91faafab
Priority: MESSAGE_PRIORITY_HIGH
Content:
You are Explorer M3-1 for Milestone 3 (Native Rust StateGraph & 6 Fusion Nodes).
Your working directory is d:\harfile\music\.agents\teamwork\explorer_m3_1\.
Read d:\harfile\music\ORIGINAL_REQUEST.md, d:\harfile\music\PROJECT.md, d:\harfile\music\TEST_INFRA.md, and your DISPATCH.md at d:\harfile\music\.agents\teamwork\explorer_m3_1\DISPATCH.md.
Design the native Rust StateGraph async DAG / Pregel execution engine, node trait, edge routing (direct and conditional), checkpointer, and strongly typed PipelineError.
Write your design and handoff report to d:\harfile\music\.agents\teamwork\explorer_m3_1\handoff.md.
Send a message back to parent when done.
