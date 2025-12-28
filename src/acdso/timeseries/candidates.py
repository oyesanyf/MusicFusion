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


def get_default_candidates(
    target_col: str,
    datetime_col: Optional[str] = None,
    seasonality: Optional[int] = None,
    complexity: str = 'all',
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
    )
    
    candidates = generator.generate()
    
    if complexity != 'all':
        candidates = [c for c in candidates if c.complexity == complexity or complexity == 'all']
    
    return candidates

