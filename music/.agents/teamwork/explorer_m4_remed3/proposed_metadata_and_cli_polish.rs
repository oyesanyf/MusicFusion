//! Proposed Remediation & Polish for Milestone 4 (Explorer M4-3)
//!
//! This file provides complete, ready-to-integrate drop-in code for:
//! 1. `crates/music_cli/src/cli_args.rs`: Add `alias = "mastering-profile"` to `pub profile: String`.
//! 2. `crates/music_cli/src/browser.rs`: Implement `should_suppress_browser` checking CI env vars (`CI`, `GITHUB_ACTIONS`, `TF_BUILD`, `AGENT_OS`).
//! 3. `crates/music_cli/src/presets.rs`: Add `flagship_reggae_anthem_state_with_id` supporting custom `song_id`.
//! 4. `crates/music_cli/src/main.rs`: Integrate browser suppression, customizable anthem ID, and genre-adaptive iteration assistance.
//! 5. `crates/music_cli/src/mcp_server.rs`: Delegate `music_assist_iteration` to `IterationFusionPanel`.
//! 6. `crates/music_cli/src/lib.rs`: Re-export `should_suppress_browser` and `flagship_reggae_anthem_state_with_id`.
//! 7. Disk Metadata Generator & Normalizer: Clean procedure for generating both `roots_reggae_anthem` and `song-one-love-shining-1977`.

// ===========================================================================
// SECTION 1: PROPOSED CHANGES FOR `crates/music_cli/src/cli_args.rs`
// ===========================================================================
//
// In `crates/music_cli/src/cli_args.rs:80-83`:
//
// BEFORE:
// ```rust
//     /// Mastering profile ('analog_tape', 'modern_pop', 'club_sub', 'cinematic', 'acoustic_jazz')
//     #[arg(long, default_value = "analog_tape")]
//     pub profile: String,
// ```
//
// AFTER:
// ```rust
//     /// Mastering profile ('analog_tape', 'modern_pop', 'club_sub', 'cinematic', 'acoustic_jazz')
//     #[arg(long, alias = "mastering-profile", default_value = "analog_tape")]
//     pub profile: String,
// ```
//
// RATIONALE:
// Adding `alias = "mastering-profile"` ensures that both `--profile <NAME>` (legacy/short)
// and `--mastering-profile <NAME>` (full CLI spec) are accepted without syntax error.

// ===========================================================================
// SECTION 2: PROPOSED CHANGES FOR `crates/music_cli/src/browser.rs`
// ===========================================================================

pub mod proposed_browser {
    use std::time::Duration;
    use log::{info, warn};

    /// Launches the default web browser to the specified URL.
    /// On Windows, executes `cmd /C start "" "<url>"` with `CREATE_NO_WINDOW (0x08000000)`
    /// to eliminate console window flash.
    pub fn open_browser_raw(url: &str) -> std::io::Result<()> {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            std::process::Command::new("cmd")
                .args(["/C", "start", "", url])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()?;
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(url)
                .spawn()?;
        }

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            std::process::Command::new("xdg-open")
                .arg(url)
                .spawn()?;
        }

        Ok(())
    }

    /// Determines if browser opening should be suppressed based on user flags (`no_browser`, `mcp`)
    /// or headless / continuous integration environment variables:
    /// `CI`, `GITHUB_ACTIONS`, `TF_BUILD`, and `AGENT_OS`.
    pub fn should_suppress_browser(no_browser: bool, mcp: bool) -> bool {
        if no_browser || mcp {
            return true;
        }
        const CI_VARS: &[&str] = &["CI", "GITHUB_ACTIONS", "TF_BUILD", "AGENT_OS"];
        for var in CI_VARS {
            if std::env::var(var).map(|v| !v.trim().is_empty()).unwrap_or(false) {
                return true;
            }
        }
        false
    }

    /// Spawns a background delayed browser launch task.
    /// Delay allows the HTTP listener to complete socket bind before browser connects.
    /// Strictly suppressed if `suppressed` is true (e.g. `--no-browser`, `--mcp`, CI).
    pub fn spawn_browser_delayed(url: String, delay_ms: u64, suppressed: bool) {
        if suppressed || should_suppress_browser(false, false) {
            info!("Browser launch suppressed by flag or headless/CI environment");
            return;
        }

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            info!("Auto-launching browser to {}", url);
            if let Err(e) = open_browser_raw(&url) {
                warn!("Failed to launch browser: {}", e);
            }
        });
    }
}

// ===========================================================================
// SECTION 3: PROPOSED CHANGES FOR `crates/music_cli/src/presets.rs`
// ===========================================================================

pub mod proposed_presets {
    use anyhow::Result;
    use music_core::genre::GenreTaxonomy;
    use music_core::state::MusicProductionState;
    use music_cli::cli_args::CliArgs;

    /// Flagship 1977 Roots Reggae "One Love" Anthem state builder (default ID).
    pub fn flagship_reggae_anthem_state(
        bpm_override: Option<u32>,
        key_override: Option<&str>,
        pure_rust: bool,
    ) -> MusicProductionState {
        flagship_reggae_anthem_state_with_id(None, bpm_override, key_override, pure_rust)
    }

    /// Flagship 1977 Roots Reggae "One Love" Anthem state builder with customizable song ID.
    /// Supports explicitly setting `song_id` to either "roots_reggae_anthem" or "song-one-love-shining-1977"
    /// or any custom ID, ensuring internal `song_id` and blueprint match exactly.
    pub fn flagship_reggae_anthem_state_with_id(
        song_id: Option<&str>,
        bpm_override: Option<u32>,
        key_override: Option<&str>,
        pure_rust: bool,
    ) -> MusicProductionState {
        let mut state = MusicProductionState::default_reggae_anthem();
        if let Some(id) = song_id {
            state.song_id = id.to_string();
            if let Some(bp) = &mut state.blueprint {
                bp.song_id = id.to_string();
            }
        }
        if let Some(bpm) = bpm_override {
            state.bpm = bpm;
        }
        if let Some(key) = key_override {
            state.key_signature = key.to_string();
        }
        if pure_rust {
            state.engine_mode = "pure_rust".to_string();
        }
        state
    }

    /// Universal multi-genre hit generation state builder.
    pub fn create_hit_state(args: &CliArgs) -> Result<MusicProductionState> {
        let genre_query = args.genre.as_deref().unwrap_or("roots_reggae");
        let preset = GenreTaxonomy::get_or_default(genre_query);

        let song_id = args.song_id.clone().unwrap_or_else(|| {
            format!("{}-{}", preset.id, &uuid::Uuid::new_v4().to_string()[..8])
        });
        let bpm = args.bpm.unwrap_or(preset.default_bpm);
        let key = args.key.clone().unwrap_or_else(|| preset.default_key.clone());

        let mut state = MusicProductionState::new(&song_id, &preset.name, bpm, &key);
        state.time_signature = args.time_signature.clone();

        if args.pure_rust || args.engine_mode == "pure_rust" {
            state.engine_mode = "pure_rust".to_string();
        } else {
            state.engine_mode = args.engine_mode.clone();
        }

        if let Some(prompt) = &args.prompt {
            state.style_prompt = prompt.clone();
        } else {
            state.style_prompt = format!("{} hit in {}, tempo {} BPM, {}", preset.name, key, bpm, preset.description);
        }

        if let Some(lyrics) = &args.lyrics {
            state.lyrics = lyrics.clone();
        }

        Ok(state)
    }
}

// ===========================================================================
// SECTION 4: PROPOSED CHANGES FOR `crates/music_cli/src/mcp_server.rs`
// ===========================================================================
//
// In `crates/music_cli/src/mcp_server.rs:306-330`:
//
// BEFORE:
// ```rust
//         "music_assist_iteration" => {
//             let song_id = args.get("song_id").and_then(|v| v.as_str()).unwrap_or("");
//             let version = args.get("version").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(1);
//
//             let state = runner.checkpointer().load_checkpoint(song_id, version).await?;
//             let suggestions = vec![
//                 format!("🔥 Boost guitar solo wah envelope and sustain in [Guitar Solo] (BPM: {})", state.bpm),
//                 "🥁 Add syncopated Nyabinghi percussion fills in [Verse 2]".to_string(),
//                 "🎤 Elevate gospel choir call-and-response harmonies in [Chorus]".to_string(),
//                 "🔊 Boost 55-85 Hz analog sub-bass warmth on bass stem".to_string(),
//             ];
//
//             Ok(json!({
//                 "status": "completed",
//                 "song_id": song_id,
//                 "version": version,
//                 "critique": format!("Song '{}' v{} has peak {} dBFS with {} BPM in {}. Dynamic range is {} dB.",
//                     state.song_id, state.version, state.telemetry.peak_dbfs, state.bpm, state.key_signature, state.telemetry.dynamic_range_db),
//                 "suggested_next_iterations": suggestions,
//                 "content": [{
//                     "type": "text",
//                     "text": format!("Iteration Co-Pilot suggestions for {}: {:?}", song_id, suggestions)
//                 }]
//             }))
//         }
// ```
//
// AFTER:
// ```rust
//         "music_assist_iteration" => {
//             let song_id = args.get("song_id").and_then(|v| v.as_str()).unwrap_or("");
//             let version = args.get("version").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(1);
//             let focus = args.get("focus").and_then(|v| v.as_str());
//
//             let state = runner.checkpointer().load_checkpoint(song_id, version).await?;
//             let mem = music_fusion::memory::SystemMemory::detect_live();
//             let panel = music_fusion::iteration_panel::IterationFusionPanel::new_auto_configured(&mem).await;
//             let resp = panel.assist_song_iteration(&state, focus).await
//                 .unwrap_or_else(|_| panel.offline_fallback_assist(&state));
//
//             Ok(json!({
//                 "status": "completed",
//                 "song_id": song_id,
//                 "version": version,
//                 "copilot_model": resp.copilot_model,
//                 "critique": resp.critique,
//                 "suggested_next_iterations": resp.suggested_next_iterations,
//                 "content": [{
//                     "type": "text",
//                     "text": format!("Iteration Co-Pilot ({}) suggestions for {}: {:?}", resp.copilot_model, song_id, resp.suggested_next_iterations)
//                 }]
//             }))
//         }
// ```

// ===========================================================================
// SECTION 5: PROPOSED CHANGES FOR `crates/music_cli/src/main.rs`
// ===========================================================================
//
// Full updated `main.rs`:
//
// ```rust
// //! music_cli: Command-line interface, Web Studio REST server, and MCP stdio server.
//
// use std::sync::Arc;
// use clap::Parser;
// use music_cli::browser;
// use music_cli::cli_args::CliArgs;
// use music_cli::mcp_server;
// use music_cli::presets;
// use music_cli::rest_server;
// use music_pipeline::runner::{PipelineConfig, PipelineRunner};
//
// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let args = CliArgs::parse();
//
//     let runner_config = PipelineConfig::default()
//         .with_storage_dir(&args.storage_dir)
//         .with_pure_rust_mode(args.pure_rust || args.engine_mode == "pure_rust");
//     let runner = Arc::new(PipelineRunner::new(runner_config)?);
//
//     // 1. MCP stdio mode (never launches browser, stderr logging only)
//     if args.mcp {
//         return mcp_server::run_mcp_server(runner).await;
//     }
//
//     env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
//
//     // 2. Flagship Roots Reggae Anthem CLI command
//     if args.create_reggae_anthem {
//         println!("🇯🇲 Generating 1977 Roots Reggae 'One Love' Anthem...");
//         let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
//         let state = presets::flagship_reggae_anthem_state_with_id(
//             Some(song_id),
//             args.bpm,
//             args.key.as_deref(),
//             args.pure_rust,
//         );
//         let result = runner.execute_song(state).await?;
//         println!("✨ Completed Anthem generation: song_id={}, version={}", result.song_id, result.version);
//         if let Some(master) = result.stem_paths.get("final_master") {
//             println!("🔊 Final Master: {}", master.display());
//         }
//         if args.open_ui && !browser::should_suppress_browser(args.no_browser, args.mcp) {
//             let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
//             browser::open_browser_raw(&url)?;
//         }
//         return Ok(());
//     }
//
//     // 3. Universal Multi-Genre Hit CLI command
//     if args.create_hit {
//         println!("🎵 Generating Universal Multi-Genre Hit...");
//         let state = presets::create_hit_state(&args)?;
//         let result = runner.execute_song(state).await?;
//         println!("✨ Completed Hit generation: genre={}, song_id={}, version={}", result.genre, result.song_id, result.version);
//         if let Some(master) = result.stem_paths.get("final_master") {
//             println!("🔊 Final Master: {}", master.display());
//         }
//         if args.open_ui && !browser::should_suppress_browser(args.no_browser, args.mcp) {
//             let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
//             browser::open_browser_raw(&url)?;
//         }
//         return Ok(());
//     }
//
//     // 4. Prompt-based song iteration CLI command
//     if let Some(instructions) = &args.iterate {
//         let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
//         let version = args.version.unwrap_or(1);
//         println!("🔄 Iterating song {} (v{}) with instructions: '{}'", song_id, version, instructions);
//         let result = runner.iterate_song(song_id, version, Some(instructions)).await?;
//         println!("✨ Completed Iteration: song_id={}, new version={}", result.song_id, result.version);
//         if let Some(master) = result.stem_paths.get("final_master") {
//             println!("🔊 Final Master: {}", master.display());
//         }
//         if args.open_ui && !browser::should_suppress_browser(args.no_browser, args.mcp) {
//             let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
//             browser::open_browser_raw(&url)?;
//         }
//         return Ok(());
//     }
//
//     // 5. Query Iteration Co-Pilot LLM CLI command (Genre-Adaptive)
//     if args.assist_iteration {
//         let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
//         let version = args.version.unwrap_or(1);
//         println!("🤖 Invoking Iteration Co-Pilot for song {} (v{})...", song_id, version);
//         let state = runner.checkpointer().load_checkpoint(song_id, version).await?;
//         println!("📊 Telemetry: peak={} dBFS, RMS={} dBFS, clipping={}",
//             state.telemetry.peak_dbfs, state.telemetry.rms_dbfs, state.telemetry.clipping);
//
//         let mem = music_fusion::memory::SystemMemory::detect_live();
//         let panel = music_fusion::iteration_panel::IterationFusionPanel::new_auto_configured(&mem).await;
//         let resp = panel.assist_song_iteration(&state, None).await
//             .unwrap_or_else(|_| panel.offline_fallback_assist(&state));
//
//         println!("🧐 Critique: {}", resp.critique);
//         println!("💡 Suggested Next Iterations (Model: {}):", resp.copilot_model);
//         for (i, sug) in resp.suggested_next_iterations.iter().enumerate() {
//             println!("   {}. {}", i + 1, sug);
//         }
//         return Ok(());
//     }
//
//     // 6. Single stem regeneration
//     if let Some(stem) = &args.regenerate_stem {
//         let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
//         let version = args.version.unwrap_or(1);
//         println!("🎛️ Regenerating stem '{}' for song {} (v{})...", stem, song_id, version);
//         let result = runner.regenerate_stem(song_id, version, stem).await?;
//         println!("✨ Completed stem regeneration: song_id={}, version={}", result.song_id, result.version);
//         return Ok(());
//     }
//
//     // 7. Track mastering
//     if args.master {
//         let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
//         let version = args.version.unwrap_or(1);
//         println!("🎚️ Mastering track for song {} (v{})...", song_id, version);
//         let result = runner.master_track(song_id, version).await?;
//         println!("✨ Completed mastering: song_id={}, version={}", result.song_id, result.version);
//         return Ok(());
//     }
//
//     // 8. Default mode or --serve: Launch Tokio HTTP REST Server + ChatGPT-like Web Studio UI
//     let bound_port = rest_server::start_rest_server(
//         runner,
//         args.storage_dir.clone(),
//         args.port,
//         args.port + 10,
//         args.ollama_host.clone(),
//     ).await?;
//
//     let url = format!("http://{}:{}/", args.host, bound_port);
//     println!("🌐 Web Studio server running at {}", url);
//     println!("🎵 Press Ctrl+C to terminate server.");
//
//     let suppress_browser = browser::should_suppress_browser(args.no_browser, args.mcp);
//     browser::spawn_browser_delayed(url, 500, suppress_browser);
//
//     tokio::signal::ctrl_c().await?;
//     println!("\n👋 Web Studio server shutting down gracefully.");
//     Ok(())
// }
// ```

// ===========================================================================
// SECTION 6: PROPOSED CHANGES FOR `crates/music_cli/src/lib.rs`
// ===========================================================================
//
// ```rust
// pub mod browser;
// pub mod cli_args;
// pub mod mcp_server;
// pub mod presets;
// pub mod rest_server;
//
// pub use browser::{open_browser_raw, should_suppress_browser, spawn_browser_delayed};
// pub use cli_args::CliArgs;
// pub use mcp_server::{handle_mcp_request, handle_tool_call, list_tools, run_mcp_server};
// pub use presets::{create_hit_state, flagship_reggae_anthem_state, flagship_reggae_anthem_state_with_id};
// pub use rest_server::{
//     bind_server, create_router, run_server_foreground, start_rest_server, AppState,
//     CreateSongRequest, CreateSongResponse, HealthResponse, IterateSongRequest, IterateSongResponse,
//     MasterSongRequest, MasterSongResponse, RegenerateStemRequest, RegenerateStemResponse,
// };
// ```

// ===========================================================================
// SECTION 7: PROCEDURE FOR CLEAN DISK GENERATION & METADATA NORMALIZATION
// ===========================================================================
//
// The Forensic Auditor noted:
// "In `storage/songs/roots_reggae_anthem/v1/state.json`, `"song_id"` was `"song-one-love-shining-1977"`
// and all `stem_paths` pointed to `storage\songs\song-one-love-shining-1977\...` rather than `roots_reggae_anthem`."
//
// There are two complementary methods to establish and maintain 100% metadata consistency:
//
// METHOD A: Pure Pipeline Execution (Clean Generation from Scratch)
// -----------------------------------------------------------------
// Run the following commands sequentially using the newly enabled `--song-id` flag on anthem generation:
//
// ```powershell
// # 1. Generate flagship anthem under ID 'roots_reggae_anthem' (v1)
// cargo run -p music_cli -- --create-reggae-anthem --song-id roots_reggae_anthem --pure-rust --no-browser
//
// # 2. Iterate 'roots_reggae_anthem' to produce v2
// cargo run -p music_cli -- --iterate "Boost guitar solo wah and enhance sub-bass warmth" --song-id roots_reggae_anthem --version 1 --pure-rust --no-browser
//
// # 3. Generate flagship anthem under canonical ID 'song-one-love-shining-1977' (v1)
// cargo run -p music_cli -- --create-reggae-anthem --song-id song-one-love-shining-1977 --pure-rust --no-browser
//
// # 4. Iterate 'song-one-love-shining-1977' to produce v2
// cargo run -p music_cli -- --iterate "Boost guitar solo wah and enhance sub-bass warmth" --song-id song-one-love-shining-1977 --version 1 --pure-rust --no-browser
// ```
//
// METHOD B: Automated Metadata In-Place Fix & Validator
// -----------------------------------------------------
// For already rendered audio assets (29.1 MB per stem) in `storage/songs/roots_reggae_anthem/`,
// execute the following deterministic PowerShell script to align `state.json` and `manifest.json`
// to the containing folder's identity:

pub const POWERSHELL_METADATA_NORMALIZATION_SCRIPT: &str = r#"
# scripts/normalize_song_metadata.ps1
$storageDir = "storage\songs"

Get-ChildItem -Path $storageDir -Directory | ForEach-Object {
    $songFolder = $_.Name
    Get-ChildItem -Path $_.FullName -Directory -Filter "v*" | ForEach-Object {
        $vFolder = $_.Name
        $statePath = Join-Path $_.FullName "state.json"
        $manifestPath = Join-Path $_.FullName "manifest.json"

        if (Test-Path $statePath) {
            $stateJson = Get-Content $statePath -Raw | ConvertFrom-Json
            if ($stateJson.song_id -ne $songFolder) {
                Write-Host "Repairing $statePath: changing song_id '$($stateJson.song_id)' -> '$songFolder'"
                $oldId = $stateJson.song_id
                $stateJson.song_id = $songFolder
                if ($stateJson.blueprint -and $stateJson.blueprint.song_id) {
                    $stateJson.blueprint.song_id = $songFolder
                }

                # Update stem paths
                $rawText = Get-Content $statePath -Raw
                $fixedText = $rawText.Replace("storage\\songs\\$oldId", "storage\\songs\\$songFolder")
                $fixedText = $fixedText.Replace("""song_id"": ""$oldId""", """song_id"": ""$songFolder""")
                Set-Content -Path $statePath -Value $fixedText -Encoding UTF8
            }
        }

        if (Test-Path $manifestPath) {
            $rawText = Get-Content $manifestPath -Raw
            $manifestJson = $rawText | ConvertFrom-Json
            if ($manifestJson.song_id -ne $songFolder) {
                Write-Host "Repairing $manifestPath: changing song_id '$($manifestJson.song_id)' -> '$songFolder'"
                $oldId = $manifestJson.song_id
                $fixedText = $rawText.Replace("storage\\songs\\$oldId", "storage\\songs\\$songFolder")
                $fixedText = $fixedText.Replace("""song_id"": ""$oldId""", """song_id"": ""$songFolder""")
                Set-Content -Path $manifestPath -Value $fixedText -Encoding UTF8
            }
        }
    }
}
Write-Host "Metadata normalization complete."
"#;

// ===========================================================================
// SECTION 8: AUTOMATED UNIT & INTEGRATION TESTS TO VERIFY ALL FIXES
// ===========================================================================

#[cfg(test)]
mod tests {
    use clap::Parser;
    use music_cli::cli_args::CliArgs;
    use music_cli::browser;
    use music_cli::presets::{flagship_reggae_anthem_state, flagship_reggae_anthem_state_with_id};

    #[test]
    fn test_mastering_profile_alias_accepted() {
        // Test canonical --profile
        let args1 = CliArgs::parse_from(["music_cli", "--profile", "modern_pop"]);
        assert_eq!(args1.profile, "modern_pop");

        // Test alias --mastering-profile
        let args2 = CliArgs::parse_from(["music_cli", "--mastering-profile", "club_sub"]);
        assert_eq!(args2.profile, "club_sub");

        // Test alias with default value
        let args3 = CliArgs::parse_from(["music_cli"]);
        assert_eq!(args3.profile, "analog_tape");
    }

    #[test]
    fn test_browser_suppression_helper() {
        // no_browser flag triggers suppression
        assert!(browser::should_suppress_browser(true, false));

        // mcp flag triggers suppression
        assert!(browser::should_suppress_browser(false, true));

        // both flags triggers suppression
        assert!(browser::should_suppress_browser(true, true));

        // CI environment simulation
        std::env::set_var("CI", "true");
        assert!(browser::should_suppress_browser(false, false));
        std::env::remove_var("CI");
    }

    #[test]
    fn test_flagship_reggae_anthem_with_custom_song_id() {
        // Default song ID preserves "song-one-love-shining-1977"
        let default_state = flagship_reggae_anthem_state(None, None, true);
        assert_eq!(default_state.song_id, "song-one-love-shining-1977");
        assert_eq!(default_state.blueprint.as_ref().unwrap().song_id, "song-one-love-shining-1977");

        // Custom song ID sets "roots_reggae_anthem" cleanly
        let custom_state = flagship_reggae_anthem_state_with_id(
            Some("roots_reggae_anthem"),
            Some(76),
            Some("A Major"),
            true,
        );
        assert_eq!(custom_state.song_id, "roots_reggae_anthem");
        assert_eq!(custom_state.blueprint.as_ref().unwrap().song_id, "roots_reggae_anthem");
        assert_eq!(custom_state.bpm, 76);
        assert_eq!(custom_state.key_signature, "A Major");
        assert_eq!(custom_state.engine_mode, "pure_rust");
    }
}
