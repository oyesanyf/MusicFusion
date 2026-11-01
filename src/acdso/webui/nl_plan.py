"""Turn a free-text request into a run_acdso.py column-role plan.

Mirrors the OpenAI-provider pattern run_acdso.py already uses for target/
datetime auto-detection (see ``_llm_detect_target_col``), and falls back to
keyword matching against the actual column names when no LLM is configured
-- the same "LLM on top, rules underneath" shape as column_validator.py.

This never starts a run. It only proposes column roles for the webui to
show back to the user, who applies them to the existing form and presses
Run themselves -- training can take hours and --force deletes prior
models/reports, so nothing here should fire off a job unattended.
"""

from __future__ import annotations

import json
import os
import re
from typing import Any

import pandas as pd

from .column_validator import predictable_columns, suggest_target_columns

_TASK_CHOICES = ("auto", "classification", "regression")
_WORD_RE = re.compile(r"[a-z0-9]+")


def build_plan(prompt: str, frame: pd.DataFrame) -> dict[str, Any]:
    """Return {predict, task, exclude, datetime_col, explanation, method, warnings}."""
    columns = [str(c) for c in frame.columns]
    plan = _llm_plan(prompt, frame, columns)
    if plan is None:
        plan = _rule_based_plan(prompt, frame, columns)
    plan["warnings"] = _target_warnings(frame, plan["predict"])
    return plan


def _target_warnings(frame: pd.DataFrame, predict: list[str]) -> list[str]:
    """Flag a predict column that looks like a categorical identifier/dimension
    rather than a genuine outcome to score.

    Neither the LLM nor the keyword fallback can reliably tell "a label worth
    predicting" from "a high-cardinality facet that happens to share a word
    with the request" (e.g. an app-name column in network logs) -- this
    can't be fixed by picking harder, only flagged so the user catches it
    before pressing Run, same as the column-role plan being shown at all
    rather than run automatically.
    """
    warnings: list[str] = []
    n_rows = len(frame)
    if n_rows == 0:
        return warnings
    for col in predict:
        if col not in frame.columns or str(frame[col].dtype) not in ("object", "category"):
            continue
        nunique = frame[col].nunique(dropna=True)
        ratio = nunique / n_rows
        if nunique > 50 or ratio > 0.3:
            warnings.append(
                f"'{col}' has {nunique} distinct values ({ratio:.0%} of rows) -- this looks more "
                "like a categorical identifier or dimension (e.g. an app/service name) than a "
                "genuine outcome to score. Double-check this is really what you want to predict."
            )
    return warnings


def _extract_json(text: str) -> str:
    """Pull the {...} block out of a response that may carry extra prose."""
    match = re.search(r"\{.*\}", text, re.DOTALL)
    return match.group(0) if match else text


def _llm_plan(prompt: str, frame: pd.DataFrame, columns: list[str]) -> dict[str, Any] | None:
    if not os.environ.get("OPENAI_API_KEY"):
        return None
    try:
        from acdso.advisors import LLMConfig, OpenAIProvider

        provider = OpenAIProvider()
        col_info = "\n".join(
            f"  - {c}: {frame[c].dtype}, nunique={frame[c].nunique()}" for c in columns
        )
        system_prompt = (
            "You translate a plain-English ML request into a JSON column-role "
            "plan for an AutoML tool that trains one model per `predict` column, "
            "using every OTHER column in the dataset as a feature by default.\n\n"
            "Role rules:\n"
            "- `predict`: ONLY the outcome/label column(s) the model should learn "
            "to output -- e.g. a column already named like a label, class, "
            "category, score, or outcome. NEVER put a column here just because "
            "the request mentions it as a signal to use; a signal is an input, "
            "not the thing being predicted.\n"
            "- Everything not in `predict` or `exclude` is automatically used as "
            "a feature -- do not list feature/signal columns anywhere, just leave "
            "them alone. If the request names a signal (e.g. 'request volume', "
            "'user spread', 'DNS pattern'), the matching column must stay OUT of "
            "both `predict` and `exclude`.\n"
            "- `exclude`: ONLY pure identifiers, columns that duplicate the "
            "target, or columns genuinely irrelevant to the request. Keep this "
            "as short as possible -- it is not a dumping ground.\n"
            "- If no column looks like an existing label/outcome, pick the "
            "single column whose name is closest to one; do not guess by "
            "putting several unrelated columns in `predict`.\n"
            "- Default `exclude` to an empty list. Only add a column to it if "
            "you have a specific reason (raw identifier, exact duplicate of the "
            "target) -- a column simply not being the target is NOT a reason to "
            "exclude it.\n\n"
            "Worked example -- do not deviate from this pattern:\n"
            "Request: \"score domains by similarity to known providers and "
            "request volume\"\n"
            "Columns: domain, request_volume, label\n"
            "Correct: {\"predict\": [\"label\"], \"exclude\": [], "
            "\"task\": \"classification\", \"datetime_col\": [], "
            "\"explanation\": \"...\"}\n"
            "Wrong: putting request_volume or domain in exclude or predict -- "
            "they are features, so they are left out of both lists entirely.\n\n"
            "Only use column names that appear in the list given. Respond with "
            "JSON only, no prose."
        )
        user_prompt = (
            f"Request: {prompt}\n\n"
            f"Available columns:\n{col_info}\n\n"
            "Respond with exactly this JSON shape:\n"
            '{"predict": ["<column>", ...], "task": "auto|classification|regression", '
            '"exclude": ["<column>", ...], "datetime_col": ["<column>"] or [], '
            '"explanation": "<one sentence>"}'
        )
        config = LLMConfig(max_tokens=400, temperature=0.1)
        response = provider.complete(system_prompt, user_prompt, config)
        data = json.loads(_extract_json(response))
    except Exception:
        return None

    predict = [c for c in data.get("predict", []) if c in columns]
    if not predict:
        return None
    task = data.get("task") if data.get("task") in _TASK_CHOICES else "auto"
    exclude = [c for c in data.get("exclude", []) if c in columns and c not in predict]
    datetime_col = [c for c in data.get("datetime_col", []) if c in columns][:1]
    return {
        "predict": predict,
        "task": task,
        "exclude": exclude,
        "datetime_col": datetime_col,
        "explanation": str(data.get("explanation", "")).strip(),
        "method": "llm",
    }


def _rule_based_plan(prompt: str, frame: pd.DataFrame, columns: list[str]) -> dict[str, Any]:
    """No LLM configured -- match words in the prompt against column names."""
    prompt_words = set(_WORD_RE.findall(prompt.lower()))
    candidates = predictable_columns(frame)

    def word_overlap(col: str) -> int:
        return len(prompt_words & set(_WORD_RE.findall(col.lower())))

    scored = sorted(((c, word_overlap(c)) for c in candidates), key=lambda item: item[1], reverse=True)
    mentioned = [c for c, score in scored if score > 0]

    if mentioned:
        predict = mentioned[:3]
        explanation = "Matched column name(s) mentioned in the request: " + ", ".join(predict)
    else:
        ranked = suggest_target_columns(frame, candidates, top_n=1)
        suggestions = ranked.get("suggestions") or []
        predict = [suggestions[0]["column"]] if suggestions else candidates[:1]
        explanation = (
            "No column names recognized in the request -- falling back to the "
            "top-ranked predictable column."
        )

    datetime_col = [
        c for c in columns if any(kw in c.lower() for kw in ("date", "time", "timestamp", "datetime"))
    ][:1]

    return {
        "predict": predict,
        "task": "auto",
        "exclude": [],
        "datetime_col": datetime_col,
        "explanation": explanation,
        "method": "rule-based",
    }
