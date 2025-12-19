"""
ACDSO v2.0 - Safe Feature Importance Module
============================================

Production-safe feature importance that addresses common pitfalls:

1. PERMUTATION IMPORTANCE (not model-specific)
   - Works with any model
   - Measures actual predictive contribution
   - Detects features that only appear important due to overfitting

2. CROSS-VALIDATED IMPORTANCE
   - Averages importance across CV folds
   - Provides confidence intervals
   - Reduces variance from single-split evaluation

3. MULTICOLLINEARITY-AWARE
   - Groups correlated features
   - Reports group importance, not just individual
   - Warns about unreliable individual importance for correlated features

4. STABILITY ANALYSIS
   - Tests importance consistency across bootstrap samples
   - Flags unstable features
   - Provides reliability scores

Usage:
    from acdso import ACDSOClassifier, SafeFeatureImportance
    
    clf = ACDSOClassifier()
    clf.fit(X_train, y_train)
    
    # Get safe importance
    importance = SafeFeatureImportance(clf)
    report = importance.compute(X_train, y_train)
    
    # Print ranked features
    importance.print_report()
    
    # Get reliable features only
    reliable = importance.get_reliable_features(min_stability=0.7)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple, Union

import numpy as np
import pandas as pd
from sklearn.base import BaseEstimator, clone
from sklearn.model_selection import cross_val_score, StratifiedKFold, KFold
from sklearn.inspection import permutation_importance
from sklearn.linear_model import LinearRegression
from scipy import stats
from scipy.cluster import hierarchy


@dataclass
class FeatureImportanceResult:
    """
    Safe feature importance result for a single feature.
    
    Attributes
    ----------
    name : str
        Feature name.
    importance : float
        Mean importance score (higher = more important).
    importance_std : float
        Standard deviation of importance across folds/samples.
    stability : float
        Stability score (0-1). Higher = more consistent importance.
    is_reliable : bool
        Whether this importance is considered reliable.
    correlated_with : List[str]
        Other features this is highly correlated with.
    correlation_group : Optional[int]
        Group ID if part of a correlated cluster.
    group_importance : Optional[float]
        Combined importance of the correlation group.
    rank : int
        Rank among all features (1 = most important).
    notes : List[str]
        Warnings or notes about this feature's importance.
    """
    name: str
    importance: float
    importance_std: float
    stability: float
    is_reliable: bool
    correlated_with: List[str] = field(default_factory=list)
    correlation_group: Optional[int] = None
    group_importance: Optional[float] = None
    rank: int = 0
    notes: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "importance": self.importance,
            "importance_std": self.importance_std,
            "stability": self.stability,
            "is_reliable": self.is_reliable,
            "correlated_with": self.correlated_with,
            "correlation_group": self.correlation_group,
            "group_importance": self.group_importance,
            "rank": self.rank,
            "notes": self.notes,
        }


@dataclass
class SafeImportanceReport:
    """
    Complete safe feature importance report.
    
    Attributes
    ----------
    features : List[FeatureImportanceResult]
        Importance results for each feature, sorted by importance.
    correlation_groups : Dict[int, List[str]]
        Groups of correlated features.
    method : str
        Method used for importance calculation.
    n_repeats : int
        Number of permutation repeats.
    cv_folds : int
        Number of cross-validation folds.
    scorer : str
        Scoring metric used.
    reliable_count : int
        Number of features with reliable importance.
    warnings : List[str]
        Overall warnings about the importance analysis.
    """
    features: List[FeatureImportanceResult]
    correlation_groups: Dict[int, List[str]] = field(default_factory=dict)
    method: str = "permutation"
    n_repeats: int = 10
    cv_folds: int = 5
    scorer: str = "accuracy"
    reliable_count: int = 0
    warnings: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "features": [f.to_dict() for f in self.features],
            "correlation_groups": self.correlation_groups,
            "method": self.method,
            "n_repeats": self.n_repeats,
            "cv_folds": self.cv_folds,
            "scorer": self.scorer,
            "reliable_count": self.reliable_count,
            "warnings": self.warnings,
        }
    
    def get_top_features(self, n: int = 10, reliable_only: bool = True) -> List[str]:
        """Get top N feature names."""
        if reliable_only:
            features = [f for f in self.features if f.is_reliable]
        else:
            features = self.features
        return [f.name for f in features[:n]]
    
    def get_feature_importance(self, name: str) -> Optional[FeatureImportanceResult]:
        """Get importance result for a specific feature."""
        for f in self.features:
            if f.name == name:
                return f
        return None


class SafeFeatureImportance:
    """
    Production-safe feature importance calculator.
    
    Addresses common pitfalls in feature importance:
    - Model-specific biases (uses permutation importance)
    - Single-split variance (uses cross-validation)
    - Multicollinearity issues (groups correlated features)
    - Instability (bootstrap analysis)
    
    Parameters
    ----------
    estimator : BaseEstimator
        Fitted scikit-learn compatible estimator. Can be an ACDSO model.
    scorer : str, default="accuracy"
        Scoring metric for importance calculation.
    n_repeats : int, default=10
        Number of permutation repeats.
    cv_folds : int, default=5
        Number of cross-validation folds for stability analysis.
    n_bootstrap : int, default=20
        Number of bootstrap samples for stability analysis.
    correlation_threshold : float, default=0.7
        Threshold for considering features as correlated.
    stability_threshold : float, default=0.5
        Minimum stability score for reliable importance.
    random_state : int, default=42
        Random seed for reproducibility.
    n_jobs : int, default=-1
        Number of parallel jobs.
    verbose : int, default=0
        Verbosity level.
        
    Attributes
    ----------
    report_ : SafeImportanceReport
        Complete importance report after compute().
    feature_names_ : List[str]
        Feature names.
    correlation_matrix_ : np.ndarray
        Pairwise correlation matrix.
    
    Examples
    --------
    >>> from acdso import ACDSOClassifier, SafeFeatureImportance
    >>> 
    >>> clf = ACDSOClassifier(scorer="f1")
    >>> clf.fit(X_train, y_train)
    >>> 
    >>> # Compute safe importance
    >>> fi = SafeFeatureImportance(clf)
    >>> report = fi.compute(X_train, y_train)
    >>> 
    >>> # Print ranked features
    >>> fi.print_report()
    >>> 
    >>> # Get only reliable features
    >>> reliable = fi.get_reliable_features()
    >>> print(reliable)
    """
    
    def __init__(
        self,
        estimator: BaseEstimator,
        scorer: str = "accuracy",
        n_repeats: int = 10,
        cv_folds: int = 5,
        n_bootstrap: int = 20,
        correlation_threshold: float = 0.7,
        stability_threshold: float = 0.5,
        random_state: int = 42,
        n_jobs: int = -1,
        verbose: int = 0,
    ):
        self.estimator = estimator
        self.scorer = scorer
        self.n_repeats = n_repeats
        self.cv_folds = cv_folds
        self.n_bootstrap = n_bootstrap
        self.correlation_threshold = correlation_threshold
        self.stability_threshold = stability_threshold
        self.random_state = random_state
        self.n_jobs = n_jobs
        self.verbose = verbose
        
        # Results
        self.report_: Optional[SafeImportanceReport] = None
        self.feature_names_: List[str] = []
        self.correlation_matrix_: Optional[np.ndarray] = None
    
    def _log(self, msg: str) -> None:
        if self.verbose:
            print(f"[SafeFeatureImportance] {msg}")
    
    def _get_pipeline(self) -> BaseEstimator:
        """Extract the fitted pipeline/model."""
        if hasattr(self.estimator, 'best_estimator_'):
            return self.estimator.best_estimator_
        return self.estimator
    
    def _compute_correlations(self, X: pd.DataFrame) -> Tuple[np.ndarray, Dict[int, List[str]]]:
        """Compute correlation matrix and identify correlated groups."""
        # Get numeric columns only
        numeric_cols = X.select_dtypes(include=[np.number]).columns.tolist()
        
        if len(numeric_cols) < 2:
            return np.eye(len(X.columns)), {}
        
        X_numeric = X[numeric_cols].copy()
        
        # Handle missing values
        X_numeric = X_numeric.fillna(X_numeric.median())
        
        # Compute correlation matrix
        corr_matrix = np.abs(X_numeric.corr().values)
        np.fill_diagonal(corr_matrix, 0)
        
        # Cluster correlated features
        groups = {}
        if corr_matrix.shape[0] > 1:
            try:
                # Hierarchical clustering
                linkage = hierarchy.linkage(1 - corr_matrix, method='complete')
                clusters = hierarchy.fcluster(linkage, t=1 - self.correlation_threshold, criterion='distance')
                
                for i, cluster_id in enumerate(clusters):
                    if cluster_id not in groups:
                        groups[cluster_id] = []
                    groups[cluster_id].append(numeric_cols[i])
                
                # Only keep groups with more than one feature
                groups = {k: v for k, v in groups.items() if len(v) > 1}
            except Exception:
                pass
        
        return corr_matrix, groups
    
    def _compute_permutation_importance(
        self,
        X: np.ndarray,
        y: np.ndarray,
        model: BaseEstimator,
    ) -> Tuple[np.ndarray, np.ndarray]:
        """Compute permutation importance."""
        result = permutation_importance(
            model, X, y,
            n_repeats=self.n_repeats,
            random_state=self.random_state,
            n_jobs=self.n_jobs,
            scoring=self.scorer,
        )
        return result.importances_mean, result.importances_std
    
    def _compute_cv_importance(
        self,
        X: pd.DataFrame,
        y: np.ndarray,
    ) -> Tuple[np.ndarray, np.ndarray]:
        """Compute cross-validated permutation importance."""
        X_arr = X.values if hasattr(X, 'values') else np.asarray(X)
        y_arr = np.asarray(y)
        
        n_features = X_arr.shape[1]
        all_importances = []
        
        # Determine CV splitter
        if len(np.unique(y_arr)) <= 10:  # Classification
            cv = StratifiedKFold(n_splits=self.cv_folds, shuffle=True, random_state=self.random_state)
        else:  # Regression
            cv = KFold(n_splits=self.cv_folds, shuffle=True, random_state=self.random_state)
        
        pipeline = self._get_pipeline()
        
        for fold_idx, (train_idx, val_idx) in enumerate(cv.split(X_arr, y_arr)):
            self._log(f"Computing importance for fold {fold_idx + 1}/{self.cv_folds}")
            
            X_train, X_val = X_arr[train_idx], X_arr[val_idx]
            y_train, y_val = y_arr[train_idx], y_arr[val_idx]
            
            # Clone and fit model
            try:
                model = clone(pipeline)
                model.fit(X_train, y_train)
            except Exception:
                # Use original fitted model if cloning fails
                model = pipeline
            
            # Compute permutation importance on validation set
            imp_mean, _ = self._compute_permutation_importance(X_val, y_val, model)
            all_importances.append(imp_mean)
        
        all_importances = np.array(all_importances)
        mean_importance = np.mean(all_importances, axis=0)
        std_importance = np.std(all_importances, axis=0)
        
        return mean_importance, std_importance
    
    def _compute_stability(
        self,
        X: pd.DataFrame,
        y: np.ndarray,
    ) -> np.ndarray:
        """Compute importance stability via bootstrap."""
        X_arr = X.values if hasattr(X, 'values') else np.asarray(X)
        y_arr = np.asarray(y)
        
        n_samples = X_arr.shape[0]
        n_features = X_arr.shape[1]
        
        rng = np.random.RandomState(self.random_state)
        bootstrap_rankings = []
        
        pipeline = self._get_pipeline()
        
        for i in range(self.n_bootstrap):
            # Bootstrap sample
            idx = rng.choice(n_samples, size=n_samples, replace=True)
            X_boot = X_arr[idx]
            y_boot = y_arr[idx]
            
            # Fit and compute importance
            try:
                model = clone(pipeline)
                model.fit(X_boot, y_boot)
                imp_mean, _ = self._compute_permutation_importance(X_boot, y_boot, model)
            except Exception:
                imp_mean, _ = self._compute_permutation_importance(X_boot, y_boot, pipeline)
            
            # Get ranking
            ranking = np.argsort(np.argsort(-imp_mean))  # Rank (0 = most important)
            bootstrap_rankings.append(ranking)
        
        bootstrap_rankings = np.array(bootstrap_rankings)
        
        # Stability = 1 - normalized std of rankings
        rank_std = np.std(bootstrap_rankings, axis=0)
        max_std = n_features / 2  # Maximum possible std
        stability = 1 - (rank_std / max_std)
        stability = np.clip(stability, 0, 1)
        
        return stability
    
    def compute(
        self,
        X: Union[pd.DataFrame, np.ndarray],
        y: Union[pd.Series, np.ndarray],
    ) -> SafeImportanceReport:
        """
        Compute safe feature importance.
        
        Parameters
        ----------
        X : DataFrame or array
            Features.
        y : Series or array
            Target.
            
        Returns
        -------
        SafeImportanceReport
            Complete importance report.
        """
        # Convert to DataFrame
        if not isinstance(X, pd.DataFrame):
            X = pd.DataFrame(X, columns=[f"feature_{i}" for i in range(X.shape[1])])
        
        self.feature_names_ = list(X.columns)
        y_arr = np.asarray(y)
        
        self._log(f"Computing safe importance for {len(self.feature_names_)} features")
        
        # Step 1: Compute correlations and groups
        self._log("Analyzing feature correlations...")
        self.correlation_matrix_, correlation_groups = self._compute_correlations(X)
        
        # Step 2: Cross-validated permutation importance
        self._log("Computing cross-validated permutation importance...")
        mean_importance, std_importance = self._compute_cv_importance(X, y_arr)
        
        # Step 3: Stability analysis
        self._log("Analyzing importance stability...")
        stability = self._compute_stability(X, y_arr)
        
        # Step 4: Build feature results
        warnings = []
        features = []
        
        # Find correlated features for each feature
        feature_to_group = {}
        for group_id, group_features in correlation_groups.items():
            for feat in group_features:
                feature_to_group[feat] = group_id
        
        # Compute group importances
        group_importances = {}
        for group_id, group_features in correlation_groups.items():
            group_indices = [self.feature_names_.index(f) for f in group_features if f in self.feature_names_]
            if group_indices:
                group_importances[group_id] = np.mean(mean_importance[group_indices])
        
        for i, name in enumerate(self.feature_names_):
            # Find correlated features
            correlated = []
            group_id = feature_to_group.get(name)
            if group_id is not None:
                correlated = [f for f in correlation_groups[group_id] if f != name]
            
            # Determine reliability
            is_reliable = stability[i] >= self.stability_threshold
            
            # Notes
            notes = []
            if len(correlated) > 0:
                notes.append(f"Correlated with {len(correlated)} other features")
                is_reliable = False  # Correlated features have unreliable individual importance
            if stability[i] < 0.3:
                notes.append("Very unstable importance - treat with caution")
            if mean_importance[i] < 0:
                notes.append("Negative importance - may hurt predictions")
            
            features.append(FeatureImportanceResult(
                name=name,
                importance=float(mean_importance[i]),
                importance_std=float(std_importance[i]),
                stability=float(stability[i]),
                is_reliable=is_reliable,
                correlated_with=correlated,
                correlation_group=group_id,
                group_importance=group_importances.get(group_id),
                notes=notes,
            ))
        
        # Sort by importance and assign ranks
        features.sort(key=lambda x: x.importance, reverse=True)
        for rank, feat in enumerate(features, 1):
            feat.rank = rank
        
        # Overall warnings
        n_correlated = len([f for f in features if len(f.correlated_with) > 0])
        if n_correlated > len(features) * 0.3:
            warnings.append(f"High multicollinearity: {n_correlated}/{len(features)} features are correlated")
        
        n_unstable = len([f for f in features if f.stability < self.stability_threshold])
        if n_unstable > len(features) * 0.3:
            warnings.append(f"Many unstable features: {n_unstable}/{len(features)} have low stability")
        
        reliable_count = len([f for f in features if f.is_reliable])
        
        self.report_ = SafeImportanceReport(
            features=features,
            correlation_groups=correlation_groups,
            method="permutation_cv",
            n_repeats=self.n_repeats,
            cv_folds=self.cv_folds,
            scorer=self.scorer,
            reliable_count=reliable_count,
            warnings=warnings,
        )
        
        self._log(f"Done. {reliable_count}/{len(features)} features have reliable importance.")
        
        return self.report_
    
    def get_reliable_features(self, min_stability: float = None) -> List[str]:
        """
        Get list of features with reliable importance.
        
        Parameters
        ----------
        min_stability : float, optional
            Minimum stability threshold. Uses instance default if not provided.
            
        Returns
        -------
        List[str]
            Feature names with reliable importance.
        """
        if self.report_ is None:
            raise RuntimeError("Call compute() first")
        
        threshold = min_stability if min_stability is not None else self.stability_threshold
        return [
            f.name for f in self.report_.features
            if f.stability >= threshold and len(f.correlated_with) == 0
        ]
    
    def get_top_features(self, n: int = 10, reliable_only: bool = True) -> List[str]:
        """Get top N most important features."""
        if self.report_ is None:
            raise RuntimeError("Call compute() first")
        return self.report_.get_top_features(n, reliable_only)
    
    def print_report(self, top_n: int = 20) -> None:
        """Print formatted importance report."""
        if self.report_ is None:
            raise RuntimeError("Call compute() first")
        
        print("\n" + "=" * 80)
        print("  SAFE FEATURE IMPORTANCE REPORT")
        print("=" * 80)
        
        print(f"\n  Method: {self.report_.method}")
        print(f"  Scorer: {self.report_.scorer}")
        print(f"  CV Folds: {self.report_.cv_folds}")
        print(f"  Permutation Repeats: {self.report_.n_repeats}")
        print(f"  Reliable Features: {self.report_.reliable_count}/{len(self.report_.features)}")
        
        if self.report_.warnings:
            print("\n  ⚠ WARNINGS:")
            for w in self.report_.warnings:
                print(f"    • {w}")
        
        if self.report_.correlation_groups:
            print(f"\n  CORRELATED FEATURE GROUPS ({len(self.report_.correlation_groups)}):")
            for group_id, features in list(self.report_.correlation_groups.items())[:5]:
                print(f"    Group {group_id}: {', '.join(features[:5])}" + 
                      (f" (+{len(features)-5} more)" if len(features) > 5 else ""))
        
        print("\n  TOP FEATURES:")
        print("-" * 78)
        print(f"  {'Rank':<5} {'Feature':<25} {'Importance':>12} {'Stability':>10} {'Reliable':>10}")
        print("-" * 78)
        
        for f in self.report_.features[:top_n]:
            reliable = "✓" if f.is_reliable else "✗"
            if f.correlated_with:
                reliable = "~"  # Correlated
            print(f"  {f.rank:<5} {f.name[:25]:<25} {f.importance:>12.4f} {f.stability:>10.2f} {reliable:>10}")
        
        if len(self.report_.features) > top_n:
            print(f"\n  ... and {len(self.report_.features) - top_n} more features")
        
        print("\n  Legend: ✓ = Reliable, ✗ = Unstable, ~ = Correlated (use group importance)")
        print("=" * 80)
    
    def to_dataframe(self) -> pd.DataFrame:
        """Convert report to DataFrame."""
        if self.report_ is None:
            raise RuntimeError("Call compute() first")
        
        return pd.DataFrame([f.to_dict() for f in self.report_.features])


def compute_safe_importance(
    estimator: BaseEstimator,
    X: Union[pd.DataFrame, np.ndarray],
    y: Union[pd.Series, np.ndarray],
    scorer: str = "accuracy",
    verbose: int = 0,
) -> SafeImportanceReport:
    """
    Convenience function to compute safe feature importance.
    
    Parameters
    ----------
    estimator : BaseEstimator
        Fitted model.
    X : DataFrame or array
        Features.
    y : Series or array
        Target.
    scorer : str
        Scoring metric.
    verbose : int
        Verbosity level.
        
    Returns
    -------
    SafeImportanceReport
        Importance report.
        
    Examples
    --------
    >>> from acdso import ACDSOClassifier, compute_safe_importance
    >>> clf = ACDSOClassifier()
    >>> clf.fit(X_train, y_train)
    >>> report = compute_safe_importance(clf, X_train, y_train)
    >>> print(report.get_top_features(10))
    """
    fi = SafeFeatureImportance(estimator, scorer=scorer, verbose=verbose)
    return fi.compute(X, y)

