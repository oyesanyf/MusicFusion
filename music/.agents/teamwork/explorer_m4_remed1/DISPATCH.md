## 2026-11-01T05:13:12Z
You are Remediation Explorer M4-1 (teamwork_explorer).
Your assigned working directory is: d:\harfile\music\.agents\teamwork\explorer_m4_remed1\

Read these mandatory files first:
1. `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md`
2. `d:\harfile\music\PROJECT.md`
3. `d:\harfile\music\.agents\teamwork\auditor_m4\handoff.md` (FULL FORENSIC AUDITOR EVIDENCE REPORT — READ COMPLETELY)

### AUDIT FAILURE CONTEXT
Milestone 4 Gate 1 failed unconditionally due to FORENSIC AUDITOR INTEGRITY VIOLATION.
Specifically:
`cargo test -p music_cli --test challenger_m4_adversarial_tests` fails on `test_rest_endpoints_regenerate_stem_and_mastering`:
`assertion left == right failed (left: 404, right: 200)`
Root Cause: In `crates/music_cli/src/rest_server.rs:509-531`, `handle_stream_audio` attempts to resolve audio stem file paths strictly within `v{version}/stems/` on disk without inspecting `state.runner.checkpointer().load_checkpoint(&clean_id, version)` and `state.stem_paths`.
When a targeted stem is regenerated (e.g. `POST /api/songs/regenerate-stem` for `guitar`), Node 4 writes only accompaniment stems to `v2/stems/`. The untargeted stems (e.g. `raspy_tenor_vocals`) remain in `v1/stems/`, and `state.stem_paths["raspy_tenor_vocals"]` correctly references `v1`. However, `handle_stream_audio` does not inspect `state.stem_paths` and only searches `v2/stems/`, returning 404.

### YOUR TASK
Investigate `crates/music_cli/src/rest_server.rs:493-572` and `crates/music_cli/tests/challenger_m4_adversarial_tests.rs`.
Design a robust, secure, and drop-in fix for `handle_stream_audio`:
1. Checkpoint state lookup: Load the state for `(&clean_id, version)` via `state.runner.checkpointer().load_checkpoint(&clean_id, version).await`.
2. Path resolution: Check if `st.stem_paths.get(&clean_stem)` (or canonical aliases like `final_master`/`master`) contains a valid existing path. Fall back to probing `v{version}/stems/{stem}.wav` and `v{version}/{stem}.wav`.
3. Path containment & security: Ensure the resolved path stays within `state.storage_dir` to prevent any directory traversal.
4. Also verify URL alias flexibility: support both `/api/songs/:id/versions/:v/audio/:stem` and `/api/songs/:id/:version/audio/:file`.

Write a complete, ready-to-integrate proposal in `d:\harfile\music\.agents\teamwork\explorer_m4_remed1\proposed_rest_streaming_fix.rs` and summarize your findings in `handoff.md`.
Send a message back to the orchestrator when complete.
