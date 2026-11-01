# Milestone 6 Review & Adversarial Challenge Report

- **Reviewer**: `reviewer_m6_2` (Milestone 6 Reviewer 2: WebUI, REST Endpoints, Resilience & Lyrics)
- **Roles**: reviewer, critic
- **Date**: 2026-11-01T13:48:00Z
- **Verdict**: **APPROVE**
- **Integrity Check**: **PASS — ZERO INTEGRITY VIOLATIONS DETECTED**

---

## 1. Observation

1. **Dynamic Port Assignment & REST Server Startup**:
   - In `crates/music_cli/src/rest_server.rs` (lines 1295–1343):
     - `bind_server(start_port, end_port)` delegates to `bind_server_on_host("127.0.0.1", start_port, end_port)`.
     - `bind_server_on_host` scans `start_port..=end_port` sequentially. If `start_port == 0`, it immediately binds port `0` for OS dynamic port assignment.
     - `bind_server_dynamic` and `bind_server_dynamic_on_host` (lines 1325–1343) wrap the scan with an automatic fallback: if all ports in `start_port..=end_port` are occupied, it binds `{host}:0`, guaranteeing zero-conflict startup.
   - In `crates/music_cli/src/main.rs` (lines 136–146):
     - `let end_port = args.port.saturating_add(20);` scans `5050..=5070` by default.
     - Calls `rest_server::start_rest_server_with_host` and formats the running URL with `bound_port`.

2. **Dedicated Download Endpoints & TTS Vocals**:
   - In `crates/music_cli/src/rest_server.rs` (lines 633–674, 1280–1282):
     - `GET /api/songs/:id/versions/:v/download`: delegates to `handle_stream_audio` with `clean_id`, `version`, `stem: "final_master"`, and `download: Some("1".to_string())`.
     - `GET /api/songs/:id/download`: resolves latest version using `state.runner.checkpointer().list_versions(&clean_id).unwrap_or_default().into_iter().max().unwrap_or(1)` and streams the latest master WAV with download headers.
     - Lines 1045–1051 set `Content-Disposition: attachment; filename="{clean_id}_v{version}_{raw_stem}.wav"` when `is_download == true`.
   - In `crates/music_cli/src/rest_server.rs` (lines 553–630, 1276):
     - `POST /api/songs/tts-vocals`: receives `TtsVocalRequest`, invokes `FusionTtsEngine::synthesize_vocal_stems`, saves checkpoints with updated stems, triggers `state.runner.master_track` with vocal-priority ducking (-1.8 dB), and returns `TtsVocalResponse` with complete `audio_urls` dictionary.

3. **WebUI Chat Producer Experience, Inline Actions & 404 Resilience**:
   - In `crates/music_cli/webui/app.js`:
     - Lines 1873–1884 (`findLastValidAudioUrls`): Traverses `song.versions` backwards from `maxVer` to version 1 to find the last valid `audioUrls.master`. If missing, inspects all version records.
     - Lines 1906–1918 & 1927–1955: When `/api/songs/iterate` completes or catches network/offline errors, `onIterationSuccess` uses `findLastValidAudioUrls` to ensure `resolvedAudioUrls` never point to non-existent 404 endpoints.
     - Lines 2006–2071 (`appendAssistantProducerReply`): Renders stage feedback badges (`🎤 VOCALS ADDED`, `🎸 ALL INSTRUMENTS MIXED`, `🛠️ SCRATCH REMOVED`), conversational explanations for user questions ("WHAT WAS THE SCRATCH" / "REMOVE THAT SCRATCH"), and inline actions:
       - `▶ Test This Version` (line 2056, wired lines 2076–2080)
       - `🎤 Solo Vocals Only` (line 2059, wired lines 2083–2096)
       - `⏪ Revert to v${version - 1}` (lines 2061–2064, wired lines 2099–2105, omitted when `version <= 1`)
       - `⬇ Download WAV` (line 2065–2067, links to `/api/songs/:id/versions/:v/download`)
   - In `crates/music_cli/webui/app.js` (lines 800–851):
     - `applyAudioOutputDevice`: Synchronizes `DOM.headerAudioOutputSelect` and `DOM.deckAudioOutputSelect`, checks `typeof audioEl.setSinkId === 'function'`, sets `audioEl.setSinkId(targetSinkId)`, and cleanly falls back to `audioEl.setSinkId('')` if rejected.

4. **Multi-Section Lyrics Persistence & Drawer Display**:
   - In `crates/music_cli/webui/index.html` (lines 733–740):
     - `#toggle-lyrics-btn` has `aria-expanded="true"`.
     - `#lyrics-drawer-body` has `class="section-collapsible-body"` without `collapsed`, making lyrics expanded and visible by default.
   - In `crates/music_cli/webui/app.js` (lines 1153–1220, 1313–1316):
     - `renderLyrics` parses bracketed tags (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Verse 2]`, `[Guitar Solo]`, `[Bridge]`, `[Outro]`), escapes HTML with `escapeHTML`, updates raw textarea, YuE preview, and ModelFusion TTS custom text input.
     - `switchVersion` invokes `renderLyrics(ver.lyrics)` on every version/song change.
   - On disk in `storage/songs/`:
     - All 4 sample songs across all 8 version directories (`roots_reggae_anthem/v1`, `v2`, `song-one-love-shining-1977/v1`, `v2`, `song_afrobeats_002/v1`, `v2`, `song_reggae_anthem_001/v1`, `v2`) contain complete multi-section English lyrics in `lyrics.txt`, `manifest.json`, and `state.json`.
     - Files are verified clean UTF-8 without BOM.

5. **Independent Build and Test Verification**:
   - `cargo test -p music_cli` executed synchronously through completion:
     - `tests\challenger_m4_adversarial_tests.rs`: 3 passed, 0 failed
     - `tests\challenger_tier5_2_empirical_stress_tests.rs`: 5 passed, 0 failed (including empirical decoding and peak measurement of all 44 sample song WAV files, finished in 106.25s)
     - `tests\cli_and_rest_tests.rs`: 7 passed, 0 failed
     - `tests\m4_adversarial_challenge_tests.rs`: 13 passed, 0 failed
     - `tests\tier5_adversarial_challenge_suite.rs`: 8 passed, 0 failed
     - Total: **36 tests passed, 0 failed, 0 ignored**.
   - `cargo check --workspace` exited with code 0 (0 compilation warnings or errors).

---

## 2. Logic Chain

1. **Verification of Port Assignment Resilience**:
   - Observation 1 demonstrates that port scanning starts at `args.port` (5050) and scans through `end_port` (5070).
   - If intermediate ports are bound by other processes, `bind_server_on_host` logs a warning and tests `port + 1`.
   - If all ports 5050..=5070 are occupied, `bind_server_dynamic_on_host` executes `TcpListener::bind(format!("{}:0", host))`, which the OS kernel satisfies with an ephemeral port.
   - Conclusion: Port binding is 100% resilient and satisfies the user requirement ("PORT 5050 MAY BE IN USE DO PORTS SHOULD BE DYNAMICALLY ASSIGNED").

2. **Verification of Download Endpoints**:
   - Observation 2 demonstrates dedicated routes for both specific version downloads (`/api/songs/:id/versions/:v/download`) and latest master downloads (`/api/songs/:id/download`).
   - Both routes sanitize `:id` with `CheckpointManager::sanitize_path_component`, assert canonical containment within `storage_dir`, and return `Content-Disposition: attachment; filename="..."`.
   - Conclusion: User download failures ("WHEN i DOWNLOAD IT i GET AN ERROR") are resolved.

3. **Verification of 404 Resilience During Iteration**:
   - Observation 3 shows that previously, `onIterationSuccess` blindly assumed files existed at `.../versions/${nextVer}/...`. When a user typed "REMOVE THE SCRATCH", an ungenerated version caused 404 MediaErrors.
   - With `findLastValidAudioUrls`, the client traverses backwards to the last known valid audio URL set (e.g. v2 or v1) if backend generation failed or is offline.
   - Concurrently on the backend, `rest_server::serve_stem` implements backwards lineage fallback across previous versions for any missing stem.
   - Conclusion: 404 playback crashes are eliminated.

4. **Verification of Assistant Producer Experience & Lyrics Visibility**:
   - Observations 3 and 4 confirm that Assistant reply bubbles provide stage feedback badges (`🎤 VOCALS ADDED`, `🎸 ALL INSTRUMENTS MIXED`, `🛠️ SCRATCH REMOVED`) and inline actions (`▶ Test This Version`, `🎤 Solo Vocals Only`, `⏪ Revert`, `⬇ Download WAV`).
   - Observations 4 and 5 confirm that complete multi-section English lyrics (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Verse 2]`, `[Guitar Solo]`, `[Bridge]`, `[Outro]`) exist across all 8 version directories on disk and are expanded by default in the WebUI (`#lyrics-drawer-body` non-collapsed).

---

## 3. Caveats & Adversarial Challenges

1. **IPv6 Literal Formatting in `bind_server_on_host` (Minor)**:
   - In `rest_server.rs` line 1309: `let addr = format!("{}:{}", host, port);`
   - If a user passes an unbracketed IPv6 address (e.g. `--host ::1`), `TcpListener::bind("::1:5050")` will return an error because IPv6 literals require brackets (`[::1]:5050`).
   - *Risk*: Minimal. The default host is `"127.0.0.1"`.
   - *Recommendation*: In a future maintenance pass, wrap `host` with brackets if `host.contains(':') && !host.starts_with('[')`.

2. **Browser Compatibility for `setSinkId` (Low)**:
   - In non-Chromium browsers (Firefox default, Safari iOS), `HTMLMediaElement.setSinkId` may not be supported.
   - *Observation*: `app.js` line 836 guards with `if (typeof audioEl.setSinkId === 'function')`. If unavailable, playback continues through default system audio without script errors.

3. **Orphaned Submodule Dirty States Outside Workspace**:
   - Sibling repositories (`../AgentOctopus`, `../OshoosiClaw`, `../killswitch`) contain uncommitted files from external agents. These are strictly outside `d:\harfile\music\` and do not impact the music workspace.

---

## 4. Conclusion

Milestone 6 implementation is **rigorous, resilient, and fully compliant** with all prompt and user requirements:
- Dynamic port assignment (`5050..=5070` with OS ephemeral fallback) prevents port conflict crashes.
- Dedicated download endpoints (`/api/songs/:id/versions/:v/download` and `/api/songs/:id/download`) deliver valid 48 kHz stereo WAVs with proper attachment headers.
- ModelFusion Text-to-Voice (`POST /api/songs/tts-vocals`) synthesizes lead vocals, updates checkpoints, and remixes masters with vocal-priority ducking.
- WebUI Chat Producer interface displays stage-by-stage badges, conversational replies, inline players, solo vocal toggles, version revert buttons, and output device selection via `setSinkId`.
- Playback 404 errors during prompt iteration are eliminated via dual-layer client and server fallback.
- Complete multi-section English lyrics exist across all 8 sample song version directories on disk and render by default in the WebUI.
- All 36 `music_cli` tests and workspace checks pass cleanly.
- Zero integrity violations detected.

**Final Verdict: APPROVE**

---

## 5. Verification Method

To independently verify this evaluation:

1. **Verify Cargo Compilation & Tests**:
   ```powershell
   cargo check --workspace
   cargo test -p music_cli
   ```
   *Expected: Exit code 0, 36 passed, 0 failed.*

2. **Verify Disk Lyrics & Audio Integrity**:
   ```powershell
   cargo test -p music_cli --test challenger_tier5_2_empirical_stress_tests test_disk_song_artifacts_roots_reggae_and_one_love
   ```
   *Expected: ok. Validates all 44 WAV files, 48 kHz stereo, peak <= -0.3 dBFS, and SHA256 non-identity.*

3. **Verify WebUI Lyrics Drawer Default State**:
   Inspect `crates/music_cli/webui/index.html` lines 733–740. Assert `#lyrics-drawer-body` does not possess the `collapsed` class and `#toggle-lyrics-btn` has `aria-expanded="true"`.
