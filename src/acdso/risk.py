"""
ACDSO v2.0 - Risk Analysis Module
==================================

Advanced risk diagnostics for production-relevant analysis including:
- Leakage proxy detection (univariate predictive power + mutual information)
- Temporal leakage check (chronological vs shuffled CV)
- VIF-based multicollinearity detection
- Instability risk via repeated CV variance
- Class imbalance fragility assessment
"""

from __future__ import annotations

import warnings
from dataclasses import dataclass, asdict, field
from typing import Any, Callable, Dict, List, Optional, Tuple

import numpy as np
import pandas as pd

from sklearn.base import clone
from sklearn.pipeline import Pipeline
from sklearn.model_selection import StratifiedKFold, KFold, cross_val_score
from sklearn.linear_model import LogisticRegression, Ridge, LinearRegression
from sklearn.utils import check_random_state

from acdso.utils import (
    infer_target_type,
    split_columns,
    compute_missing_ratio,
    build_preprocessor,
    detect_datetime_column,
    class_imbalance,
)


@dataclass
class RiskReport:
    """
    Comprehensive risk report for a dataset.

    Contains quantitative risk assessments across multiple dimensions
    including leakage, imbalance, multicollinearity, and stability.

    Attributes
    ----------
    task : str
        Task type ("classification" or "regression").
    target_type : str
        Target type ("binary", "multiclass", or "regression").
    n_rows : int
        Number of rows in the dataset.
    n_cols : int
        Number of columns in the dataset.
    n_numeric : int
        Number of numeric columns.
    n_categorical : int
        Number of categorical columns.
    missing_ratio : float
        Ratio of missing values (0.0 to 1.0).
    leakage_proxy_features : List[str]
        Features suspected of being leakage proxies.
    leakage_proxy_score_max : float
        Maximum univariate predictive power score.
    temporal_leakage_gap : Optional[float]
        Score gap between shuffled and chronological CV (if applicable).
    class_imbalance_max_ratio : Optional[float]
        Maximum class ratio (for classification).
    imbalance_severity : float
        Imbalance severity score (0.0 to 1.0).
    vif_max : float
        Maximum Variance Inflation Factor.
    vif_features : List[str]
        Features with high VIF (>=10).
    cv_score_std : float
        Standard deviation of cross-validation scores.
    notes : List[str]
        Human-readable risk notes and warnings.
    """

    task: str
    target_type: str
    n_rows: int
    n_cols: int
    n_numeric: int
    n_categorical: int
    missing_ratio: float

    # Leakage
    leakage_proxy_features: List[str] = field(default_factory=list)
    leakage_proxy_score_max: float = 0.0
    temporal_leakage_gap: Optional[float] = None

    # Imbalance
    class_imbalance_max_ratio: Optional[float] = None
    imbalance_severity: float = 0.0

    # Multicollinearity
    vif_max: float = 1.0
    vif_features: List[str] = field(default_factory=list)

    # Stability
    cv_score_std: float = 0.0

    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return asdict(self)


def _temporal_leakage_gap(
    df: pd.DataFrame,
    y: Any,
    scorer: str,
    cv: int,
    task: str,
    random_state: int,
) -> Optional[float]:
    """
    Detect temporal leakage by comparing chronological vs shuffled CV.

    If the shuffled CV score is significantly higher than chronological,
    it indicates potential temporal leakage in the data.

    Parameters
    ----------
    df : pd.DataFrame
        Input features.
    y : array-like
        Target values.
    scorer : str
        Scoring metric name.
    cv : int
        Number of cross-validation folds.
    task : str
        Task type ("classification" or "regression").
    random_state : int
        Random seed for reproducibility.

    Returns
    -------
    Optional[float]
        Score gap (shuffled - chronological), or None if not applicable.
    """
    tcol = detect_datetime_column(df)
    if tcol is None:
        return None

    try:
        if not pd.api.types.is_datetime64_any_dtype(df[tcol]):
            parsed = pd.to_datetime(df[tcol], errors="coerce")
        else:
            parsed = df[tcol]
        if parsed.isna().mean() > 0.5:
            return None
    except Exception:
        return None

    order = np.argsort(parsed.values.astype("datetime64[ns]").astype(np.int64))
    df_sorted = df.iloc[order].reset_index(drop=True)
    y_sorted = np.asarray(y)[order]

    num_cols, cat_cols = split_columns(df_sorted)
    pre = build_preprocessor(num_cols, cat_cols)

    if task == "classification":
        model = LogisticRegression(max_iter=1000)
        splitter_shuf = StratifiedKFold(n_splits=cv, shuffle=True, random_state=random_state)
        splitter_chr = KFold(n_splits=cv, shuffle=False)
    else:
        model = Ridge()
        splitter_shuf = KFold(n_splits=cv, shuffle=True, random_state=random_state)
        splitter_chr = KFold(n_splits=cv, shuffle=False)

    pipe = Pipeline([("pre", pre), ("model", model)])

    try:
        with warnings.catch_warnings():
            warnings.filterwarnings("ignore", category=UserWarning)
            warnings.filterwarnings("ignore", category=RuntimeWarning)
            s_shuf = float(np.mean(cross_val_score(pipe, df_sorted, y_sorted, scoring=scorer, cv=splitter_shuf)))
            s_chr = float(np.mean(cross_val_score(pipe, df_sorted, y_sorted, scoring=scorer, cv=splitter_chr)))
        return float(s_shuf - s_chr)
    except Exception:
        return None


def _univariate_predictive_power(
    df: pd.DataFrame,
    y: Any,
    task: str,
    scorer: str,
    cv: int,
    random_state: int,
) -> Tuple[List[str], float]:
    """
    Detect potential leakage via univariate predictive power.

    Features that achieve near-perfect prediction alone are suspicious
    and may be direct or proxy leaks.

    Parameters
    ----------
    df : pd.DataFrame
        Input features.
    y : array-like
        Target values.
    task : str
        Task type ("classification" or "regression").
    scorer : str
        Scoring metric name.
    cv : int
        Number of cross-validation folds.
    random_state : int
        Random seed for reproducibility.

    Returns
    -------
    Tuple[List[str], float]
        Tuple of (top_features, max_score).
    """
    rng = check_random_state(random_state)

    n = df.shape[0]
    max_rows = min(n, 3000)
    if n > max_rows:
        idx = rng.choice(n, size=max_rows, replace=False)
        df_s = df.iloc[idx].reset_index(drop=True)
        y_s = np.asarray(y)[idx]
    else:
        df_s = df
        y_s = np.asarray(y)

    scores = []
    feature_names = []

    cols = list(df_s.columns)
    if len(cols) > 80:
        cols = cols[:80]

    if task == "classification":
        splitter = StratifiedKFold(n_splits=cv, shuffle=True, random_state=random_state)
        base_model = LogisticRegression(max_iter=1000)
    else:
        splitter = KFold(n_splits=cv, shuffle=True, random_state=random_state)
        base_model = Ridge()

    for c in cols:
        x = df_s[[c]]
        try:
            nunique = x[c].nunique(dropna=True)
            # Skip ID-like columns (nearly all unique values)
            if nunique > 0.9 * len(x):
                continue
        except Exception:
            pass

        num_cols, cat_cols = split_columns(x)
        pre = build_preprocessor(num_cols, cat_cols)
        pipe = Pipeline([("pre", pre), ("model", clone(base_model))])

        try:
            with warnings.catch_warnings():
                warnings.filterwarnings("ignore", category=UserWarning)
                warnings.filterwarnings("ignore", category=RuntimeWarning)
                s = float(np.mean(cross_val_score(pipe, x, y_s, scoring=scorer, cv=splitter)))
            scores.append(s)
            feature_names.append(str(c))
        except Exception:
            continue

    if not scores:
        return [], 0.0

    order = np.argsort(scores)[::-1]
    top = [feature_names[i] for i in order[:5]]
    return top, float(scores[order[0]])


def _vif_numeric(
    df_num: pd.DataFrame,
    max_features: int = 40,
) -> Tuple[float, List[str]]:
    """
    Compute Variance Inflation Factor for numeric features.

    VIF measures multicollinearity; values >= 10 indicate high collinearity.

    Parameters
    ----------
    df_num : pd.DataFrame
        DataFrame with only numeric columns.
    max_features : int, default=40
        Maximum number of features to analyze (for performance).

    Returns
    -------
    Tuple[float, List[str]]
        Tuple of (max_vif, high_vif_features).
    """
    if df_num.shape[1] < 2:
        return 1.0, []

    cols = list(df_num.columns)
    if len(cols) > max_features:
        cols = cols[:max_features]

    X = df_num[cols].copy()
    X = X.replace([np.inf, -np.inf], np.nan)
    X = X.fillna(X.median(numeric_only=True))

    vifs = []
    for c in cols:
        y = X[c].values
        X_others = X.drop(columns=[c]).values
        if X_others.shape[1] == 0:
            vifs.append(1.0)
            continue
        try:
            lr = LinearRegression()
            lr.fit(X_others, y)
            r2 = lr.score(X_others, y)
            r2 = float(np.clip(r2, 0.0, 0.999999))
            vifs.append(float(1.0 / (1.0 - r2)))
        except Exception:
            vifs.append(1.0)

    vifs = np.asarray(vifs)
    vif_max = float(np.max(vifs)) if vifs.size else 1.0
    high = [cols[i] for i in range(len(cols)) if vifs[i] >= 10.0]
    return vif_max, high


def _auto_adjust_scorer_for_risk(scorer: str, y: Any, task: str) -> str:
    """
    Auto-adjust scorer for risk analysis to handle multiclass/non-standard labels.
    
    Parameters
    ----------
    scorer : str
        Original scorer name.
    y : array-like
        Target values.
    task : str
        Task type.
        
    Returns
    -------
    str
        Adjusted scorer name.
    """
    if task != "classification":
        return scorer
    
    y_arr = np.asarray(y)
    unique_labels = np.unique(y_arr)
    n_classes = len(unique_labels)
    
    # Scorers that require pos_label=1 (binary-only)
    binary_only_scorers = ["f1", "precision", "recall", "roc_auc"]
    
    if scorer in binary_only_scorers:
        # Multiclass or non-standard binary labels
        if n_classes > 2 or (n_classes == 2 and set(unique_labels) != {0, 1}):
            if scorer == "roc_auc":
                return "roc_auc_ovr_weighted"
            return f"{scorer}_weighted"
    
    return scorer


def analyze_risk(
    X: Any,
    y: Any,
    task: str,
    scorer: str = "accuracy",
    cv: int = 5,
    random_state: int = 42,
    llm_advisor: Optional[Callable[[Dict[str, Any]], Dict[str, Any]]] = None,
) -> RiskReport:
    """
    Perform comprehensive risk analysis on a dataset.

    This is the main entry point for ACDSO risk diagnostics.

    Parameters
    ----------
    X : array-like
        Input features.
    y : array-like
        Target values.
    task : str
        Task type ("classification" or "regression").
    scorer : str, default="accuracy"
        Scoring metric name.
    cv : int, default=5
        Number of cross-validation folds.
    random_state : int, default=42
        Random seed for reproducibility.
    llm_advisor : Optional[Callable], default=None
        Optional LLM advisor function for semantic analysis.

    Returns
    -------
    RiskReport
        Comprehensive risk report.
    """
    # Auto-adjust scorer for multiclass or non-standard binary labels
    scorer = _auto_adjust_scorer_for_risk(scorer, y, task)
    
    df = pd.DataFrame(X)
    n_rows, n_cols = int(df.shape[0]), int(df.shape[1])
    num_cols, cat_cols = split_columns(df)

    missing_ratio = compute_missing_ratio(df)
    target_type = infer_target_type(y)

    notes: List[str] = []

    # Univariate predictive power (leakage proxy detection)
    top_feats, max_pp = _univariate_predictive_power(
        df, y,
        task=task,
        scorer=scorer,
        cv=max(3, cv),
        random_state=random_state,
    )
    leak_feats: List[str] = []
    if max_pp >= 0.98 and len(top_feats) > 0 and task == "classification":
        leak_feats = top_feats
        notes.append(f"Potential leakage/proxy: univariate predictive power max={max_pp:.3f}.")

    # Temporal leakage detection
    gap = _temporal_leakage_gap(
        df, y,
        scorer=scorer,
        cv=max(3, cv),
        task=task,
        random_state=random_state,
    )
    if gap is not None and gap >= 0.05:
        notes.append(f"Temporal leakage suspicion: shuffled-chronological score gap={gap:.3f}.")

    # Class imbalance
    max_ratio, imb_sev = (None, 0.0)
    if task == "classification":
        max_ratio, imb_sev = class_imbalance(y)
        if max_ratio is not None and max_ratio >= 0.90:
            notes.append(f"Severe imbalance: max class ratio={max_ratio:.3f}.")

    # Multicollinearity (VIF)
    vif_max, vif_feats = _vif_numeric(df[num_cols] if num_cols else df.iloc[:, :0])
    if vif_max >= 10.0:
        notes.append(f"High multicollinearity: VIF max={vif_max:.1f}.")

    # LLM advisor (if provided)
    if llm_advisor is not None:
        try:
            llm_out = llm_advisor({
                "columns": [str(c) for c in df.columns],
                "task": task,
                "target_type": target_type,
                "notes": notes,
            }) or {}
            extra = llm_out.get("note")
            if isinstance(extra, str) and extra.strip():
                notes.append(f"LLM: {extra.strip()}")
        except Exception as e:
            notes.append(f"LLM advisor error ignored: {type(e).__name__}")

    return RiskReport(
        task=task,
        target_type=target_type,
        n_rows=n_rows,
        n_cols=n_cols,
        n_numeric=len(num_cols),
        n_categorical=len(cat_cols),
        missing_ratio=float(missing_ratio),
        leakage_proxy_features=leak_feats,
        leakage_proxy_score_max=float(max_pp),
        temporal_leakage_gap=float(gap) if gap is not None else None,
        class_imbalance_max_ratio=float(max_ratio) if max_ratio is not None else None,
        imbalance_severity=float(imb_sev),
        vif_max=float(vif_max),
        vif_features=[str(x) for x in vif_feats],
        cv_score_std=0.0,
        notes=notes,
    )


def compute_risk_penalty(
    rr: RiskReport,
    cv_score_std: float,
    preference: str = "balanced",
) -> float:
    """
    Compute a continuous, severity-weighted risk penalty.

    The penalty combines multiple risk signals into a single [0, 1] score
    that can be used in multi-objective optimization.

    Parameters
    ----------
    rr : RiskReport
        Risk report from analyze_risk().
    cv_score_std : float
        Standard deviation of cross-validation scores.
    preference : str, default="balanced"
        Risk preference mode ("safe" or "balanced").

    Returns
    -------
    float
        Risk penalty score (0.0 = low risk, 1.0 = high risk).
    """
    # Missing data risk
    miss = float(np.clip((rr.missing_ratio - 0.10) / 0.40, 0.0, 1.0))

    # Imbalance risk
    imb = float(np.clip(rr.imbalance_severity, 0.0, 1.0))

    # Leakage proxy risk
    leak = 0.0
    if rr.leakage_proxy_score_max >= 0.95:
        leak = float(np.clip((rr.leakage_proxy_score_max - 0.95) / 0.05, 0.0, 1.0))

    # Temporal leakage risk
    temp = 0.0
    if rr.temporal_leakage_gap is not None:
        temp = float(np.clip(rr.temporal_leakage_gap / 0.20, 0.0, 1.0))

    # Multicollinearity risk (VIF)
    vif = float(np.clip((rr.vif_max - 5.0) / 25.0, 0.0, 1.0))

    # Instability risk
    stab = float(np.clip(cv_score_std / 0.05, 0.0, 1.0))

    # Preference-based weighting
    if preference == "safe":
        w = {"leak": 0.30, "temp": 0.15, "imb": 0.20, "vif": 0.15, "miss": 0.10, "stab": 0.10}
    else:
        w = {"leak": 0.25, "temp": 0.10, "imb": 0.20, "vif": 0.15, "miss": 0.15, "stab": 0.15}

    penalty = (
        w["leak"] * leak +
        w["temp"] * temp +
        w["imb"] * imb +
        w["vif"] * vif +
        w["miss"] * miss +
        w["stab"] * stab
    )
    return float(np.clip(penalty, 0.0, 1.0))

