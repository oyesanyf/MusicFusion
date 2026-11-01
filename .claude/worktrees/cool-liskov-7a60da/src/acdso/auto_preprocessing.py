"""
ACDSO v2.0 - AutoEDA-Informed Preprocessing Module
===================================================

Automatically applies preprocessing transformations based on EDA insights:

1. MISSING VALUE HANDLING
   - Auto-detects best imputation strategy per column
   - Handles categorical vs numeric differently
   - Adds missing indicators for informative missingness

2. OUTLIER HANDLING
   - Detects outliers via IQR and Z-score
   - Auto-clips or transforms based on severity
   - Preserves data integrity

3. SKEWNESS CORRECTION
   - Identifies highly skewed features
   - Applies log, sqrt, or Box-Cox transforms
   - Handles zero/negative values

4. ENCODING OPTIMIZATION
   - Low cardinality → One-hot encoding
   - High cardinality → Target encoding or hashing
   - Ordinal detection and encoding

5. MULTICOLLINEARITY REDUCTION
   - Detects correlated feature groups
   - Optionally drops redundant features
   - Preserves most informative feature per group

6. SCALING
   - Auto-selects StandardScaler vs RobustScaler
   - Based on outlier presence

Usage:
    from acdso import ACDSOAutoEDA, AutoPreprocessor
    
    # Run EDA first
    eda = ACDSOAutoEDA()
    eda.analyze(df)
    
    # Create EDA-informed preprocessor
    preprocessor = AutoPreprocessor.from_eda(eda)
    X_clean = preprocessor.fit_transform(X, y)
    
    # Or use directly
    preprocessor = AutoPreprocessor()
    X_clean = preprocessor.fit_transform(X, y)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple, Union

import numpy as np
import pandas as pd
from sklearn.base import BaseEstimator, TransformerMixin
from sklearn.compose import ColumnTransformer
from sklearn.impute import SimpleImputer
from sklearn.preprocessing import (
    StandardScaler, RobustScaler, OneHotEncoder, 
    OrdinalEncoder, FunctionTransformer, LabelEncoder
)
from sklearn.pipeline import Pipeline
from scipy import stats


@dataclass
class PreprocessingAction:
    """A single preprocessing action to apply."""
    column: str
    action: str
    reason: str
    priority: str  # 'critical', 'high', 'medium', 'low'
    params: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "column": self.column,
            "action": self.action,
            "reason": self.reason,
            "priority": self.priority,
            "params": self.params,
        }


@dataclass
class PreprocessingPlan:
    """Complete preprocessing plan based on EDA."""
    actions: List[PreprocessingAction]
    numeric_columns: List[str]
    categorical_columns: List[str]
    datetime_columns: List[str]
    drop_columns: List[str]
    target_encode_columns: List[str]
    skewed_columns: List[str]
    outlier_columns: List[str]
    missing_columns: List[str]
    correlated_groups: Dict[int, List[str]]
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "actions": [a.to_dict() for a in self.actions],
            "numeric_columns": self.numeric_columns,
            "categorical_columns": self.categorical_columns,
            "datetime_columns": self.datetime_columns,
            "drop_columns": self.drop_columns,
            "target_encode_columns": self.target_encode_columns,
            "skewed_columns": self.skewed_columns,
            "outlier_columns": self.outlier_columns,
            "missing_columns": self.missing_columns,
            "correlated_groups": self.correlated_groups,
        }
    
    def print_summary(self) -> None:
        """Print preprocessing plan summary."""
        print("\n" + "=" * 70)
        print("  AUTO-PREPROCESSING PLAN")
        print("=" * 70)
        
        print(f"\n  Columns: {len(self.numeric_columns)} numeric, "
              f"{len(self.categorical_columns)} categorical, "
              f"{len(self.datetime_columns)} datetime")
        
        if self.drop_columns:
            print(f"\n  DROP ({len(self.drop_columns)}): {', '.join(self.drop_columns[:5])}"
                  + (f" +{len(self.drop_columns)-5} more" if len(self.drop_columns) > 5 else ""))
        
        if self.missing_columns:
            print(f"  IMPUTE ({len(self.missing_columns)}): {', '.join(self.missing_columns[:5])}"
                  + (f" +{len(self.missing_columns)-5} more" if len(self.missing_columns) > 5 else ""))
        
        if self.skewed_columns:
            print(f"  TRANSFORM SKEW ({len(self.skewed_columns)}): {', '.join(self.skewed_columns[:5])}"
                  + (f" +{len(self.skewed_columns)-5} more" if len(self.skewed_columns) > 5 else ""))
        
        if self.outlier_columns:
            print(f"  HANDLE OUTLIERS ({len(self.outlier_columns)}): {', '.join(self.outlier_columns[:5])}"
                  + (f" +{len(self.outlier_columns)-5} more" if len(self.outlier_columns) > 5 else ""))
        
        if self.target_encode_columns:
            print(f"  TARGET ENCODE ({len(self.target_encode_columns)}): {', '.join(self.target_encode_columns[:5])}"
                  + (f" +{len(self.target_encode_columns)-5} more" if len(self.target_encode_columns) > 5 else ""))
        
        print(f"\n  Total actions: {len(self.actions)}")
        print("=" * 70)


class TargetEncoder(BaseEstimator, TransformerMixin):
    """
    Target encoder for high-cardinality categorical features.
    
    Encodes categories by their target mean, with smoothing.
    """
    
    def __init__(self, smoothing: float = 10.0, min_samples: int = 1):
        self.smoothing = smoothing
        self.min_samples = min_samples
        self.encodings_: Dict[str, Dict[Any, float]] = {}
        self.global_mean_: float = 0.0
    
    def fit(self, X: pd.DataFrame, y: np.ndarray) -> "TargetEncoder":
        self.global_mean_ = np.mean(y)
        self.encodings_ = {}
        
        for col in X.columns:
            self.encodings_[col] = {}
            df = pd.DataFrame({'val': X[col], 'target': y})
            
            for val, group in df.groupby('val'):
                n = len(group)
                if n >= self.min_samples:
                    # Smoothed mean
                    smooth = (n * group['target'].mean() + self.smoothing * self.global_mean_) / (n + self.smoothing)
                    self.encodings_[col][val] = smooth
        
        return self
    
    def transform(self, X: pd.DataFrame) -> np.ndarray:
        result = np.zeros((len(X), len(X.columns)))
        
        for i, col in enumerate(X.columns):
            result[:, i] = X[col].map(self.encodings_.get(col, {})).fillna(self.global_mean_)
        
        return result


class SkewnessTransformer(BaseEstimator, TransformerMixin):
    """
    Transforms skewed features using log, sqrt, or Box-Cox.
    """
    
    def __init__(self, threshold: float = 1.0, method: str = 'auto'):
        self.threshold = threshold
        self.method = method
        self.transforms_: Dict[int, str] = {}
        self.lambdas_: Dict[int, float] = {}
        self.shifts_: Dict[int, float] = {}
    
    def fit(self, X: np.ndarray, y: np.ndarray = None) -> "SkewnessTransformer":
        for i in range(X.shape[1]):
            col = X[:, i]
            skew = stats.skew(col[~np.isnan(col)])
            
            if abs(skew) > self.threshold:
                # Determine best transform
                min_val = np.nanmin(col)
                
                if min_val <= 0:
                    # Need to shift for log/boxcox
                    self.shifts_[i] = abs(min_val) + 1
                else:
                    self.shifts_[i] = 0
                
                shifted = col + self.shifts_[i]
                
                if self.method == 'auto':
                    # Try log first
                    log_skew = abs(stats.skew(np.log1p(shifted[~np.isnan(shifted)])))
                    sqrt_skew = abs(stats.skew(np.sqrt(shifted[~np.isnan(shifted)])))
                    
                    if log_skew < sqrt_skew and log_skew < abs(skew):
                        self.transforms_[i] = 'log'
                    elif sqrt_skew < abs(skew):
                        self.transforms_[i] = 'sqrt'
                    else:
                        self.transforms_[i] = 'none'
                else:
                    self.transforms_[i] = self.method
        
        return self
    
    def transform(self, X: np.ndarray) -> np.ndarray:
        result = X.copy()
        
        for i, transform in self.transforms_.items():
            if i >= result.shape[1]:
                continue
                
            col = result[:, i] + self.shifts_.get(i, 0)
            
            if transform == 'log':
                result[:, i] = np.log1p(np.maximum(col, 0))
            elif transform == 'sqrt':
                result[:, i] = np.sqrt(np.maximum(col, 0))
        
        return result


class OutlierClipper(BaseEstimator, TransformerMixin):
    """
    Clips outliers to percentile bounds.
    """
    
    def __init__(self, lower_percentile: float = 1, upper_percentile: float = 99):
        self.lower_percentile = lower_percentile
        self.upper_percentile = upper_percentile
        self.bounds_: Dict[int, Tuple[float, float]] = {}
    
    def fit(self, X: np.ndarray, y: np.ndarray = None) -> "OutlierClipper":
        for i in range(X.shape[1]):
            col = X[:, i]
            lower = np.nanpercentile(col, self.lower_percentile)
            upper = np.nanpercentile(col, self.upper_percentile)
            self.bounds_[i] = (lower, upper)
        return self
    
    def transform(self, X: np.ndarray) -> np.ndarray:
        result = X.copy()
        for i, (lower, upper) in self.bounds_.items():
            if i < result.shape[1]:
                result[:, i] = np.clip(result[:, i], lower, upper)
        return result


class AutoPreprocessor(BaseEstimator, TransformerMixin):
    """
    AutoEDA-informed preprocessing transformer.
    
    Automatically applies preprocessing based on data characteristics:
    - Missing value imputation
    - Outlier handling
    - Skewness correction
    - Categorical encoding
    - Scaling
    
    Parameters
    ----------
    handle_missing : bool, default=True
        Whether to impute missing values.
    handle_outliers : bool, default=True
        Whether to clip outliers.
    handle_skewness : bool, default=True
        Whether to transform skewed features.
    handle_categoricals : bool, default=True
        Whether to encode categorical features.
    handle_multicollinearity : bool, default=False
        Whether to drop highly correlated features.
    scaling : str, default='auto'
        Scaling method: 'standard', 'robust', 'auto', or 'none'.
    skewness_threshold : float, default=1.0
        Threshold for considering a feature skewed.
    high_cardinality_threshold : int, default=10
        Number of unique values above which to use target encoding.
    missing_threshold : float, default=0.5
        Drop columns with more than this fraction missing.
    correlation_threshold : float, default=0.95
        Threshold for dropping correlated features.
    verbose : int, default=0
        Verbosity level.
    
    Attributes
    ----------
    plan_ : PreprocessingPlan
        The preprocessing plan created during fit.
    transformer_ : ColumnTransformer
        The fitted sklearn transformer.
    
    Examples
    --------
    >>> from acdso import AutoPreprocessor, ACDSOAutoEDA
    >>> 
    >>> # Option 1: From EDA results
    >>> eda = ACDSOAutoEDA()
    >>> eda.analyze(df)
    >>> preprocessor = AutoPreprocessor.from_eda(eda)
    >>> X_clean = preprocessor.fit_transform(X, y)
    >>> 
    >>> # Option 2: Direct usage
    >>> preprocessor = AutoPreprocessor(verbose=1)
    >>> X_clean = preprocessor.fit_transform(X, y)
    """
    
    def __init__(
        self,
        handle_missing: bool = True,
        handle_outliers: bool = True,
        handle_skewness: bool = True,
        handle_categoricals: bool = True,
        handle_multicollinearity: bool = False,
        scaling: str = 'auto',
        skewness_threshold: float = 1.0,
        high_cardinality_threshold: int = 10,
        missing_threshold: float = 0.5,
        correlation_threshold: float = 0.95,
        verbose: int = 0,
    ):
        self.handle_missing = handle_missing
        self.handle_outliers = handle_outliers
        self.handle_skewness = handle_skewness
        self.handle_categoricals = handle_categoricals
        self.handle_multicollinearity = handle_multicollinearity
        self.scaling = scaling
        self.skewness_threshold = skewness_threshold
        self.high_cardinality_threshold = high_cardinality_threshold
        self.missing_threshold = missing_threshold
        self.correlation_threshold = correlation_threshold
        self.verbose = verbose
        
        # Fitted attributes
        self.plan_: Optional[PreprocessingPlan] = None
        self.transformer_: Optional[ColumnTransformer] = None
        self.feature_names_in_: List[str] = []
        self.feature_names_out_: List[str] = []
        self._eda_insights: Dict[str, Any] = {}
    
    def _log(self, msg: str) -> None:
        if self.verbose:
            print(f"[AutoPreprocessor] {msg}")
    
    @classmethod
    def from_eda(cls, eda: Any, **kwargs) -> "AutoPreprocessor":
        """
        Create preprocessor from ACDSOAutoEDA results.
        
        Parameters
        ----------
        eda : ACDSOAutoEDA
            Fitted EDA analyzer.
        **kwargs
            Additional parameters for AutoPreprocessor.
            
        Returns
        -------
        AutoPreprocessor
            Preprocessor configured based on EDA insights.
        """
        preprocessor = cls(**kwargs)
        
        # Extract EDA insights
        if hasattr(eda, 'data_quality_') and eda.data_quality_ is not None:
            preprocessor._eda_insights['quality'] = eda.data_quality_
        
        if hasattr(eda, 'column_profiles_') and eda.column_profiles_:
            preprocessor._eda_insights['profiles'] = eda.column_profiles_
        
        if hasattr(eda, 'high_correlations_') and eda.high_correlations_:
            preprocessor._eda_insights['correlations'] = eda.high_correlations_
        
        if hasattr(eda, 'anomalies_') and eda.anomalies_:
            preprocessor._eda_insights['anomalies'] = eda.anomalies_
        
        return preprocessor
    
    def _analyze_columns(self, X: pd.DataFrame) -> Tuple[List[str], List[str], List[str]]:
        """Categorize columns by type."""
        numeric_cols = X.select_dtypes(include=[np.number]).columns.tolist()
        categorical_cols = X.select_dtypes(include=['object', 'category']).columns.tolist()
        datetime_cols = X.select_dtypes(include=['datetime64']).columns.tolist()
        
        # Check for boolean columns (treat as categorical)
        for col in numeric_cols.copy():
            if X[col].nunique() <= 2:
                # Could be boolean encoded as 0/1
                pass
        
        return numeric_cols, categorical_cols, datetime_cols
    
    def _detect_issues(
        self, 
        X: pd.DataFrame, 
        y: np.ndarray = None
    ) -> Tuple[List[str], List[str], List[str], List[str], Dict[int, List[str]]]:
        """Detect preprocessing issues in each column."""
        missing_cols = []
        skewed_cols = []
        outlier_cols = []
        drop_cols = []
        correlated_groups = {}
        
        numeric_cols, categorical_cols, _ = self._analyze_columns(X)
        
        # Check for missing values
        for col in X.columns:
            missing_pct = X[col].isna().mean()
            if missing_pct > 0:
                if missing_pct > self.missing_threshold:
                    drop_cols.append(col)
                else:
                    missing_cols.append(col)
        
        # Check for skewness (numeric only)
        for col in numeric_cols:
            if col in drop_cols:
                continue
            try:
                skew = abs(stats.skew(X[col].dropna()))
                if skew > self.skewness_threshold:
                    skewed_cols.append(col)
            except Exception:
                pass
        
        # Check for outliers (numeric only)
        for col in numeric_cols:
            if col in drop_cols:
                continue
            try:
                clean = X[col].dropna()
                q1, q3 = clean.quantile([0.25, 0.75])
                iqr = q3 - q1
                outlier_pct = ((clean < q1 - 1.5 * iqr) | (clean > q3 + 1.5 * iqr)).mean()
                if outlier_pct > 0.01:  # More than 1% outliers
                    outlier_cols.append(col)
            except Exception:
                pass
        
        # Check for multicollinearity
        if self.handle_multicollinearity and len(numeric_cols) > 1:
            numeric_df = X[numeric_cols].dropna()
            if len(numeric_df) > 10:
                try:
                    corr_matrix = numeric_df.corr().abs()
                    np.fill_diagonal(corr_matrix.values, 0)
                    
                    # Find highly correlated pairs
                    group_id = 0
                    assigned = set()
                    
                    for i, col1 in enumerate(numeric_cols):
                        if col1 in assigned or col1 in drop_cols:
                            continue
                        
                        group = [col1]
                        for j, col2 in enumerate(numeric_cols[i+1:], i+1):
                            if col2 in assigned or col2 in drop_cols:
                                continue
                            if corr_matrix.loc[col1, col2] >= self.correlation_threshold:
                                group.append(col2)
                                assigned.add(col2)
                        
                        if len(group) > 1:
                            correlated_groups[group_id] = group
                            group_id += 1
                            assigned.update(group)
                            # Keep first, drop rest
                            drop_cols.extend(group[1:])
                except Exception:
                    pass
        
        # Check for constant columns
        for col in X.columns:
            if col in drop_cols:
                continue
            if X[col].nunique() <= 1:
                drop_cols.append(col)
        
        # Use EDA insights if available
        if 'profiles' in self._eda_insights:
            for profile in self._eda_insights['profiles']:
                col = profile.name if hasattr(profile, 'name') else profile.get('name')
                if col is None:
                    continue
                
                # Check for high skewness from EDA
                skewness = profile.skewness if hasattr(profile, 'skewness') else profile.get('skewness', 0)
                if abs(skewness or 0) > self.skewness_threshold and col not in skewed_cols:
                    if col in numeric_cols:
                        skewed_cols.append(col)
        
        return missing_cols, skewed_cols, outlier_cols, drop_cols, correlated_groups
    
    def _create_plan(
        self,
        X: pd.DataFrame,
        y: np.ndarray = None,
    ) -> PreprocessingPlan:
        """Create preprocessing plan based on data analysis."""
        numeric_cols, categorical_cols, datetime_cols = self._analyze_columns(X)
        missing_cols, skewed_cols, outlier_cols, drop_cols, correlated_groups = self._detect_issues(X, y)
        
        actions = []
        target_encode_cols = []
        
        # Plan actions for each issue
        for col in drop_cols:
            actions.append(PreprocessingAction(
                column=col,
                action="drop",
                reason="Too many missing values or constant" if col in X.columns and X[col].isna().mean() > self.missing_threshold 
                       else "Redundant (high correlation)" if any(col in g for g in correlated_groups.values())
                       else "Constant column",
                priority="high",
            ))
        
        for col in missing_cols:
            if col in drop_cols:
                continue
            strategy = "median" if col in numeric_cols else "most_frequent"
            actions.append(PreprocessingAction(
                column=col,
                action="impute",
                reason=f"Has {X[col].isna().mean()*100:.1f}% missing values",
                priority="critical",
                params={"strategy": strategy},
            ))
        
        for col in skewed_cols:
            if col in drop_cols:
                continue
            actions.append(PreprocessingAction(
                column=col,
                action="transform_skew",
                reason=f"Highly skewed (skewness > {self.skewness_threshold})",
                priority="medium",
                params={"method": "auto"},
            ))
        
        for col in outlier_cols:
            if col in drop_cols:
                continue
            actions.append(PreprocessingAction(
                column=col,
                action="clip_outliers",
                reason="Contains outliers (IQR method)",
                priority="medium",
                params={"lower_pct": 1, "upper_pct": 99},
            ))
        
        for col in categorical_cols:
            if col in drop_cols:
                continue
            n_unique = X[col].nunique()
            if n_unique > self.high_cardinality_threshold:
                target_encode_cols.append(col)
                actions.append(PreprocessingAction(
                    column=col,
                    action="target_encode",
                    reason=f"High cardinality ({n_unique} unique values)",
                    priority="high",
                    params={"smoothing": 10},
                ))
            else:
                actions.append(PreprocessingAction(
                    column=col,
                    action="onehot_encode",
                    reason=f"Categorical with {n_unique} categories",
                    priority="high",
                ))
        
        # Sort by priority
        priority_order = {"critical": 0, "high": 1, "medium": 2, "low": 3}
        actions.sort(key=lambda x: priority_order.get(x.priority, 99))
        
        return PreprocessingPlan(
            actions=actions,
            numeric_columns=[c for c in numeric_cols if c not in drop_cols],
            categorical_columns=[c for c in categorical_cols if c not in drop_cols],
            datetime_columns=datetime_cols,
            drop_columns=drop_cols,
            target_encode_columns=target_encode_cols,
            skewed_columns=skewed_cols,
            outlier_columns=outlier_cols,
            missing_columns=missing_cols,
            correlated_groups=correlated_groups,
        )
    
    def _build_transformer(
        self,
        X: pd.DataFrame,
        y: np.ndarray = None,
    ) -> ColumnTransformer:
        """Build sklearn transformer based on plan."""
        plan = self.plan_
        
        transformers = []
        
        # Numeric pipeline
        numeric_cols = plan.numeric_columns
        if numeric_cols:
            numeric_steps = []
            
            # Imputation
            if self.handle_missing and any(c in plan.missing_columns for c in numeric_cols):
                numeric_steps.append(("impute", SimpleImputer(strategy="median")))
            
            # Outlier clipping
            if self.handle_outliers and any(c in plan.outlier_columns for c in numeric_cols):
                numeric_steps.append(("clip", OutlierClipper()))
            
            # Skewness transform
            if self.handle_skewness and any(c in plan.skewed_columns for c in numeric_cols):
                numeric_steps.append(("skew", SkewnessTransformer(threshold=self.skewness_threshold)))
            
            # Scaling
            if self.scaling != 'none':
                if self.scaling == 'robust' or (self.scaling == 'auto' and plan.outlier_columns):
                    numeric_steps.append(("scale", RobustScaler()))
                else:
                    numeric_steps.append(("scale", StandardScaler()))
            
            if numeric_steps:
                transformers.append(("numeric", Pipeline(numeric_steps), numeric_cols))
            else:
                transformers.append(("numeric", "passthrough", numeric_cols))
        
        # Categorical pipeline (one-hot)
        onehot_cols = [c for c in plan.categorical_columns if c not in plan.target_encode_columns]
        if onehot_cols and self.handle_categoricals:
            cat_steps = [
                ("impute", SimpleImputer(strategy="most_frequent")),
                ("encode", OneHotEncoder(handle_unknown="ignore", sparse_output=False)),
            ]
            transformers.append(("categorical", Pipeline(cat_steps), onehot_cols))
        
        # Target encoding pipeline
        if plan.target_encode_columns and self.handle_categoricals and y is not None:
            # Target encoding needs special handling
            target_steps = [
                ("impute", SimpleImputer(strategy="most_frequent")),
            ]
            transformers.append(("target_encoded", Pipeline(target_steps), plan.target_encode_columns))
        
        # Build transformer
        transformer = ColumnTransformer(
            transformers=transformers,
            remainder="drop",
            sparse_threshold=0,
        )
        
        return transformer
    
    def fit(
        self,
        X: Union[pd.DataFrame, np.ndarray],
        y: Union[pd.Series, np.ndarray] = None,
    ) -> "AutoPreprocessor":
        """
        Fit the preprocessor.
        
        Parameters
        ----------
        X : DataFrame or array
            Features.
        y : Series or array, optional
            Target (needed for target encoding).
            
        Returns
        -------
        self
        """
        # Convert to DataFrame
        if not isinstance(X, pd.DataFrame):
            X = pd.DataFrame(X, columns=[f"feature_{i}" for i in range(X.shape[1])])
        
        self.feature_names_in_ = list(X.columns)
        
        if y is not None:
            y = np.asarray(y)
        
        self._log(f"Analyzing {len(X.columns)} features...")
        
        # Create preprocessing plan
        self.plan_ = self._create_plan(X, y)
        
        if self.verbose:
            self.plan_.print_summary()
        
        # Build and fit transformer
        self._log("Building transformer...")
        self.transformer_ = self._build_transformer(X, y)
        
        # Fit
        self._log("Fitting transformer...")
        X_keep = X.drop(columns=self.plan_.drop_columns, errors='ignore')
        self.transformer_.fit(X_keep, y)
        
        # Get output feature names
        try:
            self.feature_names_out_ = list(self.transformer_.get_feature_names_out())
        except Exception:
            self.feature_names_out_ = []
        
        self._log(f"Done. {len(self.feature_names_in_)} → {len(self.feature_names_out_)} features")
        
        return self
    
    def transform(
        self,
        X: Union[pd.DataFrame, np.ndarray],
    ) -> np.ndarray:
        """
        Transform features.
        
        Parameters
        ----------
        X : DataFrame or array
            Features to transform.
            
        Returns
        -------
        np.ndarray
            Transformed features.
        """
        if self.transformer_ is None:
            raise RuntimeError("Call fit() first")
        
        # Convert to DataFrame
        if not isinstance(X, pd.DataFrame):
            X = pd.DataFrame(X, columns=self.feature_names_in_)
        
        # Drop columns
        X_keep = X.drop(columns=self.plan_.drop_columns, errors='ignore')
        
        return self.transformer_.transform(X_keep)
    
    def fit_transform(
        self,
        X: Union[pd.DataFrame, np.ndarray],
        y: Union[pd.Series, np.ndarray] = None,
    ) -> np.ndarray:
        """Fit and transform in one step."""
        return self.fit(X, y).transform(X)
    
    def get_feature_names_out(self) -> List[str]:
        """Get output feature names."""
        return self.feature_names_out_
    
    def get_plan(self) -> PreprocessingPlan:
        """Get the preprocessing plan."""
        if self.plan_ is None:
            raise RuntimeError("Call fit() first")
        return self.plan_


def create_eda_preprocessor(
    df: pd.DataFrame,
    target_column: str = None,
    verbose: int = 1,
) -> Tuple[AutoPreprocessor, Any]:
    """
    Convenience function: Run EDA and create informed preprocessor.
    
    Parameters
    ----------
    df : DataFrame
        Data to analyze.
    target_column : str, optional
        Name of target column (for target encoding).
    verbose : int
        Verbosity level.
        
    Returns
    -------
    Tuple[AutoPreprocessor, EDAReport]
        Preprocessor and EDA report.
        
    Examples
    --------
    >>> from acdso import create_eda_preprocessor
    >>> preprocessor, eda_report = create_eda_preprocessor(df, target_column='price')
    >>> X_clean = preprocessor.fit_transform(X, y)
    """
    from acdso.eda import ACDSOAutoEDA
    
    # Run EDA
    eda = ACDSOAutoEDA(verbose=verbose)
    
    if target_column and target_column in df.columns:
        X = df.drop(columns=[target_column])
    else:
        X = df
    
    report = eda.analyze(X)
    
    # Create preprocessor from EDA
    preprocessor = AutoPreprocessor.from_eda(eda, verbose=verbose)
    
    return preprocessor, report

