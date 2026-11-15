#!/usr/bin/env python3
"""100% Local Bespoke-Nimble Typed Single-Pass Logit Worker (CPU/RAM GGUF & HF).

Exits cleanly with code 2 when cached local checkpoint or runtime backend is absent so
the Rust `NimbleDecisionEngine` seamlessly delegates to Ollama logprobs or System-1 scoring.
"""

from __future__ import annotations

import argparse
import gc
import json
import os
import sys
from pathlib import Path


def resolve_hf_home() -> Path:
    for var in ("MUSIC_MODELS_DIR", "HF_HOME"):
        val = os.environ.get(var, "").strip()
        if val:
            return Path(val)
    return Path(r"D:\music\models") if os.name == "nt" else Path("models")


def release_cuda_cache() -> None:
    gc.collect()
    try:
        import torch

        if torch.cuda.is_available():
            torch.cuda.empty_cache()
    except Exception:
        pass


def main() -> int:
    parser = argparse.ArgumentParser(description="Local Bespoke-Nimble single-pass decision worker")
    parser.add_argument("--json", required=False, default="", help="Decision request JSON payload")
    parser.add_argument("--check-env", action="store_true")
    args = parser.parse_args()

    if args.check_env:
        print(f"nimble_decide ready hf_home={resolve_hf_home()}")
        return 0

    if not args.json.strip():
        sys.stderr.write("Missing --json payload\n")
        return 2

    req = json.loads(args.json)
    model_id = str(req.get("model_id", "bespokelabs/Bespoke-Nimble-9B"))
    question = str(req.get("question", ""))
    choices = [str(c) for c in (req.get("choices") or [])]
    context = req.get("context") or {}
    cpu_offload = bool(req.get("cpu_offload", True))

    if not choices:
        sys.stderr.write("Empty choice list\n")
        return 2

    hf_home = resolve_hf_home()
    sanitized = model_id.replace("/", "--")
    cached_dirs = [
        hf_home / "hub" / f"models--{sanitized}",
        hf_home / f"models--{sanitized}",
        hf_home / model_id.split("/")[-1],
    ]
    if not any(d.exists() for d in cached_dirs):
        sys.stderr.write(f"Local Nimble checkpoint '{model_id}' not cached in '{hf_home}'\n")
        return 2

    try:
        import torch
        from transformers import AutoModelForCausalLM, AutoTokenizer

        device = "cpu" if cpu_offload or not torch.cuda.is_available() else "cuda"
        tokenizer = AutoTokenizer.from_pretrained(model_id, cache_dir=str(hf_home / "hub"), local_files_only=True)
        model = AutoModelForCausalLM.from_pretrained(
            model_id,
            cache_dir=str(hf_home / "hub"),
            local_files_only=True,
            torch_dtype=torch.float32 if device == "cpu" else torch.float16,
            device_map="cpu" if device == "cpu" else "auto",
        )

        numbered = "\n".join(f"{i + 1}: {opt}" for i, opt in enumerate(choices))
        prompt = (
            f"Question: {question}\n"
            f"Context: {json.dumps(context)}\n"
            f"Options:\n{numbered}\n"
            f"Select the single digit option (1-{len(choices)}): "
        )
        inputs = tokenizer(prompt, return_tensors="pt")
        if device != "cpu":
            inputs = {k: v.to(model.device) for k, v in inputs.items()}

        with torch.no_grad():
            outputs = model(**inputs)
            last_logits = outputs.logits[0, -1, :]

        extracted = []
        for idx in range(len(choices)):
            tok_ids = tokenizer.encode(str(idx + 1), add_special_tokens=False)
            tok_id = tok_ids[-1] if tok_ids else 0
            extracted.append(float(last_logits[tok_id].item()))

        best_idx = max(range(len(choices)), key=lambda i: extracted[i])
        del model
        release_cuda_cache()

        print(
            json.dumps(
                {
                    "logits": extracted,
                    "selected_choice": choices[best_idx],
                    "rationale": f"Local Bespoke-Nimble single-pass forward logits on {device}",
                }
            )
        )
        return 0
    except Exception as exc:
        release_cuda_cache()
        sys.stderr.write(f"Local Nimble worker fallback: {exc}\n")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
