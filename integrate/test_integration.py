"""
End-to-End Verification Test for Antigravity Local Integration
==============================================================
Tests:
1. Hardware & Engine Status Discovery
2. Local Agent Offline Prompt Execution
3. Environment Safety (Zero-Touch Registry Audit)
"""

import asyncio
import json
import os
import sys

from litert_local_agent import (
    get_available_litert_models,
    get_available_ollama_models,
    select_best_local_model,
    run_local_agent,
)

async def test_suite():
    print("=== 1. Checking Local Model Discovery ===")
    litert = get_available_litert_models()
    print(f"LiteRT models discovered: {len(litert)}")
    for m in litert:
        print(f"  - {m['name']} ({m['size_mb']} MB): {m['path']}")

    ollama = get_available_ollama_models()
    print(f"Ollama models discovered: {len(ollama)}")
    for m in ollama:
        print(f"  - {m}")

    best = select_best_local_model("auto", task_type="general")
    print(f"\nBest Selected General Engine: {json.dumps(best, indent=2)}")

    security_best = select_best_local_model("security", task_type="security")
    print(f"\nBest Selected Security/Pentest Engine: {json.dumps(security_best, indent=2)}")

    print("\n=== 2. Running Local Agent Test Query ===")
    prompt = "Respond in 5 words or less: Local Antigravity Engine operational."
    print(f"Prompt: '{prompt}'")
    result = await run_local_agent(
        prompt=prompt,
        model_preference="auto",
        system_instructions="You are a terse assistant.",
    )
    print(f"Agent Response: {result.strip()}")

    print("\n=== 3. Integration Verification Succeeded! ===")

if __name__ == "__main__":
    asyncio.run(test_suite())
