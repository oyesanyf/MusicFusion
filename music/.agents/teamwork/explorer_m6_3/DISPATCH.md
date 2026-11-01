# DISPATCH: Playback Resilience & Dynamic Port Explorer (explorer_m6_3)

## Objective
Investigate playback fallback logic, 404 prevention when typing "REMOVE THE SCRATCH", dynamic port assignment when 5050 is in use, download endpoints, and REST server daemon startup.

## Scope & Files
- `d:\harfile\music\crates\music_cli\src\rest_server.rs`
- `d:\harfile\music\crates\music_cli\src\main.rs`
- `d:\harfile\music\crates\music_cli\src\cli_args.rs`
- `d:\harfile\music\crates\music_cli\webui\app.js` (`executeIteration()`, `onIterationSuccess()`)
- Reference request in `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md`

## Requirements
1. Analyze why typing "REMOVE THE SCRATCH" broke the song: in `executeIteration()` / `onIterationSuccess()`, when the user typed a 3rd/4th message (`nextVer = 3, 4`), if `/api/songs/iterate` returned an error or network error, `onIterationSuccess` unconditionally overwrote `audioUrls.master` with `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/final_master` which did not exist on disk (HTTP 404), breaking playback. Formulate exact fix: use `data.audio_urls` when returned, and if offline/fallback or version doesn't exist on disk, fall back to the last valid existing version's audio URL (`v1` or `v2`) so playback NEVER breaks.
2. Analyze Dynamic Port Assignment in `crates/music_cli/src/rest_server.rs` and `main.rs`: if port 5050 is in use, dynamically assign an available port (e.g. 5050..5070) and report the bound URL.
3. Analyze local file loading and download endpoints (`/api/songs/:id/versions/:ver/download` / `/api/songs/:id/download`) to ensure zero errors when downloading or playing audio files locally.

## Output
Write comprehensive handoff report to `d:\harfile\music\.agents\teamwork\explorer_m6_3\handoff.md`.


## 2026-11-01T12:51:31Z
You are explorer_m6_3 (Playback Resilience & Dynamic Port Explorer).
Your working directory is `d:\harfile\music\.agents\teamwork\explorer_m6_3\`.
Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` before starting work.
Read `d:\harfile\music\.agents\teamwork\explorer_m6_3\DISPATCH.md` for your full task description.

Task:
1. Analyze why typing "REMOVE THE SCRATCH" broke the song: in `executeIteration()` / `onIterationSuccess()`, when the user typed a 3rd/4th message (`nextVer = 3, 4`), if `/api/songs/iterate` returned an error or network error, `onIterationSuccess` unconditionally overwrote `audioUrls.master` with `/api/songs/${State.activeSongId}/versions/${nextVer}/audio/final_master` which did not exist on disk (HTTP 404), breaking playback. Formulate exact fix: use `data.audio_urls` when returned, and if offline/fallback or version doesn't exist on disk, fall back to the last valid existing version's audio URL (`v1` or `v2`) so playback NEVER breaks.
2. Analyze Dynamic Port Assignment in `crates/music_cli/src/rest_server.rs` and `main.rs`: if port 5050 is in use, dynamically assign an available port (e.g. 5050..5070) and report the bound URL.
3. Analyze local file loading and download endpoints (`/api/songs/:id/versions/:ver/download` / `/api/songs/:id/download`) to ensure zero errors when downloading or playing audio files locally.

Write a complete, structured handoff report to `d:\harfile\music\.agents\teamwork\explorer_m6_3\handoff.md`.
When finished, notify the orchestrator with send_message.
