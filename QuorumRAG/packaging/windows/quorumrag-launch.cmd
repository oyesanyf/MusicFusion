@echo off
setlocal
REM QuorumRAG Process-Scoped Windows Launcher (Zero-Touch Registry/System PATH)
set "APP_DIR=%~dp0"
set "PATH=%APP_DIR%;%PATH%"
set "DATA_DIR=%LOCALAPPDATA%\QuorumRAG\data"
if not exist "%DATA_DIR%" mkdir "%DATA_DIR%"

echo Starting QuorumRAG Local-First P2P Agentic RAG on http://127.0.0.1:3031 ...
"%APP_DIR%quorumrag.exe" --host 0.0.0.0 --port 3031 --p2p-port 3032 --data-dir "%DATA_DIR%" %*
endlocal
