## 2026-11-01T05:23:39Z
You are the Milestone 4 Remediation Worker (teamwork_worker).
Your assigned working directory is: d:\harfile\music\.agents\teamwork\worker_m4_remed\

### MANDATORY INTEGRITY WARNING
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Read these mandatory files first:
1. `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md`
2. `d:\harfile\music\PROJECT.md`
3. `d:\harfile\music\.agents\teamwork\auditor_m4\handoff.md` (FULL FORENSIC AUDITOR EVIDENCE REPORT)
4. `d:\harfile\music\.agents\teamwork\explorer_m4_remed1\proposed_rest_streaming_fix.rs` & `proposed_challenger_test_update.patch`
5. `d:\harfile\music\.agents\teamwork\explorer_m4_remed2\proposed_iteration_audio_variation_fix.rs`
6. `d:\harfile\music\.agents\teamwork\explorer_m4_remed3\proposed_metadata_and_cli_polish.rs`

### EXCLUSIVE WRITE OWNERSHIP
You exclusively own and may edit:
- `crates/music_cli/src/rest_server.rs`
- `crates/music_cli/src/cli_args.rs`
- `crates/music_cli/src/browser.rs`
- `crates/music_cli/src/presets.rs`
- `crates/music_cli/src/mcp_server.rs`
- `crates/music_cli/src/main.rs`
- `crates/music_cli/tests/challenger_m4_adversarial_tests.rs`
- `crates/music_cli/tests/cli_and_rest_tests.rs`
- `crates/music_pipeline/src/runner.rs`
- `crates/music_pipeline/src/nodes/node1_producer_panel.rs`
- `crates/music_pipeline/src/nodes/node3_vocal_timbre.rs`
- `crates/music_pipeline/src/nodes/node6_analog_mastering.rs`
- `crates/music_fusion/src/iteration_panel.rs`
- `storage/songs/roots_reggae_anthem/` and `storage/songs/song-one-love-shining-1977/` (audio files & metadata)

### REQUIRED ACTIONS
1. **Fix Audio Streaming Path Resolution**:
   - In `crates/music_cli/src/rest_server.rs`, apply `handle_stream_audio` and `is_path_contained_in_storage` from `explorer_m4_remed1/proposed_rest_streaming_fix.rs`.
   - Ensure `st.stem_paths` from `state.runner.checkpointer().load_checkpoint(&clean_id, version)` is checked for stem resolution across version lineages, falling back to probing `v{version}` and earlier versions.
   - Register dual routes in `create_router`:
     `.route("/api/songs/:id/versions/:v/audio/:stem", get(handle_stream_audio))`
     `.route("/api/songs/:id/:version/audio/:file", get(handle_stream_audio))`
   - Apply the test update from `explorer_m4_remed1/proposed_challenger_test_update.patch` to `crates/music_cli/tests/challenger_m4_adversarial_tests.rs`.
2. **Implement Distinct Audio Waveform Synthesis on Iteration**:
   - Apply the changes from `explorer_m4_remed2/proposed_iteration_audio_variation_fix.rs`:
     - In `crates/music_pipeline/src/runner.rs`, enhance `iterate_song` and add `iterate_song_with_tweaks` to modulate state parameters (BPM, key, lyrics, and stem_mix).
     - In `crates/music_pipeline/src/nodes/node1_producer_panel.rs`, preserve `blueprint.stem_mix`.
     - In `crates/music_pipeline/src/nodes/node3_vocal_timbre.rs`, derive vocal rasp from `blueprint.stem_mix.vocal_rasp_intensity`.
     - In `crates/music_pipeline/src/nodes/node6_analog_mastering.rs`, scale premaster stems by `stem_mix` dB levels and sub-bass boost.
     - In `crates/music_fusion/src/iteration_panel.rs`, add heuristic BPM modulation in `offline_fallback_iteration`.
     - In `crates/music_cli/src/main.rs`, forward `--bpm` and `--key` to `runner.iterate_song_with_tweaks`.
3. **Metadata Consistency & Polish**:
   - Apply changes from `explorer_m4_remed3/proposed_metadata_and_cli_polish.rs`:
     - In `crates/music_cli/src/presets.rs`, implement `flagship_reggae_anthem_state_with_id(song_id, ...)`.
     - In `crates/music_cli/src/cli_args.rs`, add `alias = "mastering-profile"` to `pub profile: String`.
     - In `crates/music_cli/src/browser.rs`, implement `should_suppress_browser(no_browser, mcp)` checking CI env vars (`CI`, `GITHUB_ACTIONS`, `TF_BUILD`, `AGENT_OS`).
     - In `crates/music_cli/src/main.rs`, use `should_suppress_browser` for all browser launch call sites.
     - In `crates/music_cli/src/mcp_server.rs` and `main.rs`, delegate `music_assist_iteration` to `IterationFusionPanel`.
4. **Regenerate Test Songs on Disk**:
   - Re-generate `roots_reggae_anthem` and `song-one-love-shining-1977` (`v1` and iterated `v2`) on disk in `storage/songs/`.
   - Verify that `v1` and `v2` produce distinct SHA256 hashes (`final_master.wav` hashes differ) and distinct telemetry, while obeying audio ceiling $\le 0.966$.
   - Verify that all `state.json` and `manifest.json` files have matching `song_id` and correct stem paths.
5. **Run Verification Commands**:
   - `cargo clippy --workspace -- -D warnings` (0 warnings).
   - `cargo test -p music_cli` (all pass).
   - `cargo test --workspace` (100% of workspace targets pass, including all challenger tests).
   - `cargo test --manifest-path tests/Cargo.toml` (all 191 E2E tests pass).
6. **Commit Changes**:
   Stage and commit your changes with git (`git commit -m "fix(cli): resolve audio streaming path resolution, song iteration distinctness, and metadata consistency"`).
7. Write `handoff.md` in your working directory and notify the orchestrator with `send_message`.
