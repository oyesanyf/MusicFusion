"""
ACDSO v2.0 - Auto EDA Module
=============================

Production-grade Automatic Exploratory Data Analysis with multi-objective optimization.

This module applies the same algorithmic approach as ACDSO AutoML:
- Multi-objective evaluation (quality, thoroughness, speed)
- Risk-aware analysis (data quality issues, anomalies)
- Pareto optimization for analysis trade-offs
- Comprehensive audit trail for reproducibility

Key Features:
- Statistical profiling (distributions, correlations, outliers)
- Data quality assessment (missing values, duplicates, cardinality)
- Anomaly detection
- Feature relationship analysis
- Visualization recommendations
- LLM-powered insights (optional)
- Integration with popular EDA tools (ydata-profiling, sweetviz, dataprep)

Integrations:
    - ydata-profiling (formerly pandas-profiling): Full dataset profiling
    - sweetviz: Comparison and target analysis
    - dataprep: Fast EDA with visualizations
    - great_expectations: Data validation
    - D-Tale: Interactive exploration

Basic Usage:
    >>> from acdso import ACDSOAutoEDA
    >>> eda = ACDSOAutoEDA(verbose=1)
    >>> report = eda.analyze(df)
    >>> eda.print_summary()
    
With External Tools:
    >>> eda = ACDSOAutoEDA(backend="ydata-profiling")
    >>> report = eda.analyze(df)
    >>> eda.to_html("report.html")

Standalone Functions:
    >>> from acdso.eda import quick_profile, compare_datasets, validate_data
    >>> profile = quick_profile(df)
    >>> comparison = compare_datasets(df1, df2)
"""

from __future__ import annotations

import json
import math
import os
import time
import warnings
from abc import ABC, abstractmethod
from dataclasses import dataclass, field, asdict
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple, Union, Literal

import numpy as np
import pandas as pd
from scipy import stats

from acdso.utils import split_columns, compute_missing_ratio


# ============================================================================
# Backend Detection
# ============================================================================

def _check_ydata_profiling() -> bool:
    """Check if ydata-profiling is available."""
    try:
        import ydata_profiling
        return True
    except ImportError:
        try:
            import pandas_profiling
            return True
        except ImportError:
            return False

def _check_sweetviz() -> bool:
    """Check if sweetviz is available."""
    try:
        import sweetviz
        return True
    except ImportError:
        return False

def _check_dataprep() -> bool:
    """Check if dataprep is available."""
    try:
        from dataprep.eda import create_report
        return True
    except ImportError:
        return False

def _check_great_expectations() -> bool:
    """Check if great_expectations is available."""
    try:
        import great_expectations
        return True
    except ImportError:
        return False

def _check_dtale() -> bool:
    """Check if D-Tale is available."""
    try:
        import dtale
        return True
    except ImportError:
        return False


# Available backends
_AVAILABLE_BACKENDS: Dict[str, bool] = {}

def get_available_backends() -> Dict[str, bool]:
    """
    Get dictionary of available EDA backends.
    
    Returns
    -------
    Dict[str, bool]
        Dictionary mapping backend names to availability status.
    
    Example
    -------
    >>> from acdso.eda import get_available_backends
    >>> backends = get_available_backends()
    >>> print(backends)
    {'native': True, 'ydata-profiling': True, 'sweetviz': False, ...}
    """
    global _AVAILABLE_BACKENDS
    if not _AVAILABLE_BACKENDS:
        _AVAILABLE_BACKENDS = {
            "native": True,  # Always available
            "ydata-profiling": _check_ydata_profiling(),
            "sweetviz": _check_sweetviz(),
            "dataprep": _check_dataprep(),
            "great_expectations": _check_great_expectations(),
            "dtale": _check_dtale(),
        }
    return _AVAILABLE_BACKENDS.copy()


class EDABackend(str, Enum):
    """Supported EDA backends."""
    NATIVE = "native"
    YDATA_PROFILING = "ydata-profiling"
    SWEETVIZ = "sweetviz"
    DATAPREP = "dataprep"
    GREAT_EXPECTATIONS = "great_expectations"
    DTALE = "dtale"


# ============================================================================
# Data Classes for EDA Results
# ============================================================================

@dataclass
class ColumnProfile:
    """Profile for a single column."""
    name: str
    dtype: str
    n_unique: int
    n_missing: int
    missing_pct: float
    
    # For numeric columns
    mean: Optional[float] = None
    std: Optional[float] = None
    min_val: Optional[float] = None
    max_val: Optional[float] = None
    median: Optional[float] = None
    q25: Optional[float] = None
    q75: Optional[float] = None
    skewness: Optional[float] = None
    kurtosis: Optional[float] = None
    n_zeros: Optional[int] = None
    n_negative: Optional[int] = None
    n_outliers_iqr: Optional[int] = None
    n_outliers_zscore: Optional[int] = None
    
    # For categorical columns
    top_value: Optional[str] = None
    top_value_count: Optional[int] = None
    top_value_pct: Optional[float] = None
    cardinality_ratio: Optional[float] = None
    
    # Quality flags
    is_constant: bool = False
    is_id_like: bool = False
    is_highly_skewed: bool = False
    has_outliers: bool = False
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return asdict(self)


@dataclass
class CorrelationResult:
    """Correlation analysis result."""
    feature_1: str
    feature_2: str
    correlation: float
    p_value: Optional[float] = None
    method: str = "pearson"
    
    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass 
class AnomalyResult:
    """Anomaly detection result."""
    row_index: int
    anomaly_score: float
    anomaly_type: str
    affected_columns: List[str]
    description: str
    
    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class DataQualityReport:
    """Comprehensive data quality report."""
    n_rows: int
    n_cols: int
    n_numeric: int
    n_categorical: int
    n_datetime: int
    
    # Missing data
    total_missing: int
    total_missing_pct: float
    cols_with_missing: int
    cols_all_missing: int
    
    # Duplicates
    n_duplicate_rows: int
    duplicate_pct: float
    
    # Constants
    n_constant_cols: int
    constant_cols: List[str]
    
    # High cardinality
    n_id_like_cols: int
    id_like_cols: List[str]
    
    # Memory
    memory_mb: float
    
    # Quality score (0-100)
    quality_score: float
    
    # Issues and recommendations
    issues: List[str] = field(default_factory=list)
    recommendations: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class EDACandidate:
    """
    Represents an EDA analysis candidate with multi-objective metrics.
    
    Similar to CandidateResult in AutoML, but for EDA analysis types.
    """
    name: str
    analysis_type: str
    
    # Metrics
    quality_score: float       # How thorough/accurate the analysis is (0-1)
    compute_time_s: float      # How long the analysis took
    coverage: float            # What fraction of data was analyzed (0-1)
    insight_count: int         # Number of insights generated
    
    # Multi-objective vector
    objectives: Tuple[float, ...]
    
    # Results
    results: Dict[str, Any] = field(default_factory=dict)
    meta: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d['objectives'] = list(self.objectives)
        return d


@dataclass
class EDAReport:
    """
    Complete EDA report with all analysis results.
    
    JSON-serializable for audit trail and reproducibility.
    """
    # Dataset overview
    dataset_name: str
    n_rows: int
    n_cols: int
    analysis_timestamp: str
    
    # Profiling
    column_profiles: List[Dict[str, Any]]
    data_quality: Dict[str, Any]
    
    # Correlations
    high_correlations: List[Dict[str, Any]]
    correlation_matrix: Optional[Dict[str, Dict[str, float]]] = None
    
    # Anomalies
    anomalies: List[Dict[str, Any]] = field(default_factory=list)
    
    # Statistics
    statistical_summary: Dict[str, Any] = field(default_factory=dict)
    
    # Insights (AI Information if LLM used)
    insights: List[str] = field(default_factory=list)
    
    # Recommendations
    recommendations: List[str] = field(default_factory=list)
    
    # Performance
    total_analysis_time_s: float = 0.0
    
    # Pareto optimization results
    pareto_front: List[Dict[str, Any]] = field(default_factory=list)
    selected_analyses: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# ============================================================================
# EDA Analysis Functions
# ============================================================================

def _profile_numeric_column(series: pd.Series, name: str) -> ColumnProfile:
    """Profile a numeric column."""
    # Convert boolean to int to avoid numpy issues with quantile/stats
    if series.dtype == 'bool' or series.dtype == 'boolean':
        series = series.astype(int)
    
    clean = series.dropna()
    n_total = len(series)
    n_missing = int(series.isna().sum())
    
    # Also convert clean series if it's boolean
    if clean.dtype == 'bool' or clean.dtype == 'boolean':
        clean = clean.astype(int)
    
    if len(clean) == 0:
        return ColumnProfile(
            name=name,
            dtype=str(series.dtype),
            n_unique=0,
            n_missing=n_missing,
            missing_pct=100.0,
            is_constant=True,
        )
    
    # Basic stats
    mean_val = float(clean.mean())
    std_val = float(clean.std()) if len(clean) > 1 else 0.0
    min_val = float(clean.min())
    max_val = float(clean.max())
    median_val = float(clean.median())
    q25 = float(clean.quantile(0.25))
    q75 = float(clean.quantile(0.75))
    
    # Distribution stats
    try:
        skew = float(stats.skew(clean))
        kurt = float(stats.kurtosis(clean))
    except Exception:
        skew, kurt = 0.0, 0.0
    
    # Counts
    n_unique = int(clean.nunique())
    n_zeros = int((clean == 0).sum())
    n_negative = int((clean < 0).sum())
    
    # Outliers (IQR method)
    iqr = q75 - q25
    lower_bound = q25 - 1.5 * iqr
    upper_bound = q75 + 1.5 * iqr
    n_outliers_iqr = int(((clean < lower_bound) | (clean > upper_bound)).sum())
    
    # Outliers (Z-score method)
    if std_val > 0:
        z_scores = np.abs((clean - mean_val) / std_val)
        n_outliers_zscore = int((z_scores > 3).sum())
    else:
        n_outliers_zscore = 0
    
    return ColumnProfile(
        name=name,
        dtype=str(series.dtype),
        n_unique=n_unique,
        n_missing=n_missing,
        missing_pct=round(n_missing / n_total * 100, 2) if n_total > 0 else 0.0,
        mean=round(mean_val, 6),
        std=round(std_val, 6),
        min_val=round(min_val, 6),
        max_val=round(max_val, 6),
        median=round(median_val, 6),
        q25=round(q25, 6),
        q75=round(q75, 6),
        skewness=round(skew, 4),
        kurtosis=round(kurt, 4),
        n_zeros=n_zeros,
        n_negative=n_negative,
        n_outliers_iqr=n_outliers_iqr,
        n_outliers_zscore=n_outliers_zscore,
        is_constant=n_unique == 1,
        is_id_like=n_unique == n_total - n_missing,
        is_highly_skewed=abs(skew) > 2.0,
        has_outliers=n_outliers_iqr > 0,
    )


def _profile_categorical_column(series: pd.Series, name: str) -> ColumnProfile:
    """Profile a categorical column."""
    n_total = len(series)
    n_missing = int(series.isna().sum())
    clean = series.dropna()
    
    if len(clean) == 0:
        return ColumnProfile(
            name=name,
            dtype=str(series.dtype),
            n_unique=0,
            n_missing=n_missing,
            missing_pct=100.0,
            is_constant=True,
        )
    
    n_unique = int(clean.nunique())
    value_counts = clean.value_counts()
    top_value = str(value_counts.index[0]) if len(value_counts) > 0 else None
    top_count = int(value_counts.iloc[0]) if len(value_counts) > 0 else 0
    
    return ColumnProfile(
        name=name,
        dtype=str(series.dtype),
        n_unique=n_unique,
        n_missing=n_missing,
        missing_pct=round(n_missing / n_total * 100, 2) if n_total > 0 else 0.0,
        top_value=top_value,
        top_value_count=top_count,
        top_value_pct=round(top_count / len(clean) * 100, 2) if len(clean) > 0 else 0.0,
        cardinality_ratio=round(n_unique / len(clean), 4) if len(clean) > 0 else 0.0,
        is_constant=n_unique == 1,
        is_id_like=n_unique >= 0.95 * (n_total - n_missing),
    )


def _compute_correlations(
    df: pd.DataFrame,
    numeric_cols: List[str],
    threshold: float = 0.7,
    method: str = "pearson",
) -> Tuple[List[CorrelationResult], Dict[str, Dict[str, float]]]:
    """Compute correlations between numeric columns."""
    if len(numeric_cols) < 2:
        return [], {}
    
    # Limit columns for performance
    cols = numeric_cols[:50]
    df_num = df[cols].copy()
    
    # Compute correlation matrix
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        if method == "pearson":
            corr_matrix = df_num.corr(method="pearson")
        elif method == "spearman":
            corr_matrix = df_num.corr(method="spearman")
        else:
            corr_matrix = df_num.corr()
    
    # Convert to dictionary
    corr_dict = {}
    for col in corr_matrix.columns:
        corr_dict[str(col)] = {str(idx): float(corr_matrix.loc[idx, col]) 
                               for idx in corr_matrix.index}
    
    # Find high correlations
    high_corrs = []
    for i, c1 in enumerate(cols):
        for j, c2 in enumerate(cols):
            if i < j:
                corr_val = corr_matrix.loc[c1, c2]
                if pd.notna(corr_val) and abs(corr_val) >= threshold:
                    # Compute p-value
                    try:
                        clean_idx = df_num[[c1, c2]].dropna().index
                        if len(clean_idx) > 2:
                            _, p_val = stats.pearsonr(
                                df_num.loc[clean_idx, c1],
                                df_num.loc[clean_idx, c2]
                            )
                        else:
                            p_val = None
                    except Exception:
                        p_val = None
                    
                    high_corrs.append(CorrelationResult(
                        feature_1=str(c1),
                        feature_2=str(c2),
                        correlation=round(float(corr_val), 4),
                        p_value=round(float(p_val), 6) if p_val is not None else None,
                        method=method,
                    ))
    
    # Sort by absolute correlation
    high_corrs.sort(key=lambda x: abs(x.correlation), reverse=True)
    return high_corrs, corr_dict


def _detect_anomalies(
    df: pd.DataFrame,
    numeric_cols: List[str],
    max_anomalies: int = 100,
) -> List[AnomalyResult]:
    """Detect anomalies using statistical methods."""
    anomalies = []
    
    if not numeric_cols:
        return anomalies
    
    # Use subset of columns for performance
    cols = numeric_cols[:20]
    df_num = df[cols].copy()
    
    # Convert boolean columns to int to avoid numpy issues
    for col in df_num.columns:
        if df_num[col].dtype == 'bool' or df_num[col].dtype == 'boolean':
            df_num[col] = df_num[col].astype(int)
    
    # Z-score based anomalies
    for col in cols:
        series = df_num[col].dropna()
        if len(series) < 10:
            continue
        
        mean_val = series.mean()
        std_val = series.std()
        
        if std_val == 0:
            continue
        
        z_scores = np.abs((df_num[col] - mean_val) / std_val)
        extreme_idx = z_scores[z_scores > 4].index.tolist()[:10]
        
        for idx in extreme_idx:
            if len(anomalies) >= max_anomalies:
                break
            anomalies.append(AnomalyResult(
                row_index=int(idx),
                anomaly_score=round(float(z_scores.loc[idx]), 2),
                anomaly_type="extreme_value",
                affected_columns=[str(col)],
                description=f"Value {df_num.loc[idx, col]:.4g} is {z_scores.loc[idx]:.1f} std from mean",
            ))
    
    # Multi-variate anomalies (rows with multiple outliers)
    if len(cols) >= 3:
        df_scaled = (df_num - df_num.mean()) / df_num.std()
        df_scaled = df_scaled.fillna(0)
        
        # Mahalanobis-like: count of extreme values per row
        extreme_counts = (np.abs(df_scaled) > 3).sum(axis=1)
        multi_anomaly_idx = extreme_counts[extreme_counts >= 3].index.tolist()[:20]
        
        for idx in multi_anomaly_idx:
            if len(anomalies) >= max_anomalies:
                break
            affected = [str(c) for c in cols if abs(df_scaled.loc[idx, c]) > 3]
            anomalies.append(AnomalyResult(
                row_index=int(idx),
                anomaly_score=round(float(extreme_counts.loc[idx]), 2),
                anomaly_type="multivariate_outlier",
                affected_columns=affected,
                description=f"Row has extreme values in {len(affected)} columns",
            ))
    
    return anomalies


def _assess_data_quality(
    df: pd.DataFrame,
    profiles: List[ColumnProfile],
) -> DataQualityReport:
    """Assess overall data quality."""
    n_rows, n_cols = df.shape
    num_cols, cat_cols = split_columns(df)
    
    # Datetime columns
    dt_cols = [c for c in df.columns if pd.api.types.is_datetime64_any_dtype(df[c])]
    
    # Missing data
    total_missing = int(df.isna().sum().sum())
    cols_with_missing = int((df.isna().sum() > 0).sum())
    cols_all_missing = int((df.isna().sum() == n_rows).sum())
    
    # Duplicates
    try:
        n_dups = int(df.duplicated().sum())
    except Exception:
        n_dups = 0
    
    # Constants and ID-like
    constant_cols = [p.name for p in profiles if p.is_constant]
    id_like_cols = [p.name for p in profiles if p.is_id_like and not p.is_constant]
    
    # Memory
    try:
        memory_mb = float(df.memory_usage(deep=True).sum() / (1024 * 1024))
    except Exception:
        memory_mb = 0.0
    
    # Quality score calculation (0-100)
    # Penalize: missing data, duplicates, constant columns, etc.
    score = 100.0
    
    # Missing data penalty (up to -30 points)
    missing_pct = total_missing / (n_rows * n_cols) if n_rows * n_cols > 0 else 0
    score -= min(30, missing_pct * 100)
    
    # Duplicate penalty (up to -20 points)
    dup_pct = n_dups / n_rows if n_rows > 0 else 0
    score -= min(20, dup_pct * 50)
    
    # Constant columns penalty (up to -15 points)
    const_pct = len(constant_cols) / n_cols if n_cols > 0 else 0
    score -= min(15, const_pct * 30)
    
    # ID-like columns penalty (up to -10 points)
    id_pct = len(id_like_cols) / n_cols if n_cols > 0 else 0
    score -= min(10, id_pct * 20)
    
    score = max(0, min(100, score))
    
    # Issues and recommendations
    issues = []
    recommendations = []
    
    if missing_pct > 0.05:
        issues.append(f"High missing data: {missing_pct:.1%} of values are missing")
        recommendations.append("Consider imputation strategies or removing columns with >50% missing")
    
    if dup_pct > 0.01:
        issues.append(f"Duplicate rows detected: {n_dups} ({dup_pct:.1%})")
        recommendations.append("Review and potentially remove duplicate rows")
    
    if constant_cols:
        issues.append(f"Constant columns found: {constant_cols[:5]}")
        recommendations.append("Remove constant columns as they provide no predictive value")
    
    if id_like_cols:
        issues.append(f"ID-like columns detected: {id_like_cols[:5]}")
        recommendations.append("Exclude ID columns from modeling or use for grouping only")
    
    # Check for highly skewed columns
    skewed_cols = [p.name for p in profiles if p.skewness is not None and abs(p.skewness) > 2]
    if skewed_cols:
        issues.append(f"Highly skewed columns: {skewed_cols[:5]}")
        recommendations.append("Consider log or Box-Cox transformation for skewed features")
    
    # Check for columns with many outliers
    outlier_cols = [p.name for p in profiles 
                    if p.n_outliers_iqr is not None and p.n_outliers_iqr > 0.1 * n_rows]
    if outlier_cols:
        issues.append(f"Columns with many outliers: {outlier_cols[:5]}")
        recommendations.append("Investigate outliers; consider winsorization or robust scaling")
    
    return DataQualityReport(
        n_rows=n_rows,
        n_cols=n_cols,
        n_numeric=len(num_cols),
        n_categorical=len(cat_cols),
        n_datetime=len(dt_cols),
        total_missing=total_missing,
        total_missing_pct=round(missing_pct * 100, 2),
        cols_with_missing=cols_with_missing,
        cols_all_missing=cols_all_missing,
        n_duplicate_rows=n_dups,
        duplicate_pct=round(dup_pct * 100, 2),
        n_constant_cols=len(constant_cols),
        constant_cols=constant_cols,
        n_id_like_cols=len(id_like_cols),
        id_like_cols=id_like_cols,
        memory_mb=round(memory_mb, 2),
        quality_score=round(score, 1),
        issues=issues,
        recommendations=recommendations,
    )


def _compute_statistical_summary(
    df: pd.DataFrame,
    profiles: List[ColumnProfile],
) -> Dict[str, Any]:
    """Compute overall statistical summary."""
    num_profiles = [p for p in profiles if p.mean is not None]
    cat_profiles = [p for p in profiles if p.top_value is not None]
    
    summary = {
        "numeric_features": {
            "count": len(num_profiles),
            "total_missing": sum(p.n_missing for p in num_profiles),
            "avg_missing_pct": np.mean([p.missing_pct for p in num_profiles]) if num_profiles else 0,
            "n_with_zeros": sum(1 for p in num_profiles if p.n_zeros and p.n_zeros > 0),
            "n_with_negatives": sum(1 for p in num_profiles if p.n_negative and p.n_negative > 0),
            "n_with_outliers": sum(1 for p in num_profiles if p.has_outliers),
            "n_highly_skewed": sum(1 for p in num_profiles if p.is_highly_skewed),
        },
        "categorical_features": {
            "count": len(cat_profiles),
            "total_unique_values": sum(p.n_unique for p in cat_profiles),
            "avg_cardinality": np.mean([p.n_unique for p in cat_profiles]) if cat_profiles else 0,
            "n_high_cardinality": sum(1 for p in cat_profiles 
                                      if p.cardinality_ratio and p.cardinality_ratio > 0.5),
        },
        "constants_and_ids": {
            "n_constant": sum(1 for p in profiles if p.is_constant),
            "n_id_like": sum(1 for p in profiles if p.is_id_like),
        },
    }
    
    return summary


# ============================================================================
# Pareto Optimization for EDA
# ============================================================================

def _eda_pareto_front(candidates: List[EDACandidate]) -> List[EDACandidate]:
    """
    Compute Pareto front for EDA candidates.
    
    Objectives (all to maximize):
    - Quality score
    - Coverage
    - Insight count (normalized)
    - Negative compute time (minimize time = maximize -time)
    """
    if not candidates:
        return []
    
    front = []
    for cand in candidates:
        dominated = False
        for other in candidates:
            if other.name == cand.name:
                continue
            # Check if 'other' dominates 'cand'
            # other dominates if better or equal in all objectives, strictly better in at least one
            better_eq = (
                other.quality_score >= cand.quality_score and
                other.coverage >= cand.coverage and
                other.insight_count >= cand.insight_count and
                other.compute_time_s <= cand.compute_time_s
            )
            strictly_better = (
                other.quality_score > cand.quality_score or
                other.coverage > cand.coverage or
                other.insight_count > cand.insight_count or
                other.compute_time_s < cand.compute_time_s
            )
            if better_eq and strictly_better:
                dominated = True
                break
        
        if not dominated:
            front.append(cand)
    
    return front


def _eda_knee_point(candidates: List[EDACandidate]) -> EDACandidate:
    """Select knee-point from EDA Pareto front."""
    if len(candidates) == 1:
        return candidates[0]
    
    # Score based on combined objectives
    scores = []
    for c in candidates:
        # Weighted combination favoring quality and coverage
        score = (
            0.4 * c.quality_score +
            0.3 * c.coverage +
            0.2 * (c.insight_count / max(1, max(x.insight_count for x in candidates))) +
            0.1 * (1.0 - c.compute_time_s / max(0.001, max(x.compute_time_s for x in candidates)))
        )
        scores.append(score)
    
    best_idx = int(np.argmax(scores))
    return candidates[best_idx]


# ============================================================================
# Main Auto EDA Class
# ============================================================================

class ACDSOAutoEDA:
    """
    ACDSO v2.0 Auto EDA - Risk-Optimized Exploratory Data Analysis.
    
    A production-oriented Auto EDA that uses multi-objective optimization
    to balance analysis thoroughness, speed, and insight quality.
    
    Parameters
    ----------
    correlation_threshold : float, default=0.7
        Minimum correlation to flag as "high correlation".
    
    outlier_method : str, default="iqr"
        Method for outlier detection ("iqr", "zscore", or "both").
    
    max_columns : int, default=100
        Maximum columns to profile in detail.
    
    max_correlations : int, default=50
        Maximum correlations to compute (for performance).
    
    max_anomalies : int, default=100
        Maximum anomalies to return.
    
    time_budget_s : float, optional
        Maximum time budget for analysis.
    
    llm_advisor : callable, optional
        Optional LLM advisor for generating insights.
    
    verbose : int, default=0
        Verbosity level (0=silent, 1=progress, 2=detailed).
    
    Attributes
    ----------
    report_ : EDAReport
        Complete EDA report after calling analyze().
    
    column_profiles_ : List[ColumnProfile]
        Column profiles after analysis.
    
    data_quality_ : DataQualityReport
        Data quality assessment.
    
    pareto_front_ : List[EDACandidate]
        Pareto-optimal analysis candidates.
    
    Examples
    --------
    >>> from acdso import ACDSOAutoEDA
    >>> eda = ACDSOAutoEDA(verbose=1)
    >>> report = eda.analyze(df)
    >>> print(report.data_quality)
    >>> print(report.recommendations)
    """
    
    def __init__(
        self,
        correlation_threshold: float = 0.7,
        outlier_method: str = "iqr",
        max_columns: int = 100,
        max_correlations: int = 50,
        max_anomalies: int = 100,
        time_budget_s: Optional[float] = None,
        llm_advisor: Optional[Callable[[Dict[str, Any]], Dict[str, Any]]] = None,
        verbose: int = 0,
    ) -> None:
        self.correlation_threshold = correlation_threshold
        self.outlier_method = outlier_method
        self.max_columns = max_columns
        self.max_correlations = max_correlations
        self.max_anomalies = max_anomalies
        self.time_budget_s = time_budget_s
        self.llm_advisor = llm_advisor
        self.verbose = verbose
        
        # Results (populated after analyze)
        self.report_: Optional[EDAReport] = None
        self.column_profiles_: List[ColumnProfile] = []
        self.data_quality_: Optional[DataQualityReport] = None
        self.pareto_front_: List[EDACandidate] = []
        self.all_candidates_: List[EDACandidate] = []
    
    def _log(self, msg: str, level: int = 1) -> None:
        """Log message if verbose level is sufficient."""
        if self.verbose >= level:
            print(f"[ACDSO AutoEDA] {msg}")
    
    def analyze(
        self,
        data: Union[pd.DataFrame, np.ndarray],
        dataset_name: str = "dataset",
    ) -> EDAReport:
        """
        Perform comprehensive EDA on the dataset.
        
        Parameters
        ----------
        data : pd.DataFrame or np.ndarray
            Input data to analyze.
        dataset_name : str, default="dataset"
            Name for the dataset (for reporting).
        
        Returns
        -------
        EDAReport
            Comprehensive EDA report.
        """
        t_start = time.time()
        timestamp = pd.Timestamp.now().isoformat()
        
        # Convert to DataFrame if needed
        if isinstance(data, np.ndarray):
            df = pd.DataFrame(data, columns=[f"col_{i}" for i in range(data.shape[1])])
        else:
            df = data.copy()
        
        self._log(f"Starting analysis of '{dataset_name}' ({df.shape[0]} rows, {df.shape[1]} cols)")
        
        candidates: List[EDACandidate] = []
        
        # =====================================================================
        # Analysis 1: Column Profiling
        # =====================================================================
        self._log("Running column profiling...", level=1)
        t0 = time.time()
        
        num_cols, cat_cols = split_columns(df)
        profiles: List[ColumnProfile] = []
        
        # Profile columns (up to max_columns)
        cols_to_profile = list(df.columns)[:self.max_columns]
        for col in cols_to_profile:
            if col in num_cols:
                profiles.append(_profile_numeric_column(df[col], str(col)))
            else:
                profiles.append(_profile_categorical_column(df[col], str(col)))
        
        profiling_time = time.time() - t0
        profiling_insights = sum([
            sum(1 for p in profiles if p.is_constant),
            sum(1 for p in profiles if p.is_id_like),
            sum(1 for p in profiles if p.has_outliers),
            sum(1 for p in profiles if p.is_highly_skewed),
            sum(1 for p in profiles if p.missing_pct > 10),
        ])
        
        candidates.append(EDACandidate(
            name="column_profiling",
            analysis_type="profiling",
            quality_score=0.95,
            compute_time_s=profiling_time,
            coverage=len(cols_to_profile) / len(df.columns) if len(df.columns) > 0 else 1.0,
            insight_count=profiling_insights,
            objectives=(0.95, len(cols_to_profile) / max(1, len(df.columns)), 
                       profiling_insights, -profiling_time),
            results={"n_profiles": len(profiles)},
        ))
        
        self.column_profiles_ = profiles
        
        # =====================================================================
        # Analysis 2: Data Quality Assessment
        # =====================================================================
        self._log("Assessing data quality...", level=1)
        t0 = time.time()
        
        quality_report = _assess_data_quality(df, profiles)
        quality_time = time.time() - t0
        
        candidates.append(EDACandidate(
            name="data_quality",
            analysis_type="quality",
            quality_score=0.90,
            compute_time_s=quality_time,
            coverage=1.0,
            insight_count=len(quality_report.issues) + len(quality_report.recommendations),
            objectives=(0.90, 1.0, len(quality_report.issues), -quality_time),
            results={"quality_score": quality_report.quality_score},
        ))
        
        self.data_quality_ = quality_report
        
        # =====================================================================
        # Analysis 3: Correlation Analysis
        # =====================================================================
        self._log("Computing correlations...", level=1)
        t0 = time.time()
        
        high_corrs, corr_matrix = _compute_correlations(
            df, num_cols,
            threshold=self.correlation_threshold,
            method="pearson",
        )
        corr_time = time.time() - t0
        
        candidates.append(EDACandidate(
            name="correlation_analysis",
            analysis_type="correlation",
            quality_score=0.85 if len(num_cols) >= 2 else 0.0,
            compute_time_s=corr_time,
            coverage=min(1.0, len(num_cols) / self.max_correlations) if num_cols else 0.0,
            insight_count=len(high_corrs),
            objectives=(0.85, min(1.0, len(num_cols) / 50), len(high_corrs), -corr_time),
            results={"n_high_correlations": len(high_corrs)},
        ))
        
        # =====================================================================
        # Analysis 4: Anomaly Detection
        # =====================================================================
        self._log("Detecting anomalies...", level=1)
        t0 = time.time()
        
        anomalies = _detect_anomalies(df, num_cols, max_anomalies=self.max_anomalies)
        anomaly_time = time.time() - t0
        
        candidates.append(EDACandidate(
            name="anomaly_detection",
            analysis_type="anomaly",
            quality_score=0.80,
            compute_time_s=anomaly_time,
            coverage=min(1.0, len(num_cols) / 20) if num_cols else 0.0,
            insight_count=len(anomalies),
            objectives=(0.80, min(1.0, len(num_cols) / 20), len(anomalies), -anomaly_time),
            results={"n_anomalies": len(anomalies)},
        ))
        
        # =====================================================================
        # Analysis 5: Statistical Summary
        # =====================================================================
        self._log("Computing statistical summary...", level=1)
        t0 = time.time()
        
        stat_summary = _compute_statistical_summary(df, profiles)
        stat_time = time.time() - t0
        
        candidates.append(EDACandidate(
            name="statistical_summary",
            analysis_type="statistics",
            quality_score=0.90,
            compute_time_s=stat_time,
            coverage=1.0,
            insight_count=sum(len(v) if isinstance(v, dict) else 1 for v in stat_summary.values()),
            objectives=(0.90, 1.0, 10, -stat_time),
            results=stat_summary,
        ))
        
        self.all_candidates_ = candidates
        
        # =====================================================================
        # Pareto Optimization
        # =====================================================================
        self._log("Computing Pareto front...", level=1)
        front = _eda_pareto_front(candidates)
        self.pareto_front_ = front
        
        # Select best analyses (knee-point)
        if front:
            best = _eda_knee_point(front)
            selected_analyses = [c.name for c in front]
            self._log(f"Pareto front size: {len(front)}, Best: {best.name}")
        else:
            selected_analyses = [c.name for c in candidates]
        
        # =====================================================================
        # LLM Insights (if advisor provided)
        # =====================================================================
        insights = []
        if self.llm_advisor is not None:
            self._log("Generating LLM insights...", level=1)
            try:
                llm_input = {
                    "dataset_name": dataset_name,
                    "n_rows": int(df.shape[0]),
                    "n_cols": int(df.shape[1]),
                    "columns": [str(c) for c in df.columns],
                    "quality_score": quality_report.quality_score,
                    "issues": quality_report.issues,
                    "high_correlations": [c.to_dict() for c in high_corrs[:10]],
                    "n_anomalies": len(anomalies),
                }
                llm_out = self.llm_advisor(llm_input) or {}
                
                if "insights" in llm_out:
                    raw_insights = llm_out["insights"]
                    if isinstance(raw_insights, list):
                        insights = [f"[AI Information] {i}" for i in raw_insights]
                    elif isinstance(raw_insights, str):
                        insights = [f"[AI Information] {raw_insights}"]
                
                if "note" in llm_out and llm_out["note"]:
                    insights.append(f"[AI Information] {llm_out['note']}")
                    
            except Exception as e:
                self._log(f"LLM advisor error: {e}", level=2)
        
        # =====================================================================
        # Generate Recommendations
        # =====================================================================
        recommendations = list(quality_report.recommendations)
        
        if high_corrs:
            recommendations.append(
                f"Consider removing or combining highly correlated features "
                f"({len(high_corrs)} pairs with |r| >= {self.correlation_threshold})"
            )
        
        if anomalies:
            recommendations.append(
                f"Investigate {len(anomalies)} detected anomalies before modeling"
            )
        
        # =====================================================================
        # Build Final Report
        # =====================================================================
        total_time = time.time() - t_start
        
        report = EDAReport(
            dataset_name=dataset_name,
            n_rows=int(df.shape[0]),
            n_cols=int(df.shape[1]),
            analysis_timestamp=timestamp,
            column_profiles=[p.to_dict() for p in profiles],
            data_quality=quality_report.to_dict(),
            high_correlations=[c.to_dict() for c in high_corrs],
            correlation_matrix=corr_matrix if len(corr_matrix) <= 30 else None,
            anomalies=[a.to_dict() for a in anomalies],
            statistical_summary=stat_summary,
            insights=insights,
            recommendations=recommendations,
            total_analysis_time_s=round(total_time, 3),
            pareto_front=[c.to_dict() for c in front],
            selected_analyses=selected_analyses,
        )
        
        self.report_ = report
        
        self._log(f"Analysis complete in {total_time:.2f}s")
        self._log(f"Quality Score: {quality_report.quality_score:.1f}/100")
        self._log(f"Issues Found: {len(quality_report.issues)}")
        self._log(f"Recommendations: {len(recommendations)}")
        
        return report
    
    def print_summary(self) -> None:
        """Print a formatted summary of the EDA results."""
        if self.report_ is None:
            print("No analysis performed yet. Call analyze() first.")
            return
        
        r = self.report_
        q = self.data_quality_
        
        print("\n" + "=" * 70)
        print(f"  ACDSO AUTO EDA REPORT: {r.dataset_name}")
        print("=" * 70)
        
        # Dataset Overview
        print("\n[DATASET OVERVIEW]")
        print(f"  • Rows: {r.n_rows:,}")
        print(f"  • Columns: {r.n_cols}")
        print(f"  • Numeric: {q.n_numeric}")
        print(f"  • Categorical: {q.n_categorical}")
        print(f"  • Datetime: {q.n_datetime}")
        print(f"  • Memory: {q.memory_mb:.2f} MB")
        
        # Data Quality
        print("\n[DATA QUALITY]")
        print(f"  ★ Quality Score: {q.quality_score:.1f}/100")
        print(f"  • Missing Values: {q.total_missing:,} ({q.total_missing_pct:.1f}%)")
        print(f"  • Duplicate Rows: {q.n_duplicate_rows:,} ({q.duplicate_pct:.1f}%)")
        print(f"  • Constant Columns: {q.n_constant_cols}")
        print(f"  • ID-like Columns: {q.n_id_like_cols}")
        
        # Issues
        if q.issues:
            print("\n[ISSUES DETECTED]")
            for i, issue in enumerate(q.issues, 1):
                print(f"  ⚠ {i}. {issue}")
        
        # High Correlations
        if r.high_correlations:
            print(f"\n[HIGH CORRELATIONS] (top 5 of {len(r.high_correlations)})")
            for corr in r.high_correlations[:5]:
                print(f"  • {corr['feature_1']} ↔ {corr['feature_2']}: r={corr['correlation']:.3f}")
        
        # Anomalies
        if r.anomalies:
            print(f"\n[ANOMALIES DETECTED] ({len(r.anomalies)} total)")
            for anom in r.anomalies[:5]:
                print(f"  • Row {anom['row_index']}: {anom['description']}")
            if len(r.anomalies) > 5:
                print(f"  ... and {len(r.anomalies) - 5} more")
        
        # Recommendations
        if r.recommendations:
            print("\n[RECOMMENDATIONS]")
            for i, rec in enumerate(r.recommendations, 1):
                print(f"  {i}. {rec}")
        
        # AI Insights
        if r.insights:
            print("\n[AI INSIGHTS]")
            for insight in r.insights:
                print(f"  • {insight}")
        
        # Performance
        print(f"\n[ANALYSIS PERFORMANCE]")
        print(f"  • Total Time: {r.total_analysis_time_s:.2f}s")
        print(f"  • Pareto Front: {len(r.pareto_front)} analyses")
        
        print("\n" + "=" * 70)
    
    def get_column_profile(self, column_name: str) -> Optional[ColumnProfile]:
        """Get profile for a specific column."""
        for p in self.column_profiles_:
            if p.name == column_name:
                return p
        return None
    
    def to_dict(self) -> Dict[str, Any]:
        """Export report as dictionary."""
        if self.report_ is None:
            return {}
        return self.report_.to_dict()
    
    def to_json(self, filepath: str) -> None:
        """Save report to JSON file."""
        with open(filepath, "w") as f:
            json.dump(self.to_dict(), f, indent=2)
    
    def to_html(self, filepath: str) -> None:
        """
        Export report as HTML file.
        
        Parameters
        ----------
        filepath : str
            Path to save HTML report.
        """
        if self.report_ is None:
            raise RuntimeError("No analysis performed yet. Call analyze() first.")
        
        html = self._generate_html_report()
        with open(filepath, "w", encoding="utf-8") as f:
            f.write(html)
    
    def _generate_html_report(self) -> str:
        """Generate HTML report from EDA results."""
        r = self.report_
        q = self.data_quality_
        
        html = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ACDSO Auto EDA Report - {r.dataset_name}</title>
    <style>
        :root {{
            --bg-primary: #0d1117;
            --bg-secondary: #161b22;
            --text-primary: #c9d1d9;
            --text-secondary: #8b949e;
            --accent: #58a6ff;
            --success: #3fb950;
            --warning: #d29922;
            --error: #f85149;
        }}
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.6;
            padding: 2rem;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        h1, h2, h3 {{ color: var(--accent); margin-bottom: 1rem; }}
        h1 {{ font-size: 2rem; border-bottom: 2px solid var(--accent); padding-bottom: 0.5rem; }}
        h2 {{ font-size: 1.5rem; margin-top: 2rem; }}
        .card {{
            background: var(--bg-secondary);
            border-radius: 8px;
            padding: 1.5rem;
            margin: 1rem 0;
            border: 1px solid #30363d;
        }}
        .metric {{
            display: inline-block;
            background: var(--bg-primary);
            padding: 0.5rem 1rem;
            border-radius: 4px;
            margin: 0.25rem;
        }}
        .metric-label {{ color: var(--text-secondary); font-size: 0.875rem; }}
        .metric-value {{ font-size: 1.25rem; font-weight: bold; }}
        .quality-score {{
            font-size: 3rem;
            font-weight: bold;
            color: var(--success);
        }}
        .issue {{ color: var(--warning); margin: 0.5rem 0; }}
        .recommendation {{ color: var(--text-secondary); margin: 0.5rem 0; padding-left: 1rem; }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin: 1rem 0;
        }}
        th, td {{
            padding: 0.75rem;
            text-align: left;
            border-bottom: 1px solid #30363d;
        }}
        th {{ background: var(--bg-primary); color: var(--accent); }}
        tr:hover {{ background: rgba(88, 166, 255, 0.1); }}
        .footer {{
            margin-top: 2rem;
            padding-top: 1rem;
            border-top: 1px solid #30363d;
            color: var(--text-secondary);
            font-size: 0.875rem;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🔍 ACDSO Auto EDA Report</h1>
        <p style="color: var(--text-secondary);">Dataset: <strong>{r.dataset_name}</strong> | Generated: {r.analysis_timestamp}</p>
        
        <div class="card">
            <h2>📊 Dataset Overview</h2>
            <div class="metric">
                <div class="metric-label">Rows</div>
                <div class="metric-value">{r.n_rows:,}</div>
            </div>
            <div class="metric">
                <div class="metric-label">Columns</div>
                <div class="metric-value">{r.n_cols}</div>
            </div>
            <div class="metric">
                <div class="metric-label">Numeric</div>
                <div class="metric-value">{q.n_numeric}</div>
            </div>
            <div class="metric">
                <div class="metric-label">Categorical</div>
                <div class="metric-value">{q.n_categorical}</div>
            </div>
            <div class="metric">
                <div class="metric-label">Memory</div>
                <div class="metric-value">{q.memory_mb:.1f} MB</div>
            </div>
        </div>
        
        <div class="card">
            <h2>⭐ Data Quality Score</h2>
            <div class="quality-score">{q.quality_score:.0f}/100</div>
            <div style="margin-top: 1rem;">
                <div class="metric">
                    <div class="metric-label">Missing Values</div>
                    <div class="metric-value">{q.total_missing:,} ({q.total_missing_pct:.1f}%)</div>
                </div>
                <div class="metric">
                    <div class="metric-label">Duplicates</div>
                    <div class="metric-value">{q.n_duplicate_rows:,} ({q.duplicate_pct:.1f}%)</div>
                </div>
                <div class="metric">
                    <div class="metric-label">Constant Cols</div>
                    <div class="metric-value">{q.n_constant_cols}</div>
                </div>
            </div>
        </div>
        
        {"<div class='card'><h2>⚠️ Issues Detected</h2>" + "".join(f"<div class='issue'>• {issue}</div>" for issue in q.issues) + "</div>" if q.issues else ""}
        
        {"<div class='card'><h2>📈 High Correlations</h2><table><tr><th>Feature 1</th><th>Feature 2</th><th>Correlation</th></tr>" + "".join(f"<tr><td>{c['feature_1']}</td><td>{c['feature_2']}</td><td>{c['correlation']:.4f}</td></tr>" for c in r.high_correlations[:10]) + "</table></div>" if r.high_correlations else ""}
        
        <div class="card">
            <h2>💡 Recommendations</h2>
            {"".join(f"<div class='recommendation'>{i}. {rec}</div>" for i, rec in enumerate(r.recommendations, 1)) if r.recommendations else "<p>No specific recommendations.</p>"}
        </div>
        
        <div class="card">
            <h2>📋 Column Profiles</h2>
            <table>
                <tr>
                    <th>Column</th>
                    <th>Type</th>
                    <th>Missing %</th>
                    <th>Unique</th>
                    <th>Flags</th>
                </tr>
                {"".join(f"<tr><td>{p['name']}</td><td>{p['dtype']}</td><td>{p['missing_pct']:.1f}%</td><td>{p['n_unique']}</td><td>{'⚠️ Constant' if p.get('is_constant') else ''}{'🔑 ID-like' if p.get('is_id_like') else ''}{'📊 Skewed' if p.get('is_highly_skewed') else ''}</td></tr>" for p in r.column_profiles[:50])}
            </table>
        </div>
        
        <div class="footer">
            <p>Generated by ACDSO v2.0 Auto EDA | Analysis time: {r.total_analysis_time_s:.2f}s</p>
        </div>
    </div>
</body>
</html>"""
        return html
    
    def to_markdown(self, filepath: str) -> None:
        """Export report as Markdown file."""
        if self.report_ is None:
            raise RuntimeError("No analysis performed yet. Call analyze() first.")
        
        md = self._generate_markdown_report()
        with open(filepath, "w", encoding="utf-8") as f:
            f.write(md)
    
    def _generate_markdown_report(self) -> str:
        """Generate Markdown report from EDA results."""
        r = self.report_
        q = self.data_quality_
        
        lines = [
            f"# ACDSO Auto EDA Report: {r.dataset_name}",
            f"",
            f"*Generated: {r.analysis_timestamp}*",
            f"",
            f"## Dataset Overview",
            f"",
            f"| Metric | Value |",
            f"|--------|-------|",
            f"| Rows | {r.n_rows:,} |",
            f"| Columns | {r.n_cols} |",
            f"| Numeric | {q.n_numeric} |",
            f"| Categorical | {q.n_categorical} |",
            f"| Memory | {q.memory_mb:.2f} MB |",
            f"",
            f"## Data Quality Score: {q.quality_score:.0f}/100",
            f"",
            f"| Metric | Value |",
            f"|--------|-------|",
            f"| Missing Values | {q.total_missing:,} ({q.total_missing_pct:.1f}%) |",
            f"| Duplicate Rows | {q.n_duplicate_rows:,} ({q.duplicate_pct:.1f}%) |",
            f"| Constant Columns | {q.n_constant_cols} |",
            f"| ID-like Columns | {q.n_id_like_cols} |",
            f"",
        ]
        
        if q.issues:
            lines.extend([
                "## Issues Detected",
                "",
            ])
            for issue in q.issues:
                lines.append(f"- ⚠️ {issue}")
            lines.append("")
        
        if r.high_correlations:
            lines.extend([
                "## High Correlations",
                "",
                "| Feature 1 | Feature 2 | Correlation |",
                "|-----------|-----------|-------------|",
            ])
            for c in r.high_correlations[:10]:
                lines.append(f"| {c['feature_1']} | {c['feature_2']} | {c['correlation']:.4f} |")
            lines.append("")
        
        if r.recommendations:
            lines.extend([
                "## Recommendations",
                "",
            ])
            for i, rec in enumerate(r.recommendations, 1):
                lines.append(f"{i}. {rec}")
            lines.append("")
        
        lines.extend([
            "---",
            f"*Analysis time: {r.total_analysis_time_s:.2f}s | Generated by ACDSO v2.0*",
        ])
        
        return "\n".join(lines)


# ============================================================================
# Backend Adapters
# ============================================================================

class EDABackendAdapter(ABC):
    """Abstract base class for EDA backend adapters."""
    
    @abstractmethod
    def analyze(self, df: pd.DataFrame, **kwargs) -> Dict[str, Any]:
        """Run analysis and return results."""
        pass
    
    @abstractmethod
    def to_html(self, filepath: str) -> None:
        """Export report as HTML."""
        pass
    
    @property
    @abstractmethod
    def name(self) -> str:
        """Backend name."""
        pass


class YDataProfilingAdapter(EDABackendAdapter):
    """
    Adapter for ydata-profiling (formerly pandas-profiling).
    
    ydata-profiling provides comprehensive statistical profiling
    with interactive HTML reports.
    
    Install: pip install ydata-profiling
    
    Parameters
    ----------
    minimal : bool, default=False
        Use minimal mode for faster analysis.
    explorative : bool, default=True
        Enable explorative mode with more details.
    title : str, default="ACDSO EDA Report"
        Report title.
    """
    
    def __init__(
        self,
        minimal: bool = False,
        explorative: bool = True,
        title: str = "ACDSO EDA Report",
    ):
        self.minimal = minimal
        self.explorative = explorative
        self.title = title
        self._profile = None
        
        # Import check
        try:
            from ydata_profiling import ProfileReport
            self._ProfileReport = ProfileReport
        except ImportError:
            try:
                from pandas_profiling import ProfileReport
                self._ProfileReport = ProfileReport
            except ImportError:
                raise ImportError(
                    "ydata-profiling is not installed. "
                    "Install with: pip install ydata-profiling"
                )
    
    @property
    def name(self) -> str:
        return "ydata-profiling"
    
    def analyze(self, df: pd.DataFrame, **kwargs) -> Dict[str, Any]:
        """Run ydata-profiling analysis."""
        self._profile = self._ProfileReport(
            df,
            title=self.title,
            minimal=self.minimal,
            explorative=self.explorative,
            **kwargs,
        )
        
        # Extract summary statistics
        try:
            description = self._profile.get_description()
            # Handle both dict and BaseDescription object
            if hasattr(description, 'variables'):
                n_variables = len(description.variables) if description.variables else len(df.columns)
            elif isinstance(description, dict):
                n_variables = len(description.get("variables", {}))
            else:
                n_variables = len(df.columns)
        except Exception:
            n_variables = len(df.columns)
        
        return {
            "backend": self.name,
            "n_rows": len(df),
            "n_cols": len(df.columns),
            "variables": n_variables,
            "profile_available": True,
        }
    
    def to_html(self, filepath: str) -> None:
        """Export ydata-profiling report as HTML."""
        if self._profile is None:
            raise RuntimeError("No analysis performed. Call analyze() first.")
        self._profile.to_file(filepath)
    
    def to_widgets(self):
        """Get Jupyter widgets for interactive display."""
        if self._profile is None:
            raise RuntimeError("No analysis performed. Call analyze() first.")
        return self._profile.to_widgets()


class SweetvizAdapter(EDABackendAdapter):
    """
    Adapter for Sweetviz EDA library.
    
    Sweetviz provides beautiful visualizations and comparison reports.
    
    Install: pip install sweetviz
    
    Parameters
    ----------
    target_feat : str, optional
        Target feature for analysis.
    compare_df : pd.DataFrame, optional
        DataFrame to compare against.
    """
    
    def __init__(
        self,
        target_feat: Optional[str] = None,
        compare_df: Optional[pd.DataFrame] = None,
    ):
        self.target_feat = target_feat
        self.compare_df = compare_df
        self._report = None
        
        try:
            import sweetviz as sv
            self._sv = sv
        except ImportError:
            raise ImportError(
                "sweetviz is not installed. "
                "Install with: pip install sweetviz"
            )
    
    @property
    def name(self) -> str:
        return "sweetviz"
    
    def analyze(self, df: pd.DataFrame, **kwargs) -> Dict[str, Any]:
        """Run Sweetviz analysis."""
        if self.compare_df is not None:
            self._report = self._sv.compare(
                [df, "Source"],
                [self.compare_df, "Compare"],
                target_feat=self.target_feat,
                **kwargs,
            )
        else:
            self._report = self._sv.analyze(
                df,
                target_feat=self.target_feat,
                **kwargs,
            )
        
        return {
            "backend": self.name,
            "n_rows": len(df),
            "n_cols": len(df.columns),
            "target_feature": self.target_feat,
            "comparison_mode": self.compare_df is not None,
        }
    
    def to_html(self, filepath: str) -> None:
        """Export Sweetviz report as HTML."""
        if self._report is None:
            raise RuntimeError("No analysis performed. Call analyze() first.")
        self._report.show_html(filepath, open_browser=False)


class DataprepAdapter(EDABackendAdapter):
    """
    Adapter for Dataprep EDA library.
    
    Dataprep provides fast EDA with interactive visualizations.
    
    Install: pip install dataprep
    
    Parameters
    ----------
    display : List[str], optional
        Sections to display (e.g., ["Overview", "Variables"]).
    """
    
    def __init__(self, display: Optional[List[str]] = None):
        self.display = display
        self._report = None
        
        try:
            from dataprep.eda import create_report
            self._create_report = create_report
        except ImportError:
            raise ImportError(
                "dataprep is not installed. "
                "Install with: pip install dataprep"
            )
    
    @property
    def name(self) -> str:
        return "dataprep"
    
    def analyze(self, df: pd.DataFrame, **kwargs) -> Dict[str, Any]:
        """Run Dataprep analysis."""
        self._report = self._create_report(df, **kwargs)
        
        return {
            "backend": self.name,
            "n_rows": len(df),
            "n_cols": len(df.columns),
        }
    
    def to_html(self, filepath: str) -> None:
        """Export Dataprep report as HTML."""
        if self._report is None:
            raise RuntimeError("No analysis performed. Call analyze() first.")
        self._report.save(filepath)


class DTaleAdapter(EDABackendAdapter):
    """
    Adapter for D-Tale interactive EDA.
    
    D-Tale provides interactive browser-based data exploration.
    
    Install: pip install dtale
    """
    
    def __init__(self):
        self._dtale_instance = None
        
        try:
            import dtale
            self._dtale = dtale
        except ImportError:
            raise ImportError(
                "dtale is not installed. "
                "Install with: pip install dtale"
            )
    
    @property
    def name(self) -> str:
        return "dtale"
    
    def analyze(self, df: pd.DataFrame, **kwargs) -> Dict[str, Any]:
        """Launch D-Tale instance."""
        self._dtale_instance = self._dtale.show(df, **kwargs)
        
        return {
            "backend": self.name,
            "n_rows": len(df),
            "n_cols": len(df.columns),
            "url": self._dtale_instance._url if self._dtale_instance else None,
        }
    
    def to_html(self, filepath: str) -> None:
        """D-Tale is interactive; export not directly supported."""
        raise NotImplementedError(
            "D-Tale is an interactive tool. Use open_browser() instead."
        )
    
    def open_browser(self) -> None:
        """Open D-Tale in browser."""
        if self._dtale_instance is None:
            raise RuntimeError("No D-Tale instance. Call analyze() first.")
        self._dtale_instance.open_browser()


def get_backend_adapter(
    backend: Union[str, EDABackend],
    **kwargs,
) -> EDABackendAdapter:
    """
    Get the appropriate backend adapter.
    
    Parameters
    ----------
    backend : str or EDABackend
        Backend to use.
    **kwargs
        Additional arguments for the backend adapter.
    
    Returns
    -------
    EDABackendAdapter
        Configured backend adapter.
    
    Raises
    ------
    ValueError
        If backend is not supported or not installed.
    """
    if isinstance(backend, EDABackend):
        backend = backend.value
    
    backends = get_available_backends()
    
    if backend == "native":
        return None  # Use native ACDSOAutoEDA
    
    if backend not in backends:
        raise ValueError(
            f"Unknown backend: {backend}. "
            f"Available: {list(backends.keys())}"
        )
    
    if not backends[backend]:
        raise ImportError(
            f"Backend '{backend}' is not installed. "
            f"See installation instructions in documentation."
        )
    
    adapter_map = {
        "ydata-profiling": YDataProfilingAdapter,
        "sweetviz": SweetvizAdapter,
        "dataprep": DataprepAdapter,
        "dtale": DTaleAdapter,
    }
    
    return adapter_map[backend](**kwargs)


# ============================================================================
# Standalone Utility Functions
# ============================================================================

def quick_profile(
    df: pd.DataFrame,
    columns: Optional[List[str]] = None,
) -> Dict[str, ColumnProfile]:
    """
    Quick column profiling without full EDA.
    
    Parameters
    ----------
    df : pd.DataFrame
        DataFrame to profile.
    columns : List[str], optional
        Specific columns to profile. If None, profiles all columns.
    
    Returns
    -------
    Dict[str, ColumnProfile]
        Dictionary mapping column names to their profiles.
    
    Example
    -------
    >>> from acdso.eda import quick_profile
    >>> profiles = quick_profile(df)
    >>> print(profiles['age'].mean)
    >>> print(profiles['age'].has_outliers)
    """
    num_cols, cat_cols = split_columns(df)
    cols = columns if columns else list(df.columns)
    
    result = {}
    for col in cols:
        if col not in df.columns:
            continue
        if col in num_cols:
            result[col] = _profile_numeric_column(df[col], col)
        else:
            result[col] = _profile_categorical_column(df[col], col)
    
    return result


def compare_datasets(
    df1: pd.DataFrame,
    df2: pd.DataFrame,
    name1: str = "Dataset 1",
    name2: str = "Dataset 2",
) -> Dict[str, Any]:
    """
    Compare two datasets for schema and distribution differences.
    
    Parameters
    ----------
    df1, df2 : pd.DataFrame
        DataFrames to compare.
    name1, name2 : str
        Names for the datasets.
    
    Returns
    -------
    Dict[str, Any]
        Comparison results including schema differences and statistics.
    
    Example
    -------
    >>> from acdso.eda import compare_datasets
    >>> comparison = compare_datasets(train_df, test_df, "Train", "Test")
    >>> print(comparison['schema_differences'])
    >>> print(comparison['distribution_drift'])
    """
    result = {
        "dataset_1": {"name": name1, "shape": df1.shape},
        "dataset_2": {"name": name2, "shape": df2.shape},
        "schema_differences": [],
        "distribution_drift": [],
        "summary": {},
    }
    
    # Schema comparison
    cols1 = set(df1.columns)
    cols2 = set(df2.columns)
    
    only_in_1 = cols1 - cols2
    only_in_2 = cols2 - cols1
    common = cols1 & cols2
    
    if only_in_1:
        result["schema_differences"].append({
            "type": "missing_in_dataset_2",
            "columns": list(only_in_1),
        })
    
    if only_in_2:
        result["schema_differences"].append({
            "type": "missing_in_dataset_1",
            "columns": list(only_in_2),
        })
    
    # Type mismatches
    for col in common:
        if df1[col].dtype != df2[col].dtype:
            result["schema_differences"].append({
                "type": "dtype_mismatch",
                "column": col,
                "dtype_1": str(df1[col].dtype),
                "dtype_2": str(df2[col].dtype),
            })
    
    # Distribution drift (for numeric columns)
    num_cols = [c for c in common if pd.api.types.is_numeric_dtype(df1[c])]
    
    for col in num_cols[:20]:  # Limit for performance
        try:
            clean1 = df1[col].dropna()
            clean2 = df2[col].dropna()
            
            # Convert boolean to int for statistics
            if clean1.dtype == 'bool' or clean1.dtype == 'boolean':
                clean1 = clean1.astype(int)
            if clean2.dtype == 'bool' or clean2.dtype == 'boolean':
                clean2 = clean2.astype(int)
            
            if len(clean1) < 10 or len(clean2) < 10:
                continue
            
            # KS test for distribution difference
            statistic, pvalue = stats.ks_2samp(clean1, clean2)
            
            if pvalue < 0.05:  # Significant drift
                result["distribution_drift"].append({
                    "column": col,
                    "ks_statistic": round(float(statistic), 4),
                    "p_value": round(float(pvalue), 6),
                    "mean_1": round(float(clean1.mean()), 4),
                    "mean_2": round(float(clean2.mean()), 4),
                    "std_1": round(float(clean1.std()), 4),
                    "std_2": round(float(clean2.std()), 4),
                })
        except Exception:
            continue
    
    # Summary
    result["summary"] = {
        "common_columns": len(common),
        "schema_issues": len(result["schema_differences"]),
        "columns_with_drift": len(result["distribution_drift"]),
        "compatible": len(result["schema_differences"]) == 0,
    }
    
    return result


def validate_data(
    df: pd.DataFrame,
    rules: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    """
    Validate data against quality rules.
    
    Parameters
    ----------
    df : pd.DataFrame
        DataFrame to validate.
    rules : Dict[str, Any], optional
        Validation rules. If None, uses default rules.
        
        Supported rules:
        - max_missing_pct: Maximum missing percentage per column
        - max_duplicate_pct: Maximum duplicate row percentage
        - required_columns: List of required columns
        - column_types: Dict mapping columns to expected types
        - value_ranges: Dict mapping columns to (min, max) tuples
    
    Returns
    -------
    Dict[str, Any]
        Validation results with passed/failed status.
    
    Example
    -------
    >>> from acdso.eda import validate_data
    >>> rules = {
    ...     'max_missing_pct': 0.1,
    ...     'required_columns': ['id', 'value'],
    ...     'value_ranges': {'age': (0, 120)},
    ... }
    >>> result = validate_data(df, rules)
    >>> print(result['passed'])
    >>> print(result['failures'])
    """
    if rules is None:
        rules = {
            "max_missing_pct": 0.5,
            "max_duplicate_pct": 0.1,
        }
    
    result = {
        "passed": True,
        "checks_run": 0,
        "checks_passed": 0,
        "failures": [],
        "warnings": [],
    }
    
    # Check missing percentage
    if "max_missing_pct" in rules:
        result["checks_run"] += 1
        max_pct = rules["max_missing_pct"]
        
        for col in df.columns:
            missing_pct = df[col].isna().mean()
            if missing_pct > max_pct:
                result["passed"] = False
                result["failures"].append({
                    "rule": "max_missing_pct",
                    "column": col,
                    "expected": f"<= {max_pct:.1%}",
                    "actual": f"{missing_pct:.1%}",
                })
        
        if not any(f["rule"] == "max_missing_pct" for f in result["failures"]):
            result["checks_passed"] += 1
    
    # Check duplicates
    if "max_duplicate_pct" in rules:
        result["checks_run"] += 1
        max_pct = rules["max_duplicate_pct"]
        dup_pct = df.duplicated().mean()
        
        if dup_pct > max_pct:
            result["passed"] = False
            result["failures"].append({
                "rule": "max_duplicate_pct",
                "expected": f"<= {max_pct:.1%}",
                "actual": f"{dup_pct:.1%}",
            })
        else:
            result["checks_passed"] += 1
    
    # Check required columns
    if "required_columns" in rules:
        result["checks_run"] += 1
        missing = [c for c in rules["required_columns"] if c not in df.columns]
        
        if missing:
            result["passed"] = False
            result["failures"].append({
                "rule": "required_columns",
                "missing": missing,
            })
        else:
            result["checks_passed"] += 1
    
    # Check column types
    if "column_types" in rules:
        result["checks_run"] += 1
        type_errors = []
        
        for col, expected_type in rules["column_types"].items():
            if col not in df.columns:
                continue
            
            actual_type = str(df[col].dtype)
            if expected_type not in actual_type:
                type_errors.append({
                    "column": col,
                    "expected": expected_type,
                    "actual": actual_type,
                })
        
        if type_errors:
            result["passed"] = False
            result["failures"].append({
                "rule": "column_types",
                "errors": type_errors,
            })
        else:
            result["checks_passed"] += 1
    
    # Check value ranges
    if "value_ranges" in rules:
        result["checks_run"] += 1
        range_errors = []
        
        for col, (min_val, max_val) in rules["value_ranges"].items():
            if col not in df.columns:
                continue
            
            if not pd.api.types.is_numeric_dtype(df[col]):
                continue
            
            clean = df[col].dropna()
            actual_min = clean.min()
            actual_max = clean.max()
            
            if actual_min < min_val or actual_max > max_val:
                range_errors.append({
                    "column": col,
                    "expected_range": (min_val, max_val),
                    "actual_range": (float(actual_min), float(actual_max)),
                })
        
        if range_errors:
            result["passed"] = False
            result["failures"].append({
                "rule": "value_ranges",
                "errors": range_errors,
            })
        else:
            result["checks_passed"] += 1
    
    return result


def infer_data_types(df: pd.DataFrame) -> Dict[str, Dict[str, Any]]:
    """
    Infer semantic data types for columns.
    
    Parameters
    ----------
    df : pd.DataFrame
        DataFrame to analyze.
    
    Returns
    -------
    Dict[str, Dict[str, Any]]
        Dictionary mapping columns to inferred type information.
    
    Example
    -------
    >>> from acdso.eda import infer_data_types
    >>> types = infer_data_types(df)
    >>> print(types['email']['semantic_type'])  # 'email'
    >>> print(types['age']['semantic_type'])    # 'numeric_continuous'
    """
    result = {}
    
    # Patterns for common types
    patterns = {
        "email": r'^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$',
        "url": r'^https?://[^\s]+$',
        "phone": r'^\+?[\d\s\-\(\)]{7,}$',
        "ip_address": r'^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$',
        "uuid": r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$',
        "date": r'^\d{4}-\d{2}-\d{2}$',
    }
    
    for col in df.columns:
        series = df[col]
        dtype = str(series.dtype)
        n_unique = series.nunique()
        n_total = len(series) - series.isna().sum()
        
        info = {
            "pandas_dtype": dtype,
            "semantic_type": None,
            "suggested_dtype": None,
            "cardinality": n_unique,
            "cardinality_ratio": n_unique / n_total if n_total > 0 else 0,
        }
        
        # Numeric types
        if pd.api.types.is_numeric_dtype(series):
            if n_unique <= 10 and n_unique < 0.05 * n_total:
                info["semantic_type"] = "categorical_numeric"
                info["suggested_dtype"] = "category"
            elif pd.api.types.is_integer_dtype(series):
                info["semantic_type"] = "numeric_discrete"
            else:
                info["semantic_type"] = "numeric_continuous"
        
        # DateTime
        elif pd.api.types.is_datetime64_any_dtype(series):
            info["semantic_type"] = "datetime"
        
        # Object/String types
        elif series.dtype == object:
            sample = series.dropna().head(100).astype(str)
            
            # Check patterns
            for pattern_name, pattern in patterns.items():
                try:
                    matches = sample.str.match(pattern, na=False).mean()
                    if matches > 0.8:
                        info["semantic_type"] = pattern_name
                        break
                except Exception:
                    continue
            
            if info["semantic_type"] is None:
                # Check if categorical
                if n_unique <= 20 or n_unique < 0.01 * n_total:
                    info["semantic_type"] = "categorical"
                    info["suggested_dtype"] = "category"
                elif n_unique == n_total:
                    info["semantic_type"] = "identifier"
                else:
                    info["semantic_type"] = "text"
        
        result[col] = info
    
    return result


def get_preprocessing_recommendations(
    df: pd.DataFrame,
    target: Optional[str] = None,
) -> List[Dict[str, Any]]:
    """
    Get preprocessing recommendations for modeling.
    
    Parameters
    ----------
    df : pd.DataFrame
        DataFrame to analyze.
    target : str, optional
        Target column name.
    
    Returns
    -------
    List[Dict[str, Any]]
        List of preprocessing recommendations.
    
    Example
    -------
    >>> from acdso.eda import get_preprocessing_recommendations
    >>> recs = get_preprocessing_recommendations(df, target='price')
    >>> for rec in recs:
    ...     print(f"{rec['column']}: {rec['action']}")
    """
    recommendations = []
    profiles = quick_profile(df)
    num_cols, cat_cols = split_columns(df)
    
    for col, profile in profiles.items():
        if col == target:
            continue
        
        rec = {
            "column": col,
            "current_dtype": profile.dtype,
            "action": None,
            "reason": None,
            "priority": "low",
        }
        
        # Missing value handling
        if profile.missing_pct > 0:
            if profile.missing_pct > 50:
                rec["action"] = "drop_column"
                rec["reason"] = f"High missing rate ({profile.missing_pct:.1f}%)"
                rec["priority"] = "high"
            elif col in num_cols:
                rec["action"] = "impute_median"
                rec["reason"] = f"Missing values ({profile.missing_pct:.1f}%)"
                rec["priority"] = "medium"
            else:
                rec["action"] = "impute_mode"
                rec["reason"] = f"Missing values ({profile.missing_pct:.1f}%)"
                rec["priority"] = "medium"
            recommendations.append(rec.copy())
        
        # Constant columns
        if profile.is_constant:
            recommendations.append({
                "column": col,
                "action": "drop_column",
                "reason": "Constant value - no predictive power",
                "priority": "high",
            })
            continue
        
        # ID-like columns
        if profile.is_id_like:
            recommendations.append({
                "column": col,
                "action": "drop_column",
                "reason": "ID-like column - not useful for modeling",
                "priority": "high",
            })
            continue
        
        # Highly skewed numeric
        if profile.is_highly_skewed and col in num_cols:
            recommendations.append({
                "column": col,
                "action": "log_transform",
                "reason": f"Highly skewed (skewness={profile.skewness:.2f})",
                "priority": "medium",
            })
        
        # High cardinality categorical
        if col in cat_cols and profile.cardinality_ratio and profile.cardinality_ratio > 0.5:
            recommendations.append({
                "column": col,
                "action": "reduce_cardinality",
                "reason": f"High cardinality ({profile.n_unique} unique values)",
                "priority": "medium",
            })
        
        # Outliers
        if profile.has_outliers and profile.n_outliers_iqr:
            pct = profile.n_outliers_iqr / (len(df) - profile.n_missing) * 100
            if pct > 5:
                recommendations.append({
                    "column": col,
                    "action": "clip_outliers",
                    "reason": f"Many outliers ({profile.n_outliers_iqr}, {pct:.1f}%)",
                    "priority": "medium",
                })
    
    # Sort by priority
    priority_order = {"high": 0, "medium": 1, "low": 2}
    recommendations.sort(key=lambda x: priority_order.get(x.get("priority", "low"), 2))
    
    return recommendations

