"""
ACDSO v2.0 - Utility Functions
==============================

Core utility functions for data preprocessing, type inference,
and column handling used throughout ACDSO.
"""

from __future__ import annotations

from typing import List, Optional, Tuple, Any

import numpy as np
import pandas as pd

from sklearn.pipeline import Pipeline
from sklearn.compose import ColumnTransformer
from sklearn.preprocessing import OneHotEncoder, StandardScaler
from sklearn.impute import SimpleImputer


def infer_target_type(y: Any) -> str:
    """
    Infer the target type from the target array.

    Parameters
    ----------
    y : array-like
        Target values.

    Returns
    -------
    str
        One of "binary", "multiclass", or "regression".
    """
    y_arr = np.asarray(y)
    if np.issubdtype(y_arr.dtype, np.floating):
        # Check if it's actually discrete floats (like 0.0, 1.0)
        unique_vals = np.unique(y_arr)
        if len(unique_vals) <= 20 and np.allclose(unique_vals, unique_vals.astype(int)):
            if len(unique_vals) == 2:
                return "binary"
            return "multiclass"
        return "regression"
    uniques = np.unique(y_arr)
    if len(uniques) == 2:
        return "binary"
    return "multiclass"


def split_columns(df: pd.DataFrame) -> Tuple[List[str], List[str]]:
    """
    Split DataFrame columns into numeric and categorical columns.

    Parameters
    ----------
    df : pd.DataFrame
        Input DataFrame.

    Returns
    -------
    Tuple[List[str], List[str]]
        Tuple of (numeric_columns, categorical_columns).
    """
    num_cols = [c for c in df.columns if pd.api.types.is_numeric_dtype(df[c])]
    cat_cols = [c for c in df.columns if c not in num_cols]
    return num_cols, cat_cols


def compute_missing_ratio(df: pd.DataFrame) -> float:
    """
    Compute the ratio of missing values in a DataFrame.

    Parameters
    ----------
    df : pd.DataFrame
        Input DataFrame.

    Returns
    -------
    float
        Ratio of missing values (0.0 to 1.0).
    """
    total = df.shape[0] * df.shape[1]
    return float(df.isna().sum().sum() / total) if total else 0.0


def build_preprocessor(
    numeric_cols: List[str],
    categorical_cols: List[str],
    sparse_threshold: float = 0.3,
) -> ColumnTransformer:
    """
    Build a preprocessing ColumnTransformer for numeric and categorical columns.

    The preprocessor applies:
    - Numeric: Median imputation + StandardScaler (without centering for sparse compatibility)
    - Categorical: Mode imputation + OneHotEncoder

    Parameters
    ----------
    numeric_cols : List[str]
        List of numeric column names.
    categorical_cols : List[str]
        List of categorical column names.
    sparse_threshold : float, default=0.3
        Threshold for sparse matrix output (0.0-1.0).

    Returns
    -------
    ColumnTransformer
        Configured preprocessing transformer.
    """
    num_pipe = Pipeline([
        ("imputer", SimpleImputer(strategy="median")),
        ("scaler", StandardScaler(with_mean=False)),
    ])

    cat_pipe = Pipeline([
        ("imputer", SimpleImputer(strategy="most_frequent")),
        ("onehot", OneHotEncoder(handle_unknown="ignore")),
    ])

    return ColumnTransformer(
        transformers=[
            ("num", num_pipe, numeric_cols),
            ("cat", cat_pipe, categorical_cols),
        ],
        remainder="drop",
        sparse_threshold=sparse_threshold,
    )


def detect_datetime_column(df: pd.DataFrame) -> Optional[str]:
    """
    Detect a datetime-like column in the DataFrame.

    First checks for actual datetime64 columns, then falls back to
    word-boundary name heuristics (date, time, timestamp, etc.),
    skipping numeric columns to avoid false positives like
    ``sentiment_score_control`` matching on "time".

    Parameters
    ----------
    df : pd.DataFrame
        Input DataFrame.

    Returns
    -------
    Optional[str]
        Name of detected datetime column, or None if not found.
    """
    import re

    # First, check for actual datetime columns
    for c in df.columns:
        if pd.api.types.is_datetime64_any_dtype(df[c]):
            return str(c)

    # Fall back to word-boundary name heuristics, skipping numeric columns
    _dt_pattern = re.compile(
        r'(?<![a-z])(?:date|time|timestamp|ts|datetime|created|updated|week|period)(?![a-z])',
        re.IGNORECASE,
    )
    name_hits = [
        c for c in df.columns
        if _dt_pattern.search(str(c)) and not pd.api.types.is_numeric_dtype(df[c])
    ]
    return str(name_hits[0]) if name_hits else None


def class_imbalance(y: Any) -> Tuple[Optional[float], float]:
    """
    Compute class imbalance metrics.

    Parameters
    ----------
    y : array-like
        Target values.

    Returns
    -------
    Tuple[Optional[float], float]
        Tuple of (max_class_ratio, imbalance_severity).
        - max_class_ratio: Ratio of the most common class (0.0 to 1.0)
        - imbalance_severity: Severity score (0.0 = balanced, 1.0 = severely imbalanced)
    """
    y_arr = np.asarray(y)
    values, counts = np.unique(y_arr, return_counts=True)
    if len(counts) == 0:
        return None, 0.0
    max_ratio = float(np.max(counts) / np.sum(counts))
    # Severity: 0 when max_ratio=0.5 (balanced), 1 when max_ratio=1.0 (one class)
    severity = float(np.clip((max_ratio - 0.5) / 0.5, 0.0, 1.0))
    return max_ratio, severity


def measure_latency_ms_per_row(
    pipeline: Pipeline,
    X: Any,
    n_rows: int = 256,
) -> float:
    """
    Measure inference latency in milliseconds per row.

    Parameters
    ----------
    pipeline : Pipeline
        Fitted sklearn Pipeline.
    X : array-like
        Input data for prediction.
    n_rows : int, default=256
        Number of rows to use for measurement.

    Returns
    -------
    float
        Latency in milliseconds per row.
    """
    import time

    try:
        if hasattr(X, "iloc"):
            Xs = X.iloc[:n_rows]
            rows = min(n_rows, len(Xs))
        else:
            Xs = X[:n_rows]
            rows = min(n_rows, len(Xs))

        if rows <= 0:
            return 0.0

        t0 = time.time()
        _ = pipeline.predict(Xs)
        t1 = time.time()

        return float((t1 - t0) * 1000.0 / rows)
    except Exception:
        return float("inf")

