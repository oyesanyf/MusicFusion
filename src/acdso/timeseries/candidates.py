"""
Time Series Forecasting Candidates
===================================

Candidate pipelines for time series forecasting:
- Baseline models (Naive, Seasonal Naive)
- ML-based models (RF, LightGBM, XGBoost with lag features)
- Statistical models (ARIMA, ETS) - optional
- Prophet - optional
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any, Union
import numpy as np
import pandas as pd
from sklearn.base import BaseEstimator, RegressorMixin
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler
from sklearn.linear_model import Ridge, ElasticNet, Lasso
from sklearn.ensemble import RandomForestRegressor, GradientBoostingRegressor
from sklearn.neighbors import KNeighborsRegressor

from .transformers import (
    LagFeatureTransformer,
    RollingWindowTransformer,
    DateTimeFeatureTransformer,
    TimeSeriesFeatureUnion,
    create_ts_feature_pipeline,
    ColumnDropper,
    DropNaNRowsTransformer,
)


class NaiveForecaster(BaseEstimator, RegressorMixin):
    """
    Naive forecaster: predicts the last known value.
    
    Essential baseline for comparing more complex models.
    If your model can't beat naive, something is wrong.
    
    Parameters
    ----------
    strategy : str
        'last': Use last value (default)
        'mean': Use historical mean
        'drift': Use last value with trend
    """
    
    def __init__(self, strategy: str = 'last'):
        self.strategy = strategy
        self._last_value: float = 0.0
        self._mean_value: float = 0.0
        self._drift: float = 0.0
    
    def fit(self, X, y):
        """Fit the naive forecaster."""
        y = np.asarray(y)
        
        self._last_value = float(y[-1])
        self._mean_value = float(np.mean(y))
        
        if len(y) > 1:
            self._drift = (y[-1] - y[0]) / (len(y) - 1)
        else:
            self._drift = 0.0
        
        return self
    
    def predict(self, X):
        """Make predictions."""
        n = len(X)
        
        if self.strategy == 'last':
            return np.full(n, self._last_value)
        elif self.strategy == 'mean':
            return np.full(n, self._mean_value)
        elif self.strategy == 'drift':
            return np.array([self._last_value + i * self._drift for i in range(1, n + 1)])
        else:
            raise ValueError(f"Unknown strategy: {self.strategy}")


class SeasonalNaiveForecaster(BaseEstimator, RegressorMixin):
    """
    Seasonal naive forecaster: predicts the value from one season ago.
    
    Parameters
    ----------
    season_length : int
        Length of seasonal period (e.g., 7 for weekly, 12 for monthly).
    """
    
    def __init__(self, season_length: int = 7):
        self.season_length = season_length
        self._seasonal_values: np.ndarray = np.array([])
    
    def fit(self, X, y):
        """Store the last season of values."""
        y = np.asarray(y)
        
        # Store last season_length values
        self._seasonal_values = y[-self.season_length:]
        
        return self
    
    def predict(self, X):
        """Predict using seasonal pattern."""
        n = len(X)
        
        # Tile the seasonal pattern
        repeats = (n // self.season_length) + 1
        pattern = np.tile(self._seasonal_values, repeats)
        
        return pattern[:n]


class MovingAverageForecaster(BaseEstimator, RegressorMixin):
    """
    Moving average forecaster.
    
    Parameters
    ----------
    window : int
        Window size for moving average.
    """
    
    def __init__(self, window: int = 7):
        self.window = window
        self._ma_value: float = 0.0
    
    def fit(self, X, y):
        """Compute moving average."""
        y = np.asarray(y)
        
        self._ma_value = float(np.mean(y[-self.window:]))
        
        return self
    
    def predict(self, X):
        """Predict using moving average."""
        n = len(X)
        return np.full(n, self._ma_value)


class ExponentialSmoothingForecaster(BaseEstimator, RegressorMixin):
    """
    Simple exponential smoothing forecaster.
    
    Parameters
    ----------
    alpha : float
        Smoothing parameter (0-1). Higher = more weight on recent.
    """
    
    def __init__(self, alpha: float = 0.3):
        self.alpha = alpha
        self._smoothed_value: float = 0.0
    
    def fit(self, X, y):
        """Apply exponential smoothing."""
        y = np.asarray(y)
        
        self._smoothed_value = y[0]
        for val in y[1:]:
            self._smoothed_value = self.alpha * val + (1 - self.alpha) * self._smoothed_value
        
        return self
    
    def predict(self, X):
        """Predict using smoothed value."""
        n = len(X)
        return np.full(n, self._smoothed_value)


# Loaded model checkpoints are cached at the class level, keyed by
# checkpoint id, so cross-validation folds don't re-download/re-load the
# pretrained weights on every fit -- that would make TimesFM by far the
# slowest candidate for no benefit, since the weights don't change per fold.
_TIMESFM_MODEL_CACHE: Dict[str, Any] = {}


class TimesFMForecaster(BaseEstimator, RegressorMixin):
    """
    Adapter for Google's TimesFM pretrained time-series foundation model
    (https://github.com/google-research/timesfm).

    Zero-shot: ``fit`` just stores the context (the training series) and
    lazily loads the pretrained checkpoint; ``predict`` calls the model's
    own forecasting API for ``len(X)`` steps ahead, matching the same
    "predict(X) uses len(X) as the horizon" convention as the other
    baseline forecasters in this module.

    Parameters
    ----------
    checkpoint : str, default="google/timesfm-2.5-200m-pytorch"
        Hugging Face checkpoint id to load. Defaults to a 2.5 checkpoint
        deliberately: TimesFM's source code is Apache-2.0, and pretrained
        weights **up to 2.5** are Apache-2.0, but **3.0's weights are
        licensed non-commercial/non-production only**
        (timesfm-non-commercial-license-v1.0). Only pass a 3.0 checkpoint
        if non-commercial use is confirmed for this deployment.
    min_context_length : int, default=64
        TimesFM is a foundation model built for long-context zero-shot
        inference; on very short series (the common case for this
        project's own Forecast card, e.g. a dozen monthly aggregates) it
        has no real advantage over a two-line trend method and just adds
        a heavy dependency and load time for nothing. This is enforced by
        the candidate generator (see ``include_timesfm``/``n_samples`` on
        ``TimeSeriesCandidateGenerator``), not by this class itself, so
        directly instantiating it bypasses the guard intentionally.
    device : str, default="cpu"
        "cpu" or "cuda". GPU inference is much faster but not required.

    Notes
    -----
    Cannot run in a network environment where huggingface.co is blocked
    by policy: ``ModelConfig(checkpoint_path=...)`` fetches the pretrained
    weights from the Hugging Face Hub at load time, and there is no local-
    weights fallback wired up here. This is a hard requirement of TimesFM
    itself, not something this adapter can work around -- if your network
    policy blocks Hugging Face, this candidate will always fail on its
    first ``predict()`` call (model loading is lazy, so ``fit()`` alone
    succeeds; the generator's ImportError-only availability check won't
    catch this either, since the package import itself succeeds) and
    there is no point enabling ``include_timesfm`` at all.

    The exact ``timesfm`` Python API also differs between model versions
    (2.5 vs 3.0 ship different top-level modules/classes), and this
    adapter was written against the *documented* 3.0 API
    (``timesfm3.TimesFM3Evaluator`` / ``ModelConfig``) since that's what's
    publicly documented with code examples at the time of writing. It has
    not been run against a real installation in an environment with both
    package-install and Hugging Face access -- confirm the 2.5 checkpoint
    loads through the same evaluator class before relying on this.
    """

    def __init__(
        self,
        checkpoint: str = "google/timesfm-2.5-200m-pytorch",
        min_context_length: int = 64,
        device: str = "cpu",
        per_core_batch_size: int = 8,
    ):
        self.checkpoint = checkpoint
        self.min_context_length = min_context_length
        self.device = device
        self.per_core_batch_size = per_core_batch_size
        self._context: np.ndarray = np.array([])

    def _load_model(self):
        cached = _TIMESFM_MODEL_CACHE.get(self.checkpoint)
        if cached is not None:
            return cached

        try:
            from timesfm3 import ModelConfig, TimesFM3Evaluator
        except ImportError as exc:
            raise ImportError(
                "TimesFM is not installed. Install with: pip install timesfm[torch]"
            ) from exc

        config = ModelConfig(
            checkpoint_path=self.checkpoint,
            per_core_batch_size=self.per_core_batch_size,
            device=self.device,
        )
        model = TimesFM3Evaluator(config)
        _TIMESFM_MODEL_CACHE[self.checkpoint] = model
        return model

    def fit(self, X, y):
        self._context = np.asarray(y, dtype=np.float32)
        return self

    def predict(self, X):
        horizon = len(X)
        model = self._load_model()
        outputs = list(
            model.predict_batch(
                [self._context], horizon=horizon, return_quantiles=False,
                use_symmetric_averaging=False,
            )
        )
        return np.asarray(outputs[0].forecast, dtype=np.float64)


class TimeSeriesRegressor(BaseEstimator, RegressorMixin):
    """
    Wrapper that handles time series feature engineering with proper X/y alignment.
    
    This wrapper:
    1. Creates lag and rolling features from the target column in X
    2. Creates datetime features if datetime_col is provided
    3. Drops the original target and datetime columns
    4. Drops NaN rows (from lag features) and aligns y accordingly
    5. Fits the inner model on the transformed features
    
    Parameters
    ----------
    model : estimator
        sklearn-compatible regressor.
    target_col : str
        Target column name (must be present in X during fit).
    datetime_col : str, optional
        Datetime column for calendar features.
    lags : list of int
        Lag values to create.
    rolling_windows : list of int
        Window sizes for rolling stats.
    """
    
    def __init__(
        self,
        model: BaseEstimator,
        target_col: str,
        datetime_col: Optional[str] = None,
        lags: List[int] = [1, 7, 14],
        rolling_windows: List[int] = [7, 14],
    ):
        self.model = model
        self.target_col = target_col
        self.datetime_col = datetime_col
        self.lags = lags
        self.rolling_windows = rolling_windows
        
        self._feature_transformer = None
        self._scaler = None
        self._fitted_model = None
        self._feature_cols = None
    
    def fit(self, X, y):
        """Fit the model with proper feature engineering and alignment."""
        if isinstance(X, np.ndarray):
            X = pd.DataFrame(X)
        else:
            X = X.copy()
        
        y = np.asarray(y)
        
        # First, drop non-numeric columns (except target and datetime which are handled later)
        # This prevents datetime arithmetic errors
        cols_to_drop_first = []
        for col in X.columns:
            if col != self.target_col and col != self.datetime_col:
                try:
                    if not np.issubdtype(X[col].dtype, np.number):
                        cols_to_drop_first.append(col)
                except Exception:
                    cols_to_drop_first.append(col)
        if cols_to_drop_first:
            X = X.drop(columns=cols_to_drop_first, errors='ignore')
        
        # Build feature transformer
        self._feature_transformer = create_ts_feature_pipeline(
            target_col=self.target_col,
            datetime_col=self.datetime_col,
            lags=self.lags,
            rolling_windows=self.rolling_windows,
            rolling_functions=['mean', 'std'],
        )
        
        # Transform X to create features
        X_transformed = self._feature_transformer.fit_transform(X)
        
        # Determine columns to drop (original target, datetime, and non-numeric)
        cols_to_drop = []
        if self.target_col in X_transformed.columns:
            cols_to_drop.append(self.target_col)
        if self.datetime_col and self.datetime_col in X_transformed.columns:
            cols_to_drop.append(self.datetime_col)
        
        # Also drop any remaining non-numeric columns
        for col in X_transformed.columns:
            if col not in cols_to_drop:
                if not np.issubdtype(X_transformed[col].dtype, np.number):
                    cols_to_drop.append(col)
        
        X_transformed = X_transformed.drop(columns=cols_to_drop, errors='ignore')
        
        # Get valid indices (non-NaN rows)
        valid_mask = ~X_transformed.isna().any(axis=1)
        X_clean = X_transformed[valid_mask].reset_index(drop=True)
        y_clean = y[valid_mask.values]
        
        if len(X_clean) < 10:
            raise ValueError(f"Only {len(X_clean)} valid samples after lag feature creation")
        
        # Store feature columns
        self._feature_cols = X_clean.columns.tolist()
        
        # Scale features
        self._scaler = StandardScaler()
        X_scaled = self._scaler.fit_transform(X_clean)
        
        # Fit the model
        from sklearn.base import clone
        self._fitted_model = clone(self.model)
        self._fitted_model.fit(X_scaled, y_clean)
        
        return self
    
    def predict(self, X):
        """Predict using the fitted model."""
        if isinstance(X, np.ndarray):
            X = pd.DataFrame(X)
        else:
            X = X.copy()
        
        # First, drop non-numeric columns (except target and datetime which are handled later)
        cols_to_drop_first = []
        for col in X.columns:
            if col != self.target_col and col != self.datetime_col:
                try:
                    if not np.issubdtype(X[col].dtype, np.number):
                        cols_to_drop_first.append(col)
                except Exception:
                    cols_to_drop_first.append(col)
        if cols_to_drop_first:
            X = X.drop(columns=cols_to_drop_first, errors='ignore')
        
        # Transform features
        X_transformed = self._feature_transformer.transform(X)
        
        # Drop columns
        cols_to_drop = []
        if self.target_col in X_transformed.columns:
            cols_to_drop.append(self.target_col)
        if self.datetime_col and self.datetime_col in X_transformed.columns:
            cols_to_drop.append(self.datetime_col)
        for col in X_transformed.columns:
            if col not in cols_to_drop and col not in self._feature_cols:
                cols_to_drop.append(col)
        
        X_transformed = X_transformed.drop(columns=cols_to_drop, errors='ignore')
        
        # Select only the features we trained on
        X_transformed = X_transformed[self._feature_cols]
        
        # Handle NaN: fill with 0 for prediction (or could use last valid value)
        X_transformed = X_transformed.fillna(0)
        
        # Scale
        X_scaled = self._scaler.transform(X_transformed)
        
        return self._fitted_model.predict(X_scaled)


@dataclass
class ForecastingCandidate:
    """
    A candidate forecasting pipeline.
    
    Attributes
    ----------
    name : str
        Human-readable name.
    pipeline : Pipeline
        sklearn Pipeline object.
    family : str
        Model family: 'baseline', 'linear', 'tree', 'boosting', 'statistical'.
    complexity : str
        Complexity level: 'simple', 'medium', 'complex'.
    requires_stationarity : bool
        Whether the model assumes stationarity.
    handles_seasonality : bool
        Whether the model handles seasonality natively.
    hyperparams : dict
        Hyperparameters used.
    """
    
    name: str
    pipeline: Any  # Pipeline or model
    family: str
    complexity: str = 'medium'
    requires_stationarity: bool = False
    handles_seasonality: bool = False
    hyperparams: Dict[str, Any] = field(default_factory=dict)


def create_ts_pipeline(
    model: BaseEstimator,
    target_col: str,
    datetime_col: Optional[str] = None,
    lags: List[int] = [1, 7, 14],
    rolling_windows: List[int] = [7, 14],
    include_scaler: bool = True,
) -> TimeSeriesRegressor:
    """
    Create a forecasting pipeline with feature engineering.
    
    Uses TimeSeriesRegressor wrapper that handles:
    - Lag feature creation from target column
    - Rolling statistics
    - DateTime features
    - Proper X/y alignment after NaN removal
    - Scaling
    
    Parameters
    ----------
    model : estimator
        sklearn-compatible regressor.
    target_col : str
        Target column name.
    datetime_col : str, optional
        Datetime column for calendar features.
    lags : list
        Lag values to create.
    rolling_windows : list
        Window sizes for rolling stats.
    include_scaler : bool
        Ignored (scaler always included for consistency).
    
    Returns
    -------
    TimeSeriesRegressor
        Configured forecasting wrapper.
    """
    return TimeSeriesRegressor(
        model=model,
        target_col=target_col,
        datetime_col=datetime_col,
        lags=lags,
        rolling_windows=rolling_windows,
    )


class TimeSeriesCandidateGenerator:
    """
    Generate candidate forecasting pipelines.
    
    Similar to ACDSO's candidate generation but for time series.
    
    Parameters
    ----------
    target_col : str
        Target column name.
    datetime_col : str, optional
        Datetime column for calendar features.
    horizon : int
        Forecast horizon.
    seasonality : int, optional
        Seasonal period.
    include_baselines : bool
        Whether to include baseline models.
    include_linear : bool
        Whether to include linear models.
    include_trees : bool
        Whether to include tree models.
    include_boosting : bool
        Whether to include boosting models.
    include_statistical : bool
        Whether to include statistical models (requires statsmodels).
    include_timesfm : bool
        Whether to include Google's TimesFM pretrained foundation model
        (requires ``pip install timesfm[torch]``). Off by default: it's a
        heavy optional dependency, and only helps once there's enough
        history to matter -- see ``n_samples``.
    n_samples : int, optional
        Length of the training series. Only used to gate ``include_timesfm``
        against ``TimesFMForecaster.min_context_length`` -- irrelevant to
        every other candidate family.
    """

    def __init__(
        self,
        target_col: str,
        datetime_col: Optional[str] = None,
        horizon: int = 1,
        seasonality: Optional[int] = None,
        include_baselines: bool = True,
        include_linear: bool = True,
        include_trees: bool = True,
        include_boosting: bool = True,
        include_statistical: bool = False,
        include_timesfm: bool = False,
        n_samples: Optional[int] = None,
    ):
        self.target_col = target_col
        self.datetime_col = datetime_col
        self.horizon = horizon
        self.seasonality = seasonality or 7
        self.include_baselines = include_baselines
        self.include_linear = include_linear
        self.include_trees = include_trees
        self.include_boosting = include_boosting
        self.include_statistical = include_statistical
        self.include_timesfm = include_timesfm
        self.n_samples = n_samples

    def generate(self) -> List[ForecastingCandidate]:
        """
        Generate all candidate pipelines.

        Returns
        -------
        list of ForecastingCandidate
            All candidate models.
        """
        candidates = []

        # Baseline models
        if self.include_baselines:
            candidates.extend(self._generate_baselines())

        # Linear models
        if self.include_linear:
            candidates.extend(self._generate_linear())

        # Tree models
        if self.include_trees:
            candidates.extend(self._generate_trees())

        # Boosting models
        if self.include_boosting:
            candidates.extend(self._generate_boosting())

        # Statistical models
        if self.include_statistical:
            candidates.extend(self._generate_statistical())

        # TimesFM (pretrained foundation model, opt-in, needs real history)
        if self.include_timesfm:
            candidates.extend(self._generate_timesfm())

        return candidates
    
    def _generate_baselines(self) -> List[ForecastingCandidate]:
        """Generate baseline candidates."""
        return [
            ForecastingCandidate(
                name='Naive (Last)',
                pipeline=NaiveForecaster(strategy='last'),
                family='baseline',
                complexity='simple',
            ),
            ForecastingCandidate(
                name='Naive (Mean)',
                pipeline=NaiveForecaster(strategy='mean'),
                family='baseline',
                complexity='simple',
            ),
            ForecastingCandidate(
                name='Naive (Drift)',
                pipeline=NaiveForecaster(strategy='drift'),
                family='baseline',
                complexity='simple',
            ),
            ForecastingCandidate(
                name=f'Seasonal Naive ({self.seasonality})',
                pipeline=SeasonalNaiveForecaster(season_length=self.seasonality),
                family='baseline',
                complexity='simple',
                handles_seasonality=True,
            ),
            ForecastingCandidate(
                name='Moving Average (7)',
                pipeline=MovingAverageForecaster(window=7),
                family='baseline',
                complexity='simple',
            ),
            ForecastingCandidate(
                name='Exponential Smoothing',
                pipeline=ExponentialSmoothingForecaster(alpha=0.3),
                family='baseline',
                complexity='simple',
            ),
        ]
    
    def _generate_linear(self) -> List[ForecastingCandidate]:
        """Generate linear model candidates."""
        lags = [1, self.seasonality, self.seasonality * 2] if self.seasonality else [1, 7, 14]
        
        candidates = []
        
        for alpha in [0.1, 1.0, 10.0]:
            candidates.append(ForecastingCandidate(
                name=f'Ridge (alpha={alpha})',
                pipeline=create_ts_pipeline(
                    Ridge(alpha=alpha),
                    target_col=self.target_col,
                    datetime_col=self.datetime_col,
                    lags=lags,
                ),
                family='linear',
                complexity='simple',
                hyperparams={'alpha': alpha},
            ))
        
        candidates.append(ForecastingCandidate(
            name='ElasticNet',
            pipeline=create_ts_pipeline(
                ElasticNet(alpha=0.1, l1_ratio=0.5, max_iter=1000),
                target_col=self.target_col,
                datetime_col=self.datetime_col,
                lags=lags,
            ),
            family='linear',
            complexity='simple',
            hyperparams={'alpha': 0.1, 'l1_ratio': 0.5},
        ))
        
        candidates.append(ForecastingCandidate(
            name='Lasso',
            pipeline=create_ts_pipeline(
                Lasso(alpha=0.1, max_iter=1000),
                target_col=self.target_col,
                datetime_col=self.datetime_col,
                lags=lags,
            ),
            family='linear',
            complexity='simple',
            hyperparams={'alpha': 0.1},
        ))
        
        return candidates
    
    def _generate_trees(self) -> List[ForecastingCandidate]:
        """Generate tree model candidates."""
        lags = [1, self.seasonality, self.seasonality * 2] if self.seasonality else [1, 7, 14]
        
        candidates = []
        
        for n_estimators in [50, 100]:
            for max_depth in [5, 10, None]:
                candidates.append(ForecastingCandidate(
                    name=f'RandomForest (n={n_estimators}, d={max_depth})',
                    pipeline=create_ts_pipeline(
                        RandomForestRegressor(
                            n_estimators=n_estimators,
                            max_depth=max_depth,
                            random_state=42,
                            n_jobs=-1,
                        ),
                        target_col=self.target_col,
                        datetime_col=self.datetime_col,
                        lags=lags,
                    ),
                    family='tree',
                    complexity='medium',
                    hyperparams={'n_estimators': n_estimators, 'max_depth': max_depth},
                ))
        
        # KNN
        for n_neighbors in [5, 10]:
            candidates.append(ForecastingCandidate(
                name=f'KNN (k={n_neighbors})',
                pipeline=create_ts_pipeline(
                    KNeighborsRegressor(n_neighbors=n_neighbors, n_jobs=-1),
                    target_col=self.target_col,
                    datetime_col=self.datetime_col,
                    lags=lags,
                ),
                family='tree',
                complexity='simple',
                hyperparams={'n_neighbors': n_neighbors},
            ))
        
        return candidates
    
    def _generate_boosting(self) -> List[ForecastingCandidate]:
        """Generate boosting model candidates."""
        lags = [1, self.seasonality, self.seasonality * 2] if self.seasonality else [1, 7, 14]
        
        candidates = []
        
        # Gradient Boosting (sklearn)
        candidates.append(ForecastingCandidate(
            name='GradientBoosting',
            pipeline=create_ts_pipeline(
                GradientBoostingRegressor(
                    n_estimators=100,
                    max_depth=5,
                    learning_rate=0.1,
                    random_state=42,
                ),
                target_col=self.target_col,
                datetime_col=self.datetime_col,
                lags=lags,
            ),
            family='boosting',
            complexity='medium',
        ))
        
        # Try LightGBM
        try:
            from lightgbm import LGBMRegressor
            
            for n_estimators in [100, 200]:
                candidates.append(ForecastingCandidate(
                    name=f'LightGBM (n={n_estimators})',
                    pipeline=create_ts_pipeline(
                        LGBMRegressor(
                            n_estimators=n_estimators,
                            max_depth=5,
                            learning_rate=0.1,
                            random_state=42,
                            verbosity=-1,
                        ),
                        target_col=self.target_col,
                        datetime_col=self.datetime_col,
                        lags=lags,
                    ),
                    family='boosting',
                    complexity='medium',
                    hyperparams={'n_estimators': n_estimators},
                ))
        except ImportError:
            pass
        
        # Try XGBoost
        try:
            from xgboost import XGBRegressor
            
            for n_estimators in [100, 200]:
                candidates.append(ForecastingCandidate(
                    name=f'XGBoost (n={n_estimators})',
                    pipeline=create_ts_pipeline(
                        XGBRegressor(
                            n_estimators=n_estimators,
                            max_depth=5,
                            learning_rate=0.1,
                            random_state=42,
                            verbosity=0,
                        ),
                        target_col=self.target_col,
                        datetime_col=self.datetime_col,
                        lags=lags,
                    ),
                    family='boosting',
                    complexity='medium',
                    hyperparams={'n_estimators': n_estimators},
                ))
        except ImportError:
            pass
        
        return candidates
    
    def _generate_statistical(self) -> List[ForecastingCandidate]:
        """Generate statistical model candidates (ARIMA, ETS, etc.)."""
        candidates = []
        
        try:
            from .statistical import (
                AutoARIMAWrapper,
                ETSWrapper,
                ThetaWrapper,
            )
            
            candidates.append(ForecastingCandidate(
                name='AutoARIMA',
                pipeline=AutoARIMAWrapper(seasonal=True, m=self.seasonality),
                family='statistical',
                complexity='complex',
                handles_seasonality=True,
            ))
            
            candidates.append(ForecastingCandidate(
                name='ETS',
                pipeline=ETSWrapper(seasonal=True, seasonal_periods=self.seasonality),
                family='statistical',
                complexity='complex',
                handles_seasonality=True,
            ))
            
            candidates.append(ForecastingCandidate(
                name='Theta',
                pipeline=ThetaWrapper(),
                family='statistical',
                complexity='medium',
            ))
        except ImportError:
            pass

        return candidates

    def _generate_timesfm(self) -> List[ForecastingCandidate]:
        """Generate the TimesFM candidate, if installed and there's enough history."""
        min_len = TimesFMForecaster().min_context_length
        if self.n_samples is not None and self.n_samples < min_len:
            return []

        try:
            import timesfm3  # noqa: F401 -- availability check only; loaded lazily on fit
        except ImportError:
            return []

        return [
            ForecastingCandidate(
                name="TimesFM (2.5, pretrained)",
                pipeline=TimesFMForecaster(),
                family="foundation",
                complexity="complex",
                handles_seasonality=True,
            )
        ]


def get_default_candidates(
    target_col: str,
    datetime_col: Optional[str] = None,
    seasonality: Optional[int] = None,
    complexity: str = 'all',
    include_timesfm: bool = False,
    n_samples: Optional[int] = None,
) -> List[ForecastingCandidate]:
    """
    Get default candidate pipelines.

    Parameters
    ----------
    target_col : str
        Target column.
    datetime_col : str, optional
        Datetime column.
    seasonality : int, optional
        Seasonal period.
    complexity : str
        'simple', 'medium', 'complex', or 'all'.
    include_timesfm : bool
        Whether to include Google's TimesFM pretrained model, if installed
        and there's enough history. Off by default (heavy dependency).
    n_samples : int, optional
        Training series length, used only to gate ``include_timesfm``.

    Returns
    -------
    list
        List of ForecastingCandidate.
    """
    generator = TimeSeriesCandidateGenerator(
        target_col=target_col,
        datetime_col=datetime_col,
        seasonality=seasonality,
        include_statistical=(complexity in ['complex', 'all']),
        include_timesfm=include_timesfm,
        n_samples=n_samples,
    )

    candidates = generator.generate()
    
    if complexity != 'all':
        candidates = [c for c in candidates if c.complexity == complexity or complexity == 'all']
    
    return candidates

