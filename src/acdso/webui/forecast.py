"""Period aggregation and short-series forecasting for the web GUI.

Why this exists
---------------
``AutoTS`` selects a model but cannot extrapolate: ``AutoTSResult`` exposes
``predict(X)``, which needs future feature rows, and there is no
``forecast(horizon)`` anywhere in the package.  So the autonomous report can
show a leaderboard but never actual future periods.

The other half of the problem is grain.  Vendor exports are usually one row
per (period, entity, action) -- a JFrog transfer export is 4,135 rows across
13 months.  Fitting on that grain asks "how many bytes did this one row
move", which is not the question anyone has, and it scores negative R2.  The
question is "how much will we move next month", and that needs the rows
collapsed to one value per period first.

So: aggregate to the period grain, then forecast with methods that are
honest about a 13-point series.  Nothing here is clever, deliberately -- on a
dozen points a transparent trend you can defend in a meeting beats a
boosted-tree score nobody can explain.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

import numpy as np
import pandas as pd

#: Below this many periods, a fitted trend is not meaningfully better than
#: eyeballing it, and we say so in the warnings.
MIN_POINTS_FOR_TREND = 6


@dataclass
class SeriesSpec:
    """How to collapse a raw export into one value per period."""

    date_col: str
    value_col: str
    agg: str = "sum"  # sum | mean | max | count
    grain: str = "auto"  # auto | D | W | M | Q | Y
    group_filters: dict[str, list[str]] = field(default_factory=dict)
    drop_first: bool = False
    drop_last: bool = False


def _infer_grain(dates: pd.Series) -> str:
    """Guess the period grain from the parsed dates."""
    unique = dates.dropna().drop_duplicates().sort_values()
    if len(unique) < 2:
        return "M"
    # All values on the first of a month, one per month -> monthly export.
    if (unique.dt.day == 1).all() and unique.dt.to_period("M").nunique() == len(unique):
        return "M"
    deltas = unique.diff().dropna().dt.days
    median = float(deltas.median())
    if median <= 1.5:
        return "D"
    if median <= 9:
        return "W"
    if median <= 45:
        return "M"
    if median <= 135:
        return "Q"
    return "Y"


def aggregate_series(df: pd.DataFrame, spec: SeriesSpec) -> tuple[pd.Series, dict[str, Any]]:
    """Collapse ``df`` to one ``value_col`` figure per period.

    Returns the series (PeriodIndex) and metadata describing what was done,
    including how many raw rows fed each period so partial periods are
    visible rather than silently blended into the trend.
    """
    missing = [c for c in (spec.date_col, spec.value_col) if c not in df.columns]
    if missing:
        raise ValueError(f"Column(s) not in the file: {', '.join(missing)}")

    work = df.copy()
    applied_filters = {}
    for column, allowed in (spec.group_filters or {}).items():
        if column in work.columns and allowed:
            work = work[work[column].astype(str).isin([str(a) for a in allowed])]
            applied_filters[column] = allowed
    if work.empty:
        raise ValueError("No rows left after filtering.")

    work[spec.value_col] = pd.to_numeric(work[spec.value_col], errors="coerce")
    parsed = pd.to_datetime(work[spec.date_col], errors="coerce")
    if parsed.isna().all():
        raise ValueError(
            f"Could not read '{spec.date_col}' as dates. "
            "Pick the column holding the period (e.g. Date)."
        )
    work = work.assign(_parsed=parsed).dropna(subset=["_parsed", spec.value_col])

    grain = _infer_grain(work["_parsed"]) if spec.grain == "auto" else spec.grain
    periods = work["_parsed"].dt.to_period(grain)

    grouped = work.groupby(periods)[spec.value_col]
    series = grouped.count() if spec.agg == "count" else getattr(grouped, spec.agg)()
    row_counts = grouped.size()
    series = series.sort_index()
    row_counts = row_counts.reindex(series.index)

    # Fill gaps so the trend sees real spacing, not compressed time.
    if len(series) > 1:
        full = pd.period_range(series.index.min(), series.index.max(), freq=grain)
        series = series.reindex(full, fill_value=0.0)
        row_counts = row_counts.reindex(full, fill_value=0)

    dropped = []
    if spec.drop_first and len(series) > 2:
        dropped.append(str(series.index[0]))
        series, row_counts = series.iloc[1:], row_counts.iloc[1:]
    if spec.drop_last and len(series) > 2:
        dropped.append(str(series.index[-1]))
        series, row_counts = series.iloc[:-1], row_counts.iloc[:-1]

    meta = {
        "grain": grain,
        "agg": spec.agg,
        "rows_in": int(len(df)),
        "rows_used": int(len(work)),
        "periods": int(len(series)),
        "row_counts": {str(p): int(c) for p, c in row_counts.items()},
        "dropped_periods": dropped,
        "filters": applied_filters,
    }
    return series.astype(float), meta


# ---------------------------------------------------------------------------
# Forecasters -- each maps history -> the next ``h`` values
# ---------------------------------------------------------------------------


def _naive(y: np.ndarray, h: int) -> np.ndarray:
    return np.repeat(y[-1], h)


def _mean_last3(y: np.ndarray, h: int) -> np.ndarray:
    return np.repeat(y[-min(3, len(y)):].mean(), h)


def _drift(y: np.ndarray, h: int) -> np.ndarray:
    if len(y) < 2:
        return _naive(y, h)
    slope = (y[-1] - y[0]) / (len(y) - 1)
    return y[-1] + slope * np.arange(1, h + 1)


def _linear_trend(y: np.ndarray, h: int) -> np.ndarray:
    if len(y) < 3:
        return _drift(y, h)
    x = np.arange(len(y), dtype=float)
    slope, intercept = np.polyfit(x, y, 1)
    return intercept + slope * np.arange(len(y), len(y) + h)


def _trend_last6(y: np.ndarray, h: int) -> np.ndarray:
    """Linear trend on the recent window -- ignores an old, flatter regime."""
    window = y[-min(6, len(y)):]
    if len(window) < 3:
        return _drift(y, h)
    x = np.arange(len(window), dtype=float)
    slope, intercept = np.polyfit(x, window, 1)
    return intercept + slope * np.arange(len(window), len(window) + h)


FORECASTERS: dict[str, Any] = {
    "last value": _naive,
    "mean of last 3": _mean_last3,
    "drift (first to last)": _drift,
    "linear trend (all)": _linear_trend,
    "linear trend (last 6)": _trend_last6,
}


def _backtest(y: np.ndarray, fn: Any) -> float | None:
    """One-step-ahead expanding-window MAE. ``None`` if too short to test."""
    n = len(y)
    folds = min(4, n - 3)
    if folds < 1:
        return None
    errors = []
    for k in range(folds):
        cut = n - folds + k
        pred = fn(y[:cut], 1)[0]
        errors.append(abs(pred - y[cut]))
    return float(np.mean(errors))


def forecast_series(series: pd.Series, horizon: int = 3) -> dict[str, Any]:
    """Forecast ``horizon`` periods, choosing the method by backtest MAE."""
    y = series.to_numpy(dtype=float)
    if len(y) < 3:
        raise ValueError(
            f"Only {len(y)} period(s) after aggregation -- need at least 3 to forecast. "
            "Check the date column and the aggregation grain."
        )

    scored = []
    for name, fn in FORECASTERS.items():
        mae = _backtest(y, fn)
        scored.append({"method": name, "mae": mae, "values": fn(y, horizon).tolist()})

    testable = [s for s in scored if s["mae"] is not None]
    chosen = min(testable, key=lambda s: s["mae"]) if testable else scored[0]

    # Negative volume is meaningless; a downward-fitted trend can produce it.
    values = [max(0.0, v) for v in chosen["values"]]
    clipped = any(v < 0 for v in chosen["values"])

    last_period = series.index[-1]
    future = [str(last_period + i) for i in range(1, horizon + 1)]

    warnings = []
    if len(y) < MIN_POINTS_FOR_TREND:
        warnings.append(
            f"Only {len(y)} periods of history -- treat this as a direction, not a number."
        )
    if not testable:
        warnings.append("Series too short to backtest; showing the simplest method untested.")
    if clipped:
        warnings.append("Fitted trend went negative and was clipped to zero.")
    recent = y[-min(3, len(y)):].mean()
    earlier = y[: max(1, len(y) - 3)].mean()
    if earlier > 0 and recent / earlier > 2:
        warnings.append(
            f"Recent periods average {recent / earlier:.1f}x the earlier ones -- there is a "
            "level shift in this series, so a single trend line understates the recent regime."
        )
    warnings.append(
        "First and last periods are partial if the export window does not align to "
        "period boundaries. Use the drop-edge options to exclude them."
    )

    return {
        "history": [{"period": str(p), "value": float(v)} for p, v in series.items()],
        "forecast": [{"period": p, "value": v} for p, v in zip(future, values)],
        "chosen_method": chosen["method"],
        "chosen_mae": chosen["mae"],
        "methods": [
            {"method": s["method"], "mae": s["mae"],
             "values": [max(0.0, v) for v in s["values"]]}
            for s in scored
        ],
        "history_total": float(y.sum()),
        "forecast_total": float(sum(values)),
        "warnings": warnings,
    }


def run_forecast(df: pd.DataFrame, spec: SeriesSpec, horizon: int = 3) -> dict[str, Any]:
    """Aggregate then forecast, returning both halves plus provenance."""
    series, meta = aggregate_series(df, spec)
    result = forecast_series(series, horizon)
    result["aggregation"] = meta
    result["value_col"] = spec.value_col
    result["date_col"] = spec.date_col
    return result
