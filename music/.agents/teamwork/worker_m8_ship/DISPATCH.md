## 2026-11-02T16:28:30Z
You are worker_m8_ship in the orchestrator team for d:\harfile\music.
Working directory: d:\harfile\music\.agents\teamwork\worker_m8_ship\
Scope document: d:\harfile\music\.agents\teamwork\orchestrator\SCOPE.md
Authoritative request: d:\harfile\music\.agents\teamwork\ORIGINAL_REQUEST.md

Milestone 8 has passed all gates with unanimous approval from Reviewers, Challengers, and the Forensic Auditor.
Your tasks:
1. Check `git status -s`. Stage all remaining test files (such as `crates/music_fusion/tests/challenger_m8_empirical_tests.rs` and `crates/music_cli/tests/challenger_m8_2_empirical_tests.rs`), commit, and ship via solo PR:
   `powershell -ExecutionPolicy Bypass -File scripts/solo_pr_ship.ps1 -Title "test: add challenger empirical suites for milestone 8"`
2. Build `music_cli` in release mode (`cargo build --release -p music_cli`).
3. Ensure the live server is running and accessible on port 5050 so that the Web Studio UI is immediately usable by the user. If an existing `music_cli.exe --serve` process is running, restart it with the updated binary. Verify with a quick HTTP probe to `http://127.0.0.1:5050/api/health` or `GET /index.html`.
4. Verify `git status -s` shows a clean working tree.
5. Write your handoff report to `d:\harfile\music\.agents\teamwork\worker_m8_ship\handoff.md`.
Update your `progress.md`. When done, use send_message to report your results.

## 2026-11-02T16:52:58Z
**Context**: URGENT DESKTOP ENVIRONMENT INVARIANT
**Content**: The user observed a visible console window when `music_cli.exe` was launched.
MANDATORY CONSTRAINT: Never launch processes with visible console windows on the user's desktop. Any background server launcher must strictly use `-WindowStyle Hidden` (e.g. `Start-Process -FilePath "..." -ArgumentList "..." -WindowStyle Hidden` or background job) so no black terminal window pops up on the user's screen.
**Action**: Ensure any live server process launched uses `-WindowStyle Hidden`.
