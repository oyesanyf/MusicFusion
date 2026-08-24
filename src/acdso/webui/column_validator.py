"""LLM-assisted sanity check for the GUI's column-role selections.

Picking the wrong column for a role (an ID column as the forecast target,
a numeric column as the datetime column) doesn't surface as an error until
deep into a multi-minute training run. This gives an early, advisory check
before the run starts: does each selected column actually look like it fits
the role it's been given?

Mirrors LLMColumnAnalyzer's pattern in acdso.advisors: try the LLM, fall
back to plain rules if no provider is configured or the call fails. The
rule-based pass alone already catches the unambiguous case -- a column
already claimed by one role can't also serve another -- and always runs;
the LLM pass adds softer judgment on top when a provider is available.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from typing import Any

import pandas as pd

ROLE_LABELS = {
    "predict": "prediction target",
    "target": "training target",
    "exclude": "excluded feature",
    "datetime_col": "datetime/period column",
}


@dataclass
class ColumnIssue:
    column: str
    role: str
    severity: str  # "block" | "warn"
    message: str


@dataclass
class ValidationResult:
    ok: bool
    issues: list = field(default_factory=list)
    source: str = "rules"  # "rules" | "llm"

    def to_dict(self) -> dict[str, Any]:
        return {
            "ok": self.ok,
            "source": self.source,
            "issues": [
                {"column": i.column, "role": i.role, "severity": i.severity, "message": i.message}
                for i in self.issues
            ],
        }


def _profile_columns(df: pd.DataFrame, columns: list[str]) -> dict[str, dict[str, Any]]:
    profile = {}
    for col in columns:
        if col not in df.columns:
            continue
        s = df[col]
        profile[col] = {
            "dtype": str(s.dtype),
            "n_unique": int(s.nunique()),
            "n_rows": int(len(s)),
            "null_pct": round(float(s.isnull().mean() * 100), 1),
            "sample": [str(v) for v in s.dropna().head(3).tolist()],
        }
    return profile


def _is_datetime_like(s: pd.Series) -> bool:
    if pd.api.types.is_datetime64_any_dtype(s):
        return True
    if pd.api.types.is_numeric_dtype(s):
        # pd.to_datetime happily parses large floats/ints as Unix
        # timestamps, which misclassifies ordinary numeric measurement
        # columns (byte counts, prices) as date-like. A real date column
        # read from CSV comes in as text ("2026-08"), not a raw number.
        return False
    try:
        parsed = pd.to_datetime(s.dropna().head(20), errors="coerce")
        return len(parsed) > 0 and parsed.notna().mean() > 0.8
    except Exception:
        return False


ID_LIKE_WORDS = ("_id", "id_", "uuid", "guid", "index", "rowid")


def predictable_columns(df: pd.DataFrame) -> list[str]:
    """Columns reasonable to offer as a Predict/Target choice at all.

    Filters out what the column-role validator would flag anyway (a
    date/period column, a raw identifier), so the dropdown only ever shows
    columns that could plausibly be predicted -- rather than showing every
    column and warning after the fact.

    A column is excluded if it's:
    - date/time-like (that's what --datetime-col is for, not a target)
    - constant (nothing to learn to predict)
    - a raw identifier: fully unique AND either ID-named, or non-float
      (a fully-unique float column is normal for a continuous measurement
      and is NOT excluded -- e.g. a byte-count column with no repeats is
      still a perfectly good regression target)
    """
    n = len(df)
    keep = []
    for col in df.columns:
        s = df[col]
        if _is_datetime_like(s):
            continue
        nunique = s.nunique()
        if n == 0 or nunique <= 1:
            continue
        fully_unique = nunique >= n
        col_lower = col.lower()
        is_id_named = any(w in col_lower for w in ID_LIKE_WORDS)
        if is_id_named and nunique > 0.9 * n:
            continue
        if fully_unique and not pd.api.types.is_float_dtype(s):
            continue
        keep.append(col)
    return keep


def _rule_based_validate(df: pd.DataFrame, roles: dict[str, list]) -> ValidationResult:
    issues: list[ColumnIssue] = []

    # A column can't serve two roles at once -- unambiguous, always blocks.
    seen: dict[str, str] = {}
    for role, cols in roles.items():
        for col in cols:
            if col in seen and seen[col] != role:
                issues.append(ColumnIssue(
                    column=col, role=role, severity="block",
                    message=f"'{col}' is already selected as {ROLE_LABELS.get(seen[col], seen[col])}; "
                            f"it can't also be the {ROLE_LABELS.get(role, role)}.",
                ))
            else:
                seen[col] = role

    for col in roles.get("datetime_col", []):
        if col in df.columns and not _is_datetime_like(df[col]):
            issues.append(ColumnIssue(
                column=col, role="datetime_col", severity="warn",
                message=f"'{col}' doesn't look like a date/time column "
                        f"(dtype {df[col].dtype}) -- double check this is the period column.",
            ))

    id_like_words = ("_id", "id_", "uuid", "guid", "index", "rowid")
    for role in ("predict", "target"):
        for col in roles.get(role, []):
            if col not in df.columns:
                continue
            s = df[col]
            col_lower = col.lower()
            if any(w in col_lower for w in id_like_words) and s.nunique() > 0.9 * len(s):
                issues.append(ColumnIssue(
                    column=col, role=role, severity="warn",
                    message=f"'{col}' looks like an identifier column (mostly unique values) "
                            f"-- unusual choice for a {ROLE_LABELS[role]}.",
                ))
            elif _is_datetime_like(s) and not pd.api.types.is_numeric_dtype(s):
                issues.append(ColumnIssue(
                    column=col, role=role, severity="warn",
                    message=f"'{col}' looks like a date/time column, not a value to predict.",
                ))

    ok = not any(i.severity == "block" for i in issues)
    return ValidationResult(ok=ok, issues=issues, source="rules")


VALIDATION_PROMPT = """You are sanity-checking column-role selections for a data pipeline, \
before it runs. You are NOT picking columns yourself -- only judging the roles a user already chose.

COLUMN PROFILES:
{profiles}

SELECTED ROLES:
{roles}

Role meanings:
- predict / target: the column being predicted or trained on. Should be a real value \
of interest (a measurement, an outcome) -- not an ID, not a date/time column, not a column \
whose name suggests it duplicates another selected column.
- exclude: a feature dropped from training. Any column is a reasonable choice here.
- datetime_col: the period/date column for time-series aggregation. Should genuinely be a \
date, timestamp, or period label -- not a numeric measurement.

Flag ONLY clear mismatches -- a column used for a role it plainly doesn't fit. Do not flag \
reasonable choices you're merely unsure about, and do not suggest alternative columns.

Respond in JSON:
{{
    "issues": [
        {{"column": "...", "role": "predict|target|exclude|datetime_col", \
"severity": "block|warn", "message": "one sentence, plain language"}}
    ]
}}
Use "block" only when the column is unmistakably the wrong type for the role (e.g. a raw ID \
as a forecast target). Use "warn" for anything softer. Empty "issues" list if everything looks fine.
"""


def _llm_validate(df: pd.DataFrame, roles: dict[str, list], provider) -> ValidationResult:
    from acdso.advisors import LLMConfig

    all_cols = sorted({c for cols in roles.values() for c in cols})
    profile = _profile_columns(df, all_cols)
    profile_lines = [
        f"- {col}: dtype={p['dtype']}, unique={p['n_unique']}/{p['n_rows']}, "
        f"null%={p['null_pct']}, sample={p['sample']}"
        for col, p in profile.items()
    ]
    role_lines = [f"- {role}: {cols}" for role, cols in roles.items() if cols]

    prompt = VALIDATION_PROMPT.format(
        profiles="\n".join(profile_lines) or "(none selected)",
        roles="\n".join(role_lines) or "(none selected)",
    )

    config = LLMConfig(max_tokens=500, temperature=0.0)
    response = provider.complete(
        system_prompt="You are a careful data pipeline reviewer. Be conservative -- "
                      "only flag genuine mismatches.",
        user_prompt=prompt,
        config=config,
    )

    text = response if isinstance(response, str) else str(response)
    start, end = text.find("{"), text.rfind("}") + 1
    if start < 0 or end <= start:
        raise ValueError("No JSON in LLM response")
    data = json.loads(text[start:end])

    # The LLM's severity claim is advisory only. Capping every LLM issue at
    # "warn" means a hallucinated or overly literal judgment (e.g. reading
    # a float column's stringified sample as "this is text") can never lock
    # out a selection on its own -- only the deterministic role-conflict
    # check above is allowed to block.
    issues = [
        ColumnIssue(
            column=i.get("column", ""),
            role=i.get("role", ""),
            severity="warn",
            message=i.get("message", ""),
        )
        for i in data.get("issues", [])
    ]
    return ValidationResult(ok=True, issues=issues, source="llm")


def _get_provider():
    """Reuse ACDSO's own provider selection: same env vars, same fallback order."""
    from acdso.advisors import AnthropicProvider, OllamaProvider, OpenAIProvider

    if os.environ.get("OPENAI_API_KEY"):
        p = OpenAIProvider(model="gpt-4o-mini")
        if p.is_available():
            return p
    if os.environ.get("ANTHROPIC_API_KEY"):
        p = AnthropicProvider(model="claude-3-haiku-20240307")
        if p.is_available():
            return p
    try:
        p = OllamaProvider(model="llama3.2")
        if p.is_available():
            return p
    except Exception:
        pass
    return None


def validate_columns(df: pd.DataFrame, roles: dict[str, list]) -> ValidationResult:
    """Validate column-role selections; combines rule-based + LLM checks.

    The rule-based conflict check (same column, two roles) always runs and
    is decisive on its own -- if it blocks, there is no ambiguity to hand
    to an LLM. Softer judgment (does this column's content actually fit
    its assigned role) is layered on top when a provider is configured.
    """
    result = _rule_based_validate(df, roles)
    if not result.ok:
        return result

    provider = _get_provider()
    if provider is None:
        return result

    try:
        llm_result = _llm_validate(df, roles, provider)
    except Exception:
        return result

    combined_issues = result.issues + llm_result.issues
    ok = not any(i.severity == "block" for i in combined_issues)
    return ValidationResult(ok=ok, issues=combined_issues, source="llm")


# ---------------------------------------------------------------------------
# Target suggestion -- which predictable column looks like the outcome
# ---------------------------------------------------------------------------

_OUTCOME_WORDS = (
    "total", "amount", "price", "revenue", "cost", "sales", "value", "score",
    "transfer", "count", "rate", "target", "label", "outcome", "result",
    "spend", "volume", "churn", "conversion", "profit", "quantity", "qty",
    "duration", "response",
)


def _rule_based_suggestion(df: pd.DataFrame, candidates: list[str]) -> str:
    """Score candidates by dtype + name; no LLM available, so keep it simple."""
    n = max(len(df), 1)
    best_col, best_score = candidates[0], -1.0
    for col in candidates:
        s = df[col]
        score = 0.0
        if pd.api.types.is_float_dtype(s):
            score += 2
        elif pd.api.types.is_numeric_dtype(s):
            score += 1
        if any(w in col.lower() for w in _OUTCOME_WORDS):
            score += 2
        score += min(s.nunique() / n, 1.0)  # more variation is mildly favored
        if score > best_score:
            best_col, best_score = col, score
    return best_col


SUGGEST_PROMPT = """A user is about to train a model on this dataset and needs to pick ONE \
column as the prediction target -- the outcome or measurement they actually care about.

CANDIDATE COLUMNS (already filtered to exclude dates and raw identifiers):
{profiles}

Pick the single column from the candidate list above that most plausibly represents an \
outcome of interest (a measurement, a result, a value someone would want to predict or \
forecast) -- not a category/dimension column used to describe or group rows.

Respond in JSON:
{{"column": "<one of the candidate names, exactly>", "reason": "one sentence, plain language"}}

If nothing looks like a clear outcome, still pick the most plausible one and say so honestly \
in the reason rather than refusing to answer.
"""


def _llm_suggest(df: pd.DataFrame, candidates: list[str], provider) -> dict[str, Any]:
    from acdso.advisors import LLMConfig

    profile = _profile_columns(df, candidates)
    lines = [
        f"- {col}: dtype={p['dtype']}, unique={p['n_unique']}/{p['n_rows']}, sample={p['sample']}"
        for col, p in profile.items()
    ]
    prompt = SUGGEST_PROMPT.format(profiles="\n".join(lines))

    config = LLMConfig(max_tokens=200, temperature=0.0)
    response = provider.complete(
        system_prompt="You are a data analyst helping pick a sensible prediction target. "
                      "Only ever choose from the given candidate list.",
        user_prompt=prompt,
        config=config,
    )

    text = response if isinstance(response, str) else str(response)
    start, end = text.find("{"), text.rfind("}") + 1
    if start < 0 or end <= start:
        raise ValueError("No JSON in LLM response")
    data = json.loads(text[start:end])

    column = data.get("column")
    if column not in candidates:
        raise ValueError(f"LLM suggested {column!r}, not one of the candidates")
    return {"column": column, "reason": data.get("reason", ""), "source": "llm"}


def suggest_target_column(df: pd.DataFrame, candidates: list[str] | None = None) -> dict[str, Any]:
    """Recommend one predictable column as the most plausible target.

    Tries the LLM first (same provider chain as validate_columns) for a
    judgment call informed by column names and content; falls back to a
    dtype + name-keyword heuristic if no provider is configured or the
    call fails. Always returns a column from ``candidates`` (never
    suggests a date/period or identifier column) -- returns None only if
    there are no predictable columns at all.

    Deliberately separate from run_acdso.py's own ``_llm_detect_target()``,
    which does the same job for any CLI invocation that leaves --target
    blank. This one exists because the GUI needed multi-provider support
    and pre-filtered candidates for a suggestion shown before the user has
    picked anything -- see docs/web_gui.md's "Target suggestion" section
    for the full comparison before assuming this is duplicate/dead code.
    """
    if candidates is None:
        candidates = predictable_columns(df)
    if not candidates:
        return {"column": None, "reason": "No predictable columns found in this file.", "source": "rules"}

    provider = _get_provider()
    if provider is not None:
        try:
            return _llm_suggest(df, candidates, provider)
        except Exception:
            pass

    column = _rule_based_suggestion(df, candidates)
    return {
        "column": column,
        "reason": f"'{column}' looks like the most measurement-like column among the "
                  f"predictable ones (numeric type and/or an outcome-sounding name).",
        "source": "rules",
    }
