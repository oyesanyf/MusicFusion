"""
Antigravity Local Engine Runner
===============================
Provides offline local agent execution via Google Antigravity SDK:
- Google AI Edge LiteRT (`LiteRTAgentConfig`) for on-device Gemma 4 26B / .litertlm models
- Local OpenAI (`LocalOpenAIAgentConfig`) for Ollama / ModelFusion models (e.g. qwen2.5, gemma2)
- Autonomous security gauntlet & offline code auditing
"""

from __future__ import annotations

import asyncio
import os
import sys
from pathlib import Path
from typing import Any, AsyncGenerator, Dict, List, Optional

import requests
from google.antigravity import (
    Agent,
    CapabilitiesConfig,
    LiteRTAgentConfig,
    LocalOpenAIAgentConfig,
)
from google.antigravity.hooks import policy


DEFAULT_OLLAMA_URL = "http://127.0.0.1:11434"
DEFAULT_LITERT_MODEL_DIR = os.path.expanduser("~/.litert-lm/models")
GEMMA4_DEFAULT_PATH = os.path.join(DEFAULT_LITERT_MODEL_DIR, "gemma4-26b", "model.litertlm")


def get_available_litert_models() -> List[Dict[str, str]]:
    """Scan local ~/.litert-lm/models for imported LiteRT models."""
    models: List[Dict[str, str]] = []
    base_dir = Path(DEFAULT_LITERT_MODEL_DIR)
    if base_dir.exists():
        for p in base_dir.rglob("*.litertlm"):
            models.append({
                "name": p.parent.name if p.name == "model.litertlm" else p.stem,
                "path": str(p),
                "size_mb": str(round(p.stat().st_size / (1024 * 1024), 2)),
            })
    return models


def get_available_ollama_models(base_url: str = DEFAULT_OLLAMA_URL) -> List[str]:
    """Query local Ollama instance for installed models."""
    try:
        resp = requests.get(f"{base_url}/api/tags", timeout=1.0)
        if resp.status_code == 200:
            data = resp.json()
            return [m["name"] for m in data.get("models", [])]
    except Exception:
        pass
    return []


def select_best_local_model(preference: str = "auto") -> Dict[str, Any]:
    """
    Selects the optimal local model configuration based on available hardware,
    imported LiteRT models, and running Ollama models.
    """
    litert_models = get_available_litert_models()
    ollama_models = get_available_ollama_models()

    # If preference specifies litert and model exists
    if preference == "litert" or (preference == "auto" and os.path.exists(GEMMA4_DEFAULT_PATH)):
        if os.path.exists(GEMMA4_DEFAULT_PATH):
            return {
                "engine": "litert",
                "model_path": GEMMA4_DEFAULT_PATH,
                "model_name": "gemma4-26b",
            }
        elif litert_models:
            return {
                "engine": "litert",
                "model_path": litert_models[0]["path"],
                "model_name": litert_models[0]["name"],
            }

    # Fallback to Ollama
    if ollama_models:
        # Prioritize 7b-9b for responsive local execution, then larger tiers
        priorities = [
            "qwen2.5:7b",
            "gemma2:9b",
            "qwen2.5:14b",
            "qwen2.5:32b",
            "deepseek-r1:1.5b",
            "gemma2:2b",
            "strix-qwen:latest",
        ]
        selected = ollama_models[0]
        for p in priorities:
            for m in ollama_models:
                if m.startswith(p) or p in m:
                    selected = m
                    break
            else:
                continue
            break

        return {
            "engine": "ollama",
            "base_url": f"{DEFAULT_OLLAMA_URL}/v1",
            "model_name": selected,
        }

    # Fallback default if neither is detected
    return {
        "engine": "ollama",
        "base_url": f"{DEFAULT_OLLAMA_URL}/v1",
        "model_name": "qwen2.5:7b",
    }


async def run_local_agent(
    prompt: str,
    model_preference: str = "auto",
    workspace: Optional[str] = None,
    system_instructions: Optional[str] = None,
    allow_write_tools: bool = False,
) -> str:
    """
    Executes a task completely offline using the local Antigravity agent.
    Streams and aggregates response tokens.
    """
    target = select_best_local_model(model_preference)
    workspaces = [os.path.abspath(workspace)] if workspace else [os.getcwd()]

    if target["engine"] == "litert":
        policies = [policy.allow_all()]
        caps = CapabilitiesConfig(
            file_reads=True,
            file_writes=allow_write_tools,
            command_execution=allow_write_tools,
        )
        config = LiteRTAgentConfig(
            model_path=target["model_path"],
            workspaces=workspaces,
            system_instructions=system_instructions or "You are an expert local AI engineering agent.",
            capabilities=caps,
            policies=policies,
        ).lightweight()
    else:
        config = LocalOpenAIAgentConfig(
            model=target["model_name"],
            base_url=target["base_url"],
            system_instructions=system_instructions or "You are an expert local AI engineering agent.",
        )

    tokens: List[str] = []
    async with Agent(config) as agent:
        response = await agent.chat(prompt)
        async for token in response:
            tokens.append(token)

    return "".join(tokens)


async def run_offline_security_gauntlet(
    target_file: str,
    test_file: Optional[str] = None,
    workspace: Optional[str] = None,
) -> str:
    """
    Executes a 3-pass adversarial security and regression gauntlet:
    1. Pass 1: Reproduce vulnerabilities / analyze AST flaws
    2. Pass 2: Author secure candidate patch
    3. Pass 3: Validate against tests offline
    """
    file_path = Path(target_file)
    if not file_path.is_absolute():
        base = Path(workspace) if workspace else Path.cwd()
        file_path = base / file_path

    if not file_path.exists():
        return f"Error: Target file not found at {file_path}"

    content = file_path.read_text(encoding="utf-8", errors="replace")
    test_content = ""
    if test_file:
        tf_path = Path(test_file)
        if not tf_path.is_absolute():
            base = Path(workspace) if workspace else Path.cwd()
            tf_path = base / tf_path
        if tf_path.exists():
            test_content = f"\n\nAssociated Tests:\n```\n{tf_path.read_text(encoding='utf-8', errors='replace')}\n```"

    prompt = (
        f"Perform an adversarial security audit and patch gauntlet on the following module: `{file_path.name}`.\n\n"
        f"File Content:\n```\n{content}\n```"
        f"{test_content}\n\n"
        "Please provide:\n"
        "1. VULNERABILITY ANALYSIS: Identify any injection, path traversal, race conditions, memory leaks, or logic flaws.\n"
        "2. ADVERSARIAL TEST CASE: A targeted reproduction snippet exposing the flaw.\n"
        "3. PRODUCTION PATCH: Provide the complete corrected implementation with comments.\n"
        "4. REGRESSION VERIFICATION: Prove that the patch maintains backward compatibility."
    )

    instructions = (
        "You are an adversarial security auditor and principal engineer running on-device. "
        "Strictly analyze the provided code without sending data externally. Focus on precision and safety."
    )

    return await run_local_agent(
        prompt=prompt,
        model_preference="auto",
        workspace=workspace,
        system_instructions=instructions,
        allow_write_tools=False,
    )
