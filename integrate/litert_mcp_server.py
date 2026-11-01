"""
LiteRT & Local Antigravity Engine MCP Server
============================================
Exposes offline local agent execution to any IDE (Antigravity IDE, HugOS IDE, VS Code)
via the Model Context Protocol (MCP 2.x).
"""

from __future__ import annotations

import asyncio
import json
import os
import sys
from pathlib import Path
from typing import Any, Dict

import psutil
from mcp.server.mcpserver import MCPServer

# Add local directory to path for imports
current_dir = os.path.dirname(os.path.abspath(__file__))
if current_dir not in sys.path:
    sys.path.insert(0, current_dir)

import litert_local_agent as local_engine

server = MCPServer("antigravity-local-engine")


@server.tool()
async def get_local_ai_status() -> str:
    """
    Checks the status of local AI engines: Google AI Edge LiteRT (.litertlm models)
    and Ollama/ModelFusion local models, along with host CPU, RAM, and GPU status.
    """
    try:
        mem = psutil.virtual_memory()
        cpu_pct = psutil.cpu_percent(interval=None)

        litert_models = local_engine.get_available_litert_models()
        ollama_models = local_engine.get_available_ollama_models()
        best_selection = local_engine.select_best_local_model("auto")

        status_report = {
            "status": "online",
            "host_hardware": {
                "cpu_percent": f"{cpu_pct}%",
                "total_ram_gb": round(mem.total / (1024**3), 2),
                "available_ram_gb": round(mem.available / (1024**3), 2),
                "used_ram_percent": f"{mem.percent}%",
            },
            "selected_active_engine": best_selection,
            "engines": {
                "litert": {
                    "available": bool(litert_models or os.path.exists(local_engine.GEMMA4_DEFAULT_PATH)),
                    "gemma4_26b_installed": os.path.exists(local_engine.GEMMA4_DEFAULT_PATH),
                    "model_count": len(litert_models),
                    "models": litert_models,
                },
                "ollama": {
                    "available": bool(ollama_models),
                    "model_count": len(ollama_models),
                    "models": ollama_models,
                }
            }
        }
        return json.dumps(status_report, indent=2)
    except Exception as e:
        return f"Error retrieving local AI status: {str(e)}"


@server.tool()
async def run_local_agent(
    prompt: str,
    model_preference: str = "auto",
    workspace: str = "",
    allow_write_tools: bool = False,
) -> str:
    """
    Runs an agentic task completely offline and on-device using the Antigravity SDK.
    No code or prompt data leaves the local machine.
    
    Args:
        prompt: The task instruction or question to send to the local agent.
        model_preference: 'auto' (best fit), 'litert' (Gemma 4 26B LiteRT), or 'ollama' (local Ollama).
        workspace: Optional workspace directory path for file tools.
        allow_write_tools: Whether to allow the local agent to edit files or run commands.
    """
    try:
        ws = workspace.strip() if workspace else os.getcwd()
        result = await local_engine.run_local_agent(
            prompt=prompt,
            model_preference=model_preference,
            workspace=ws,
            allow_write_tools=allow_write_tools,
        )
        return result
    except Exception as e:
        return f"Local agent error: {str(e)}"


@server.tool()
async def audit_file_offline(
    file_path: str,
    audit_type: str = "security_and_bugs",
    workspace: str = "",
) -> str:
    """
    Audits a local code file completely offline for vulnerabilities, logic bugs,
    memory leaks, and performance bottlenecks without uploading code to any cloud API.
    
    Args:
        file_path: Absolute or workspace-relative path to the file to audit.
        audit_type: 'security_and_bugs', 'performance', or 'refactor'.
        workspace: Optional project workspace root.
    """
    try:
        p = Path(file_path)
        if not p.is_absolute() and workspace:
            p = Path(workspace) / p

        if not p.exists():
            return f"Error: File '{file_path}' does not exist."

        content = p.read_text(encoding="utf-8", errors="replace")
        prompt = (
            f"Perform an in-depth {audit_type} audit of the following file (`{p.name}`):\n\n"
            f"```\n{content}\n```\n\n"
            "Report:\n"
            "1. Security & Safety risks (injection, bounds, race conditions)\n"
            "2. Logic bugs and edge cases\n"
            "3. Actionable remediation steps with diff/snippet recommendations"
        )

        return await local_engine.run_local_agent(
            prompt=prompt,
            model_preference="auto",
            workspace=workspace or str(p.parent),
            allow_write_tools=False,
        )
    except Exception as e:
        return f"Offline audit error: {str(e)}"


@server.tool()
async def run_security_gauntlet(
    target_file: str,
    test_file: str = "",
    workspace: str = "",
) -> str:
    """
    Executes the 3-step Adversarial Security Gauntlet on a target module offline:
    1. Reproduce vulnerabilities and state flaws.
    2. Author a minimal, verified candidate patch.
    3. Validate against existing unit tests offline.
    
    Args:
        target_file: Target source file to audit and patch.
        test_file: Optional unit test file to validate against.
        workspace: Workspace root.
    """
    try:
        return await local_engine.run_offline_security_gauntlet(
            target_file=target_file,
            test_file=test_file if test_file.strip() else None,
            workspace=workspace.strip() if workspace else None,
        )
    except Exception as e:
        return f"Security gauntlet error: {str(e)}"


if __name__ == "__main__":
    # Ensure unbuffered binary stdout on Windows
    sys.stderr.write("Starting Antigravity Local Engine MCP Server on stdio...\n")
    sys.stderr.flush()
    server.run(transport="stdio")
