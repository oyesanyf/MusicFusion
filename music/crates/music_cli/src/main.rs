//! music_cli: Command-line interface, Web Studio REST server, and MCP stdio server.

use std::sync::Arc;
use clap::Parser;
use music_cli::browser;
use music_cli::cli_args::CliArgs;
use music_cli::mcp_server;
use music_cli::presets;
use music_cli::rest_server;
use music_pipeline::runner::{PipelineConfig, PipelineRunner};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    let runner_config = PipelineConfig::default()
        .with_storage_dir(&args.storage_dir)
        .with_pure_rust_mode(args.pure_rust || args.engine_mode == "pure_rust");
    let runner = Arc::new(PipelineRunner::new(runner_config)?);

    // 1. MCP stdio mode (never launches browser, stderr logging only)
    if args.mcp {
        return mcp_server::run_mcp_server(runner).await;
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 1b. Hugging Face Music Model Auto-Discovery & Upgrade CLI command
    if args.update_models {
        println!("🔄 Auto-Discovering & Upgrading Hugging Face Music & Vocal Models...");
        let mem = music_fusion::memory::SystemMemory::detect_live();
        mem.print_telemetry();
        let engine = music_fusion::hf_model_discovery::HfModelDiscoveryEngine::new(
            args.storage_dir.join("hf_music_models_registry.json"),
        );
        let registry = engine.evaluate_and_upgrade(&mem).await?;
        println!("✨ Discovered & Evaluated {} models across pipeline stages:", registry.discovered_count);
        for (stage, model_id) in &registry.active_models {
            println!("   • {:<22} => {}", stage, model_id);
        }
        println!("💾 Active registry saved to {}", args.storage_dir.join("hf_music_models_registry.json").display());
        return Ok(());
    }

    // 2. Flagship Roots Reggae Anthem CLI command
    if args.create_reggae_anthem {
        println!("🇯🇲 Generating 1977 Roots Reggae 'One Love' Anthem...");
        let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
        let state = presets::flagship_reggae_anthem_state_with_id(
            Some(song_id),
            args.bpm,
            args.key.as_deref(),
            args.pure_rust,
        );
        let result = runner.execute_song(state).await?;
        println!("✨ Completed Anthem generation: song_id={}, version={}", result.song_id, result.version);
        if let Some(master) = result.stem_paths.get("final_master") {
            println!("🔊 Final Master: {}", master.display());
        }
        if args.open_ui && !browser::should_suppress_browser(args.no_browser, args.mcp) {
            let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
            browser::open_browser_raw(&url)?;
        }
        return Ok(());
    }

    // 3. Universal Multi-Genre Hit CLI command
    if args.create_hit {
        println!("🎵 Generating Universal Multi-Genre Hit...");
        let state = presets::create_hit_state(&args)?;
        let result = runner.execute_song(state).await?;
        println!("✨ Completed Hit generation: genre={}, song_id={}, version={}", result.genre, result.song_id, result.version);
        if let Some(master) = result.stem_paths.get("final_master") {
            println!("🔊 Final Master: {}", master.display());
        }
        if args.open_ui && !browser::should_suppress_browser(args.no_browser, args.mcp) {
            let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
            browser::open_browser_raw(&url)?;
        }
        return Ok(());
    }

    // 4. Prompt-based song iteration CLI command with ParameterTweaks
    if let Some(instructions) = &args.iterate {
        let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
        let version = args.version.unwrap_or(1);
        println!("🔄 Iterating song {} (v{}) with instructions: '{}'", song_id, version, instructions);

        let tweaks = music_fusion::iteration_panel::ParameterTweaks {
            bpm: args.bpm,
            key_signature: args.key.clone(),
            ..Default::default()
        };

        let result = runner.iterate_song_with_tweaks(song_id, version, Some(instructions), Some(tweaks)).await?;
        println!(
            "✨ Completed Iteration: song_id={}, new version={} (BPM: {}, Key: {})",
            result.song_id, result.version, result.bpm, result.key_signature
        );
        if let Some(master) = result.stem_paths.get("final_master") {
            println!("🔊 Final Master: {}", master.display());
        }
        if args.open_ui && !browser::should_suppress_browser(args.no_browser, args.mcp) {
            let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
            browser::open_browser_raw(&url)?;
        }
        return Ok(());
    }

    // 5. Query Iteration Co-Pilot LLM CLI command
    if args.assist_iteration {
        let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
        let version = args.version.unwrap_or(1);
        println!("🤖 Invoking Iteration Co-Pilot for song {} (v{})...", song_id, version);
        let state = runner.checkpointer().load_checkpoint(song_id, version).await?;
        println!("📊 Telemetry: peak={} dBFS, RMS={} dBFS, clipping={}",
            state.telemetry.peak_dbfs, state.telemetry.rms_dbfs, state.telemetry.clipping);

        let mem = music_fusion::memory::SystemMemory::detect_live();
        let panel = music_fusion::iteration_panel::IterationFusionPanel::new_auto_configured(&mem).await;
        let resp = panel.assist_song_iteration(&state, None).await
            .unwrap_or_else(|_| panel.offline_fallback_assist(&state));

        println!("🧐 Critique: {}", resp.critique);
        println!("💡 Suggested Next Iterations (Model: {}):", resp.copilot_model);
        for (i, sug) in resp.suggested_next_iterations.iter().enumerate() {
            println!("   {}. {}", i + 1, sug);
        }
        return Ok(());
    }

    // 6. Single stem regeneration
    if let Some(stem) = &args.regenerate_stem {
        let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
        let version = args.version.unwrap_or(1);
        println!("🎛️ Regenerating stem '{}' for song {} (v{})...", stem, song_id, version);
        let result = runner.regenerate_stem(song_id, version, stem).await?;
        println!("✨ Completed stem regeneration: song_id={}, version={}", result.song_id, result.version);
        return Ok(());
    }

    // 7. Track mastering
    if args.master {
        let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
        let version = args.version.unwrap_or(1);
        println!("🎚️ Mastering track for song {} (v{})...", song_id, version);
        let result = runner.master_track(song_id, version).await?;
        println!("✨ Completed mastering: song_id={}, version={}", result.song_id, result.version);
        return Ok(());
    }

    // 8. Default mode or --serve: Launch Tokio HTTP REST Server + ChatGPT-like Web Studio UI
    // Dynamic port selection: scans 5050..=5070 (or args.port..=(args.port + 20)) and falls back to OS-assigned port if busy
    let end_port = args.port.saturating_add(20);
    let bound_port = rest_server::start_rest_server_with_host(
        runner,
        args.storage_dir.clone(),
        &args.host,
        args.port,
        end_port,
        args.ollama_host.clone(),
    ).await?;

    let browser_host = if args.host == "0.0.0.0" || args.host == "::" {
        "127.0.0.1"
    } else {
        args.host.as_str()
    };
    let url = format!("http://{}:{}/", browser_host, bound_port);
    if bound_port != args.port {
        println!(
            "⚠️  Port {} was in use — automatically switched to free port {}",
            args.port, bound_port
        );
    }
    println!("🌐 Web Studio server running at {}", url);
    println!("🎵 Press Ctrl+C to terminate server.");

    let suppress_browser = browser::should_suppress_browser(args.no_browser, args.mcp);
    browser::spawn_browser_delayed(url, 500, suppress_browser);

    tokio::signal::ctrl_c().await?;
    println!("\n👋 Web Studio server shutting down gracefully.");
    Ok(())
}
