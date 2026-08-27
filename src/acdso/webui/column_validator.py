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


def _rule_based_scores(df: pd.DataFrame, candidates: list[str]) -> dict[str, float]:
    """Score candidates by dtype + name -- one independent 'vote' on target-ness."""
    n = max(len(df), 1)
    scores = {}
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
        scores[col] = score
    return scores


def _learnability_scores(
    df: pd.DataFrame, candidates: list[str], max_rows: int = 1500
) -> dict[str, dict[str, Any] | None]:
    """A second, independent 'vote': can each candidate actually be predicted
    from the *other* candidate columns? Not feature importance -- just a fast
    sanity check that a semantically-plausible target isn't pure noise given
    what else is in the file. Uses the sklearn tools that fit this job
    (a quick model + cross-validation), aimed at the target-selection
    question rather than feature selection.
    """
    from sklearn.ensemble import RandomForestClassifier, RandomForestRegressor
    from sklearn.model_selection import cross_val_score

    scores: dict[str, dict[str, Any] | None] = {}
    for target_col in candidates:
        feature_cols = [c for c in candidates if c != target_col]
        if not feature_cols:
            scores[target_col] = None
            continue
        try:
            work = df[[target_col] + feature_cols].dropna(subset=[target_col])
            if len(work) > max_rows:
                work = work.sample(max_rows, random_state=42)
            if len(work) < 20:
                scores[target_col] = None
                continue

            y = work[target_col]
            X = pd.get_dummies(work[feature_cols], dummy_na=True)
            if X.shape[1] == 0:
                scores[target_col] = None
                continue
            X = X.apply(lambda c: c.fillna(c.median()) if pd.api.types.is_numeric_dtype(c) else c)
            X = X.fillna(0)

            # A high-cardinality int column (a count, e.g.) is continuous in
            # spirit even though its dtype isn't float -- treating it as
            # classification would score "predict this exact integer out of
            # 1000+ classes" and produce a near-zero accuracy that looks
            # like noise but is really just the wrong metric.
            is_regression = pd.api.types.is_numeric_dtype(y) and y.nunique() > 20
            if is_regression:
                model = RandomForestRegressor(n_estimators=50, max_depth=6, random_state=42, n_jobs=-1)
                cv_scores = cross_val_score(model, X, y, cv=3, scoring="r2")
                scores[target_col] = {"metric": "R2", "score": float(cv_scores.mean())}
            else:
                model = RandomForestClassifier(n_estimators=50, max_depth=6, random_state=42, n_jobs=-1)
                cv_scores = cross_val_score(model, X, y, cv=3, scoring="accuracy")
                scores[target_col] = {"metric": "accuracy", "score": float(cv_scores.mean())}
        except Exception:
            scores[target_col] = None
    return scores


SUGGEST_PROMPT = """A user is about to train a model on this dataset and needs to pick a \
column (or a few) as the prediction target -- the outcome or measurement they actually care \
about. More than one column can be a reasonable target; rank the best candidates.

Two independent, mechanical methods have already voted on these candidates. Neither \
understands what the columns *mean* -- weigh their votes as evidence, not as the final answer, \
and use your own judgment of the column names/samples to decide where they're right and where \
they're misleading (e.g. a column that happens to be easy to predict from the others isn't \
automatically a good target if it's clearly a dimension/category column, not an outcome).

CANDIDATE COLUMNS AND VOTES:
{profiles}

- naming/dtype vote: a rank from a heuristic that favors numeric columns and outcome-sounding \
names (total, amount, price, revenue, etc). Rank 1 = most target-like by this heuristic alone.
- learnability vote: how well a quick model predicts this column from the OTHER candidate \
columns (R2 for continuous columns, accuracy for categorical). Higher = more learnable from \
the rest of the data. "n/a" means the probe couldn't run (too little data, etc) -- treat as no \
vote either way, not a negative signal.

Rank up to {top_n} columns by how plausibly each represents a genuine outcome of interest, \
using both the votes above AND the column names/samples. Best first.

Respond in JSON:
{{"suggestions": [
    {{"column": "<one of the candidate names, exactly>", "reason": "one sentence, referencing the votes where they influenced your ranking"}}
]}}

Only include columns that are genuinely plausible targets -- pad with less-likely candidates \
only if you have fewer than {top_n} good ones, and say so honestly in the reason rather than \
inventing a strong justification for a weak candidate.
"""


def _format_votes(
    candidates: list[str],
    profile: dict[str, dict[str, Any]],
    rule_scores: dict[str, float],
    learn_scores: dict[str, dict[str, Any] | None],
) -> str:
    rule_rank = {
        col: i + 1
        for i, col in enumerate(sorted(candidates, key=lambda c: rule_scores.get(c, 0), reverse=True))
    }
    lines = []
    for col in candidates:
        p = profile.get(col, {})
        learn = learn_scores.get(col)
        learn_text = f"{learn['metric']}={learn['score']:.2f}" if learn else "n/a"
        lines.append(
            f"- {col}: dtype={p.get('dtype')}, unique={p.get('n_unique')}/{p.get('n_rows')}, "
            f"sample={p.get('sample')} | naming/dtype vote: rank {rule_rank.get(col)} of "
            f"{len(candidates)} | learnability vote: {learn_text}"
        )
    return "\n".join(lines)


def _llm_suggest(
    df: pd.DataFrame,
    candidates: list[str],
    provider,
    top_n: int,
    rule_scores: dict[str, float],
    learn_scores: dict[str, dict[str, Any] | None],
) -> dict[str, Any]:
    from acdso.advisors import LLMConfig

    profile = _profile_columns(df, candidates)
    prompt = SUGGEST_PROMPT.format(
        profiles=_format_votes(candidates, profile, rule_scores, learn_scores),
        top_n=top_n,
    )

    config = LLMConfig(max_tokens=500, temperature=0.0)
    response = provider.complete(
        system_prompt="You are a data analyst deciding a prediction target by weighing "
                      "mechanical votes against your own judgment of what the columns mean. "
                      "Only ever choose from the given candidate list.",
        user_prompt=prompt,
        config=config,
    )

    text = response if isinstance(response, str) else str(response)
    start, end = text.find("{"), text.rfind("}") + 1
    if start < 0 or end <= start:
        raise ValueError("No JSON in LLM response")
    data = json.loads(text[start:end])

    suggestions = []
    seen = set()
    for item in data.get("suggestions", []):
        column = item.get("column")
        if column not in candidates or column in seen:
            continue
        seen.add(column)
        suggestions.append({
            "column": column,
            "reason": item.get("reason", ""),
            "votes": {"rule_based": rule_scores.get(column), "learnability": learn_scores.get(column)},
        })
    if not suggestions:
        raise ValueError("LLM returned no valid candidate columns")
    return {"suggestions": suggestions[:top_n], "source": "llm"}


def _combined_vote_ranking(
    candidates: list[str],
    rule_scores: dict[str, float],
    learn_scores: dict[str, dict[str, Any] | None],
    top_n: int,
) -> dict[str, Any]:
    """No LLM available: combine the two mechanical votes deterministically."""
    max_rule = max(rule_scores.values(), default=1.0) or 1.0
    learn_values = [v["score"] for v in learn_scores.values() if v]
    lo, hi = (min(learn_values), max(learn_values)) if learn_values else (0.0, 1.0)
    spread = (hi - lo) or 1.0

    def combined(col: str) -> float:
        rule_norm = rule_scores.get(col, 0) / max_rule
        learn = learn_scores.get(col)
        learn_norm = (learn["score"] - lo) / spread if learn else 0.5  # no data -> neutral
        return 0.5 * rule_norm + 0.5 * learn_norm

    ranked = sorted(candidates, key=combined, reverse=True)[:top_n]
    suggestions = []
    for col in ranked:
        learn = learn_scores.get(col)
        learn_text = f"{learn['metric']}={learn['score']:.2f}" if learn else "not testable"
        suggestions.append({
            "column": col,
            "reason": f"Combined vote: naming/dtype heuristic and learnability ({learn_text}) "
                      f"both weighed in -- '{col}' ranked highest of the two combined.",
            "votes": {"rule_based": rule_scores.get(col), "learnability": learn},
        })
    return {"suggestions": suggestions, "source": "rules"}


def suggest_target_columns(
    df: pd.DataFrame, candidates: list[str] | None = None, top_n: int = 3
) -> dict[str, Any]:
    """Rank up to ``top_n`` predictable columns as plausible targets.

    More than one column can be a reasonable target -- a user might train
    on any of several outcome-shaped columns, or pick more than one for a
    multi-target run (comma-separated --target/--predict). So this returns
    a ranked list, not a single answer; the caller decides how many to use.

    Two independent, mechanical "votes" are computed for every candidate
    first: a naming/dtype heuristic rank, and a learnability score (can a
    quick model predict this column from the *other* candidates -- not
    feature importance, just a sanity check that a plausible-looking
    target isn't pure noise). Those votes are then either handed to the
    LLM as evidence for it to weigh against its own semantic judgment
    (LLM configured), or combined deterministically (no LLM) -- either way
    the final ranking is a synthesis of both votes, not just one signal.

    Every returned column comes from ``candidates`` (never a date/period or
    identifier column) -- returns an empty list only if there are no
    predictable columns at all.

    Deliberately separate from run_acdso.py's own ``_llm_detect_target()``,
    which does the same job (single column, no voting) for any CLI
    invocation that leaves --target blank. See docs/web_gui.md's "Target
    suggestion" section for the full comparison before assuming this is
    duplicate/dead code.
    """
    if candidates is None:
        candidates = predictable_columns(df)
    if not candidates:
        return {"suggestions": [], "source": "rules"}

    rule_scores = _rule_based_scores(df, candidates)
    learn_scores = _learnability_scores(df, candidates)

    provider = _get_provider()
    if provider is not None:
        try:
            return _llm_suggest(df, candidates, provider, top_n, rule_scores, learn_scores)
        except Exception:
            pass

    return _combined_vote_ranking(candidates, rule_scores, learn_scores, top_n)


# ---------------------------------------------------------------------------
# Horizon suggestion -- same two-vote-plus-LLM pattern as target suggestion
# ---------------------------------------------------------------------------

#: Sensible default when no strong seasonality is detected, by aggregation grain.
_GRAIN_HORIZON_DEFAULTS = {"D": 7, "W": 4, "M": 3, "Q": 4, "Y": 1}


def _data_vote_horizon(series: pd.Series, grain: str) -> dict[str, Any]:
    """The 'data vote': one seasonal cycle if ACDSO's own autocorrelation
    detector finds one with reasonable strength, else a grain-typical
    default -- capped so the horizon never asks to forecast further than
    roughly a third of the observed history.
    """
    from acdso.timeseries.risk import detect_seasonality

    n = len(series)
    cap = max(1, n // 3)
    try:
        period, strength = detect_seasonality(series)
    except Exception:
        period, strength = None, 0.0

    if period and strength > 0.3:
        horizon = period
    else:
        horizon = _GRAIN_HORIZON_DEFAULTS.get(grain, 4)
    return {
        "horizon": max(1, min(horizon, cap)),
        "seasonality_period": period,
        "seasonality_strength": round(float(strength), 3),
        "series_length": n,
        "cap": cap,
    }


HORIZON_PROMPT = """A user is forecasting the column '{target_col}', aggregated to {grain} \
periods ({n} periods of history available after aggregation).

DATA VOTE (from autocorrelation-based seasonality detection on the actual series):
{data_vote_text}

Recommend a practical forecast horizon (a number of future periods) for this column. Weigh \
the data vote above against what a sensible planning horizon looks like for a column named \
'{target_col}' at {grain} grain -- e.g. a short lookout for an operational daily metric, a few \
quarters for something used in budgeting. The data vote is evidence, not the final answer; \
use your judgment of what this column represents to adjust it if the name suggests a \
different practical horizon.

Respond in JSON: {{"horizon": <positive integer>, "reason": "one sentence, plain language"}}

The horizon must not exceed {cap} periods (about a third of the available history) -- forecasts \
further out than that are unreliable regardless of what seems practical.
"""


def _llm_suggest_horizon(
    target_col: str, grain: str, data_vote: dict[str, Any], provider
) -> dict[str, Any]:
    from acdso.advisors import LLMConfig

    if data_vote["seasonality_period"]:
        data_vote_text = (
            f"seasonality period {data_vote['seasonality_period']} detected "
            f"(strength {data_vote['seasonality_strength']:.2f}); data-driven horizon: "
            f"{data_vote['horizon']} periods"
        )
    else:
        data_vote_text = (
            f"no strong seasonality detected in {data_vote['series_length']} periods; "
            f"data-driven horizon: {data_vote['horizon']} periods ({grain}-grain default)"
        )

    prompt = HORIZON_PROMPT.format(
        target_col=target_col, grain=grain, n=data_vote["series_length"],
        data_vote_text=data_vote_text, cap=data_vote["cap"],
    )
    config = LLMConfig(max_tokens=150, temperature=0.0)
    response = provider.complete(
        system_prompt="You are a forecasting analyst picking a practical horizon, weighing "
                      "statistical evidence against your own judgment of the column's meaning.",
        user_prompt=prompt,
        config=config,
    )

    text = response if isinstance(response, str) else str(response)
    start, end = text.find("{"), text.rfind("}") + 1
    if start < 0 or end <= start:
        raise ValueError("No JSON in LLM response")
    data = json.loads(text[start:end])

    horizon = int(data.get("horizon", 0))
    if horizon <= 0:
        raise ValueError(f"LLM returned a non-positive horizon: {horizon!r}")
    horizon = max(1, min(horizon, data_vote["cap"]))

    return {
        "horizon": horizon,
        "reason": data.get("reason", ""),
        "source": "llm",
        "grain": grain,
        "votes": {"data": data_vote},
    }


def suggest_horizon(df: pd.DataFrame, datetime_col: str, target_col: str) -> dict[str, Any]:
    """Recommend a forecast horizon for --horizon, combining a data vote
    (ACDSO's own seasonality detector on the aggregated series) with the
    LLM's judgment of what's a practical planning horizon for a column
    with this name -- the same two-vote-plus-synthesis pattern used for
    target suggestion, applied to a numeric parameter instead of a column
    choice.

    Reuses forecast.py's aggregation (same grain-detection the Forecast
    card uses) so the horizon lines up with however the data would
    actually be grouped for time-series work.
    """
    from .forecast import SeriesSpec, aggregate_series

    try:
        series, meta = aggregate_series(
            df, SeriesSpec(date_col=datetime_col, value_col=target_col, agg="sum")
        )
    except Exception as exc:
        return {"horizon": None, "reason": f"Could not aggregate the series: {exc}", "source": "rules"}

    if len(series) < 4:
        return {
            "horizon": None,
            "reason": f"Only {len(series)} period(s) after aggregation -- too few to suggest a horizon.",
            "source": "rules",
            "grain": meta.get("grain"),
        }

    grain = meta["grain"]
    data_vote = _data_vote_horizon(series, grain)

    provider = _get_provider()
    if provider is not None:
        try:
            return _llm_suggest_horizon(target_col, grain, data_vote, provider)
        except Exception:
            pass

    reason = (
        f"Seasonality period {data_vote['seasonality_period']} detected "
        f"(strength {data_vote['seasonality_strength']:.2f}) at {grain} grain."
        if data_vote["seasonality_period"]
        else f"No strong seasonality detected at {grain} grain; using a typical default for "
             f"this grain."
    )
    return {
        "horizon": data_vote["horizon"],
        "reason": reason,
        "source": "rules",
        "grain": grain,
        "votes": {"data": data_vote},
    }
