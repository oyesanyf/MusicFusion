# Dispatch: explorer_m5_it3_3 (Milestone 5 Iteration 3 Live IDE Script & Verification Harness Explorer)

## Mission
Investigate and design the exact fixes for `scripts/verify_live_ide_execution.ps1` to resolve the fatal crash and verification harness dependency failure identified by `auditor_m5_remediation_1`, `reviewer_m5_remediation_2`, and `challenger_m5_remediation_2`.

## Mandatory Reading
- Original User Request: `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\ORIGINAL_REQUEST.md`
- Global Architecture & Scope: `d:\harfile\QuorumRAG\PROJECT.md`
- Dead Ends Log: `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\DEAD_ENDS.md`
- Auditor Full Evidence Report: `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m5_remediation_1\handoff.md`
- Reviewer 2 Full Report: `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m5_remediation_2\handoff.md`
- Script: `scripts/verify_live_ide_execution.ps1`

## Problem Context
Auditor and Reviewer 2 identified two fatal flaws in `scripts/verify_live_ide_execution.ps1`:
1. Line 205 crashes with `PermissionDenied` (`RemoveFileSystemItemUnAuthorizedAccess`) when attempting to wipe static directory `$env:TEMP\quorumrag_codegen_verify`. Because `$ErrorActionPreference = "Stop"`, this unhandled exception stops execution and jumps to `finally`, completely skipping Gate 4 (`cargo check`), Gate 5 (`/api/langextract`), and `/api/langextract/visualize`.
2. The temporary verification harness `Cargo.toml` injected:
   ```toml
   [dependencies]
   quorum-coordinator = { path = "d:/harfile/QuorumRAG/crates/quorum-coordinator" }
   ```
   This dragged in `quorum-network`, `libp2p`, and `aws-lc-sys` outside the workspace. On Windows without NASM, `aws-lc-sys` build fails (`NASM command not found or failed to execute`, os error 3), causing `cargo check` to fail.
   However, the synthesized code in `crates/quorum-coordinator/src/offline_engine.rs` (`pub struct RrfCombiner;`, `compute_rrf_score`, and `test_rrf_combiner_scoring`) is 100% self-contained standard library Rust and requires zero external crate dependencies!

## Task
1. Inspect `scripts/verify_live_ide_execution.ps1` and `crates/quorum-coordinator/src/offline_engine.rs`.
2. Design the exact PowerShell modifications in `scripts/verify_live_ide_execution.ps1`:
   - Use a unique temporary directory for each run:
     `$verifyHarnessDir = Join-Path $env:TEMP ("quorumrag_codegen_verify_" + [System.Guid]::NewGuid().ToString("N"))`
   - Use `Remove-Item -Path $verifyHarnessDir -Recurse -Force -ErrorAction SilentlyContinue` in both pre-creation and cleanup.
   - Define a pure, self-contained `Cargo.toml` without `quorum-coordinator`:
     ```toml
     [package]
     name = "quorumrag-codegen-verify"
     version = "0.1.0"
     edition = "2021"

     [dependencies]
     ```
   - Ensure the script runs end-to-end through:
     - Gate 1: `quorumrag.exe` binary check
     - Gate 2: Daemon launch and port binding (port 3031)
     - Gate 3: `/api/health` polling
     - Gate 4: Real workspace file indexing via `/api/reindex`
     - Gate 5: Live coding prompt query `/api/chat` -> SSE stream parsing -> exact `[local:...fusion.rs:1-12]` citation extraction -> genuine compilable ````rust ... ```` code block extraction -> `cargo check` exit code 0
     - Gate 6: Live `/api/langextract` and `/api/langextract/visualize` validation
     - Cleanup: `POST /api/shutdown` graceful termination -> zero orphaned `.lock` files verification.
3. Output your analysis and complete fix recommendations into `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_3\handoff.md`.


## 2026-11-01T03:21:46Z
Your working directory is `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_3\`.
Read your task dispatch at `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_3\DISPATCH.md`.
Read all mandatory inputs including:
- `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\ORIGINAL_REQUEST.md`
- `d:\harfile\QuorumRAG\PROJECT.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\orchestrator\DEAD_ENDS.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\auditor_m5_remediation_1\handoff.md`
- `d:\harfile\QuorumRAG\.agents\teamwork\reviewer_m5_remediation_2\handoff.md`

Your objective: Investigate and design the exact fixes for `scripts/verify_live_ide_execution.ps1` to eliminate the line 205 `RemoveFileSystemItemUnAuthorizedAccess` crash (using unique GUID directory and `-ErrorAction SilentlyContinue`) and isolate the temporary verification harness Cargo.toml without `quorum-coordinator` dependency so that `cargo check` compiles the standalone `pub struct RrfCombiner;` cleanly with standard library Rust and zero external build toolchain failures.
Do NOT modify source files. Produce your investigation report at `d:\harfile\QuorumRAG\.agents\teamwork\explorer_m5_it3_3\handoff.md`.
When done, notify me via `send_message`.
