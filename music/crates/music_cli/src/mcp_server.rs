use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use music_pipeline::runner::PipelineRunner;
use music_core::genre::GenreTaxonomy;
use music_core::schema::{StemChannelStrip, StemMixConfig};
use music_core::state::{DecisionMode, LyricsMode, MusicProductionState};
use music_fusion::decision_model::{DecisionRequest, DecisionSchema, NimbleDecisionEngine};
use music_fusion::hf_model_discovery::HfModelDiscoveryEngine;
use music_fusion::lyricist_studio::{structure_user_lyrics_verbatim, write_lyrics_from_prompt};
use music_fusion::memory::SystemMemory;
use music_fusion::ollama_client::OllamaClient;

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
                        "lyrics": { "type": "string", "description": "Optional custom lyrics preserved verbatim" },
                        "lyrics_mode": { "type": "string", "enum": ["user_provided", "ai_generated", "auto"] },
                        "lyrics_topic": { "type": "string", "description": "Story or topic for AI lyric writing" },
                        "model": { "type": "string", "description": "Open-weight synthesis model ID or alias (e.g. ACE-Step/acestep-v15-turbo)" },
                        "commercial_only": { "type": "boolean" },
                        "decision_model": { "type": "string" },
                        "decision_mode": { "type": "string", "enum": ["auto", "nimble", "ollama_single_pass", "fast"] },
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
            },
            {
                "name": "music_write_lyrics",
                "description": "Dual-mode studio lyricist: formats user-supplied lyrics 100% verbatim OR writes original structured lyrics with local AI.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "mode": { "type": "string", "enum": ["user_provided", "ai_generated", "auto"] },
                        "lyrics": { "type": "string", "description": "User-supplied lyrics when mode=user_provided" },
                        "topic": { "type": "string", "description": "Theme/story prompt when mode=ai_generated" },
                        "style_prompt": { "type": "string", "description": "Musical style / genre context" }
                    }
                }
            },
            {
                "name": "music_mix_stems",
                "description": "Bounces a non-destructive 48 kHz DAW mix version v(N+1) from existing stems with faders, Solo/Mute, Pan, 3-band EQ, and Reverb.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "song_id": { "type": "string" },
                        "version": { "type": "integer" },
                        "channels": { "type": "object", "description": "Map of canonical stem_id -> StemChannelStrip fields" },
                        "sub_bass_boost_db": { "type": "number" },
                        "analog_tape_warmth": { "type": "number" },
                        "stereo_width": { "type": "number" }
                    },
                    "required": ["song_id", "version"]
                }
            },
            {
                "name": "music_decide_production",
                "description": "Evaluates a typed production decision using the local Bespoke-Nimble / single-pass / System-1 decision engine.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "gate": { "type": "string", "description": "gate1..gate5 or custom gate identifier" },
                        "question": { "type": "string" },
                        "options": { "type": "array", "items": { "type": "string" } },
                        "commercial_only": { "type": "boolean" },
                        "mode": { "type": "string", "enum": ["auto", "nimble", "ollama_single_pass", "fast"] },
                        "preferred_model": { "type": "string" }
                    },
                    "required": ["options"]
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
            if let Some(lm) = args.get("lyrics_mode").and_then(|v| v.as_str()) {
                state.lyrics_mode = LyricsMode::from_str_loose(lm);
            } else if !state.lyrics.trim().is_empty() {
                state.lyrics_mode = LyricsMode::UserProvided;
            }
            if let Some(topic) = args.get("lyrics_topic").and_then(|v| v.as_str()) {
                state.lyrics_topic = topic.to_string();
            }
            if let Some(m) = args.get("model").and_then(|v| v.as_str()) {
                let resolved = HfModelDiscoveryEngine::resolve_model_entry(m)
                    .map(|e| e.id)
                    .unwrap_or_else(|| m.trim().to_string());
                if !resolved.is_empty() {
                    state.selected_model = Some(resolved);
                }
            }
            state.commercial_only = args.get("commercial_only").and_then(|v| v.as_bool()).unwrap_or(false);
            if let Some(dm) = args.get("decision_model").and_then(|v| v.as_str()) {
                state.decision_model = Some(dm.to_string());
            }
            if let Some(dmode) = args.get("decision_mode").and_then(|v| v.as_str()) {
                state.decision_mode = DecisionMode::from_str_loose(dmode);
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
                "selected_model": result.selected_model,
                "decision_traces": result.decision_traces,
                "final_master": master_path,
                "telemetry": result.telemetry,
                "judge_score": result.judge_score,
                "quality_passed": result.quality_passed,
                "content": [{
                    "type": "text",
                    "text": format!("Generated hit song '{}' (v{}) using {:?} with master at {}", result.song_id, result.version, result.selected_model, master_path)
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
        "music_write_lyrics" => {
            let raw_lyrics = args.get("lyrics").and_then(|v| v.as_str()).unwrap_or("");
            let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("");
            let style_prompt = args
                .get("style_prompt")
                .or_else(|| args.get("prompt"))
                .and_then(|v| v.as_str())
                .unwrap_or("Modern studio song");
            let mode = args
                .get("mode")
                .and_then(|v| v.as_str())
                .map(LyricsMode::from_str_loose)
                .unwrap_or(if raw_lyrics.trim().is_empty() {
                    LyricsMode::AiGenerated
                } else {
                    LyricsMode::UserProvided
                });

            let res = if mode == LyricsMode::UserProvided
                || (mode == LyricsMode::Auto && !raw_lyrics.trim().is_empty())
            {
                structure_user_lyrics_verbatim(raw_lyrics)
            } else {
                let mem = SystemMemory::detect_live();
                let client = OllamaClient::from_env();
                write_lyrics_from_prompt(&client, &mem, topic, style_prompt, None).await
            };

            Ok(json!({
                "status": "completed",
                "mode_used": res.mode_used,
                "model_used": res.model_used,
                "title": res.title,
                "preserved_verbatim": res.preserved_verbatim,
                "formatted_lyrics": res.formatted_lyrics,
                "sections": res.sections,
                "content": [{ "type": "text", "text": res.formatted_lyrics }]
            }))
        }
        "music_mix_stems" => {
            let song_id = args.get("song_id").and_then(|v| v.as_str()).unwrap_or("");
            let version = args.get("version").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(1);
            let prev = runner.checkpointer().load_checkpoint(song_id, version).await?;
            let mut mix = prev
                .blueprint
                .as_ref()
                .map(|b| b.stem_mix.clone())
                .unwrap_or_else(StemMixConfig::default);

            if let Some(ch_obj) = args.get("channels").and_then(|v| v.as_object()) {
                for (stem_id, val) in ch_obj {
                    if let Ok(strip) = serde_json::from_value::<StemChannelStrip>(val.clone()) {
                        mix.set_channel(stem_id, strip);
                    }
                }
            }
            if let Some(v) = args.get("sub_bass_boost_db").and_then(|v| v.as_f64()) {
                mix.sub_bass_boost_db = v as f32;
            }
            if let Some(v) = args.get("analog_tape_warmth").and_then(|v| v.as_f64()) {
                mix.analog_tape_warmth = v as f32;
            }
            if let Some(v) = args.get("stereo_width").and_then(|v| v.as_f64()) {
                mix.stereo_width = v as f32;
            }

            let out_state = runner.mix_and_master_song(song_id, version, mix).await?;
            let master_path = out_state
                .stem_paths
                .get("final_master")
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            Ok(json!({
                "status": "completed",
                "song_id": out_state.song_id,
                "version": out_state.version,
                "final_master": master_path,
                "telemetry": out_state.telemetry,
                "content": [{
                    "type": "text",
                    "text": format!("Bounced DAW mix v{} for '{}' -> {}", out_state.version, out_state.song_id, master_path)
                }]
            }))
        }
        "music_decide_production" => {
            let gate = args.get("gate").and_then(|v| v.as_str()).unwrap_or("custom_gate");
            let question = args
                .get("question")
                .and_then(|v| v.as_str())
                .unwrap_or("Select best studio production option.");
            let options: Vec<String> = args
                .get("options")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            if options.is_empty() {
                anyhow::bail!("music_decide_production requires non-empty options array");
            }
            let mode = args
                .get("mode")
                .and_then(|v| v.as_str())
                .map(DecisionMode::from_str_loose)
                .unwrap_or(DecisionMode::Auto);
            let commercial_only = args.get("commercial_only").and_then(|v| v.as_bool()).unwrap_or(false);
            let pref = args.get("preferred_model").and_then(|v| v.as_str()).map(str::to_string);

            let mut req = DecisionRequest::new(gate, DecisionSchema::Categorical { options }, question);
            req.mode = mode;
            req.commercial_only = commercial_only;
            req.preferred_model = pref;

            let engine = NimbleDecisionEngine::default();
            let res = engine.decide(&req).await;
            let selected = res.selected_choice.clone();
            let conf = res.confidence;

            Ok(json!({
                "status": "completed",
                "decision": res,
                "content": [{
                    "type": "text",
                    "text": format!("Nimble gate '{}' selected '{}' ({:.1}% confidence)", gate, selected, conf * 100.0)
                }]
            }))
        }
        other => anyhow::bail!("Unknown tool: {}", other),
    }
}
