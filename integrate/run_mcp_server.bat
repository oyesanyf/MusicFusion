@echo off
rem Process-scoped launcher for Antigravity Local Engine MCP Server
rem Zero-touch of Windows Registry / System Environment Variables
setlocal enabledelayedexpansion

set "INTEGRATE_DIR=%~dp0"
set "PYTHON_EXE=%INTEGRATE_DIR%.venv\Scripts\python.exe"
set "MCP_SCRIPT=%INTEGRATE_DIR%litert_mcp_server.py"

set "PYTHONUNBUFFERED=1"
set "PYTHONIOENCODING=utf-8"

"%PYTHON_EXE%" "%MCP_SCRIPT%" %*
endlocal
