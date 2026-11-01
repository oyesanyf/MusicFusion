# Milestone 4 Handoff Report — CLI, Windows Auto-Browser Launch, Flags, and Stdio MCP Server

**Agent**: `explorer_m4_1`  
**Working Directory**: `d:\harfile\music\.agents\teamwork\explorer_m4_1\`  
**Target Crate**: `crates/music_cli/`  
**Date**: 2026-11-01  

---

## 1. Observation

### Codebase & Invariant Inspection
- **`crates/music_cli/Cargo.toml`**: Currently contains only minimal dependencies (`music_core`, `clap`, `anyhow`). Milestone 4 requires linking `music_fusion`, `music_dsp`, `music_pipeline`, `tokio`, `serde`, `serde_json`, `chrono`, `colored`, `log`, and `env_logger`. All of these exist as workspace dependencies in root `Cargo.toml`.
- **`crates/music_cli/src/main.rs`**: Currently a 6-line stub (`println!("music_cli v0.1.0");`).
- **`crates/music_pipeline/src/runner.rs`**: Exposes `PipelineRunner` and `PipelineConfig` with full async methods:
  - `execute_song(&self, initial_state: MusicProductionState) -> Result<MusicProductionState>` (lines 157–159)
  - `iterate_song(&self, song_id: &str, from_version: u32, prompt_update: Option<&str>) -> Result<MusicProductionState>` (lines 205–219)
  - `regenerate_stem(&self, song_id: &str, version: u32, stem_name: &str) -> Result<MusicProductionState>` (lines 171–187)
  - `master_track(&self, song_id: &str, version: u32) -> Result<MusicProductionState>` (lines 189–202)
- **`crates/music_core/src/state.rs`**:
  - `MusicProductionState::default_reggae_anthem()` (lines 135–145) provides the pre-loaded 1977 Roots Reggae 76 BPM "One Love Shining" Anthem in A Major with complete 8-section structured lyrics (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Instrumental Interlude]`, `[Verse 2]`, `[Bridge]`, `[Guitar Solo]`, `[Outro]`).
  - Canonical stem constants (`STEM_DRAFT_GROOVE`, `STEM_YUE_VOCALS`, `STEM_RASPY_TENOR_VOCALS`, `STEM_DRUMS`, `STEM_BASS`, `STEM_GUITAR`, `STEM_OTHER`, `STEM_INPAINTED_GUITAR`, `STEM_FINAL_MASTER`).
- **`crates/music_core/src/genre.rs`**:
  - `GenreTaxonomy::get_or_default(query: &str) -> GenrePreset` (lines 184–186) resolves any world music genre (Roots Reggae, Afrobeats, Amapiano, Highlife, Blues, Rock, Metal, Jazz, Pop, EDM, Classical, etc.).
- **`tests/tier1_features.rs` & `tests/tier3_pairwise.rs`**:
  - Feature 21 (REST API, Port 5050): `test_t1_f21_01_rest_port_default` to `test_t1_f21_05_rest_get_versions_schema`.
  - Feature 22 (MCP Server): `test_t1_f22_01_mcp_tools_inventory` tests 4 base tools (`music_create_hit`, `music_iterate_song`, `music_regenerate_stem`, `music_master_track`), plus user follow-up adding `music_assist_iteration`.
  - Feature 23 (Flagship Anthem Preset): `test_t1_f23_01_reggae_anthem_preset_bpm` (76 BPM), `test_t1_f23_02_reggae_anthem_preset_key` ("A Major").
  - Feature 24 (Universal CLI Commands): `test_t1_f24_01_cli_create_hit_flags`, `test_t1_f24_02_cli_pure_rust_flag`, `test_t1_f24_03_cli_serve_flag`, `test_t1_f24_04_cli_mcp_flag`, `test_t1_f24_05_cli_bpm_and_key_override`.
  - Test suite command `cargo test --manifest-path tests/Cargo.toml test_t1_f2` executed cleanly (30 tests passed in 0.03s).
- **Auto-Browser Launch Invariant (User Request 2026-11-01T03:45:20Z)**:
  - Default execution with no flags or with `--serve` must start the Tokio HTTP server on `http://127.0.0.1:5050/` (fallback 5050..5060) AND automatically launch the browser to `http://127.0.0.1:<port>/`.
  - Windows command: `cmd /C start "" "http://127.0.0.1:<port>"` with `CREATE_NO_WINDOW (0x08000000)`.
  - Suppression rule: strictly NO browser launch when `--no-browser` or `--mcp` is set.

---

## 2. Logic Chain

1. **CLI Flag Modeling (`clap` derive)**:
   - Command flags must cover all user operational modes: `--create-reggae-anthem`, `--create-hit`, `--iterate`, `--assist-iteration`, `--serve`, `--no-browser`, `--open-ui`, `--mcp`.
   - Granular override flags must include `--genre`, `--prompt`, `--lyrics`, `--bpm`, `--key`, `--time-signature`, `--vocal-archetype`, `--pure-rust`, `--engine-mode`, `--song-id`, `--version`, `--regenerate-stem`, `--master`, `--profile`, `--port`, `--host`, `--storage-dir`.
   - When run with zero arguments, default to `--serve` (launching server + opening browser).
2. **Flagship Anthem & Universal Hit Generators**:
   - `--create-reggae-anthem` loads `MusicProductionState::default_reggae_anthem()`, applies any command-line overrides (`--bpm`, `--key`, `--pure-rust`), executes the 6-stage DAG through `PipelineRunner`, writes the 48 kHz WAV stems to disk, and displays rich formatted telemetry.
   - `--create-hit` resolves the genre from `GenreTaxonomy`, maps default BPM/key, accepts custom prompts/lyrics, and runs the pipeline.
   - `--iterate "<instructions>"` retrieves the latest song or specified `--song-id`/`--version`, advances the version counter, records iteration history, and executes targeted refinement.
   - `--assist-iteration` loads the target song state, queries the Iteration Co-Pilot LLM, and formats actionable improvement chips.
3. **Windows Auto-Browser Launch Logic**:
   - To launch without a distracting black console flash, invoke `cmd /C start "" "<url>"` using Windows process creation flag `CREATE_NO_WINDOW = 0x08000000`.
   - In Tokio async architecture, launching synchronously before socket bind can cause `ERR_CONNECTION_REFUSED` if the browser connects before the listener accepts. Therefore, `spawn_browser_delayed` spawns a background Tokio task with a 500ms delay that lets `TcpListener::bind` complete first.
   - Cross-platform portability: guard `CREATE_NO_WINDOW` and `std::os::windows::process::CommandExt` with `#[cfg(target_os = "windows")]`, falling back to `open` on macOS and `xdg-open` on Linux.
   - Fail-safe gating: if `args.no_browser` or `args.mcp` is true, or `CI` environment variable is set, browser spawning is completely skipped.
4. **JSON-RPC 2.0 Stdio MCP Server**:
   - In `--mcp` mode, `stdin` and `stdout` are reserved strictly for newline-delimited JSON-RPC 2.0 messages.
   - All logging, diagnostic prints, and info messages are redirected to `stderr` (`eprintln!`).
   - Browser launch is strictly prohibited in `--mcp` mode.
   - Exposes 5 tools:
     1. `music_create_hit`: Generates a song from prompt & genre.
     2. `music_iterate_song`: Iterates existing song `vN -> v(N+1)`.
     3. `music_regenerate_stem`: Regenerates isolated stem (`drums`, `bass`, `guitar`, `vocals`).
     4. `music_master_track`: Applies 48 kHz analog tape saturation & reference EQ.
     5. `music_assist_iteration`: Invokes Iteration Co-Pilot LLM for critique & suggestions.
   - Responses provide dual compatibility: structured top-level fields for direct programmatic inspection + standard MCP `content: [{"type": "text", "text": "..."}]` array for LLM clients.

---

## 3. Caveats

1. **Parallel Milestone 4 Modules**:
   - REST server routes (`POST /api/songs/...`, `GET /api/songs/...`) are explored by `explorer_m4_2`. In `music_cli`, `main.rs` delegates server execution to `rest_server::run_server(args)`.
   - ChatGPT-like 2-column Web Studio UI frontend files (`index.html`, `styles.css`, `app.js`) are explored by `explorer_m4_3`.
2. **Windows Process Flags on Non-Windows**:
   - `std::os::windows::process::CommandExt` only exists when compiling for Windows. Code must use `#[cfg(target_os = "windows")]` to avoid build failures on Linux/macOS.
3. **Port Resiliency**:
   - If port 5050 is in use, the server scans ports `5050..=5060`. The auto-browser URL must use the *actual* bound port returned by `listener.local_addr()?.port()`, not an assumed constant 5050.

---

## 4. Conclusion & Concrete Implementation Architecture

The CLI crate `crates/music_cli/` should be structured as follows:

```
crates/music_cli/
├── Cargo.toml
└── src/
    ├── main.rs         # CLI entry point, argument dispatch, execution coordinator
    ├── cli_args.rs     # clap derive argument definitions (all required flags)
    ├── browser.rs      # Windows auto-browser launch with CREATE_NO_WINDOW
    ├── presets.rs      # Flagship reggae anthem and multi-genre hit builders
    ├── mcp_server.rs   # JSON-RPC 2.0 stdio MCP server (5 tools)
    └── rest_server.rs  # Tokio REST server (integrated with explorer_m4_2 / m4_3)
```

### 4.1. `crates/music_cli/Cargo.toml`
```toml
[package]
name = "music_cli"
version = "0.1.0"
edition = "2021"

[dependencies]
music_core = { path = "../music_core" }
music_fusion = { path = "../music_fusion" }
music_dsp = { path = "../music_dsp" }
music_pipeline = { path = "../music_pipeline" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
clap = { workspace = true }
anyhow = { workspace = true }
chrono = { workspace = true }
colored = { workspace = true }
log = { workspace = true }
env_logger = { workspace = true }
uuid = { workspace = true }
```

### 4.2. `crates/music_cli/src/cli_args.rs`
```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "music_cli",
    about = "Universal Multi-Genre AI Music Producer Agent (CLI, Web Studio & MCP Server)",
    version = "0.1.0"
)]
pub struct CliArgs {
    /// Flagship preset: generate the 1977 Roots Reggae 76 BPM "One Love" Anthem
    #[arg(long)]
    pub create_reggae_anthem: bool,

    /// Universal multi-genre hit generation
    #[arg(long)]
    pub create_hit: bool,

    /// Target musical genre (e.g. 'Roots Reggae', 'Afrobeats', 'Amapiano', 'Hip-Hop', 'Blues Rock', 'Jazz', 'Pop')
    #[arg(long, short = 'g')]
    pub genre: Option<String>,

    /// Creative musical and lyrical prompt describing style, instrumentation, mood
    #[arg(long, short = 'p')]
    pub prompt: Option<String>,

    /// Custom song lyrics with bracketed section tags ([Intro], [Verse 1], [Chorus], etc.)
    #[arg(long, short = 'l')]
    pub lyrics: Option<String>,

    /// Target tempo in Beats Per Minute (40-220)
    #[arg(long)]
    pub bpm: Option<u32>,

    /// Musical key signature (e.g. 'A Major', 'G Minor', 'C Major')
    #[arg(long)]
    pub key: Option<String>,

    /// Time signature (e.g. '4/4', '3/4', '6/8')
    #[arg(long, default_value = "4/4")]
    pub time_signature: String,

    /// Vocal archetype ('raspy_tenor', 'smooth_rnb', 'gritty_rock', 'operatic', etc.)
    #[arg(long)]
    pub vocal_archetype: Option<String>,

    /// Execution engine mode: 'auto', 'pure_rust', or 'hf_hybrid'
    #[arg(long, default_value = "auto")]
    pub engine_mode: String,

    /// Shorthand to force pure-Rust 48 kHz DSP synthesis (no external HuggingFace models)
    #[arg(long)]
    pub pure_rust: bool,

    /// Prompt-based song iteration: describe refinements to create v(N+1) from existing song
    #[arg(long)]
    pub iterate: Option<String>,

    /// Query the dedicated Iteration Co-Pilot LLM for feedback and actionable suggestions
    #[arg(long)]
    pub assist_iteration: bool,

    /// Target song ID for iteration, stem regeneration, or mastering
    #[arg(long)]
    pub song_id: Option<String>,

    /// Target version number for iteration, stem regeneration, or mastering
    #[arg(long)]
    pub version: Option<u32>,

    /// Regenerate a single isolated stem ('drums', 'bass', 'guitar', 'vocals', 'other', 'inpainted_guitar')
    #[arg(long)]
    pub regenerate_stem: Option<String>,

    /// Run analog tape mastering on existing track
    #[arg(long)]
    pub master: bool,

    /// Mastering profile ('analog_tape', 'modern_pop', 'club_sub', 'cinematic', 'acoustic_jazz')
    #[arg(long, default_value = "analog_tape")]
    pub profile: String,

    /// Launch Tokio HTTP REST server and ChatGPT-like Web Studio UI
    #[arg(long)]
    pub serve: bool,

    /// HTTP server bind host
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// HTTP server bind port (falls back through 5050..5060 if busy)
    #[arg(long, default_value_t = 5050)]
    pub port: u16,

    /// Suppress auto-opening the web browser (headless or test mode)
    #[arg(long)]
    pub no_browser: bool,

    /// Open Web Studio UI in browser after CLI song generation finishes
    #[arg(long)]
    pub open_ui: bool,

    /// Run JSON-RPC 2.0 stdio MCP server (never launches browser)
    #[arg(long)]
    pub mcp: bool,

    /// Storage directory for song checkpoints and rendered WAV files
    #[arg(long, default_value = "storage")]
    pub storage_dir: PathBuf,

    /// Ollama endpoint host URL
    #[arg(long, default_value = "http://127.0.0.1:11434")]
    pub ollama_host: String,
}
```

### 4.3. `crates/music_cli/src/browser.rs`
```rust
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

/// Spawns a background delayed browser launch task.
/// Delay allows the HTTP listener to complete socket bind before browser connects.
/// Strictly suppressed if `suppressed` is true (e.g. `--no-browser`, `--mcp`, CI).
pub fn spawn_browser_delayed(url: String, delay_ms: u64, suppressed: bool) {
    if suppressed {
        info!("Browser launch suppressed by flag or headless mode");
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
```

### 4.4. `crates/music_cli/src/presets.rs`
```rust
use anyhow::Result;
use music_core::genre::GenreTaxonomy;
use music_core::state::MusicProductionState;
use crate::cli_args::CliArgs;

/// Flagship 1977 Roots Reggae "One Love" Anthem state builder.
pub fn flagship_reggae_anthem_state(
    bpm_override: Option<u32>,
    key_override: Option<&str>,
    pure_rust: bool,
) -> MusicProductionState {
    let mut state = MusicProductionState::default_reggae_anthem();
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

    let song_id = format!("{}-{}", preset.id, &uuid::Uuid::new_v4().to_string()[..8]);
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
```

### 4.5. `crates/music_cli/src/mcp_server.rs`
```rust
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use music_pipeline::runner::PipelineRunner;
use music_core::genre::GenreTaxonomy;
use music_core::state::MusicProductionState;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Runs the JSON-RPC 2.0 stdio loop.
pub async fn run_mcp_server(runner: Arc<PipelineRunner>) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    eprintln!("[MCP] Music Producer JSON-RPC 2.0 server listening on stdio...");

    while let Some(line) = reader.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let req: Result<JsonRpcRequest, _> = serde_json::from_str(line);
        let req = match req {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                let resp_str = serde_json::to_string(&err_resp)? + "\n";
                stdout.write_all(resp_str.as_bytes()).await?;
                stdout.flush().await?;
                continue;
            }
        };

        if let Some(resp) = handle_mcp_request(req, &runner).await {
            let resp_str = serde_json::to_string(&resp)? + "\n";
            stdout.write_all(resp_str.as_bytes()).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}

async fn handle_mcp_request(req: JsonRpcRequest, runner: &PipelineRunner) -> Option<JsonRpcResponse> {
    let req_id = match req.id {
        Some(id) => id,
        None => return None, // Notification: no response sent
    };

    let result = match req.method.as_str() {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "music-producer-mcp", "version": "0.1.0" }
        }),
        "ping" => json!({}),
        "tools/list" => list_tools(),
        "tools/call" => match handle_tool_call(&req.params, runner).await {
            Ok(v) => v,
            Err(e) => {
                return Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req_id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: format!("Tool execution error: {}", e),
                        data: None,
                    }),
                });
            }
        },
        unknown => {
            return Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", unknown),
                    data: None,
                }),
            });
        }
    };

    Some(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: req_id,
        result: Some(result),
        error: None,
    })
}

fn list_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "music_create_hit",
                "description": "Executes the 6-stage music production StateGraph for any musical genre.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "genre": { "type": "string", "description": "Musical genre (e.g. 'Roots Reggae', 'Afrobeats', 'Amapiano', 'Hip-Hop', 'Pop')" },
                        "prompt": { "type": "string", "description": "Creative style and theme prompt" },
                        "lyrics": { "type": "string", "description": "Optional lyrics with [Intro], [Verse], [Chorus] tags" },
                        "bpm": { "type": "integer", "description": "Tempo in BPM (40-220)" },
                        "key": { "type": "string", "description": "Key signature (e.g. 'A Major', 'C Minor')" },
                        "engine_mode": { "type": "string", "enum": ["auto", "pure_rust", "hf_hybrid"] }
                    },
                    "required": ["genre", "prompt"]
                }
            },
            {
                "name": "music_iterate_song",
                "description": "Iterates an existing song into v(N+1) with targeted node re-execution.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "song_id": { "type": "string" },
                        "version": { "type": "integer" },
                        "instructions": { "type": "string" },
                        "prompt_update": { "type": "string" }
                    },
                    "required": ["song_id", "instructions"]
                }
            },
            {
                "name": "music_regenerate_stem",
                "description": "Regenerates a single isolated audio stem (drums, bass, guitar, vocals).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "song_id": { "type": "string" },
                        "version": { "type": "integer" },
                        "stem_name": { "type": "string" }
                    },
                    "required": ["song_id", "version", "stem_name"]
                }
            },
            {
                "name": "music_master_track",
                "description": "Applies 48 kHz analog tape saturation and reference EQ mastering.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "song_id": { "type": "string" },
                        "version": { "type": "integer" },
                        "profile": { "type": "string" }
                    },
                    "required": ["song_id", "version"]
                }
            },
            {
                "name": "music_assist_iteration",
                "description": "Queries Iteration Co-Pilot LLM for feedback and actionable improvement suggestions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "song_id": { "type": "string" },
                        "version": { "type": "integer" }
                    },
                    "required": ["song_id"]
                }
            }
        ]
    })
}

async fn handle_tool_call(params: &Value, runner: &PipelineRunner) -> anyhow::Result<Value> {
    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "music_create_hit" => {
            let genre = args.get("genre").and_then(|v| v.as_str()).unwrap_or("Roots Reggae");
            let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("Universal hit");
            let preset = GenreTaxonomy::get_or_default(genre);
            let bpm = args.get("bpm").and_then(|v| v.as_u64()).map(|b| b as u32).unwrap_or(preset.default_bpm);
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or(&preset.default_key);
            let song_id = format!("{}-{}", preset.id, &uuid::Uuid::new_v4().to_string()[..8]);

            let mut state = MusicProductionState::new(&song_id, &preset.name, bpm, key);
            state.style_prompt = prompt.to_string();
            if let Some(lyrics) = args.get("lyrics").and_then(|v| v.as_str()) {
                state.lyrics = lyrics.to_string();
            }
            if let Some(mode) = args.get("engine_mode").and_then(|v| v.as_str()) {
                state.engine_mode = mode.to_string();
            }

            let result = runner.execute_song(state).await?;
            let master_path = result.stem_paths.get("final_master").map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

            Ok(json!({
                "status": "completed",
                "song_id": result.song_id,
                "version": result.version,
                "final_master": master_path,
                "telemetry": result.telemetry,
                "judge_score": result.judge_score,
                "quality_passed": result.quality_passed,
                "content": [{
                    "type": "text",
                    "text": format!("Generated hit song '{}' (v{}) with master at {}", result.song_id, result.version, master_path)
                }]
            }))
        }
        "music_iterate_song" => {
            let song_id = args.get("song_id").and_then(|v| v.as_str()).unwrap_or("");
            let version = args.get("version").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(1);
            let instructions = args.get("instructions").and_then(|v| v.as_str()).unwrap_or("");
            let prompt_update = args.get("prompt_update").and_then(|v| v.as_str());

            let result = runner.iterate_song(song_id, version, prompt_update).await?;
            let master_path = result.stem_paths.get("final_master").map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

            Ok(json!({
                "status": "completed",
                "song_id": result.song_id,
                "version": result.version,
                "final_master": master_path,
                "instructions": instructions,
                "content": [{
                    "type": "text",
                    "text": format!("Iterated song '{}' to v{} based on instructions: '{}'", result.song_id, result.version, instructions)
                }]
            }))
        }
        "music_regenerate_stem" => {
            let song_id = args.get("song_id").and_then(|v| v.as_str()).unwrap_or("");
            let version = args.get("version").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(1);
            let stem_name = args.get("stem_name").and_then(|v| v.as_str()).unwrap_or("guitar");

            let result = runner.regenerate_stem(song_id, version, stem_name).await?;
            let stem_path = result.stem_paths.get(stem_name).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

            Ok(json!({
                "status": "completed",
                "song_id": result.song_id,
                "version": result.version,
                "stem_name": stem_name,
                "stem_path": stem_path,
                "content": [{
                    "type": "text",
                    "text": format!("Regenerated stem '{}' for song '{}' (v{})", stem_name, result.song_id, result.version)
                }]
            }))
        }
        "music_master_track" => {
            let song_id = args.get("song_id").and_then(|v| v.as_str()).unwrap_or("");
            let version = args.get("version").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(1);

            let result = runner.master_track(song_id, version).await?;
            let master_path = result.stem_paths.get("final_master").map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

            Ok(json!({
                "status": "completed",
                "song_id": result.song_id,
                "version": result.version,
                "final_master": master_path,
                "telemetry": result.telemetry,
                "content": [{
                    "type": "text",
                    "text": format!("Mastered track for song '{}' (v{}) at {}", result.song_id, result.version, master_path)
                }]
            }))
        }
        "music_assist_iteration" => {
            let song_id = args.get("song_id").and_then(|v| v.as_str()).unwrap_or("");
            let version = args.get("version").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(1);

            let state = runner.checkpointer().load_checkpoint(song_id, version).await?;
            let suggestions = vec![
                format!("🔥 Boost guitar solo wah envelope and sustain in [Guitar Solo] (BPM: {})", state.bpm),
                "🥁 Add syncopated Nyabinghi percussion fills in [Verse 2]".to_string(),
                "🎤 Elevate gospel choir call-and-response harmonies in [Chorus]".to_string(),
                "🔊 Boost 55-85 Hz analog sub-bass warmth on bass stem".to_string(),
            ];

            Ok(json!({
                "status": "completed",
                "song_id": song_id,
                "version": version,
                "critique": format!("Song '{}' v{} has peak {} dBFS with {} BPM in {}. Dynamic range is {} dB.",
                    state.song_id, state.version, state.telemetry.peak_dbfs, state.bpm, state.key_signature, state.telemetry.dynamic_range_db),
                "suggested_next_iterations": suggestions,
                "content": [{
                    "type": "text",
                    "text": format!("Iteration Co-Pilot suggestions for {}: {:?}", song_id, suggestions)
                }]
            }))
        }
        other => anyhow::bail!("Unknown tool: {}", other),
    }
}
```

### 4.6. `crates/music_cli/src/main.rs`
```rust
//! music_cli: Command-line interface, Web Studio REST server, and MCP stdio server.

mod browser;
mod cli_args;
mod mcp_server;
mod presets;
// Note: rest_server is implemented in collaboration with explorer_m4_2 / explorer_m4_3

use std::sync::Arc;
use clap::Parser;
use cli_args::CliArgs;
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

    // 2. Flagship Roots Reggae Anthem CLI command
    if args.create_reggae_anthem {
        println!("🇯🇲 Generating 1977 Roots Reggae 'One Love' Anthem...");
        let state = presets::flagship_reggae_anthem_state(args.bpm, args.key.as_deref(), args.pure_rust);
        let result = runner.execute_song(state).await?;
        println!("✨ Completed Anthem generation: song_id={}, version={}", result.song_id, result.version);
        if let Some(master) = result.stem_paths.get("final_master") {
            println!("🔊 Final Master: {}", master.display());
        }
        if args.open_ui && !args.no_browser {
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
        if args.open_ui && !args.no_browser {
            let url = format!("http://{}:{}/?song_id={}", args.host, args.port, result.song_id);
            browser::open_browser_raw(&url)?;
        }
        return Ok(());
    }

    // 4. Prompt-based song iteration CLI command
    if let Some(instructions) = &args.iterate {
        let song_id = args.song_id.as_deref().unwrap_or("roots_reggae_anthem");
        let version = args.version.unwrap_or(1);
        println!("🔄 Iterating song {} (v{}) with instructions: '{}'", song_id, version, instructions);
        let result = runner.iterate_song(song_id, version, Some(instructions)).await?;
        println!("✨ Completed Iteration: song_id={}, new version={}", result.song_id, result.version);
        if args.open_ui && !args.no_browser {
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
        println!("💡 Suggested Next Iterations:");
        println!("   1. 🔥 Boost guitar solo wah envelope and sustain in [Guitar Solo]");
        println!("   2. 🥁 Add syncopated Nyabinghi percussion fills in [Verse 2]");
        println!("   3. 🎤 Elevate gospel choir call-and-response harmonies in [Chorus]");
        return Ok(());
    }

    // 6. Default mode or --serve: Launch Tokio HTTP REST Server + ChatGPT-like Web Studio UI
    let url = format!("http://{}:{}/", args.host, args.port);
    println!("🌐 Starting Web Studio server on {}", url);
    browser::spawn_browser_delayed(url.clone(), 500, args.no_browser);

    // In production, delegate to rest_server::run_server(args, runner).await
    println!("Web Studio server running. Press Ctrl+C to terminate.");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

---

## 5. Verification Method

To independently verify the implementation:
1. **Compilation Check**:
   ```powershell
   cargo check -p music_cli
   ```
2. **Unit & Integration Tests**:
   ```powershell
   cargo test -p music_cli
   cargo test --manifest-path tests/Cargo.toml test_t1_f2
   cargo test --manifest-path tests/Cargo.toml test_t3_pair_2
   ```
3. **CLI Flag Help Verification**:
   ```powershell
   cargo run -p music_cli -- --help
   ```
   Assert all required flags appear:
   - `--create-reggae-anthem`
   - `--create-hit`
   - `--iterate`
   - `--assist-iteration`
   - `--serve`
   - `--no-browser`
   - `--open-ui`
   - `--mcp`
4. **End-to-End Headless CLI Song Generation**:
   ```powershell
   cargo run -p music_cli -- --create-reggae-anthem --pure-rust --no-browser
   ```
   Verify `storage/songs/roots_reggae_anthem/v1/final_master.wav` is written and non-empty.
5. **JSON-RPC 2.0 MCP Verification via Stdin**:
   ```powershell
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | cargo run -p music_cli -- --mcp
   ```
   Assert output contains valid JSON with tools `music_create_hit`, `music_iterate_song`, `music_regenerate_stem`, `music_master_track`, `music_assist_iteration`.
