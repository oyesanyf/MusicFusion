"""
ACDSO v3.0 - Core Engine Module
================================

Production-grade, scikit-learn-compatible AutoML estimators with
robustness-aware optimization.

This module contains:
- _ACDSOEngineV3: Internal optimization engine with v3 innovations
- ACDSOClassifier: scikit-learn compatible classifier
- ACDSORegressor: scikit-learn compatible regressor

Key features (v3):
- Stress testing + robustness objective (6-objective optimization)
- Successive halving (multi-fidelity search) for efficient evaluation
- Learned risk calibration (predicts robustness from dataset features)
- Continuous, severity-aware risk penalties
- Multi-objective evaluation: metric, time, memory, latency, risk, robustness
- Constraint-based pruning + Pareto knee-point selection
- JSON-serializable audit trail artifacts
"""

from __future__ import annotations

import math
import time
import tracemalloc
import warnings
from dataclasses import asdict, dataclass, field
from typing import Any, Callable, Dict, List, Optional, Tuple

import numpy as np
import pandas as pd

from sklearn.base import BaseEstimator, ClassifierMixin, RegressorMixin, clone
from sklearn.ensemble import RandomForestClassifier, RandomForestRegressor
from sklearn.feature_selection import SelectKBest, f_classif, f_regression
from sklearn.linear_model import LogisticRegression, Ridge, ElasticNet, Lasso
from sklearn.metrics import get_scorer
from sklearn.model_selection import KFold, StratifiedKFold, cross_val_score
from sklearn.pipeline import Pipeline

from acdso.pareto import CandidateResult, knee_point_selection, pareto_front
from acdso.risk import RiskReport, analyze_risk, compute_risk_penalty
from acdso.utils import build_preprocessor, measure_latency_ms_per_row, split_columns

# Optional boosting libraries
try:
    from xgboost import XGBClassifier, XGBRegressor
    _HAS_XGB = True
except ImportError:
    _HAS_XGB = False

try:
    from lightgbm import LGBMClassifier, LGBMRegressor
    _HAS_LGBM = True
except ImportError:
    _HAS_LGBM = False


# ============================================================================
# Stress Testing Infrastructure (v3 Innovation)
# ============================================================================

@dataclass
class StressTestResult:
    """Result from a single stress test."""
    name: str
    stressed_metric: float
    delta: float  # baseline - stressed (positive = degradation)


def _safe_numpy(X: Any) -> np.ndarray:
    """Convert X to numpy array without copying when possible."""
    if isinstance(X, np.ndarray):
        return X
    try:
        return np.asarray(X)
    except Exception:
        return np.array(X)


def _apply_missingness_noise(X: np.ndarray, missing_frac: float, rng: np.random.RandomState) -> np.ndarray:
    """Inject missing values into X."""
    Xn = X.copy()
    if Xn.ndim == 1:
        Xn = Xn.reshape(-1, 1)
    n, d = Xn.shape
    k = int(max(1, missing_frac * n * d))
    idx = rng.choice(n * d, size=k, replace=False)
    # Use np.nan as missing marker; preprocessor should handle imputation
    Xn.flat[idx] = np.nan
    return Xn


def _apply_feature_jitter(X: np.ndarray, frac: float, rng: np.random.RandomState) -> np.ndarray:
    """Small gaussian jitter on a subset of numeric cells."""
    Xn = X.copy()
    if Xn.ndim == 1:
        Xn = Xn.reshape(-1, 1)
    # Only jitter if numeric
    if not np.issubdtype(Xn.dtype, np.number):
        return Xn
    n, d = Xn.shape
    k = int(max(1, frac * n * d))
    idx = rng.choice(n * d, size=k, replace=False)
    std = np.nanstd(Xn) if np.isfinite(np.nanstd(Xn)) else 1.0
    noise = rng.normal(loc=0.0, scale=0.01 * (std if std > 0 else 1.0), size=k)
    Xn.flat[idx] = Xn.flat[idx] + noise
    return Xn


def _apply_label_noise(y: np.ndarray, frac: float, rng: np.random.RandomState) -> np.ndarray:
    """Flip a small percentage of labels to different classes."""
    yn = y.copy()
    n = len(yn)
    k = int(max(1, frac * n))
    idx = rng.choice(n, size=k, replace=False)

    classes = np.unique(yn)
    if len(classes) <= 1:
        return yn

    # For each selected label, swap to a different class
    for i in idx:
        current = yn[i]
        choices = classes[classes != current]
        if len(choices) > 0:
            yn[i] = rng.choice(choices)
    return yn


def run_stress_tests(
    fitted_pipe: Pipeline,
    X_val: Any,
    y_val: Any,
    task: str,
    scorer_name: str,
    random_state: int,
) -> Tuple[float, List[StressTestResult], float]:
    """
    Run stress tests to measure robustness under perturbations (v3).
    
    Uses validation data (held-out from training) to get more realistic
    estimates of robustness degradation on unseen data.
    
    Parameters
    ----------
    fitted_pipe : Pipeline
        Already fitted sklearn Pipeline.
    X_val : array-like
        Validation features (held-out from training).
    y_val : array-like
        Validation targets (held-out from training).
    task : str
        Task type ("classification" or "regression").
    scorer_name : str
        Name of the scorer to use.
    random_state : int
        Random seed for reproducibility.
    
    Returns
    -------
    Tuple[float, List[StressTestResult], float]
        (baseline_metric, stress_results, worst_case_drop)
    """
    scorer = get_scorer(scorer_name)
    baseline = float(scorer(fitted_pipe, X_val, y_val))

    rng = np.random.RandomState(random_state)
    Xnp = _safe_numpy(X_val)
    ynp = np.asarray(y_val)

    tests: List[StressTestResult] = []

    # 1) Missingness injection
    try:
        Xm = _apply_missingness_noise(Xnp, missing_frac=0.02, rng=rng)
        m_score = float(scorer(fitted_pipe, Xm, y_val))
        tests.append(StressTestResult("missingness_2pct", m_score, baseline - m_score))
    except Exception:
        pass  # Skip if fails

    # 2) Feature jitter (numeric)
    try:
        Xj = _apply_feature_jitter(Xnp, frac=0.02, rng=rng)
        j_score = float(scorer(fitted_pipe, Xj, y_val))
        tests.append(StressTestResult("feature_jitter_2pct", j_score, baseline - j_score))
    except Exception:
        pass

    # 3) Label noise (classification only) - tests robustness to potential label errors
    if task == "classification":
        try:
            y_noisy = _apply_label_noise(ynp, frac=0.02, rng=rng)
            ln_score = float(scorer(fitted_pipe, X_val, y_noisy))
            tests.append(StressTestResult("label_noise_2pct", ln_score, baseline - ln_score))
        except Exception:
            pass

    worst_drop = float(max(0.0, max(t.delta for t in tests))) if tests else 0.0
    return baseline, tests, worst_drop


def _auto_adjust_scorer(
    scorer: str,
    y: Any,
    task: str,
    llm_provider: Any = None,
    column_name: str = "target",
) -> str:
    """
    Automatically adjust the scorer based on target labels.
    
    Uses LLM-powered analysis if a provider is available, otherwise
    falls back to rule-based logic.
    
    Handles:
    - Multiclass classification: convert f1 → f1_weighted
    - Binary with non-standard labels (not 0/1): convert f1 → f1_weighted
    - Regression: no changes needed
    
    Parameters
    ----------
    scorer : str
        Original scorer name.
    y : array-like
        Target values.
    task : str
        Task type ("classification" or "regression").
    llm_provider : Any, optional
        LLM provider for intelligent label analysis.
    column_name : str, default="target"
        Name of the target column (for LLM context).
        
    Returns
    -------
    str
        Adjusted scorer name.
    """
    if task != "classification":
        return scorer
    
    # Try LLM-powered analysis if provider is available
    if llm_provider is not None:
        try:
            from acdso.advisors import LLMLabelAnalyzer
            analyzer = LLMLabelAnalyzer(provider=llm_provider)
            result = analyzer.analyze(y, column_name=column_name)
            
            # If LLM recommends a different scorer, use it
            if result.recommended_scorer and result.recommended_scorer != scorer:
                return result.recommended_scorer
        except Exception:
            # Fall back to rule-based on any error
            pass
    
    # Rule-based fallback
    y_arr = np.asarray(y)
    unique_labels = np.unique(y_arr)
    n_classes = len(unique_labels)
    
    # Scorers that require pos_label=1 (binary-only)
    binary_only_scorers = ["f1", "precision", "recall", "roc_auc"]
    
    # Check if scorer needs adjustment
    if scorer in binary_only_scorers:
        # Multiclass: always use weighted version
        if n_classes > 2:
            if scorer == "roc_auc":
                return "roc_auc_ovr_weighted"
            return f"{scorer}_weighted"
        
        # Binary but labels are not {0, 1}: use weighted version
        if n_classes == 2:
            labels_set = set(unique_labels)
            if labels_set != {0, 1}:
                if scorer == "roc_auc":
                    return "roc_auc_ovr_weighted"
                return f"{scorer}_weighted"
    
    return scorer


class _ACDSOEngineV3:
    """
    Internal ACDSO optimization engine (v3).

    This class handles the core optimization loop including:
    - Risk analysis with learned calibration
    - Candidate generation with adaptive feature selection
    - Stress testing for robustness measurement
    - Multi-objective evaluation (6 objectives)
    - Successive halving for efficient candidate evaluation
    - Pareto front construction with robustness awareness
    - Knee-point selection

    v3 Innovations:
    - Stress testing: missingness, feature jitter, label noise
    - Robustness drop as 6th optimization objective
    - Multi-fidelity search via successive halving
    - Learned risk calibration from dataset features

    Not intended for direct use; use ACDSOClassifier or ACDSORegressor instead.
    """

    def __init__(
        self,
        task: str,
        scorer: str,
        cv: int,
        random_state: Optional[int],
        time_budget_s: Optional[float],
        max_candidates: int,
        preference: str,
        max_train_time_s: Optional[float],
        max_peak_mem_mb: Optional[float],
        max_latency_ms_per_row: Optional[float],
        enable_feature_selection: bool,
        llm_advisor: Optional[Callable[[Dict[str, Any]], Dict[str, Any]]],
        verbose: int,
    ):
        self.task = task
        self.scorer = scorer  # Will be adjusted in fit() based on y
        self._original_scorer = scorer  # Keep original for reference
        self.cv = cv
        self.random_state = 42 if random_state is None else int(random_state)
        self.time_budget_s = time_budget_s
        self.max_candidates = max_candidates
        self.preference = preference

        self.max_train_time_s = max_train_time_s
        self.max_peak_mem_mb = max_peak_mem_mb
        self.max_latency_ms_per_row = max_latency_ms_per_row

        self.enable_feature_selection = enable_feature_selection
        self.llm_advisor = llm_advisor
        self.verbose = verbose

        # Results (populated after fit)
        self.risk_report_: Optional[RiskReport] = None
        self.pareto_front_: List[CandidateResult] = []
        self.all_results_: List[CandidateResult] = []
        self.best_result_: Optional[CandidateResult] = None
        self.audit_trail_: Dict[str, Any] = {}

    def _log(self, msg: str) -> None:
        """Log message if verbose mode is enabled."""
        if self.verbose:
            print(f"[ACDSO v3] {msg}")

    def _generate_candidates(
        self,
        X: Any,
        y: Any,
        rr: RiskReport,
    ) -> List[Tuple[str, Pipeline]]:
        """
        Generate candidate pipelines based on risk profile and dataset characteristics.

        Candidate generation is adaptive:
        - High multicollinearity → add regularization or feature selection
        - Severe imbalance → prefer class weights
        - Small datasets → simpler models
        - Large datasets → scalable learners (boosting if available)
        """
        df = pd.DataFrame(X)
        num_cols, cat_cols = split_columns(df)

        pre = build_preprocessor(num_cols, cat_cols)

        # Add feature selection if high multicollinearity detected
        # Calculate k to actually reduce features: keep top 50-75% based on VIF severity
        selector = None
        if self.enable_feature_selection and rr.vif_max >= 10.0:
            score_func = f_classif if self.task == "classification" else f_regression
            # Determine k: more aggressive selection for higher VIF
            # VIF 10-20: keep 75% of features, VIF 20-50: keep 60%, VIF >50: keep 50%
            if rr.vif_max >= 50.0:
                k_ratio = 0.5
            elif rr.vif_max >= 20.0:
                k_ratio = 0.6
            else:
                k_ratio = 0.75
            
            # Calculate actual k (ensure at least 5 features remain for small datasets)
            n_features = len(num_cols) if num_cols else rr.n_cols
            k = max(5, int(n_features * k_ratio))
            k = min(k, n_features)  # Don't exceed available features
            
            selector = SelectKBest(score_func=score_func, k=k)

        small = rr.n_rows < 1000
        large = rr.n_rows >= 50000

        models: List[Tuple[str, Any]] = []

        if self.task == "classification":
            # Logistic Regression
            # Logistic Regression with different regularization
            lr_l2 = LogisticRegression(max_iter=2000, penalty='l2', C=1.0, random_state=self.random_state)
            lr_weak = LogisticRegression(max_iter=2000, penalty='l2', C=0.1, random_state=self.random_state)
            
            # Random Forest with different configurations
            rf_shallow = RandomForestClassifier(
                n_estimators=80 if small else 150,
                max_depth=5,
                random_state=self.random_state,
                n_jobs=-1,
            )
            rf_medium = RandomForestClassifier(
                n_estimators=80 if small else 150,
                max_depth=10,
                random_state=self.random_state,
                n_jobs=-1,
            )
            rf_deep = RandomForestClassifier(
                n_estimators=80 if small else 200,
                random_state=self.random_state,
                n_jobs=-1,
            )

            # Handle class imbalance
            if rr.class_imbalance_max_ratio is not None and rr.class_imbalance_max_ratio >= 0.85:
                for model in [lr_l2, lr_weak]:
                    try:
                        model.set_params(class_weight="balanced")
                    except Exception:
                        pass
                for model in [rf_shallow, rf_medium, rf_deep]:
                    try:
                        model.set_params(class_weight="balanced")
                    except Exception:
                        pass

            models.append(("LogisticRegression", lr_l2))
            models.append(("LogisticRegression_C0.1", lr_weak))
            models.append(("RandomForest_d5", rf_shallow))
            models.append(("RandomForest_d10", rf_medium))
            models.append(("RandomForest", rf_deep))

            # Add boosting for medium and large datasets (≥3000 rows)
            if not small and _HAS_LGBM:
                models.append(("LightGBM", LGBMClassifier(
                    random_state=self.random_state,
                    verbose=-1,
                    n_estimators=100,
                )))
            if not small and _HAS_XGB:
                models.append(("XGBoost", XGBClassifier(
                    random_state=self.random_state,
                    n_jobs=-1,
                    verbosity=0,
                    n_estimators=100,
                )))

        else:  # regression
            # Ridge Regression (with different alpha values)
            models.append(("Ridge_alpha0.1", Ridge(alpha=0.1, random_state=self.random_state)))
            models.append(("Ridge_alpha1.0", Ridge(alpha=1.0, random_state=self.random_state)))
            models.append(("Ridge_alpha10", Ridge(alpha=10.0, random_state=self.random_state)))

            # ElasticNet
            models.append(("ElasticNet", ElasticNet(random_state=self.random_state, max_iter=2000)))
            
            # Lasso
            models.append(("Lasso", Lasso(random_state=self.random_state, max_iter=2000)))

            # Random Forest Regressor (with different configurations)
            models.append(("RandomForest_d5", RandomForestRegressor(
                n_estimators=100 if small else 200,
                max_depth=5,
                random_state=self.random_state,
                n_jobs=-1,
            )))
            models.append(("RandomForest_d10", RandomForestRegressor(
                n_estimators=100 if small else 200,
                max_depth=10,
                random_state=self.random_state,
                n_jobs=-1,
            )))
            models.append(("RandomForestRegressor", RandomForestRegressor(
                n_estimators=100 if small else 250,
                random_state=self.random_state,
                n_jobs=-1,
            )))

            # Add boosting for medium and large datasets (≥3000 rows)
            if not small and _HAS_LGBM:
                models.append(("LightGBMRegressor", LGBMRegressor(
                    random_state=self.random_state,
                    verbose=-1,
                    n_estimators=100,
                )))
            if not small and _HAS_XGB:
                models.append(("XGBoostRegressor", XGBRegressor(
                    random_state=self.random_state,
                    n_jobs=-1,
                    verbosity=0,
                    n_estimators=100,
                )))

        # Build pipelines
        candidates: List[Tuple[str, Pipeline]] = []
        for name, model in models[: max(1, self.max_candidates)]:
            steps = [("pre", pre)]
            if selector is not None:
                steps.append(("select", selector))
            steps.append(("model", model))
            candidates.append((name, Pipeline(steps=steps)))

        return candidates

    def _successive_halving_plan(self, n_candidates: int) -> List[Dict[str, Any]]:
        """
        Two-stage plan for successive halving (v3 multi-fidelity search):
        - Stage 0 (cheap): 25% rows, reduced CV folds, keep top ~60%
        - Stage 1 (full): 100% rows, full CV folds, keep all for final Pareto selection
        
        The "full" stage uses row_frac=1.0 which ensures _subsample_rows returns
        the complete dataset for final evaluation.
        
        Keep at least 3 candidates to ensure meaningful comparison.
        """
        # Keep 60% of candidates, but at least 3 (or all if fewer than 3)
        keep0 = max(min(3, n_candidates), int(math.ceil(n_candidates * 0.6)))
        return [
            {"name": "cheap", "row_frac": 0.25, "cv": max(2, self.cv // 2), "keep": keep0},
            {"name": "full", "row_frac": 1.0, "cv": self.cv, "keep": n_candidates},
        ]

    def _subsample_rows(self, X: Any, y: Any, frac: float) -> Tuple[Any, Any]:
        """
        Subsample rows for multi-fidelity evaluation.
        
        For frac >= 1.0, returns the original data unchanged (full evaluation).
        """
        # Explicit check for full data (no subsampling)
        if frac >= 1.0:
            return X, y
        
        n = len(y)
        rng = np.random.RandomState(self.random_state)
        sample_size = max(10, int(n * frac))
        idx = rng.choice(n, size=sample_size, replace=False)
        
        # Support pandas/numpy/list
        if hasattr(X, "iloc"):
            Xs = X.iloc[idx]
        else:
            Xs = _safe_numpy(X)[idx]
        ys = np.asarray(y)[idx]
        return Xs, ys

    def _evaluate_candidate(
        self,
        X: Any,
        y: Any,
        name: str,
        pipe: Pipeline,
        cv_override: Optional[int] = None,
    ) -> CandidateResult:
        """
        Evaluate a single candidate pipeline.

        Measures:
        - Predictive performance (via cross-validation)
        - Training time
        - Peak memory usage
        - Inference latency
        - Risk penalty
        """
        start = time.time()
        
        # Memory measurement: try psutil for better native allocation tracking,
        # fall back to tracemalloc (which doesn't capture NumPy/BLAS/LightGBM/XGBoost native allocations)
        use_psutil = False
        try:
            import psutil
            import os
            process = psutil.Process(os.getpid())
            mem_before = process.memory_info().rss / (1024 * 1024)  # MB
            use_psutil = True
        except ImportError:
            tracemalloc.start()

        # Cross-validation (use override if provided for multi-fidelity)
        cv_used = int(cv_override or self.cv)
        if self.task == "classification":
            splitter = StratifiedKFold(
                n_splits=cv_used,
                shuffle=True,
                random_state=self.random_state,
            )
        else:
            splitter = KFold(
                n_splits=cv_used,
                shuffle=True,
                random_state=self.random_state,
            )

        scorer = get_scorer(self.scorer)
        
        # Suppress sklearn warnings during cross-validation (constant features, etc.)
        with warnings.catch_warnings():
            warnings.filterwarnings("ignore", category=UserWarning)
            warnings.filterwarnings("ignore", category=RuntimeWarning)
            scores = cross_val_score(pipe, X, y, cv=splitter, scoring=scorer, n_jobs=None)

        # Measure peak memory
        if use_psutil:
            mem_after = process.memory_info().rss / (1024 * 1024)  # MB
            peak_mem_mb = float(mem_after - mem_before)
            # Ensure non-negative (memory might have been freed)
            peak_mem_mb = max(0.0, peak_mem_mb)
        else:
            current, peak = tracemalloc.get_traced_memory()
            tracemalloc.stop()
            peak_mem_mb = float(peak / (1024 * 1024))

        train_time_s = float(time.time() - start)
        metric = float(np.mean(scores))
        cv_std = float(np.std(scores))

        # Compute risk penalty
        assert self.risk_report_ is not None
        risk_pen = compute_risk_penalty(
            self.risk_report_,
            cv_score_std=cv_std,
            preference=self.preference,
        )

        # Create train/validation split for stress testing (v3 validation-based)
        # This ensures stress tests measure degradation on held-out data, not training data
        n_samples = len(y)
        val_size = max(10, int(n_samples * 0.2))  # 20% for validation
        
        rng = np.random.RandomState(self.random_state)
        all_idx = np.arange(n_samples)
        rng.shuffle(all_idx)
        
        train_idx = all_idx[val_size:]
        val_idx = all_idx[:val_size]
        
        if hasattr(X, "iloc"):
            X_train_stress = X.iloc[train_idx]
            X_val_stress = X.iloc[val_idx]
        else:
            X_arr = _safe_numpy(X)
            X_train_stress = X_arr[train_idx]
            X_val_stress = X_arr[val_idx]
        
        y_arr = np.asarray(y)
        y_train_stress = y_arr[train_idx]
        y_val_stress = y_arr[val_idx]
        
        # Fit the pipeline for latency measurement and stress testing
        # NOTE: This fits on the train portion, not full data, to match stress test setup
        fitted = clone(pipe)
        with warnings.catch_warnings():
            warnings.filterwarnings("ignore", category=UserWarning)
            warnings.filterwarnings("ignore", category=RuntimeWarning)
            fitted.fit(X_train_stress, y_train_stress)
        
        # Measure latency on validation data (simulates production inference)
        latency = measure_latency_ms_per_row(fitted, X_val_stress)

        # Stress tests (robustness) - v3 validation-based innovation
        # Tests are run on held-out validation data for realistic robustness estimates
        baseline_fit_metric, stress_results, worst_drop = run_stress_tests(
            fitted_pipe=fitted,
            X_val=X_val_stress,
            y_val=y_val_stress,
            task=self.task,
            scorer_name=self.scorer,
            random_state=self.random_state,
        )

        # Build objective vector:
        # - metric: maximize
        # - time/mem/latency/risk/robustness_drop: minimize
        objectives = (
            metric,
            math.log1p(train_time_s),
            math.log1p(peak_mem_mb),
            math.log1p(latency),
            risk_pen,
            worst_drop,  # v3: robustness objective
        )

        return CandidateResult(
            name=name,
            pipeline=fitted,
            metric=metric,
            train_time_s=train_time_s,
            peak_mem_mb=peak_mem_mb,
            latency_ms_per_row=latency,
            risk_penalty=risk_pen,
            objectives=objectives,
            meta={
                "cv_scores": scores.tolist(),
                "cv_score_std": cv_std,
                "stress_baseline_metric": baseline_fit_metric,
                "stress_tests": [asdict(r) for r in stress_results],
                "stress_worst_drop": worst_drop,
            },
        )

    def _passes_constraints(self, res: CandidateResult) -> bool:
        """Check if a candidate passes all constraint guardrails."""
        if self.max_train_time_s is not None and res.train_time_s > self.max_train_time_s:
            return False
        if self.max_peak_mem_mb is not None and res.peak_mem_mb > self.max_peak_mem_mb:
            return False
        if self.max_latency_ms_per_row is not None and res.latency_ms_per_row > self.max_latency_ms_per_row:
            return False
        return True

    def _risk_features(self, rr: RiskReport) -> np.ndarray:
        """Extract risk features for calibration."""
        # Handle both RiskReport object and dict (from asdict)
        if isinstance(rr, dict):
            missing_ratio = rr.get('missing_ratio', 0.0)
            n_rows = rr.get('n_rows', 0)
            n_cols = rr.get('n_cols', 0)
            vif_max = rr.get('vif_max', 1.0)
            class_imbalance = rr.get('class_imbalance_max_ratio', 0.0)
            cv_std = rr.get('cv_score_std', 0.0)
        else:
            missing_ratio = getattr(rr, 'missing_ratio', 0.0)
            n_rows = rr.n_rows
            n_cols = rr.n_cols
            vif_max = rr.vif_max
            class_imbalance = rr.class_imbalance_max_ratio or 0.0
            cv_std = rr.cv_score_std or 0.0
        
        feats = [
            float(n_rows),
            float(n_cols),
            float(vif_max),
            float(missing_ratio),
            float(class_imbalance),
            float(cv_std),
        ]
        return np.array(feats, dtype=float)

    def _fit_risk_calibrator(self, rr: RiskReport, results: List[CandidateResult]) -> Optional[Ridge]:
        """
        Fit a model that predicts worst-case robustness drop from risk features + candidate signals.
        Uses Ridge for stability.
        """
        try:
            if len(results) < 2:
                return None  # Need at least 2 samples to fit
            
            Xcal = []
            ycal = []
            base_feats = self._risk_features(rr)

            for r in results:
                worst_drop = float(r.meta.get("stress_worst_drop", 0.0))
                # Add candidate-level predictors
                x = np.concatenate([
                    base_feats,
                    np.array([
                        float(r.meta.get("cv_score_std", 0.0)),
                        float(r.latency_ms_per_row)
                    ], dtype=float),
                ])
                Xcal.append(x)
                ycal.append(worst_drop)

            Xcal = np.vstack(Xcal)
            ycal = np.array(ycal, dtype=float)

            # Only fit if we have variation in targets
            if np.std(ycal) < 1e-6:
                return None

            model = Ridge(alpha=1.0, random_state=self.random_state)
            model.fit(Xcal, ycal)
            return model
        except Exception:
            return None

    def _calibrated_risk_penalty(self, rr: RiskReport, res: CandidateResult, calibrator: Optional[Ridge]) -> float:
        """
        Combine original risk_penalty with predicted robustness drop.
        """
        base = float(res.risk_penalty)
        if calibrator is None:
            return base

        try:
            x = np.concatenate([
                self._risk_features(rr),
                np.array([
                    float(res.meta.get("cv_score_std", 0.0)),
                    float(res.latency_ms_per_row)
                ], dtype=float),
            ]).reshape(1, -1)

            pred_drop = float(calibrator.predict(x)[0])
            pred_drop = max(0.0, pred_drop)

            # Blend: keep original risk penalty, add calibrated robustness risk
            return float(base + 2.0 * pred_drop)
        except Exception:
            return base

    def fit(self, X: Any, y: Any) -> "_ACDSOEngineV3":
        """
        Run the full ACDSO optimization loop.

        1. Analyze dataset risk
        2. Generate adaptive candidates
        3. Evaluate candidates with multi-objective metrics
        4. Filter by constraints
        5. Construct Pareto front
        6. Select knee-point solution
        7. Build audit trail
        """
        # Step 0: Auto-adjust scorer for multiclass or non-standard binary labels
        adjusted_scorer = _auto_adjust_scorer(self.scorer, y, self.task)
        if adjusted_scorer != self.scorer:
            self._log(f"Auto-adjusted scorer: {self.scorer} → {adjusted_scorer}")
            self.scorer = adjusted_scorer
            # Store the adjusted scorer for persistence to public estimator
            self._adjusted_scorer = adjusted_scorer
        else:
            self._adjusted_scorer = self.scorer
        
        # Step 1: Risk analysis
        self.risk_report_ = analyze_risk(
            X, y,
            task=self.task,
            scorer=self.scorer,
            cv=self.cv,
            random_state=self.random_state,
            llm_advisor=self.llm_advisor,
        )
        self._log(f"Risk report: {asdict(self.risk_report_)}")

        # Step 2: Generate candidates
        candidates = self._generate_candidates(X, y, self.risk_report_)
        self._log(f"Generated {len(candidates)} candidates.")

        # Step 3: Evaluate candidates with successive halving (v3 innovation)
        t0 = time.time()
        results: List[CandidateResult] = []
        failures: List[Dict[str, Any]] = []

        plan = self._successive_halving_plan(len(candidates))
        survivors = candidates[:]

        for stage in plan:
            if not survivors:
                break

            self._log(f"Halving stage: {stage['name']} | row_frac={stage['row_frac']} cv={stage['cv']} keep={stage['keep']}")

            stage_results: List[CandidateResult] = []
            stage_failures: List[Dict[str, Any]] = []

            Xs, ys = self._subsample_rows(X, y, stage["row_frac"])

            for name, pipe in survivors:
                # Check time budget
                if self.time_budget_s is not None and (time.time() - t0) >= self.time_budget_s:
                    self._log("Time budget reached; stopping evaluation.")
                    break

                self._log(f"Evaluating ({stage['name']}): {name}")

                try:
                    res = self._evaluate_candidate(Xs, ys, name, pipe, cv_override=stage["cv"])

                    # Constraints apply only on full stage
                    if stage["name"] == "full" and not self._passes_constraints(res):
                        self._log(f"Pruned by constraints: {name}")
                        stage_failures.append({
                            "name": name,
                            "reason": "constraints",
                            "metrics": {
                                "train_time_s": res.train_time_s,
                                "peak_mem_mb": res.peak_mem_mb,
                                "latency_ms_per_row": res.latency_ms_per_row,
                                "risk_penalty": res.risk_penalty,
                                "metric": res.metric,
                                "stress_worst_drop": res.meta.get("stress_worst_drop"),
                            },
                        })
                        continue

                    stage_results.append(res)
                    self._log(
                        f"{name}: metric={res.metric:.4f}, time={res.train_time_s:.2f}s, "
                        f"mem={res.peak_mem_mb:.1f}MB, lat={res.latency_ms_per_row:.2f}ms/row, "
                        f"risk={res.risk_penalty:.2f}, robustness_drop={res.meta.get('stress_worst_drop', 0.0):.4f}"
                    )

                except Exception as e:
                    stage_failures.append({"name": name, "reason": f"error:{type(e).__name__}", "detail": str(e)})
                    self._log(f"Failed: {name} ({type(e).__name__})")

            # Rank survivors by composite that favors accuracy + robustness
            def _rank_key(r: CandidateResult) -> float:
                worst_drop = float(r.meta.get("stress_worst_drop", 0.0))
                return float(r.metric - 0.5 * worst_drop)

            stage_results.sort(key=_rank_key, reverse=True)

            # Keep top K pipelines for next stage
            keep = min(stage["keep"], len(stage_results))
            survivors = [(r.name, clone(r.pipeline)) for r in stage_results[:keep]]

            # Keep failures for audit
            failures.extend(stage_failures)

            # If this is the full stage, these are your final results
            if stage["name"] == "full":
                results = stage_results

        if not results:
            raise RuntimeError("No candidates passed evaluation/constraints.")

        self.all_results_ = results

        # Step 3.5: Fit risk calibrator and adjust risk penalties (v3 innovation)
        calibrator = self._fit_risk_calibrator(self.risk_report_, results)
        for r in results:
            r.risk_penalty = self._calibrated_risk_penalty(self.risk_report_, r, calibrator)
            # Update objective vector's risk slot (index 4) while keeping rest
            obj = list(r.objectives)
            if len(obj) >= 5:
                obj[4] = r.risk_penalty
            r.objectives = tuple(obj)

        # Update risk report with best stability
        best_std = float(np.min([r.meta.get("cv_score_std", 0.0) for r in results]))
        self.risk_report_.cv_score_std = best_std

        # Step 5: Construct Pareto front
        front = pareto_front(results)
        self.pareto_front_ = front
        self._log(f"Pareto front size: {len(front)}")

        # Step 6: Select knee-point solution
        best = knee_point_selection(front)
        self.best_result_ = best
        self._log(f"Selected: {best.name} (knee-point)")

        # Step 7: Build audit trail
        self.audit_trail_ = {
            "risk_report": asdict(self.risk_report_),
            "constraints": {
                "max_train_time_s": self.max_train_time_s,
                "max_peak_mem_mb": self.max_peak_mem_mb,
                "max_latency_ms_per_row": self.max_latency_ms_per_row,
            },
            "candidates_evaluated": [
                {
                    "name": r.name,
                    "metric": r.metric,
                    "train_time_s": r.train_time_s,
                    "peak_mem_mb": r.peak_mem_mb,
                    "latency_ms_per_row": r.latency_ms_per_row,
                    "risk_penalty": r.risk_penalty,
                    "cv_scores": r.meta.get("cv_scores"),
                    "cv_score_std": r.meta.get("cv_score_std"),
                    "stress_worst_drop": r.meta.get("stress_worst_drop"),
                    "stress_tests": r.meta.get("stress_tests"),
                }
                for r in results
            ],
            "pareto_front": [
                {
                    "name": r.name,
                    "metric": r.metric,
                    "train_time_s": r.train_time_s,
                    "peak_mem_mb": r.peak_mem_mb,
                    "latency_ms_per_row": r.latency_ms_per_row,
                    "risk_penalty": r.risk_penalty,
                }
                for r in front
            ],
            "pruned_or_failed": failures,
            "selected": {
                "name": best.name,
                "metric": best.metric,
                "train_time_s": best.train_time_s,
                "peak_mem_mb": best.peak_mem_mb,
                "latency_ms_per_row": best.latency_ms_per_row,
                "risk_penalty": best.risk_penalty,
                "stress_worst_drop": best.meta.get("stress_worst_drop"),
            },
            "risk_calibrator": {
                "type": "ridge" if calibrator is not None else None,
                "used": calibrator is not None,
            },
        }

        return self


class ACDSOClassifier(BaseEstimator, ClassifierMixin):
    """
    ACDSO v3.0 Classifier - Risk-Optimized AutoML for Classification.

    A production-oriented AutoML classifier that unifies contextual risk
    diagnostics, adaptive pipeline synthesis, and multi-objective optimization.
    
    v3 Features:
    - Stress testing with robustness objective (6-objective optimization)
    - Successive halving for efficient candidate evaluation
    - Validation-based robustness measurement
    - Learned risk calibration

    Parameters
    ----------
    scorer : str, default="accuracy"
        Scoring metric for evaluation. Common options:
        "accuracy", "f1", "f1_macro", "f1_weighted", "roc_auc", "precision", "recall"

    cv : int, default=5
        Number of cross-validation folds.

    preference : str, default="balanced"
        Risk preference mode:
        - "safe": Higher weight on leakage and stability risks
        - "balanced": Balanced weighting across all risk factors

    time_budget_s : float, optional
        Maximum total time budget in seconds. If exceeded, evaluation stops.

    max_candidates : int, default=12
        Maximum number of candidate pipelines to evaluate.
        Increased from 4 to provide better AutoML search coverage.

    random_state : int, optional, default=42
        Random seed for reproducibility.

    max_train_time_s : float, optional
        Maximum allowed training time per candidate (constraint).

    max_peak_mem_mb : float, optional
        Maximum allowed peak memory usage (constraint).

    max_latency_ms_per_row : float, optional
        Maximum allowed inference latency per row (constraint).

    enable_feature_selection : bool, default=True
        Whether to enable automatic feature selection for high-multicollinearity data.

    llm_advisor : callable, optional
        Optional LLM advisor function for semantic analysis.
        Should accept Dict[str, Any] and return Dict[str, Any].

    verbose : int, default=0
        Verbosity level. 0 = silent, 1+ = progress messages.

    Attributes
    ----------
    best_estimator_ : Pipeline
        The selected best pipeline after fitting.

    risk_report_ : dict
        Comprehensive risk report for the dataset.

    pareto_front_ : list
        List of Pareto-optimal candidates.

    audit_trail_ : dict
        JSON-serializable audit artifact containing all evaluation details.

    Examples
    --------
    >>> from acdso import ACDSOClassifier
    >>> clf = ACDSOClassifier(scorer="f1", cv=5, preference="safe", verbose=1)
    >>> clf.fit(X_train, y_train)
    >>> predictions = clf.predict(X_test)
    >>> print(clf.risk_report_)
    >>> print(clf.audit_trail_)

    >>> # With constraints
    >>> clf = ACDSOClassifier(
    ...     max_latency_ms_per_row=10.0,
    ...     max_peak_mem_mb=500.0,
    ...     time_budget_s=300,
    ... )
    >>> clf.fit(X, y)
    """

    def __init__(
        self,
        scorer: str = "accuracy",
        cv: int = 5,
        preference: str = "balanced",
        time_budget_s: Optional[float] = None,
        max_candidates: int = 12,
        random_state: Optional[int] = 42,
        max_train_time_s: Optional[float] = None,
        max_peak_mem_mb: Optional[float] = None,
        max_latency_ms_per_row: Optional[float] = None,
        enable_feature_selection: bool = True,
        llm_advisor: Optional[Callable[[Dict[str, Any]], Dict[str, Any]]] = None,
        verbose: int = 0,
    ) -> None:
        self.scorer = scorer
        self.cv = cv
        self.preference = preference
        self.time_budget_s = time_budget_s
        self.max_candidates = max_candidates
        self.random_state = random_state
        self.max_train_time_s = max_train_time_s
        self.max_peak_mem_mb = max_peak_mem_mb
        self.max_latency_ms_per_row = max_latency_ms_per_row
        self.enable_feature_selection = enable_feature_selection
        self.llm_advisor = llm_advisor
        self.verbose = verbose

        # Fitted attributes
        self.best_estimator_: Optional[Pipeline] = None
        self.risk_report_: Optional[Dict[str, Any]] = None
        self.pareto_front_: Optional[List[Dict[str, Any]]] = None
        self.audit_trail_: Optional[Dict[str, Any]] = None
        self._engine: Optional[_ACDSOEngineV3] = None

    def fit(self, X: Any, y: Any) -> "ACDSOClassifier":
        """
        Fit the ACDSO classifier.

        This runs the full optimization loop:
        1. Analyze dataset risk
        2. Generate adaptive candidates
        3. Evaluate with multi-objective metrics
        4. Select optimal model via Pareto knee-point

        Parameters
        ----------
        X : array-like of shape (n_samples, n_features)
            Training data.
        y : array-like of shape (n_samples,)
            Target values.

        Returns
        -------
        self : ACDSOClassifier
            Fitted estimator.
        """
        self._engine = _ACDSOEngineV3(
            task="classification",
            scorer=self.scorer,
            cv=self.cv,
            random_state=self.random_state,
            time_budget_s=self.time_budget_s,
            max_candidates=self.max_candidates,
            preference=self.preference,
            max_train_time_s=self.max_train_time_s,
            max_peak_mem_mb=self.max_peak_mem_mb,
            max_latency_ms_per_row=self.max_latency_ms_per_row,
            enable_feature_selection=self.enable_feature_selection,
            llm_advisor=self.llm_advisor,
            verbose=self.verbose,
        ).fit(X, y)

        assert self._engine.best_result_ is not None
        self.best_estimator_ = self._engine.best_result_.pipeline
        self.risk_report_ = asdict(self._engine.risk_report_) if self._engine.risk_report_ else None
        self.pareto_front_ = self._engine.audit_trail_.get("pareto_front")
        self.audit_trail_ = self._engine.audit_trail_
        
        # Persist the adjusted scorer back to public estimator
        if hasattr(self._engine, '_adjusted_scorer'):
            self.scorer = self._engine._adjusted_scorer

        return self

    def predict(self, X: Any) -> Any:
        """
        Predict class labels for samples in X.

        Parameters
        ----------
        X : array-like of shape (n_samples, n_features)
            Samples to predict.

        Returns
        -------
        y_pred : ndarray of shape (n_samples,)
            Predicted class labels.
        """
        if self.best_estimator_ is None:
            raise RuntimeError("ACDSOClassifier is not fitted. Call fit() first.")
        return self.best_estimator_.predict(X)

    def predict_proba(self, X: Any) -> Any:
        """
        Predict class probabilities for samples in X.

        Parameters
        ----------
        X : array-like of shape (n_samples, n_features)
            Samples to predict.

        Returns
        -------
        y_proba : ndarray of shape (n_samples, n_classes)
            Class probabilities.

        Raises
        ------
        AttributeError
            If the selected estimator does not support probability prediction.
        """
        if self.best_estimator_ is None:
            raise RuntimeError("ACDSOClassifier is not fitted. Call fit() first.")
        if hasattr(self.best_estimator_, "predict_proba"):
            return self.best_estimator_.predict_proba(X)
        raise AttributeError("Selected estimator does not support predict_proba().")

    def score(self, X: Any, y: Any) -> float:
        """
        Return the score on the given test data and labels.

        Parameters
        ----------
        X : array-like of shape (n_samples, n_features)
            Test samples.
        y : array-like of shape (n_samples,)
            True labels.

        Returns
        -------
        score : float
            Score according to the scorer parameter.
        """
        if self.best_estimator_ is None:
            raise RuntimeError("ACDSOClassifier is not fitted. Call fit() first.")
        scorer = get_scorer(self.scorer)
        return float(scorer(self.best_estimator_, X, y))


class ACDSORegressor(BaseEstimator, RegressorMixin):
    """
    ACDSO v3.0 Regressor - Risk-Optimized AutoML for Regression.

    A production-oriented AutoML regressor that unifies contextual risk
    diagnostics, adaptive pipeline synthesis, and multi-objective optimization.
    
    v3 Features:
    - Stress testing with robustness objective (6-objective optimization)
    - Successive halving for efficient candidate evaluation
    - Validation-based robustness measurement
    - Learned risk calibration

    Parameters
    ----------
    scorer : str, default="r2"
        Scoring metric for evaluation. Common options:
        "r2", "neg_mean_squared_error", "neg_mean_absolute_error",
        "neg_root_mean_squared_error"

    cv : int, default=5
        Number of cross-validation folds.

    preference : str, default="balanced"
        Risk preference mode:
        - "safe": Higher weight on leakage and stability risks
        - "balanced": Balanced weighting across all risk factors

    time_budget_s : float, optional
        Maximum total time budget in seconds.

    max_candidates : int, default=12
        Maximum number of candidate pipelines to evaluate.
        Increased from 4 to provide better AutoML search coverage.

    random_state : int, optional, default=42
        Random seed for reproducibility.

    max_train_time_s : float, optional
        Maximum allowed training time per candidate (constraint).

    max_peak_mem_mb : float, optional
        Maximum allowed peak memory usage (constraint).

    max_latency_ms_per_row : float, optional
        Maximum allowed inference latency per row (constraint).

    enable_feature_selection : bool, default=True
        Whether to enable automatic feature selection.

    llm_advisor : callable, optional
        Optional LLM advisor function for semantic analysis.

    verbose : int, default=0
        Verbosity level.

    Attributes
    ----------
    best_estimator_ : Pipeline
        The selected best pipeline after fitting.

    risk_report_ : dict
        Comprehensive risk report for the dataset.

    pareto_front_ : list
        List of Pareto-optimal candidates.

    audit_trail_ : dict
        JSON-serializable audit artifact.

    Examples
    --------
    >>> from acdso import ACDSORegressor
    >>> reg = ACDSORegressor(scorer="r2", cv=5, verbose=1)
    >>> reg.fit(X_train, y_train)
    >>> predictions = reg.predict(X_test)
    >>> print(reg.risk_report_)
    """

    def __init__(
        self,
        scorer: str = "r2",
        cv: int = 5,
        preference: str = "balanced",
        time_budget_s: Optional[float] = None,
        max_candidates: int = 12,
        random_state: Optional[int] = 42,
        max_train_time_s: Optional[float] = None,
        max_peak_mem_mb: Optional[float] = None,
        max_latency_ms_per_row: Optional[float] = None,
        enable_feature_selection: bool = True,
        llm_advisor: Optional[Callable[[Dict[str, Any]], Dict[str, Any]]] = None,
        verbose: int = 0,
    ) -> None:
        self.scorer = scorer
        self.cv = cv
        self.preference = preference
        self.time_budget_s = time_budget_s
        self.max_candidates = max_candidates
        self.random_state = random_state
        self.max_train_time_s = max_train_time_s
        self.max_peak_mem_mb = max_peak_mem_mb
        self.max_latency_ms_per_row = max_latency_ms_per_row
        self.enable_feature_selection = enable_feature_selection
        self.llm_advisor = llm_advisor
        self.verbose = verbose

        # Fitted attributes
        self.best_estimator_: Optional[Pipeline] = None
        self.risk_report_: Optional[Dict[str, Any]] = None
        self.pareto_front_: Optional[List[Dict[str, Any]]] = None
        self.audit_trail_: Optional[Dict[str, Any]] = None
        self._engine: Optional[_ACDSOEngineV3] = None

    def fit(self, X: Any, y: Any) -> "ACDSORegressor":
        """
        Fit the ACDSO regressor.

        Parameters
        ----------
        X : array-like of shape (n_samples, n_features)
            Training data.
        y : array-like of shape (n_samples,)
            Target values.

        Returns
        -------
        self : ACDSORegressor
            Fitted estimator.
        """
        self._engine = _ACDSOEngineV3(
            task="regression",
            scorer=self.scorer,
            cv=self.cv,
            random_state=self.random_state,
            time_budget_s=self.time_budget_s,
            max_candidates=self.max_candidates,
            preference=self.preference,
            max_train_time_s=self.max_train_time_s,
            max_peak_mem_mb=self.max_peak_mem_mb,
            max_latency_ms_per_row=self.max_latency_ms_per_row,
            enable_feature_selection=self.enable_feature_selection,
            llm_advisor=self.llm_advisor,
            verbose=self.verbose,
        ).fit(X, y)

        assert self._engine.best_result_ is not None
        self.best_estimator_ = self._engine.best_result_.pipeline
        self.risk_report_ = asdict(self._engine.risk_report_) if self._engine.risk_report_ else None
        self.pareto_front_ = self._engine.audit_trail_.get("pareto_front")
        self.audit_trail_ = self._engine.audit_trail_
        
        # Persist the adjusted scorer back to public estimator
        if hasattr(self._engine, '_adjusted_scorer'):
            self.scorer = self._engine._adjusted_scorer

        return self

    def predict(self, X: Any) -> Any:
        """
        Predict regression targets for samples in X.

        Parameters
        ----------
        X : array-like of shape (n_samples, n_features)
            Samples to predict.

        Returns
        -------
        y_pred : ndarray of shape (n_samples,)
            Predicted target values.
        """
        if self.best_estimator_ is None:
            raise RuntimeError("ACDSORegressor is not fitted. Call fit() first.")
        return self.best_estimator_.predict(X)

    def score(self, X: Any, y: Any) -> float:
        """
        Return the score on the given test data and labels.

        Parameters
        ----------
        X : array-like of shape (n_samples, n_features)
            Test samples.
        y : array-like of shape (n_samples,)
            True target values.

        Returns
        -------
        score : float
            Score according to the scorer parameter.
        """
        if self.best_estimator_ is None:
            raise RuntimeError("ACDSORegressor is not fitted. Call fit() first.")
        scorer = get_scorer(self.scorer)
        return float(scorer(self.best_estimator_, X, y))

