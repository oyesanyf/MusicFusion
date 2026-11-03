# Scope: Milestone M15 - Build, Service Deployment & Live Verification

## Tasks
1. Stop any dangling instances of `quorumrag.exe` (e.g. `Stop-Process -Name quorumrag -Force -ErrorAction SilentlyContinue`).
2. Clean/delete `target\debug\quorumrag.exe` if present.
3. Build binary:
   ```powershell
   $env:CARGO_INCREMENTAL = "0"
   cargo build -p quorum-daemon --bin quorumrag
   ```
4. Start `quorumrag.exe` in the background listening on `0.0.0.0:3031`:
   ```powershell
   Start-Process -FilePath "d:\harfile\QuorumRAG\target\debug\quorumrag.exe" -ArgumentList "--host 0.0.0.0 --port 3031 --data-dir `"d:\harfile\QuorumRAG\.quorumrag-rag-index`" --no-browser" -WindowStyle Hidden
   ```
5. Confirm port 3031 is listening:
   ```powershell
   Test-NetConnection -ComputerName 127.0.0.1 -Port 3031
   ```
   Or `curl.exe http://127.0.0.1:3031/api/settings`
6. Verify live SSE test queries:
   - `"tell me about sacred maths"`
   - `"tell me more"`
   Confirm responses stream without mid-sentence cutoffs.
7. Ship changes via:
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/solo_pr_ship.ps1 -Title "feat(deep-research): integrate deep research, iterative section RAG, parent doc retrieval, and 262k token chaining"
   ```
