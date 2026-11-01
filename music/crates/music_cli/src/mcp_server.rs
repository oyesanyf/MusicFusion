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

pub async fn handle_mcp_request(req: JsonRpcRequest, runner: &PipelineRunner) -> Option<JsonRpcResponse> {
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

pub fn list_tools() -> Value {
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

pub async fn handle_tool_call(params: &Value, runner: &PipelineRunner) -> anyhow::Result<Value> {
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
            let prompt_update = args.get("prompt_update").and_then(|v| v.as_str()).or(Some(instructions));

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
            let focus = args.get("focus").and_then(|v| v.as_str());

            let state = runner.checkpointer().load_checkpoint(song_id, version).await?;
            let mem = music_fusion::memory::SystemMemory::detect_live();
            let panel = music_fusion::iteration_panel::IterationFusionPanel::new_auto_configured(&mem).await;
            let resp = panel.assist_song_iteration(&state, focus).await
                .unwrap_or_else(|_| panel.offline_fallback_assist(&state));

            Ok(json!({
                "status": "completed",
                "song_id": song_id,
                "version": version,
                "copilot_model": resp.copilot_model,
                "critique": resp.critique,
                "suggested_next_iterations": resp.suggested_next_iterations,
                "content": [{
                    "type": "text",
                    "text": format!("Iteration Co-Pilot ({}) suggestions for {}: {:?}", resp.copilot_model, song_id, resp.suggested_next_iterations)
                }]
            }))
        }
        other => anyhow::bail!("Unknown tool: {}", other),
    }
}
