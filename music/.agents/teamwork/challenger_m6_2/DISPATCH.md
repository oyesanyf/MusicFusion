# DISPATCH: Milestone 6 Challenger 2 (challenger_m6_2)

## Objective
Empirically stress-test Playback 404 Resilience, Dynamic Port Assignment, Download Endpoints, and Disk Sample Songs & Multi-Section Lyrics.

## Scope & Verification
- `d:\harfile\music\crates\music_cli\`
- `d:\harfile\music\storage\songs\`
- Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` and `d:\harfile\music\.agents\teamwork\worker_m6\handoff.md`

## Adversarial Challenges
1. Test Playback Resilience: Simulate iteration failures and offline modes; assert that `findLastValidAudioUrls` always resolves to a valid existing master and stem set without throwing 404 errors.
2. Test Dynamic Port Assignment: Validate port binding across `5050..=5070` when ports are occupied, asserting fallback to ephemeral port `0` and clean URL reporting.
3. Test Download Endpoints: Query `/api/songs/:id/versions/:v/download` and `/api/songs/:id/download`; assert valid WAV headers, correct Content-Type, and Content-Disposition.
4. Scan and validate all 8 disk sample song version directories in `storage/songs/`:
   - Assert all 11 WAV files per version are valid 48 kHz stereo, unclipped ($\le 0.966$).
   - Assert `lyrics.txt`, `manifest.json`, and `state.json` contain complete multi-section English lyrics (`[Intro]`, `[Verse 1]`, `[Chorus]`, `[Verse 2]`, `[Guitar Solo]`, `[Bridge]`, `[Outro]`).
   - Assert valid JSON without UTF-8 BOM.
5. Render a clear verdict: **APPROVE** or **REJECT** in `handoff.md`.

## Output
Write complete 5-component report to `d:\harfile\music\.agents\teamwork\challenger_m6_2\handoff.md`.


## 2026-11-01T13:40:34Z
You are challenger_m6_2 (Milestone 6 Challenger 2: Playback Resilience, Dynamic Port & Disk Sample Songs/Lyrics).
Your working directory is `d:\harfile\music\.agents\teamwork\challenger_m6_2\`.
Read `d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md` before starting work.
Read `d:\harfile\music\.agents\teamwork\challenger_m6_2\DISPATCH.md` for your task description.
Read `d:\harfile\music\.agents\teamwork\worker_m6\handoff.md` for the worker's implementation evidence.

Tasks:
1. Empirically challenge Playback 404 resilience: verify that `findLastValidAudioUrls` in `app.js` resolves valid audio URLs under simulated errors and offline modes.
2. Empirically verify dynamic port assignment (5050..=5070 with ephemeral port fallback) in `rest_server.rs` and `main.rs`.
3. Empirically challenge download endpoints (`/api/songs/:id/versions/:v/download` and `/api/songs/:id/download`).
4. Empirically inspect and validate all 8 disk sample song versions in `storage/songs/`: assert 11 valid WAV files each, true-peak <= 0.966, complete multi-section English lyrics in `lyrics.txt`, `manifest.json`, and `state.json`, and valid JSON without BOM.
5. Write a 5-component handoff report with a clear verdict: APPROVE or REJECT to `d:\harfile\music\.agents\teamwork\challenger_m6_2\handoff.md`.
When finished, notify the orchestrator with send_message.
