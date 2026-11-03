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

import psutil
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
OLLAMA_PROBE_TIMEOUT_SEC = 0.4


def get_runtime_free_ram_gb() -> float:
    """Return runtime available/free system RAM in GiB (never total physical RAM)."""
    try:
        return round(psutil.virtual_memory().available / (1024 ** 3), 2)
    except Exception:
        return 8.0


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
    """Query local Ollama instance for installed models with strict 400ms timeout."""
    try:
        resp = requests.get(f"{base_url}/api/tags", timeout=OLLAMA_PROBE_TIMEOUT_SEC)
        if resp.status_code == 200:
            data = resp.json()
            return [m["name"] for m in data.get("models", []) if m.get("name")]
    except Exception:
        pass
    return []


def _match_installed_model(candidates: List[str], installed: List[str]) -> Optional[str]:
    """Return the first installed Ollama model matching the ordered candidate list."""
    for candidate in candidates:
        cand_lower = candidate.lower()
        for inst in installed:
            inst_lower = inst.lower()
            if inst_lower == cand_lower or inst_lower.startswith(cand_lower) or cand_lower in inst_lower:
                return inst
    return None


def select_best_local_model(preference: str = "auto", task_type: str = "general") -> Dict[str, Any]:
    """
    Selects the optimal local model configuration based on runtime available RAM
    (Dynamic Available Memory Law), task specialization ('general' vs 'security'),
    imported LiteRT models, and running Ollama models.
    """
    free_ram_gb = get_runtime_free_ram_gb()
    litert_models = get_available_litert_models()
    ollama_models = get_available_ollama_models()
    pref_norm = (preference or "auto").strip().lower()
    is_security_task = task_type.lower() == "security" or pref_norm in ("security", "pentest", "abliterated", "obliterated")

    # 1. Explicit LiteRT preference
    if pref_norm == "litert" or (pref_norm == "auto" and not is_security_task and os.path.exists(GEMMA4_DEFAULT_PATH)):
        if os.path.exists(GEMMA4_DEFAULT_PATH):
            return {
                "engine": "litert",
                "model_path": GEMMA4_DEFAULT_PATH,
                "model_name": "gemma4-26b",
                "free_ram_gb": free_ram_gb,
                "task_type": task_type,
            }
        elif litert_models:
            return {
                "engine": "litert",
                "model_path": litert_models[0]["path"],
                "model_name": litert_models[0]["name"],
                "free_ram_gb": free_ram_gb,
                "task_type": task_type,
            }

    # 2. Explicit model name requested by caller (e.g. 'deepseek-r1:32b', 'gemma4:26b', 'fenkohq/foundation-sec-8b:latest')
    if ollama_models and pref_norm not in ("auto", "ollama", "litert", "security", "pentest"):
        explicit_match = _match_installed_model([preference], ollama_models)
        if explicit_match:
            return {
                "engine": "ollama",
                "base_url": f"{DEFAULT_OLLAMA_URL}/v1",
                "model_name": explicit_match,
                "free_ram_gb": free_ram_gb,
                "task_type": task_type,
            }

    # 3. Dynamic Available Memory Law + Security/Abliterated specialization
    if ollama_models:
        if is_security_task:
            # Check for any local OBLITERATUS / abliterated models that fit available RAM first
            abliterated_installed = [
                m for m in ollama_models
                if ("obliterat" in m.lower() or "abliterat" in m.lower())
                and (free_ram_gb >= 48.0 or not any(tag in m.lower() for tag in ("70b", "72b", "755b")))
            ]
            if abliterated_installed:
                return {
                    "engine": "ollama",
                    "base_url": f"{DEFAULT_OLLAMA_URL}/v1",
                    "model_name": abliterated_installed[0],
                    "free_ram_gb": free_ram_gb,
                    "task_type": "security",
                }

            if free_ram_gb >= 48.0:
                priorities = [
                    "fenkohq/foundation-sec-8b:latest",
                    "strix-deepseek:latest",
                    "deepseek-r1:32b",
                    "gemma4:26b",
                    "qwen2.5:32b",
                    "strix-qwen:latest",
                    "qwen2.5:7b",
                ]
            elif free_ram_gb >= 12.0:
                priorities = [
                    "fenkohq/foundation-sec-8b:latest",
                    "strix-deepseek:latest",
                    "strix-qwen:latest",
                    "qwen2.5:7b",
                    "gemma2:9b",
                ]
            else:
                priorities = [
                    "deepseek-r1:1.5b",
                    "qwen2.5:1.5b",
                    "gemma2:2b",
                ]
        else:
            # Strictly evaluate runtime free/available RAM per Anti-OOM sizing law
            if free_ram_gb >= 48.0:
                priorities = [
                    "qwen2.5:32b",
                    "gemma4:26b",
                    "deepseek-r1:32b",
                    "fenkohq/foundation-sec-8b:latest",
                    "strix-deepseek:latest",
                    "qwen2.5:7b",
                ]
            elif free_ram_gb >= 24.0:
                priorities = [
                    "qwen2.5:14b",
                    "gemma4:26b",
                    "translategemma:12b",
                    "gemma2:9b",
                    "qwen2.5:7b",
                ]
            elif free_ram_gb >= 12.0:
                priorities = [
                    "qwen2.5:7b",
                    "fenkohq/foundation-sec-8b:latest",
                    "gemma2:9b",
                    "glm4:9b",
                ]
            elif free_ram_gb >= 6.0:
                priorities = [
                    "qwen2.5:3b",
                    "gemma2:2b",
                    "deepseek-r1:1.5b",
                    "qwen2.5:1.5b",
                ]
            else:
                priorities = [
                    "qwen2.5:1.5b",
                    "deepseek-r1:1.5b",
                    "gemma2:2b",
                ]

        selected = _match_installed_model(priorities, ollama_models) or ollama_models[0]
        return {
            "engine": "ollama",
            "base_url": f"{DEFAULT_OLLAMA_URL}/v1",
            "model_name": selected,
            "free_ram_gb": free_ram_gb,
            "task_type": "security" if is_security_task else task_type,
        }

    # 4. Offline fallback default sized by runtime free RAM
    default_tier = (
        "qwen2.5:32b" if free_ram_gb >= 48.0 else
        "qwen2.5:14b" if free_ram_gb >= 24.0 else
        "qwen2.5:7b" if free_ram_gb >= 12.0 else
        "qwen2.5:3b" if free_ram_gb >= 6.0 else
        "qwen2.5:1.5b"
    )
    return {
        "engine": "ollama",
        "base_url": f"{DEFAULT_OLLAMA_URL}/v1",
        "model_name": default_tier,
        "free_ram_gb": free_ram_gb,
        "task_type": "security" if is_security_task else task_type,
    }


async def run_local_agent(
    prompt: str,
    model_preference: str = "auto",
    workspace: Optional[str] = None,
    system_instructions: Optional[str] = None,
    allow_write_tools: bool = False,
    task_type: str = "general",
) -> str:
    """
    Executes a task completely offline using the local Antigravity agent.
    Streams and aggregates response tokens.
    """
    target = select_best_local_model(model_preference, task_type=task_type)
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
    model_preference: str = "security",
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
        model_preference=model_preference,
        workspace=workspace,
        system_instructions=instructions,
        allow_write_tools=False,
        task_type="security",
    )
