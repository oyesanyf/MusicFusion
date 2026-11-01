# DISPATCH — explorer_m4_1

## Assignment
Investigate CLI architecture, flags, auto-browser launch, and stdio MCP server in `crates/music_cli/`:
1. Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` and `d:\harfile\music\PROJECT.md`.
2. Inspect existing `crates/music_cli/Cargo.toml` and `crates/music_cli/src/main.rs`.
3. Design CLI clap arguments:
   - `--create-reggae-anthem`: flagship 76 BPM "One Love" Roots Reggae Anthem preset.
   - `--create-hit`: universal multi-genre hit generation (`--genre <genre>`, `--prompt <prompt>`, `--bpm <bpm>`, `--key <key>`).
   - `--iterate "<changes>"`: prompt-based song iteration creating `v(N+1)`.
   - `--assist-iteration`: invoke Iteration Co-Pilot LLM on track.
   - `--serve`: launch Tokio HTTP server on `http://127.0.0.1:5050/` (fallback 5050..5060).
   - `--no-browser`: headless mode / test mode.
   - `--open-ui`: open UI after CLI generation.
   - `--mcp`: JSON-RPC 2.0 stdio server (`music_create_hit`, `music_iterate_song`, `music_regenerate_stem`, `music_master_track`, `music_assist_iteration`). Never launch browser in `--mcp` mode.
4. Design auto-browser launch logic on Windows:
   - Command: `cmd /C start "" "http://127.0.0.1:5050"` with `CREATE_NO_WINDOW (0x08000000)`.
   - Non-blocking async background spawn with 500ms delay allowing server socket bind.
5. Provide concrete implementation snippets, module structure, and verification commands in `d:\harfile\music\.agents\teamwork\explorer_m4_1\handoff.md`.
## 2026-11-01T04:24:26Z
You are explorer_m4_1 for Milestone 4 (CLI, Auto-Browser Launch, Flags, and Stdio MCP Server).
Your working directory is: d:\harfile\music\.agents\teamwork\explorer_m4_1\
Read d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md, d:\harfile\music\PROJECT.md, crates/music_cli/Cargo.toml, and your DISPATCH.md.
Investigate and design:
1. Complete clap CLI flags (--create-reggae-anthem, --create-hit, --iterate, --assist-iteration, --serve, --no-browser, --open-ui, --mcp).
2. Windows auto-browser launch logic on startup (cmd /C start "" "http://127.0.0.1:5050" with CREATE_NO_WINDOW).
3. JSON-RPC 2.0 stdio MCP server tools.
Write your handoff report to d:\harfile\music\.agents\teamwork\explorer_m4_1\handoff.md. Send a completion message via send_message to your caller.
