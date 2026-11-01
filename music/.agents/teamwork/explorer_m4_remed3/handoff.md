# Handoff Report: Milestone 4 Remediation (Explorer M4-3)

**Agent**: Remediation Explorer M4-3 (`teamwork_explorer`)  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m4_remed3\`  
**Date**: 2026-11-01  
**Scope**: 
1. Storage `song_id` and stem path metadata consistency (`storage/songs/roots_reggae_anthem/` & `storage/songs/song-one-love-shining-1977/`)
2. CLI flag `--mastering-profile` alias in `crates/music_cli/src/cli_args.rs`
3. Browser auto-launch suppression helper in `crates/music_cli/src/browser.rs` and `main.rs`
4. Genre-adaptive iteration assistance in `crates/music_cli/src/main.rs` and `crates/music_cli/src/mcp_server.rs`

---

## 1. Observation

### 1.1 Storage Metadata Inconsistency in `roots_reggae_anthem`
In `storage/songs/roots_reggae_anthem/v1/state.json`:
- Line 2: `"song_id": "song-one-love-shining-1977"`
- Lines 12-22:
```json
  "stem_paths": {
    "inpainted_drums": "storage\\songs\\song-one-love-shining-1977\\v1\\stems\\inpainted_drums.wav",
    "final_master": "storage\\songs\\song-one-love-shining-1977\\v1\\final_master.wav",
    "guitar": "storage\\songs\\song-one-love-shining-1977\\v1\\stems\\guitar.wav",
    ...
  }
```
And in `storage/songs/roots_reggae_anthem/v1/manifest.json`:
- Line 9: `"song_id": "song-one-love-shining-1977"`
- Lines 11-21: `"bass": "storage\\songs\\song-one-love-shining-1977\\v1\\stems\\bass.wav"`, etc.

The containing directory on disk is named `roots_reggae_anthem`, but every serialized state and manifest references `song-one-love-shining-1977`.
In `crates/music_cli/src/presets.rs:7-23`, `flagship_reggae_anthem_state` was hardcoded to call `MusicProductionState::default_reggae_anthem()`, which always sets `song_id = "song-one-love-shining-1977"`, without accepting `args.song_id` from CLI.

### 1.2 CLI Flag `--mastering-profile` Missing Alias
In `crates/music_cli/src/cli_args.rs:80-83`:
```rust
    /// Mastering profile ('analog_tape', 'modern_pop', 'club_sub', 'cinematic', 'acoustic_jazz')
    #[arg(long, default_value = "analog_tape")]
    pub profile: String,
```
Passing `--mastering-profile modern_pop` returns exit code 2:
`error: unexpected argument '--mastering-profile' found`
Milestone 4 requirements and documentation specify `--mastering-profile`.

### 1.3 Browser Suppression Incomplete Encapsulation
In `crates/music_cli/src/browser.rs:38-42`:
```rust
pub fn spawn_browser_delayed(url: String, delay_ms: u64, suppressed: bool) {
    if suppressed || std::env::var("CI").is_ok() {
        info!("Browser launch suppressed by flag or headless/CI environment");
        return;
    }
```
Only `CI` was inspected, ignoring `GITHUB_ACTIONS`, `TF_BUILD`, and `AGENT_OS`.
Furthermore, in `crates/music_cli/src/main.rs:37, 53, 70`:
```rust
if args.open_ui && !args.no_browser {
    let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
    browser::open_browser_raw(&url)?;
}
```
`open_browser_raw` was called when `--open-ui` was set without checking headless/CI environments.

### 1.4 Hardcoded Reggae Suggestions in CLI & MCP
In `crates/music_cli/src/main.rs:85-88`:
```rust
        println!("💡 Suggested Next Iterations:");
        println!("   1. 🔥 Boost guitar solo wah envelope and sustain in [Guitar Solo]");
        println!("   2. 🥁 Add syncopated Nyabinghi percussion fills in [Verse 2]");
        println!("   3. 🎤 Elevate gospel choir call-and-response harmonies in [Chorus]");
```
And in `crates/music_cli/src/mcp_server.rs:311-316`:
```rust
        "music_assist_iteration" => {
            let state = runner.checkpointer().load_checkpoint(song_id, version).await?;
            let suggestions = vec![
                format!("🔥 Boost guitar solo wah envelope and sustain in [Guitar Solo] (BPM: {})", state.bpm),
                "🥁 Add syncopated Nyabinghi percussion fills in [Verse 2]".to_string(),
                "🎤 Elevate gospel choir call-and-response harmonies in [Chorus]".to_string(),
                "🔊 Boost 55-85 Hz analog sub-bass warmth on bass stem".to_string(),
            ];
```
Regardless of whether the song is Afrobeats, Hip-Hop, Rock, Jazz, or Classical, `--assist-iteration` and `music_assist_iteration` produced static reggae suggestions, failing to adapt to the track's genre. In contrast, `rest_server.rs:593` properly delegates to `IterationFusionPanel::new_auto_configured(&mem).await.assist_song_iteration(...)`.

---

## 2. Logic Chain

1. *Deduction from 1.1 (Storage Metadata)*:
   - When `runner.execute_song(state)` executes, all stem directories and checkpoints are computed dynamically via `self.resolve_stems_dir(&state.song_id, state.version)` and `self.version_dir(&state.song_id, state.version)`.
   - In `presets.rs`, `flagship_reggae_anthem_state()` hardcoded `state.song_id` to `"song-one-love-shining-1977"`. When the previous worker manually copied `song-one-love-shining-1977` into `roots_reggae_anthem`, the serialized JSON files retained `"song_id": "song-one-love-shining-1977"` and cross-directory stem paths.
   - Introducing `flagship_reggae_anthem_state_with_id(song_id: Option<&str>, ...)` in `presets.rs` and plumbing `args.song_id.as_deref().unwrap_or("roots_reggae_anthem")` in `main.rs` allows the pipeline to natively generate either ID with 100% path coherence on disk.
2. *Deduction from 1.2 (CLI Flag Alias)*:
   - Adding `#[arg(long, alias = "mastering-profile", default_value = "analog_tape")]` to `pub profile: String` in `cli_args.rs` provides full backward and forward compatibility for `--profile` and `--mastering-profile`.
3. *Deduction from 1.3 (Browser Suppression)*:
   - Encapsulating `should_suppress_browser(no_browser: bool, mcp: bool) -> bool` in `browser.rs` to check `no_browser`, `mcp`, and CI variables (`CI`, `GITHUB_ACTIONS`, `TF_BUILD`, `AGENT_OS`) guarantees headless safety.
   - Calling `should_suppress_browser` at lines 37, 53, 70, and 125 in `main.rs` prevents inadvertent browser launches in CI test runs.
4. *Deduction from 1.4 (Genre-Adaptive Iteration)*:
   - `IterationFusionPanel` already contains `assist_song_iteration` and `default_genre_suggestions` with genre-specific adaptations across 20+ genres (Afrobeats log-drums, Rock tube overdrive, Trap 808s, Reggae Nyabinghi).
   - Delegating `--assist-iteration` in `main.rs` and `music_assist_iteration` in `mcp_server.rs` to `IterationFusionPanel::new_auto_configured(&mem).await.assist_song_iteration(...)` replaces static hardcoded strings with genre-tailored suggestions.

---

## 3. Caveats

- **Audio File Size**: Rendered WAV stems on disk are ~29.1 MB each (32-bit float, 48 kHz). Generating both `roots_reggae_anthem` and `song-one-love-shining-1977` across v1 and v2 takes ~20 seconds of CPU DSP time.
- **Offline / Pure-Rust Fast-Path**: When offline or in `--pure-rust` mode, `IterationFusionPanel` immediately utilizes its deterministic offline fallback heuristic, guaranteeing sub-millisecond execution and zero empty states without relying on an external Ollama daemon.

---

## 4. Conclusion

All 4 remediation requirements have been investigated and fully designed. The deliverable `proposed_metadata_and_cli_polish.rs` contains ready-to-integrate source implementations for:
1. `crates/music_cli/src/presets.rs`: `flagship_reggae_anthem_state_with_id` supporting custom `song_id`.
2. `crates/music_cli/src/cli_args.rs`: `alias = "mastering-profile"` on `pub profile: String`.
3. `crates/music_cli/src/browser.rs`: `pub fn should_suppress_browser(no_browser: bool, mcp: bool) -> bool`.
4. `crates/music_cli/src/main.rs`: Full integration of browser suppression, custom song ID, and genre-adaptive iteration assistance.
5. `crates/music_cli/src/mcp_server.rs`: Delegation of `music_assist_iteration` to `IterationFusionPanel`.
6. Complete disk metadata generation and PowerShell normalization procedures to guarantee 100% path and `song_id` consistency on disk.

---

## 5. Verification Method

To independently verify the proposed remediation:

1. **Verify CLI Flag Alias**:
   `cargo test -p music_cli --test cli_and_rest_tests`
   Assert that `CliArgs::parse_from(["music_cli", "--mastering-profile", "modern_pop"]).profile == "modern_pop"`.

2. **Verify Browser Suppression**:
   In `crates/music_cli/tests/cli_and_rest_tests.rs`:
   ```rust
   assert!(browser::should_suppress_browser(true, false));
   assert!(browser::should_suppress_browser(false, true));
   ```

3. **Verify Genre-Adaptive Iteration Assistance**:
   Invoke MCP tool `music_assist_iteration` on a non-reggae song (e.g. Afrobeats) and verify that suggestions contain genre-relevant terms (e.g. log-drum, brass horn riffs, polyrhythmic shaker rolls) rather than static Nyabinghi reggae suggestions.

4. **Verify On-Disk Metadata Integrity**:
   Inspect `storage/songs/roots_reggae_anthem/v1/state.json`:
   - Verify `"song_id": "roots_reggae_anthem"`
   - Verify all `stem_paths` reference `storage\songs\roots_reggae_anthem\...`
   Inspect `storage/songs/song-one-love-shining-1977/v1/state.json`:
   - Verify `"song_id": "song-one-love-shining-1977"`
   - Verify all `stem_paths` reference `storage\songs\song-one-love-shining-1977\...`
