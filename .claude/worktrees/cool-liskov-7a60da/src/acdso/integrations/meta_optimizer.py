"""
ACDSO v2.0 - Meta Optimizer
============================

The ACDSOMetaOptimizer is the unified decision layer that:
1. Collects candidates from multiple AutoML tools
2. Applies ACDSO's risk diagnostics to all candidates
3. Enforces constraints (latency, memory, training time)
4. Constructs a unified Pareto front
5. Selects the best model via knee-point selection
6. Produces a comprehensive audit trail

This enables ACDSO to act as the selection policy across
different AutoML engines, ensuring production-ready models.
"""

from __future__ import annotations

import math
import time
import tracemalloc
from dataclasses import asdict
from typing import Any, Callable, Dict, List, Optional, Union

import numpy as np
import pandas as pd

from sklearn.base import BaseEstimator, ClassifierMixin, RegressorMixin, clone
from sklearn.metrics import get_scorer
from sklearn.model_selection import KFold, StratifiedKFold, cross_val_score
from sklearn.pipeline import Pipeline

from acdso.integrations.base import (
    BaseAutoMLAdapter,
    CandidateSource,
    ExternalModelWrapper,
)
from acdso.pareto import CandidateResult, knee_point_selection, pareto_front
from acdso.risk import RiskReport, analyze_risk, compute_risk_penalty
from acdso.utils import infer_target_type, measure_latency_ms_per_row


class ACDSOMetaOptimizer(BaseEstimator):
    """
    ACDSO Meta Optimizer - Unified AutoML Selection Layer.

    Collects candidates from multiple AutoML tools and applies ACDSO's
    multi-objective optimization framework to select the best model.

    The key architecture:
    - Each AutoML tool generates its best candidates under its own search
    - ACDSO applies risk diagnostics, constraint pruning, Pareto front
    - Knee-point selection chooses the optimal tradeoff
    - Audit trail documents the entire process

    Parameters
    ----------
    adapters : list of BaseAutoMLAdapter
        List of AutoML adapters to use for candidate generation.
    include_acdso_candidates : bool, default=True
        Whether to also include ACDSO's own candidates.
    scorer : str, default="accuracy"
        Scoring metric for evaluation.
    cv : int, default=5
        Cross-validation folds for candidate evaluation.
    preference : str, default="balanced"
        Risk preference ("safe" or "balanced").
    max_train_time_s : float, optional
        Maximum training time constraint.
    max_peak_mem_mb : float, optional
        Maximum peak memory constraint.
    max_latency_ms_per_row : float, optional
        Maximum inference latency constraint.
    time_budget_s : float, optional
        Total time budget for the entire process.
    llm_advisor : callable, optional
        LLM advisor for semantic analysis.
    random_state : int, default=42
        Random seed.
    verbose : int, default=0
        Verbosity level.

    Attributes
    ----------
    best_estimator_ : Any
        The selected best model/pipeline.
    best_source_ : str
        Name of the source tool for the best model.
    risk_report_ : dict
        Risk analysis report for the dataset.
    pareto_front_ : list
        Pareto-optimal candidates from all sources.
    audit_trail_ : dict
        Comprehensive audit trail including:
        - Risk report
        - Candidates from each source
        - Constraint violations
        - Pareto front
        - Selection rationale

    Examples
    --------
    >>> from acdso.integrations import (
    ...     ACDSOMetaOptimizer,
    ...     AutoSklearnAdapter,
    ...     TPOTAdapter,
    ...     FLAMLAdapter,
    ... )
    >>>
    >>> # Create adapters
    >>> adapters = [
    ...     AutoSklearnAdapter(time_budget=120),
    ...     TPOTAdapter(generations=5),
    ...     FLAMLAdapter(time_budget=60),
    ... ]
    >>>
    >>> # Create meta optimizer
    >>> meta = ACDSOMetaOptimizer(
    ...     adapters=adapters,
    ...     scorer="f1",
    ...     max_latency_ms_per_row=10.0,
    ...     preference="safe",
    ...     verbose=1,
    ... )
    >>>
    >>> # Fit and select
    >>> meta.fit(X_train, y_train)
    >>> predictions = meta.predict(X_test)
    >>>
    >>> # Review audit trail
    >>> print(meta.audit_trail_)
    """

    def __init__(
        self,
        adapters: Optional[List[BaseAutoMLAdapter]] = None,
        include_acdso_candidates: bool = True,
        scorer: str = "accuracy",
        cv: int = 5,
        preference: str = "balanced",
        max_train_time_s: Optional[float] = None,
        max_peak_mem_mb: Optional[float] = None,
        max_latency_ms_per_row: Optional[float] = None,
        time_budget_s: Optional[float] = None,
        llm_advisor: Optional[Callable[[Dict[str, Any]], Dict[str, Any]]] = None,
        random_state: int = 42,
        verbose: int = 0,
    ):
        self.adapters = adapters or []
        self.include_acdso_candidates = include_acdso_candidates
        self.scorer = scorer
        self.cv = cv
        self.preference = preference
        self.max_train_time_s = max_train_time_s
        self.max_peak_mem_mb = max_peak_mem_mb
        self.max_latency_ms_per_row = max_latency_ms_per_row
        self.time_budget_s = time_budget_s
        self.llm_advisor = llm_advisor
        self.random_state = random_state
        self.verbose = verbose

        # Fitted attributes
        self.best_estimator_: Optional[Any] = None
        self.best_source_: Optional[str] = None
        self.risk_report_: Optional[Dict[str, Any]] = None
        self.pareto_front_: Optional[List[Dict[str, Any]]] = None
        self.audit_trail_: Optional[Dict[str, Any]] = None
        self._task: Optional[str] = None

    def _log(self, msg: str) -> None:
        """Log message if verbose mode is enabled."""
        if self.verbose:
            print(f"[ACDSO Meta] {msg}")

    def _infer_task(self, y: Any) -> str:
        """Infer task type from target."""
        target_type = infer_target_type(y)
        if target_type == "regression":
            return "regression"
        return "classification"

    def _collect_candidates(
        self,
        X: Any,
        y: Any,
        task: str,
    ) -> List[CandidateSource]:
        """Collect candidates from all adapters."""
        all_candidates = []
        adapter_stats = []

        for adapter in self.adapters:
            if not adapter.is_available():
                self._log(f"Skipping {adapter.name}: not available")
                continue

            self._log(f"Generating candidates from {adapter.name}...")

            try:
                candidates = adapter.generate_candidates(X, y, task)
                all_candidates.extend(candidates)

                stats = adapter.get_generation_stats()
                stats["candidates_generated"] = len(candidates)
                adapter_stats.append(stats)

                self._log(f"  -> {len(candidates)} candidates generated")

            except Exception as e:
                self._log(f"  -> Failed: {type(e).__name__}: {e}")
                adapter_stats.append({
                    "adapter": adapter.name,
                    "error": str(e),
                    "candidates_generated": 0,
                })

        # Optionally include ACDSO's own candidates
        if self.include_acdso_candidates:
            self._log("Generating ACDSO native candidates...")
            try:
                acdso_candidates = self._generate_acdso_candidates(X, y, task)
                all_candidates.extend(acdso_candidates)
                adapter_stats.append({
                    "adapter": "ACDSO",
                    "candidates_generated": len(acdso_candidates),
                })
                self._log(f"  -> {len(acdso_candidates)} candidates generated")
            except Exception as e:
                self._log(f"  -> Failed: {type(e).__name__}: {e}")

        self._adapter_stats = adapter_stats
        return all_candidates

    def _generate_acdso_candidates(
        self,
        X: Any,
        y: Any,
        task: str,
    ) -> List[CandidateSource]:
        """Generate ACDSO's native candidates."""
        from acdso.core import _ACDSOEngineV3

        # Use a mini ACDSO run to generate candidates
        engine = _ACDSOEngineV3(
            task=task,
            scorer=self.scorer,
            cv=self.cv,
            random_state=self.random_state,
            time_budget_s=min(60, self.time_budget_s or 60),
            max_candidates=4,
            preference=self.preference,
            max_train_time_s=self.max_train_time_s,
            max_peak_mem_mb=self.max_peak_mem_mb,
            max_latency_ms_per_row=self.max_latency_ms_per_row,
            enable_feature_selection=True,
            llm_advisor=None,  # Skip LLM for internal generation
            verbose=0,
        )

        # Get the candidate pipelines without full evaluation
        risk_report = analyze_risk(
            X, y,
            task=task,
            scorer=self.scorer,
            cv=self.cv,
            random_state=self.random_state,
        )

        pipelines = engine._generate_candidates(X, y, risk_report)

        candidates = []
        for name, pipeline in pipelines:
            # Fit the pipeline
            try:
                pipeline.fit(X, y)
                candidates.append(CandidateSource(
                    name=f"acdso_{name}",
                    source="ACDSO",
                    estimator=pipeline,
                    metadata={"native": True},
                ))
            except Exception:
                pass

        return candidates

    def _evaluate_candidate(
        self,
        X: Any,
        y: Any,
        candidate: CandidateSource,
        risk_report: RiskReport,
        task: str,
    ) -> Optional[CandidateResult]:
        """Evaluate a single candidate with ACDSO's multi-objective metrics."""
        try:
            estimator = candidate.estimator

            # Check if already fitted (external models)
            is_prefitted = isinstance(estimator, ExternalModelWrapper)

            start = time.time()
            tracemalloc.start()

            if task == "classification":
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

            if is_prefitted:
                # For pre-fitted models, just score on the data
                # (This is an approximation; ideally we'd have held-out data)
                try:
                    score = float(scorer(estimator, X, y))
                    scores = np.array([score])
                except Exception:
                    # Try manual prediction and scoring
                    from sklearn.metrics import accuracy_score, r2_score
                    preds = estimator.predict(X)
                    if task == "classification":
                        score = accuracy_score(y, preds)
                    else:
                        score = r2_score(y, preds)
                    scores = np.array([score])
            else:
                # Clone and cross-validate
                cloned = clone(estimator)
                scores = cross_val_score(
                    cloned, X, y,
                    cv=splitter,
                    scoring=scorer,
                    n_jobs=None,
                )

            current, peak = tracemalloc.get_traced_memory()
            tracemalloc.stop()

            train_time_s = float(time.time() - start)
            peak_mem_mb = float(peak / (1024 * 1024))
            metric = float(np.mean(scores))
            cv_std = float(np.std(scores)) if len(scores) > 1 else 0.0

            # Compute risk penalty
            risk_pen = compute_risk_penalty(
                risk_report,
                cv_score_std=cv_std,
                preference=self.preference,
            )

            # Measure latency
            if is_prefitted:
                latency = measure_latency_ms_per_row(estimator, X)
            else:
                # Need a fitted estimator
                fitted = clone(estimator)
                fitted.fit(X, y)
                latency = measure_latency_ms_per_row(fitted, X)
                estimator = fitted  # Use fitted version

            # Build objective vector
            objectives = (
                metric,
                math.log1p(train_time_s),
                math.log1p(peak_mem_mb),
                math.log1p(latency),
                risk_pen,
            )

            return CandidateResult(
                name=candidate.name,
                pipeline=estimator,
                metric=metric,
                train_time_s=train_time_s,
                peak_mem_mb=peak_mem_mb,
                latency_ms_per_row=latency,
                risk_penalty=risk_pen,
                objectives=objectives,
                meta={
                    "source": candidate.source,
                    "cv_scores": scores.tolist(),
                    "cv_score_std": cv_std,
                    **candidate.metadata,
                },
            )

        except Exception as e:
            self._log(f"Failed to evaluate {candidate.name}: {type(e).__name__}: {e}")
            return None

    def _passes_constraints(self, result: CandidateResult) -> bool:
        """Check if result passes all constraints."""
        if self.max_train_time_s is not None and result.train_time_s > self.max_train_time_s:
            return False
        if self.max_peak_mem_mb is not None and result.peak_mem_mb > self.max_peak_mem_mb:
            return False
        if self.max_latency_ms_per_row is not None and result.latency_ms_per_row > self.max_latency_ms_per_row:
            return False
        return True

    def fit(self, X: Any, y: Any) -> "ACDSOMetaOptimizer":
        """
        Fit the meta optimizer.

        1. Infers task type
        2. Analyzes dataset risk
        3. Collects candidates from all adapters
        4. Evaluates candidates with multi-objective metrics
        5. Filters by constraints
        6. Constructs Pareto front
        7. Selects best via knee-point
        8. Builds audit trail

        Parameters
        ----------
        X : array-like
            Training features.
        y : array-like
            Target values.

        Returns
        -------
        self : ACDSOMetaOptimizer
            Fitted meta optimizer.
        """
        t0 = time.time()

        # Step 1: Infer task
        self._task = self._infer_task(y)
        self._log(f"Task: {self._task}")

        # Step 2: Risk analysis
        self._log("Analyzing dataset risk...")
        risk_report = analyze_risk(
            X, y,
            task=self._task,
            scorer=self.scorer,
            cv=self.cv,
            random_state=self.random_state,
            llm_advisor=self.llm_advisor,
        )
        self._log(f"Risk analysis complete. VIF={risk_report.vif_max:.1f}, "
                  f"Imbalance={risk_report.imbalance_severity:.2f}")

        # Step 3: Collect candidates
        self._log("Collecting candidates from adapters...")
        candidates = self._collect_candidates(X, y, self._task)
        self._log(f"Total candidates collected: {len(candidates)}")

        if not candidates:
            raise RuntimeError("No candidates were generated by any adapter.")

        # Step 4: Evaluate candidates
        self._log("Evaluating candidates with ACDSO metrics...")
        results: List[CandidateResult] = []
        failures: List[Dict[str, Any]] = []

        for candidate in candidates:
            # Check time budget
            if self.time_budget_s is not None and (time.time() - t0) >= self.time_budget_s:
                self._log("Time budget reached; stopping evaluation.")
                break

            self._log(f"  Evaluating: {candidate.name} ({candidate.source})")

            result = self._evaluate_candidate(X, y, candidate, risk_report, self._task)

            if result is None:
                failures.append({
                    "name": candidate.name,
                    "source": candidate.source,
                    "reason": "evaluation_error",
                })
                continue

            # Step 5: Filter by constraints
            if not self._passes_constraints(result):
                self._log(f"    -> Pruned by constraints")
                failures.append({
                    "name": candidate.name,
                    "source": candidate.source,
                    "reason": "constraints",
                    "metrics": {
                        "metric": result.metric,
                        "train_time_s": result.train_time_s,
                        "peak_mem_mb": result.peak_mem_mb,
                        "latency_ms_per_row": result.latency_ms_per_row,
                    },
                })
                continue

            results.append(result)
            self._log(f"    -> metric={result.metric:.4f}, latency={result.latency_ms_per_row:.2f}ms")

        if not results:
            raise RuntimeError("No candidates passed evaluation/constraints.")

        # Update risk report with best stability
        best_std = float(np.min([r.meta.get("cv_score_std", 0.0) for r in results]))
        risk_report.cv_score_std = best_std

        # Step 6: Construct Pareto front
        self._log("Constructing Pareto front...")
        front = pareto_front(results)
        self._log(f"Pareto front size: {len(front)}")

        # Step 7: Select best via knee-point
        best = knee_point_selection(front)
        self._log(f"Selected: {best.name} from {best.meta.get('source', 'unknown')}")

        # Store results
        self.best_estimator_ = best.pipeline
        self.best_source_ = best.meta.get("source", "unknown")
        self.risk_report_ = asdict(risk_report)

        self.pareto_front_ = [
            {
                "name": r.name,
                "source": r.meta.get("source"),
                "metric": r.metric,
                "train_time_s": r.train_time_s,
                "peak_mem_mb": r.peak_mem_mb,
                "latency_ms_per_row": r.latency_ms_per_row,
                "risk_penalty": r.risk_penalty,
            }
            for r in front
        ]

        # Step 8: Build audit trail
        self.audit_trail_ = {
            "task": self._task,
            "total_time_s": time.time() - t0,
            "risk_report": asdict(risk_report),
            "constraints": {
                "max_train_time_s": self.max_train_time_s,
                "max_peak_mem_mb": self.max_peak_mem_mb,
                "max_latency_ms_per_row": self.max_latency_ms_per_row,
            },
            "adapter_stats": self._adapter_stats,
            "candidates_evaluated": [
                {
                    "name": r.name,
                    "source": r.meta.get("source"),
                    "metric": r.metric,
                    "train_time_s": r.train_time_s,
                    "peak_mem_mb": r.peak_mem_mb,
                    "latency_ms_per_row": r.latency_ms_per_row,
                    "risk_penalty": r.risk_penalty,
                    "cv_score_std": r.meta.get("cv_score_std"),
                }
                for r in results
            ],
            "pareto_front": self.pareto_front_,
            "pruned_or_failed": failures,
            "selected": {
                "name": best.name,
                "source": self.best_source_,
                "metric": best.metric,
                "train_time_s": best.train_time_s,
                "peak_mem_mb": best.peak_mem_mb,
                "latency_ms_per_row": best.latency_ms_per_row,
                "risk_penalty": best.risk_penalty,
            },
        }

        self._log(f"Optimization complete in {time.time() - t0:.1f}s")
        return self

    def predict(self, X: Any) -> Any:
        """
        Make predictions using the selected model.

        Parameters
        ----------
        X : array-like
            Samples to predict.

        Returns
        -------
        predictions : ndarray
            Predicted values.
        """
        if self.best_estimator_ is None:
            raise RuntimeError("ACDSOMetaOptimizer is not fitted. Call fit() first.")
        return self.best_estimator_.predict(X)

    def predict_proba(self, X: Any) -> Any:
        """
        Predict class probabilities (classification only).

        Parameters
        ----------
        X : array-like
            Samples to predict.

        Returns
        -------
        probabilities : ndarray
            Class probabilities.
        """
        if self.best_estimator_ is None:
            raise RuntimeError("ACDSOMetaOptimizer is not fitted. Call fit() first.")
        if self._task != "classification":
            raise AttributeError("predict_proba is only available for classification.")
        if hasattr(self.best_estimator_, "predict_proba"):
            return self.best_estimator_.predict_proba(X)
        raise AttributeError("Selected estimator does not support predict_proba().")

    def score(self, X: Any, y: Any) -> float:
        """
        Return the score on test data.

        Parameters
        ----------
        X : array-like
            Test samples.
        y : array-like
            True values.

        Returns
        -------
        score : float
            Score according to the scorer parameter.
        """
        if self.best_estimator_ is None:
            raise RuntimeError("ACDSOMetaOptimizer is not fitted. Call fit() first.")
        scorer = get_scorer(self.scorer)
        return float(scorer(self.best_estimator_, X, y))

    def get_source_summary(self) -> Dict[str, Any]:
        """
        Get a summary of candidates from each source.

        Returns
        -------
        summary : dict
            Summary statistics per source.
        """
        if self.audit_trail_ is None:
            return {}

        candidates = self.audit_trail_.get("candidates_evaluated", [])
        pareto = self.audit_trail_.get("pareto_front", [])

        # Count by source
        source_counts = {}
        for c in candidates:
            source = c.get("source", "unknown")
            if source not in source_counts:
                source_counts[source] = {"total": 0, "in_pareto": 0, "selected": False}
            source_counts[source]["total"] += 1

        for c in pareto:
            source = c.get("source", "unknown")
            if source in source_counts:
                source_counts[source]["in_pareto"] += 1

        selected_source = self.audit_trail_.get("selected", {}).get("source")
        if selected_source in source_counts:
            source_counts[selected_source]["selected"] = True

        return source_counts


class ACDSOMetaClassifier(ACDSOMetaOptimizer, ClassifierMixin):
    """
    ACDSO Meta Optimizer for Classification.

    Convenience class that explicitly implements ClassifierMixin.
    """

    def __init__(self, **kwargs):
        kwargs.setdefault("scorer", "accuracy")
        super().__init__(**kwargs)


class ACDSOMetaRegressor(ACDSOMetaOptimizer, RegressorMixin):
    """
    ACDSO Meta Optimizer for Regression.

    Convenience class that explicitly implements RegressorMixin.
    """

    def __init__(self, **kwargs):
        kwargs.setdefault("scorer", "r2")
        super().__init__(**kwargs)

