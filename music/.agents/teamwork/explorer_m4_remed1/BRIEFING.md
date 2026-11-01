# BRIEFING — 2026-11-01T05:22:00Z

## Mission
Investigate and design a drop-in, secure remediation for `handle_stream_audio` in `crates/music_cli/src/rest_server.rs` to fix Milestone 4 Gate 1 test failure in `challenger_m4_adversarial_tests.rs`.

## 🔒 My Identity
- Archetype: teamwork_explorer
- Roles: Investigation, Synthesis, Root Cause Analysis, Remediation Architecture
- Working directory: d:\harfile\music\.agents\teamwork\explorer_m4_remed1\
- Original parent: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Milestone: Milestone 4 Remediation

## 🔒 Key Constraints
- Read-only investigation — do NOT directly modify source code in crates/
- All proposals, diffs, and handoff reports must be written strictly to `.agents/teamwork/explorer_m4_remed1/`
- Fail-closed path traversal and containment security (prevent path traversal outside `storage_dir`)
- Real code and robust error handling (no mocks)

## Current Parent
- Conversation ID: 4caf3f77-b3ac-4b67-8bda-934a91faafab
- Updated: 2026-11-01T05:13:30Z

## Investigation State
- **Explored paths**:
  - `crates/music_cli/src/rest_server.rs` (lines 480-580, 680-770)
  - `crates/music_cli/tests/challenger_m4_adversarial_tests.rs` (lines 110-191)
  - `crates/music_cli/tests/m4_adversarial_challenge_tests.rs` (lines 430-714)
  - `crates/music_pipeline/src/checkpointer.rs` (`sanitize_path_component`, `load_checkpoint`)
  - `crates/music_pipeline/src/runner.rs` (`regenerate_stem`, `iterate_song`)
  - `crates/music_cli/src/main.rs` (CLI command handling)
- **Key findings**:
  1. `handle_stream_audio` in `crates/music_cli/src/rest_server.rs:509-531` resolved paths strictly within `v{version}/stems/` without checking `state.runner.checkpointer().load_checkpoint(&clean_id, version)` and `state.stem_paths`.
  2. Targeted stem regeneration (e.g. `POST /api/songs/regenerate-stem` for `guitar`) only re-generates Node 4 accompaniment stems into `v2/stems/`. The untargeted stems (`raspy_tenor_vocals`) remain in `v1/stems/` and are tracked via `state.stem_paths["raspy_tenor_vocals"]`. Requesting `GET /api/songs/{id}/versions/2/audio/raspy_tenor_vocals` failed with 404.
  3. `rest_server.rs` router only bound `/api/songs/:id/versions/:v/audio/:stem` and lacked the alias `/api/songs/:id/:version/audio/:file`.
  4. Path traversal containment guard: Created `is_path_contained_in_storage` canonicalizing paths, stripping Windows `\\?\` UNC prefixes, and asserting containment in `state.storage_dir`.
  5. Secondary finding: `runner.iterate_song` in `crates/music_pipeline/src/runner.rs:205-219` did not parse critique for BPM/key adjustments, causing identical audio hashes between `v1` and `v2` during deterministic CLI pure_rust runs.
- **Unexplored areas**: None for M4 audio streaming.

## Key Decisions Made
- Designed drop-in replacement for `handle_stream_audio` with checkpoint state lookup, alias resolution, disk probing fallback, and fail-closed containment check in `is_path_contained_in_storage`.
- Added dual route support in Axum router for both `/api/songs/:id/versions/:v/audio/:stem` and `/api/songs/:id/:version/audio/:file`.
- Created `proposed_rest_streaming_fix.rs` and `proposed_challenger_test_update.patch`.

## Artifact Index
- `DISPATCH.md` — Incoming task dispatch record
- `BRIEFING.md` — Persistent working memory
- `progress.md` — Liveness and execution heartbeat
- `proposed_rest_streaming_fix.rs` — Complete drop-in code for `handle_stream_audio` and `is_path_contained_in_storage`
- `proposed_challenger_test_update.patch` — Unified diff extending challenger tests to assert unregenerated stem streaming & Pattern 2
- `handoff.md` — 5-component handoff report for orchestrator/builder
