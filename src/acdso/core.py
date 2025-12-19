"""
ACDSO v2.0 - Core Engine Module
================================

Production-grade, scikit-learn–compatible AutoML estimators.

This module contains:
- _ACDSOEngineV2: Internal optimization engine
- ACDSOClassifier: scikit-learn compatible classifier
- ACDSORegressor: scikit-learn compatible regressor

Key features:
- Continuous, severity-aware risk penalties
- Multi-objective evaluation including latency
- Constraint-based pruning + Pareto knee-point selection
- JSON-serializable audit trail artifacts
"""

from __future__ import annotations

import math
import time
import tracemalloc
from dataclasses import asdict
from typing import Any, Callable, Dict, List, Optional, Tuple

import numpy as np
import pandas as pd

from sklearn.base import BaseEstimator, ClassifierMixin, RegressorMixin, clone
from sklearn.ensemble import RandomForestClassifier, RandomForestRegressor
from sklearn.feature_selection import SelectKBest, f_classif, f_regression
from sklearn.linear_model import LogisticRegression, Ridge
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


class _ACDSOEngineV2:
    """
    Internal ACDSO optimization engine.

    This class handles the core optimization loop including:
    - Risk analysis
    - Candidate generation
    - Multi-objective evaluation
    - Pareto front construction
    - Knee-point selection

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
        self.scorer = scorer
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
            print(f"[ACDSO v2] {msg}")

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
        selector = None
        if self.enable_feature_selection and rr.vif_max >= 10.0:
            score_func = f_classif if self.task == "classification" else f_regression
            selector = SelectKBest(score_func=score_func, k="all")

        small = rr.n_rows < 1000
        large = rr.n_rows >= 50000

        models: List[Tuple[str, Any]] = []

        if self.task == "classification":
            # Logistic Regression
            lr = LogisticRegression(max_iter=2000, random_state=self.random_state)

            # Random Forest
            rf = RandomForestClassifier(
                n_estimators=80 if small else 200,
                random_state=self.random_state,
                n_jobs=-1,
            )

            # Handle class imbalance
            if rr.class_imbalance_max_ratio is not None and rr.class_imbalance_max_ratio >= 0.85:
                try:
                    lr.set_params(class_weight="balanced")
                except Exception:
                    pass
                try:
                    rf.set_params(class_weight="balanced")
                except Exception:
                    pass

            models.append(("LogisticRegression", lr))
            models.append(("RandomForest", rf))

            # Add boosting for large datasets
            if large and _HAS_LGBM:
                models.append(("LightGBM", LGBMClassifier(
                    random_state=self.random_state,
                    verbose=-1,
                )))
            if large and _HAS_XGB:
                models.append(("XGBoost", XGBClassifier(
                    random_state=self.random_state,
                    n_jobs=-1,
                    verbosity=0,
                )))

        else:  # regression
            # Ridge Regression
            ridge = Ridge(random_state=self.random_state)

            # Random Forest Regressor
            rf = RandomForestRegressor(
                n_estimators=100 if small else 250,
                random_state=self.random_state,
                n_jobs=-1,
            )

            models.append(("Ridge", ridge))
            models.append(("RandomForestRegressor", rf))

            # Add boosting for large datasets
            if large and _HAS_LGBM:
                models.append(("LightGBMRegressor", LGBMRegressor(
                    random_state=self.random_state,
                    verbose=-1,
                )))
            if large and _HAS_XGB:
                models.append(("XGBoostRegressor", XGBRegressor(
                    random_state=self.random_state,
                    n_jobs=-1,
                    verbosity=0,
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

    def _evaluate_candidate(
        self,
        X: Any,
        y: Any,
        name: str,
        pipe: Pipeline,
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
        tracemalloc.start()

        # Cross-validation
        if self.task == "classification":
            splitter = StratifiedKFold(
                n_splits=self.cv,
                shuffle=True,
                random_state=self.random_state,
            )
        else:
            splitter = KFold(
                n_splits=self.cv,
                shuffle=True,
                random_state=self.random_state,
            )

        scorer = get_scorer(self.scorer)
        scores = cross_val_score(pipe, X, y, cv=splitter, scoring=scorer, n_jobs=None)

        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()

        train_time_s = float(time.time() - start)
        peak_mem_mb = float(peak / (1024 * 1024))
        metric = float(np.mean(scores))
        cv_std = float(np.std(scores))

        # Compute risk penalty
        assert self.risk_report_ is not None
        risk_pen = compute_risk_penalty(
            self.risk_report_,
            cv_score_std=cv_std,
            preference=self.preference,
        )

        # Fit the pipeline for latency measurement
        fitted = clone(pipe)
        fitted.fit(X, y)
        latency = measure_latency_ms_per_row(fitted, X)

        # Build objective vector (log-scaled for time/memory/latency)
        objectives = (
            metric,
            math.log1p(train_time_s),
            math.log1p(peak_mem_mb),
            math.log1p(latency),
            risk_pen,
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
            meta={"cv_scores": scores.tolist(), "cv_score_std": cv_std},
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

    def fit(self, X: Any, y: Any) -> "_ACDSOEngineV2":
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

        # Step 3: Evaluate candidates
        t0 = time.time()
        results: List[CandidateResult] = []
        failures: List[Dict[str, Any]] = []

        for name, pipe in candidates:
            # Check time budget
            if self.time_budget_s is not None and (time.time() - t0) >= self.time_budget_s:
                self._log("Time budget reached; stopping evaluation.")
                break

            self._log(f"Evaluating: {name}")

            try:
                res = self._evaluate_candidate(X, y, name, pipe)

                # Step 4: Filter by constraints
                if not self._passes_constraints(res):
                    self._log(f"Pruned by constraints: {name}")
                    failures.append({
                        "name": name,
                        "reason": "constraints",
                        "metrics": {
                            "train_time_s": res.train_time_s,
                            "peak_mem_mb": res.peak_mem_mb,
                            "latency_ms_per_row": res.latency_ms_per_row,
                            "risk_penalty": res.risk_penalty,
                            "metric": res.metric,
                        },
                    })
                    continue

                results.append(res)
                self._log(
                    f"{name}: metric={res.metric:.4f}, time={res.train_time_s:.2f}s, "
                    f"mem={res.peak_mem_mb:.1f}MB, lat={res.latency_ms_per_row:.2f}ms/row, "
                    f"risk={res.risk_penalty:.2f}"
                )

                # Early stopping if max train time exceeded
                if self.max_train_time_s is not None and res.train_time_s > self.max_train_time_s:
                    self._log("Max train time exceeded; stopping further evaluation.")
                    break

            except Exception as e:
                failures.append({
                    "name": name,
                    "reason": f"error:{type(e).__name__}",
                    "detail": str(e),
                })
                self._log(f"Failed: {name} ({type(e).__name__})")

        if not results:
            raise RuntimeError("No candidates passed evaluation/constraints.")

        self.all_results_ = results

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
            },
        }

        return self


class ACDSOClassifier(BaseEstimator, ClassifierMixin):
    """
    ACDSO v2.0 Classifier - Risk-Optimized AutoML for Classification.

    A production-oriented AutoML classifier that unifies contextual risk
    diagnostics, adaptive pipeline synthesis, and multi-objective optimization.

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

    max_candidates : int, default=4
        Maximum number of candidate pipelines to evaluate.

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
        max_candidates: int = 4,
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
        self._engine: Optional[_ACDSOEngineV2] = None

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
        self._engine = _ACDSOEngineV2(
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
    ACDSO v2.0 Regressor - Risk-Optimized AutoML for Regression.

    A production-oriented AutoML regressor that unifies contextual risk
    diagnostics, adaptive pipeline synthesis, and multi-objective optimization.

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

    max_candidates : int, default=4
        Maximum number of candidate pipelines to evaluate.

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
        max_candidates: int = 4,
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
        self._engine: Optional[_ACDSOEngineV2] = None

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
        self._engine = _ACDSOEngineV2(
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

