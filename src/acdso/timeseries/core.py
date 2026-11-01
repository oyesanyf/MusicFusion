"""
ACDSO Auto Time Series Engine
==============================

Main AutoTS engine that performs:
- Automatic lag/rolling feature generation
- Time-aware cross-validation
- Multi-objective pipeline optimization
- Risk-aware model selection
- Audit trail generation

Usage:
    from acdso.timeseries import AutoTS
    
    ts = AutoTS(
        datetime_col='date',
        target_col='sales',
        horizon=7,
    )
    
    result = ts.fit(df)
    print(result.best_pipeline)
    forecast = result.predict(df_future)
"""

from __future__ import annotations

import time
import warnings
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any, Union
import numpy as np
import pandas as pd
from sklearn.base import BaseEstimator, clone

from .splits import (
    TimeSeriesSplitter,
    ExpandingWindowSplit,
    RollingWindowSplit,
    WalkForwardSplit,
    create_time_series_cv,
)
from .transformers import (
    LagFeatureTransformer,
    RollingWindowTransformer,
    DateTimeFeatureTransformer,
    TimeSeriesFeatureUnion,
    create_ts_feature_pipeline,
)
from .metrics import (
    mae,
    rmse,
    mape,
    mase,
    evaluate_forecast,
    TimeSeriesScorer,
    get_ts_scorer,
)
from .risk import (
    TimeSeriesRiskAnalyzer,
    TimeSeriesRiskReport,
)
from .candidates import (
    TimeSeriesCandidateGenerator,
    ForecastingCandidate,
    NaiveForecaster,
    SeasonalNaiveForecaster,
    get_default_candidates,
)


@dataclass
class ForecastingPipeline:
    """
    A fitted forecasting pipeline with metadata.
    
    Attributes
    ----------
    name : str
        Pipeline name.
    model : BaseEstimator
        Fitted model/pipeline.
    cv_score : float
        Cross-validation score (primary metric).
    cv_scores : Dict[str, float]
        All CV metric scores.
    fit_time : float
        Training time in seconds.
    predict_time : float
        Prediction time in seconds (per sample).
    memory_mb : float
        Approximate memory usage.
    family : str
        Model family.
    hyperparams : Dict
        Hyperparameters used.
    is_baseline : bool
        Whether this is a baseline model.
    """
    
    name: str
    model: Any
    cv_score: float
    cv_scores: Dict[str, float] = field(default_factory=dict)
    fit_time: float = 0.0
    predict_time: float = 0.0
    memory_mb: float = 0.0
    family: str = 'unknown'
    hyperparams: Dict[str, Any] = field(default_factory=dict)
    is_baseline: bool = False
    
    def predict(self, X: pd.DataFrame) -> np.ndarray:
        """Make predictions."""
        return self.model.predict(X)


@dataclass
class AutoTSResult:
    """
    Result of AutoTS optimization.
    
    Attributes
    ----------
    best_pipeline : ForecastingPipeline
        Best performing pipeline.
    all_pipelines : List[ForecastingPipeline]
        All evaluated pipelines, sorted by score.
    pareto_front : List[ForecastingPipeline]
        Pareto-optimal pipelines (score vs time/memory).
    risk_report : TimeSeriesRiskReport
        Time series risk analysis.
    baseline_score : float
        Score of the best baseline model.
    improvement_over_baseline : float
        Percentage improvement over baseline.
    feature_importance : Dict[str, float]
        Feature importance from best model (if available).
    audit_trail : Dict
        Full audit trail of decisions.
    timestamp : str
        Timestamp of the run.
    """
    
    best_pipeline: ForecastingPipeline
    all_pipelines: List[ForecastingPipeline]
    pareto_front: List[ForecastingPipeline] = field(default_factory=list)
    risk_report: Optional[TimeSeriesRiskReport] = None
    baseline_score: float = 0.0
    improvement_over_baseline: float = 0.0
    feature_importance: Dict[str, float] = field(default_factory=dict)
    audit_trail: Dict[str, Any] = field(default_factory=dict)
    timestamp: str = ''
    
    def summary(self) -> str:
        """Generate a human-readable summary."""
        lines = [
            "=" * 60,
            "ACDSO AutoTS Results",
            "=" * 60,
            f"Best Model: {self.best_pipeline.name}",
            f"  CV Score ({self.audit_trail.get('primary_metric', 'rmse')}): {self.best_pipeline.cv_score:.4f}",
            f"  Fit Time: {self.best_pipeline.fit_time:.2f}s",
            f"  Family: {self.best_pipeline.family}",
            "",
            f"Baseline Score: {self.baseline_score:.4f}",
            f"Improvement: {self.improvement_over_baseline:.1f}%",
            "",
            f"Models Evaluated: {len(self.all_pipelines)}",
            f"Pareto-Optimal: {len(self.pareto_front)}",
            "",
        ]
        
        if self.risk_report:
            lines.extend([
                f"Risk Severity: {self.risk_report.severity.upper()}",
                f"Seasonality: {self.risk_report.seasonality_period or 'None detected'}",
                f"Stationary: {'Yes' if self.risk_report.is_stationary else 'No'}",
            ])
        
        lines.append("=" * 60)
        
        return "\n".join(lines)
    
    def predict(self, X: pd.DataFrame) -> np.ndarray:
        """Make predictions using best pipeline."""
        return self.best_pipeline.predict(X)
    
    def get_leaderboard(self, top_n: int = 10) -> pd.DataFrame:
        """Get leaderboard of models."""
        data = []
        for p in self.all_pipelines[:top_n]:
            data.append({
                'Model': p.name,
                'Score': p.cv_score,
                'Fit Time (s)': p.fit_time,
                'Family': p.family,
                'Is Baseline': p.is_baseline,
            })
        
        return pd.DataFrame(data)


class AutoTS:
    """
    ACDSO Auto Time Series Engine.
    
    Automatically finds the best forecasting pipeline by:
    1. Analyzing time series characteristics (risk analysis)
    2. Generating candidate pipelines with appropriate features
    3. Evaluating using time-aware cross-validation
    4. Selecting best model using multi-objective optimization
    
    Parameters
    ----------
    datetime_col : str
        Name of the datetime column.
    target_col : str
        Name of the target column to forecast.
    horizon : int, default=1
        Forecast horizon (steps ahead).
    seasonality : int, optional
        Seasonal period. If None, auto-detected.
    cv_type : str, default='expanding'
        Cross-validation type: 'expanding', 'rolling', 'walk_forward'.
    n_splits : int, default=5
        Number of CV splits.
    primary_metric : str, default='rmse'
        Primary metric for optimization: 'mae', 'rmse', 'mape', 'smape', 'mase'.
    time_budget : int, optional
        Maximum time in seconds for optimization.
    include_baselines : bool, default=True
        Whether to include baseline models.
    include_statistical : bool, default=False
        Whether to include statistical models (requires statsmodels).
    include_timesfm : bool, default=False
        Whether to include Google's TimesFM pretrained foundation model
        (requires ``pip install timesfm[torch]``). Off by default: heavy
        optional dependency, and only added as a candidate when the
        training series is long enough to matter (see
        ``TimesFMForecaster.min_context_length`` in candidates.py) -- on
        this project's typical short aggregated series it would be
        skipped even with this flag on.
    verbose : bool, default=True
        Whether to print progress.
    random_state : int, default=42
        Random seed.

    Example
    -------
    >>> ts = AutoTS(
    ...     datetime_col='date',
    ...     target_col='sales',
    ...     horizon=7,
    ...     seasonality=7,
    ... )
    >>> result = ts.fit(df)
    >>> print(result.summary())
    >>> forecast = result.predict(df_future)
    """

    def __init__(
        self,
        datetime_col: str,
        target_col: str,
        horizon: int = 1,
        seasonality: Optional[int] = None,
        cv_type: str = 'expanding',
        n_splits: int = 5,
        primary_metric: str = 'rmse',
        time_budget: Optional[int] = None,
        include_baselines: bool = True,
        include_statistical: bool = False,
        include_timesfm: bool = False,
        verbose: bool = True,
        random_state: int = 42,
    ):
        self.datetime_col = datetime_col
        self.target_col = target_col
        self.horizon = horizon
        self.seasonality = seasonality
        self.cv_type = cv_type
        self.n_splits = n_splits
        self.primary_metric = primary_metric
        self.time_budget = time_budget
        self.include_baselines = include_baselines
        self.include_statistical = include_statistical
        self.include_timesfm = include_timesfm
        self.verbose = verbose
        self.random_state = random_state
        
        self._result: Optional[AutoTSResult] = None
        self._risk_analyzer: Optional[TimeSeriesRiskAnalyzer] = None
    
    def fit(
        self,
        df: pd.DataFrame,
        feature_cols: Optional[List[str]] = None,
    ) -> AutoTSResult:
        """
        Fit the AutoTS engine.
        
        Parameters
        ----------
        df : pd.DataFrame
            Training data with datetime and target columns.
        feature_cols : list, optional
            Additional feature columns (exogenous variables).
        
        Returns
        -------
        AutoTSResult
            Optimization results.
        """
        start_time = time.time()
        audit_trail = {
            'start_time': pd.Timestamp.now().isoformat(),
            'input_shape': df.shape,
            'datetime_col': self.datetime_col,
            'target_col': self.target_col,
            'horizon': self.horizon,
            'primary_metric': self.primary_metric,
        }
        
        if self.verbose:
            print("=" * 60)
            print("ACDSO AutoTS - Time Series Optimization")
            print("=" * 60)
        
        # Step 1: Validate and prepare data
        df = self._prepare_data(df)
        
        # Step 2: Risk analysis
        if self.verbose:
            print("\n[1/4] Analyzing time series characteristics...")
        
        self._risk_analyzer = TimeSeriesRiskAnalyzer(
            datetime_col=self.datetime_col,
            target_col=self.target_col,
            horizon=self.horizon,
            seasonality=self.seasonality,
        )
        risk_report = self._risk_analyzer.analyze(df, feature_cols)
        audit_trail['risk_report'] = risk_report.to_dict()
        
        # Auto-detect seasonality if not provided
        if self.seasonality is None and risk_report.seasonality_period:
            self.seasonality = risk_report.seasonality_period
            if self.verbose:
                print(f"   Detected seasonality: {self.seasonality}")
        
        if self.verbose:
            print(f"   Risk severity: {risk_report.severity}")
            print(f"   Stationary: {'Yes' if risk_report.is_stationary else 'No'}")
            print(f"   Frequency: {risk_report.frequency}")
        
        # Step 3: Generate candidates
        if self.verbose:
            print("\n[2/4] Generating candidate pipelines...")
        
        candidates = self._generate_candidates(df, feature_cols)
        audit_trail['n_candidates'] = len(candidates)
        
        if self.verbose:
            print(f"   Generated {len(candidates)} candidates")
        
        # Step 4: Evaluate candidates
        if self.verbose:
            print("\n[3/4] Evaluating candidates with time-aware CV...")
        
        fitted_pipelines = self._evaluate_candidates(df, candidates)
        audit_trail['n_evaluated'] = len(fitted_pipelines)
        
        # Step 5: Select best
        if self.verbose:
            print("\n[4/4] Selecting best pipeline...")
        
        fitted_pipelines.sort(key=lambda p: p.cv_score)
        best_pipeline = fitted_pipelines[0]
        
        # Compute baseline comparison
        baselines = [p for p in fitted_pipelines if p.is_baseline]
        if baselines:
            baseline_score = min(p.cv_score for p in baselines)
            if baseline_score > 0:
                improvement = (baseline_score - best_pipeline.cv_score) / baseline_score * 100
            else:
                improvement = 0.0
        else:
            baseline_score = 0.0
            improvement = 0.0
        
        # Compute Pareto front (score vs time)
        pareto_front = self._compute_pareto_front(fitted_pipelines)
        
        # Extract feature importance
        feature_importance = self._extract_feature_importance(best_pipeline)
        
        # Finalize audit trail
        audit_trail['end_time'] = pd.Timestamp.now().isoformat()
        audit_trail['total_time'] = time.time() - start_time
        audit_trail['best_model'] = best_pipeline.name
        audit_trail['best_score'] = best_pipeline.cv_score
        
        # Create result
        self._result = AutoTSResult(
            best_pipeline=best_pipeline,
            all_pipelines=fitted_pipelines,
            pareto_front=pareto_front,
            risk_report=risk_report,
            baseline_score=baseline_score,
            improvement_over_baseline=improvement,
            feature_importance=feature_importance,
            audit_trail=audit_trail,
            timestamp=pd.Timestamp.now().isoformat(),
        )
        
        if self.verbose:
            print("\n" + self._result.summary())
        
        return self._result
    
    def _prepare_data(self, df: pd.DataFrame) -> pd.DataFrame:
        """Validate and prepare the data."""
        df = df.copy()
        
        # Validate columns
        if self.datetime_col not in df.columns:
            raise ValueError(f"Datetime column '{self.datetime_col}' not found")
        if self.target_col not in df.columns:
            raise ValueError(f"Target column '{self.target_col}' not found")
        
        # Convert to datetime
        df[self.datetime_col] = pd.to_datetime(df[self.datetime_col])
        
        # Sort by time
        df = df.sort_values(self.datetime_col).reset_index(drop=True)
        
        # Drop non-numeric columns except datetime and target to avoid dtype issues
        # Keep only: datetime_col, target_col, and numeric columns
        cols_to_keep = [self.datetime_col, self.target_col]
        for col in df.columns:
            if col not in cols_to_keep:
                if np.issubdtype(df[col].dtype, np.number):
                    cols_to_keep.append(col)
        df = df[cols_to_keep]
        
        # Check for sufficient data
        min_samples = max(30, self.horizon * 3)
        if len(df) < min_samples:
            raise ValueError(
                f"Insufficient data: {len(df)} samples. Need at least {min_samples}."
            )
        
        return df
    
    def _generate_candidates(
        self,
        df: pd.DataFrame,
        feature_cols: Optional[List[str]] = None,
    ) -> List[ForecastingCandidate]:
        """Generate candidate pipelines."""
        generator = TimeSeriesCandidateGenerator(
            target_col=self.target_col,
            datetime_col=self.datetime_col,
            horizon=self.horizon,
            seasonality=self.seasonality or 7,
            include_baselines=self.include_baselines,
            include_statistical=self.include_statistical,
            include_timesfm=self.include_timesfm,
            n_samples=len(df),
        )

        return generator.generate()
    
    def _evaluate_candidates(
        self,
        df: pd.DataFrame,
        candidates: List[ForecastingCandidate],
    ) -> List[ForecastingPipeline]:
        """Evaluate all candidates using time-aware CV."""
        fitted_pipelines = []
        
        # Create CV splitter
        cv = create_time_series_cv(
            cv_type=self.cv_type,
            n_splits=self.n_splits,
        )
        
        # Prepare data
        # Keep full df for ML models that need target column for lag features
        y = df[self.target_col].values
        X_no_target = df.drop(columns=[self.target_col])  # For baseline models
        X_full = df.copy()  # For ML models with lag features
        
        # Create scorer
        scorer = get_ts_scorer(self.primary_metric, y_train=y, seasonality=self.seasonality or 1)
        
        start_budget = time.time()
        
        for i, candidate in enumerate(candidates):
            # Check time budget
            if self.time_budget and (time.time() - start_budget) > self.time_budget:
                if self.verbose:
                    print(f"   Time budget exhausted. Evaluated {i}/{len(candidates)}")
                break
            
            try:
                pipeline = self._evaluate_single_candidate(
                    X_full, X_no_target, y, candidate, cv, scorer
                )
                fitted_pipelines.append(pipeline)
                
                if self.verbose:
                    print(f"   [{i+1}/{len(candidates)}] {candidate.name}: {pipeline.cv_score:.4f}")
            
            except Exception as e:
                if self.verbose:
                    print(f"   [{i+1}/{len(candidates)}] {candidate.name}: FAILED - {str(e)[:50]}")
        
        return fitted_pipelines
    
    def _evaluate_single_candidate(
        self,
        X_full: pd.DataFrame,
        X_no_target: pd.DataFrame,
        y: np.ndarray,
        candidate: ForecastingCandidate,
        cv: TimeSeriesSplitter,
        scorer: TimeSeriesScorer,
    ) -> ForecastingPipeline:
        """Evaluate a single candidate."""
        scores = []
        fit_times = []
        predict_times = []
        
        # Handle baseline models differently (they work on y directly)
        is_baseline = candidate.family == 'baseline'
        
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            
            # Use X_no_target for CV split indices
            for train_idx, test_idx in cv.split(X_no_target.values):
                if is_baseline:
                    # Baseline models use y directly
                    y_train, y_test = y[train_idx], y[test_idx]
                    X_train_dummy = np.zeros((len(train_idx), 1))
                    X_test_dummy = np.zeros((len(test_idx), 1))
                    
                    model = clone(candidate.pipeline)
                    
                    t0 = time.time()
                    model.fit(X_train_dummy, y_train)
                    fit_times.append(time.time() - t0)
                    
                    t0 = time.time()
                    y_pred = model.predict(X_test_dummy)
                    predict_times.append((time.time() - t0) / len(test_idx))
                    
                else:
                    # ML models use full data (with target for lag features)
                    X_train = X_full.iloc[train_idx].copy()
                    X_test = X_full.iloc[test_idx].copy()
                    y_train, y_test = y[train_idx], y[test_idx]
                    
                    model = clone(candidate.pipeline)
                    
                    t0 = time.time()
                    # Fit the pipeline - it will create lag features from target column
                    if hasattr(model, 'fit'):
                        try:
                            model.fit(X_train, y_train)
                        except Exception as fit_err:
                            # If pipeline fails, try without datetime column
                            X_train_fallback = X_train.select_dtypes(include=[np.number])
                            X_test = X_test.select_dtypes(include=[np.number])
                            model.fit(X_train_fallback, y_train)
                    fit_times.append(time.time() - t0)
                    
                    t0 = time.time()
                    try:
                        y_pred = model.predict(X_test)
                    except Exception:
                        X_test_num = X_test.select_dtypes(include=[np.number])
                        y_pred = model.predict(X_test_num)
                    predict_times.append((time.time() - t0) / len(test_idx))
                
                score = scorer(y_test, y_pred)
                scores.append(score)
        
        # Compute average metrics
        cv_score = float(np.mean(scores))
        fit_time = float(np.mean(fit_times))
        predict_time = float(np.mean(predict_times)) * 1000  # ms
        
        # Estimate memory (rough)
        import sys
        memory_mb = sys.getsizeof(model) / (1024 * 1024)
        
        return ForecastingPipeline(
            name=candidate.name,
            model=model,
            cv_score=cv_score,
            cv_scores={self.primary_metric: cv_score},
            fit_time=fit_time,
            predict_time=predict_time,
            memory_mb=memory_mb,
            family=candidate.family,
            hyperparams=candidate.hyperparams,
            is_baseline=candidate.family == 'baseline',
        )
    
    def _compute_pareto_front(
        self,
        pipelines: List[ForecastingPipeline],
    ) -> List[ForecastingPipeline]:
        """Compute Pareto-optimal pipelines (score vs time)."""
        pareto = []
        
        for p in pipelines:
            is_dominated = False
            for q in pipelines:
                if q is p:
                    continue
                # q dominates p if q is better in both score and time
                if q.cv_score <= p.cv_score and q.fit_time <= p.fit_time:
                    if q.cv_score < p.cv_score or q.fit_time < p.fit_time:
                        is_dominated = True
                        break
            
            if not is_dominated:
                pareto.append(p)
        
        return pareto
    
    def _extract_feature_importance(
        self,
        pipeline: ForecastingPipeline,
    ) -> Dict[str, float]:
        """Extract feature importance from the pipeline."""
        importance = {}
        
        model = pipeline.model
        
        # Try to get feature importance
        if hasattr(model, 'feature_importances_'):
            imp = model.feature_importances_
            if hasattr(model, 'feature_names_in_'):
                names = model.feature_names_in_
            else:
                names = [f'feature_{i}' for i in range(len(imp))]
            importance = dict(zip(names, imp))
        
        # For pipelines, look at the final estimator
        elif hasattr(model, 'named_steps'):
            final = list(model.named_steps.values())[-1]
            if hasattr(final, 'feature_importances_'):
                imp = final.feature_importances_
                importance = {f'feature_{i}': float(v) for i, v in enumerate(imp)}
        
        # Sort by importance
        importance = dict(sorted(importance.items(), key=lambda x: -x[1]))
        
        return importance
    
    def predict(self, df: pd.DataFrame) -> np.ndarray:
        """Make predictions using the best pipeline."""
        if self._result is None:
            raise RuntimeError("Must call fit() before predict()")
        
        return self._result.predict(df)
    
    @property
    def result(self) -> Optional[AutoTSResult]:
        """Get the last fit result."""
        return self._result


def auto_forecast(
    df: pd.DataFrame,
    datetime_col: str,
    target_col: str,
    horizon: int = 1,
    **kwargs,
) -> AutoTSResult:
    """
    Convenience function for quick auto-forecasting.
    
    Parameters
    ----------
    df : pd.DataFrame
        Time series data.
    datetime_col : str
        Datetime column name.
    target_col : str
        Target column name.
    horizon : int
        Forecast horizon.
    **kwargs
        Additional arguments for AutoTS.
    
    Returns
    -------
    AutoTSResult
        Optimization results.
    """
    ts = AutoTS(
        datetime_col=datetime_col,
        target_col=target_col,
        horizon=horizon,
        **kwargs,
    )
    return ts.fit(df)

