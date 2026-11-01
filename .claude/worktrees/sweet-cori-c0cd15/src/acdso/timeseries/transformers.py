"""
Time Series Feature Transformers
=================================

Transformers to convert raw time series into supervised learning features:
- LagFeatureTransformer: Create lag features (y[t-1], y[t-2], ...)
- RollingWindowTransformer: Rolling statistics (mean, std, min, max)
- DateTimeFeatureTransformer: Extract calendar features (hour, day, month, etc.)
- SeasonalDecomposer: Decompose into trend, seasonal, residual
- DifferencingTransformer: Make series stationary
- ColumnDropper: Drop specified columns after feature engineering
- DropNaNRowsTransformer: Drop rows with NaN values
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, Optional, Union, Tuple, Dict, Any
import numpy as np
import pandas as pd
from sklearn.base import BaseEstimator, TransformerMixin


class ColumnDropper(BaseEstimator, TransformerMixin):
    """
    Drop specified columns from the dataframe.
    
    Used after feature engineering to remove original target/datetime columns
    that should not be passed to the model.
    
    Parameters
    ----------
    columns : list of str
        Column names to drop. Columns that don't exist are silently ignored.
    """
    
    def __init__(self, columns: List[str]):
        self.columns = columns
    
    def fit(self, X, y=None):
        return self
    
    def transform(self, X):
        if isinstance(X, np.ndarray):
            return X
        
        result = X.copy()
        cols_to_drop = [c for c in self.columns if c in result.columns]
        if cols_to_drop:
            result = result.drop(columns=cols_to_drop)
        return result
    
    def get_feature_names_out(self, input_features=None):
        if input_features is None:
            return None
        return [f for f in input_features if f not in self.columns]


class DropNaNRowsTransformer(BaseEstimator, TransformerMixin):
    """
    Drop rows containing NaN values.
    
    Important for time series pipelines where lag features create NaN rows
    at the beginning of the series.
    
    Parameters
    ----------
    how : str, default='any'
        'any' drops rows with any NaN, 'all' drops only rows that are all NaN.
    subset : list of str, optional
        Only consider these columns for NaN check.
    """
    
    def __init__(self, how: str = 'any', subset: Optional[List[str]] = None):
        self.how = how
        self.subset = subset
        self._valid_mask = None
    
    def fit(self, X, y=None):
        return self
    
    def transform(self, X):
        if isinstance(X, np.ndarray):
            X = pd.DataFrame(X)
        
        result = X.copy()
        subset = self.subset if self.subset else None
        
        # Store the valid mask for potential use in transforming y
        if subset:
            self._valid_mask = ~result[subset].isna().any(axis=1)
        else:
            self._valid_mask = ~result.isna().any(axis=1)
        
        result = result.dropna(how=self.how, subset=subset)
        return result
    
    def get_valid_mask(self):
        """Return the mask of valid (non-NaN) rows from last transform."""
        return self._valid_mask


class LagFeatureTransformer(BaseEstimator, TransformerMixin):
    """
    Create lag features from time series data.
    
    Transforms y[t] into features: y[t-1], y[t-2], ..., y[t-max_lag]
    
    Parameters
    ----------
    lags : list of int, optional
        Specific lag values to create. If None, uses range(1, max_lag+1).
    max_lag : int, default=7
        Maximum lag to create (used if lags is None).
    target_col : str, optional
        Column name to create lags from. If None, uses all numeric columns.
    drop_na : bool, default=True
        Whether to drop rows with NaN values created by lagging.
    prefix : str, default="lag"
        Prefix for lag column names.
    
    Example
    -------
    >>> transformer = LagFeatureTransformer(lags=[1, 7, 14])
    >>> X_lagged = transformer.fit_transform(df[['sales']])
    # Creates: sales_lag_1, sales_lag_7, sales_lag_14
    """
    
    def __init__(
        self,
        lags: Optional[List[int]] = None,
        max_lag: int = 7,
        target_col: Optional[str] = None,
        drop_na: bool = True,
        prefix: str = "lag",
    ):
        self.lags = lags
        self.max_lag = max_lag
        self.target_col = target_col
        self.drop_na = drop_na
        self.prefix = prefix
        self._feature_names: List[str] = []
    
    def fit(self, X: pd.DataFrame, y=None) -> "LagFeatureTransformer":
        """Fit transformer (determines columns and lag values)."""
        if isinstance(X, np.ndarray):
            X = pd.DataFrame(X)
        
        # Determine lags to use
        self._lags = self.lags if self.lags is not None else list(range(1, self.max_lag + 1))
        
        # Determine columns to lag
        if self.target_col:
            self._cols = [self.target_col]
        else:
            self._cols = X.select_dtypes(include=[np.number]).columns.tolist()
        
        # Generate feature names
        self._feature_names = [
            f"{col}_{self.prefix}_{lag}"
            for col in self._cols
            for lag in self._lags
        ]
        
        return self
    
    def transform(self, X: pd.DataFrame) -> pd.DataFrame:
        """Create lag features."""
        if isinstance(X, np.ndarray):
            X = pd.DataFrame(X)
        
        result = X.copy()
        
        for col in self._cols:
            for lag in self._lags:
                result[f"{col}_{self.prefix}_{lag}"] = X[col].shift(lag)
        
        if self.drop_na:
            result = result.dropna()
        
        return result
    
    def get_feature_names_out(self, input_features=None) -> List[str]:
        """Get output feature names."""
        return self._feature_names
    
    @property
    def min_samples_required(self) -> int:
        """Minimum samples needed after lagging."""
        return max(self._lags) if hasattr(self, '_lags') else self.max_lag


class RollingWindowTransformer(BaseEstimator, TransformerMixin):
    """
    Create rolling window statistics from time series.
    
    Parameters
    ----------
    windows : list of int
        Window sizes for rolling calculations.
    functions : list of str
        Aggregation functions: 'mean', 'std', 'min', 'max', 'sum', 'median', 'skew', 'kurt'
    target_col : str, optional
        Column to compute rolling stats for. If None, uses all numeric.
    min_periods : int, optional
        Minimum periods for rolling calculation.
    drop_na : bool, default=True
        Whether to drop NaN rows.
    
    Example
    -------
    >>> transformer = RollingWindowTransformer(windows=[7, 30], functions=['mean', 'std'])
    >>> X_rolling = transformer.fit_transform(df[['sales']])
    # Creates: sales_roll_7_mean, sales_roll_7_std, sales_roll_30_mean, sales_roll_30_std
    """
    
    SUPPORTED_FUNCTIONS = {
        'mean': 'mean',
        'std': 'std',
        'min': 'min',
        'max': 'max',
        'sum': 'sum',
        'median': 'median',
        'skew': 'skew',
        'kurt': 'kurt',
        'var': 'var',
    }
    
    def __init__(
        self,
        windows: List[int] = [7, 14, 30],
        functions: List[str] = ['mean', 'std'],
        target_col: Optional[str] = None,
        min_periods: Optional[int] = None,
        drop_na: bool = True,
    ):
        self.windows = windows
        self.functions = functions
        self.target_col = target_col
        self.min_periods = min_periods
        self.drop_na = drop_na
        self._feature_names: List[str] = []
    
    def fit(self, X: pd.DataFrame, y=None) -> "RollingWindowTransformer":
        """Fit transformer."""
        if isinstance(X, np.ndarray):
            X = pd.DataFrame(X)
        
        # Validate functions
        for func in self.functions:
            if func not in self.SUPPORTED_FUNCTIONS:
                raise ValueError(f"Unknown function '{func}'. Supported: {list(self.SUPPORTED_FUNCTIONS.keys())}")
        
        # Determine columns
        if self.target_col:
            self._cols = [self.target_col]
        else:
            self._cols = X.select_dtypes(include=[np.number]).columns.tolist()
        
        # Generate feature names
        self._feature_names = [
            f"{col}_roll_{window}_{func}"
            for col in self._cols
            for window in self.windows
            for func in self.functions
        ]
        
        return self
    
    def transform(self, X: pd.DataFrame) -> pd.DataFrame:
        """Create rolling window features."""
        if isinstance(X, np.ndarray):
            X = pd.DataFrame(X)
        
        result = X.copy()
        
        for col in self._cols:
            for window in self.windows:
                min_periods = self.min_periods or window
                rolling = X[col].rolling(window=window, min_periods=min_periods)
                
                for func in self.functions:
                    feat_name = f"{col}_roll_{window}_{func}"
                    result[feat_name] = getattr(rolling, self.SUPPORTED_FUNCTIONS[func])()
        
        if self.drop_na:
            result = result.dropna()
        
        return result
    
    def get_feature_names_out(self, input_features=None) -> List[str]:
        return self._feature_names
    
    @property
    def min_samples_required(self) -> int:
        return max(self.windows) if self.windows else 1


class DateTimeFeatureTransformer(BaseEstimator, TransformerMixin):
    """
    Extract calendar/datetime features from a datetime column.
    
    Parameters
    ----------
    datetime_col : str
        Name of the datetime column.
    features : list of str, optional
        Features to extract. If None, extracts all available.
        Options: 'year', 'month', 'day', 'dayofweek', 'dayofyear',
                 'weekofyear', 'quarter', 'hour', 'minute', 'is_weekend',
                 'is_month_start', 'is_month_end', 'is_quarter_start', 'is_quarter_end'
    cyclical : bool, default=True
        Whether to encode cyclical features (hour, month, etc.) as sin/cos.
    drop_original : bool, default=False
        Whether to drop the original datetime column.
    
    Example
    -------
    >>> transformer = DateTimeFeatureTransformer(datetime_col='date')
    >>> X_dt = transformer.fit_transform(df)
    # Creates: year, month, month_sin, month_cos, dayofweek, etc.
    """
    
    AVAILABLE_FEATURES = [
        'year', 'month', 'day', 'dayofweek', 'dayofyear', 'weekofyear',
        'quarter', 'hour', 'minute', 'second', 'is_weekend',
        'is_month_start', 'is_month_end', 'is_quarter_start', 'is_quarter_end',
        'is_year_start', 'is_year_end',
    ]
    
    CYCLICAL_FEATURES = {
        'month': 12,
        'day': 31,
        'dayofweek': 7,
        'dayofyear': 365,
        'weekofyear': 52,
        'hour': 24,
        'minute': 60,
        'quarter': 4,
    }
    
    def __init__(
        self,
        datetime_col: str,
        features: Optional[List[str]] = None,
        cyclical: bool = True,
        drop_original: bool = False,
    ):
        self.datetime_col = datetime_col
        self.features = features
        self.cyclical = cyclical
        self.drop_original = drop_original
        self._feature_names: List[str] = []
    
    def fit(self, X: pd.DataFrame, y=None) -> "DateTimeFeatureTransformer":
        """Fit transformer."""
        if self.datetime_col not in X.columns:
            raise ValueError(f"Column '{self.datetime_col}' not found in data.")
        
        # Determine features to extract
        self._features = self.features or [
            'year', 'month', 'day', 'dayofweek', 'hour', 'is_weekend'
        ]
        
        # Validate features
        for feat in self._features:
            if feat not in self.AVAILABLE_FEATURES:
                raise ValueError(f"Unknown feature '{feat}'. Available: {self.AVAILABLE_FEATURES}")
        
        return self
    
    def transform(self, X: pd.DataFrame) -> pd.DataFrame:
        """Extract datetime features."""
        result = X.copy()
        dt = pd.to_datetime(result[self.datetime_col])
        
        for feat in self._features:
            if feat == 'is_weekend':
                result[f'dt_{feat}'] = (dt.dt.dayofweek >= 5).astype(int)
            elif feat in ['is_month_start', 'is_month_end', 'is_quarter_start', 
                         'is_quarter_end', 'is_year_start', 'is_year_end']:
                result[f'dt_{feat}'] = getattr(dt.dt, feat).astype(int)
            elif feat == 'weekofyear':
                result[f'dt_{feat}'] = dt.dt.isocalendar().week.astype(int)
            else:
                result[f'dt_{feat}'] = getattr(dt.dt, feat)
            
            # Add cyclical encoding
            if self.cyclical and feat in self.CYCLICAL_FEATURES:
                period = self.CYCLICAL_FEATURES[feat]
                values = result[f'dt_{feat}']
                result[f'dt_{feat}_sin'] = np.sin(2 * np.pi * values / period)
                result[f'dt_{feat}_cos'] = np.cos(2 * np.pi * values / period)
        
        if self.drop_original:
            result = result.drop(columns=[self.datetime_col])
        
        self._feature_names = [c for c in result.columns if c.startswith('dt_')]
        
        return result
    
    def get_feature_names_out(self, input_features=None) -> List[str]:
        return self._feature_names


class SeasonalDecomposer(BaseEstimator, TransformerMixin):
    """
    Decompose time series into trend, seasonal, and residual components.
    
    Uses statsmodels seasonal_decompose under the hood.
    
    Parameters
    ----------
    target_col : str
        Column to decompose.
    period : int, optional
        Seasonal period. If None, attempts to detect automatically.
    model : str, default='additive'
        Decomposition model: 'additive' or 'multiplicative'.
    include_components : list of str, optional
        Components to include: 'trend', 'seasonal', 'residual'. Default all.
    """
    
    def __init__(
        self,
        target_col: str,
        period: Optional[int] = None,
        model: str = 'additive',
        include_components: Optional[List[str]] = None,
    ):
        self.target_col = target_col
        self.period = period
        self.model = model
        self.include_components = include_components or ['trend', 'seasonal', 'residual']
    
    def fit(self, X: pd.DataFrame, y=None) -> "SeasonalDecomposer":
        return self
    
    def transform(self, X: pd.DataFrame) -> pd.DataFrame:
        """Decompose and add component columns."""
        try:
            from statsmodels.tsa.seasonal import seasonal_decompose
        except ImportError:
            raise ImportError("statsmodels required for SeasonalDecomposer. pip install statsmodels")
        
        result = X.copy()
        series = X[self.target_col]
        
        # Detect period if not provided
        period = self.period
        if period is None:
            period = self._detect_period(series)
        
        if len(series) < 2 * period:
            # Not enough data for decomposition, return original
            return result
        
        decomposition = seasonal_decompose(series, model=self.model, period=period)
        
        if 'trend' in self.include_components:
            result[f'{self.target_col}_trend'] = decomposition.trend
        if 'seasonal' in self.include_components:
            result[f'{self.target_col}_seasonal'] = decomposition.seasonal
        if 'residual' in self.include_components:
            result[f'{self.target_col}_residual'] = decomposition.resid
        
        return result.dropna()
    
    def _detect_period(self, series: pd.Series) -> int:
        """Attempt to detect seasonal period using autocorrelation."""
        try:
            from statsmodels.tsa.stattools import acf
            
            # Compute ACF
            n = min(len(series), 365)
            acf_vals = acf(series.dropna(), nlags=n // 2)
            
            # Find first significant peak after lag 1
            for i in range(2, len(acf_vals)):
                if acf_vals[i] > acf_vals[i-1] and acf_vals[i] > acf_vals[i+1] if i+1 < len(acf_vals) else True:
                    if acf_vals[i] > 0.1:  # Threshold for significance
                        return i
            
            return 7  # Default to weekly
        except Exception:
            return 7


class DifferencingTransformer(BaseEstimator, TransformerMixin):
    """
    Apply differencing to make time series stationary.
    
    Parameters
    ----------
    target_col : str
        Column to difference.
    order : int, default=1
        Order of differencing.
    seasonal_order : int, default=0
        Seasonal differencing order.
    seasonal_period : int, default=1
        Period for seasonal differencing.
    drop_na : bool, default=True
        Whether to drop NaN rows.
    """
    
    def __init__(
        self,
        target_col: str,
        order: int = 1,
        seasonal_order: int = 0,
        seasonal_period: int = 1,
        drop_na: bool = True,
    ):
        self.target_col = target_col
        self.order = order
        self.seasonal_order = seasonal_order
        self.seasonal_period = seasonal_period
        self.drop_na = drop_na
        self._first_values: List[float] = []
    
    def fit(self, X: pd.DataFrame, y=None) -> "DifferencingTransformer":
        """Store first values for inverse transform."""
        series = X[self.target_col]
        
        # Store values needed for inverse
        self._first_values = []
        current = series.values.copy()
        
        for _ in range(self.order):
            self._first_values.append(current[0])
            current = np.diff(current)
        
        return self
    
    def transform(self, X: pd.DataFrame) -> pd.DataFrame:
        """Apply differencing."""
        result = X.copy()
        series = result[self.target_col]
        
        # Regular differencing
        for i in range(self.order):
            series = series.diff()
        
        # Seasonal differencing
        for i in range(self.seasonal_order):
            series = series.diff(self.seasonal_period)
        
        result[f'{self.target_col}_diff'] = series
        
        if self.drop_na:
            result = result.dropna()
        
        return result
    
    def inverse_transform(self, X: pd.DataFrame) -> pd.DataFrame:
        """Reverse the differencing (for forecasts)."""
        result = X.copy()
        diff_col = f'{self.target_col}_diff'
        
        if diff_col not in result.columns:
            return result
        
        series = result[diff_col].values
        
        # Reverse regular differencing
        for i in range(self.order - 1, -1, -1):
            series = np.concatenate([[self._first_values[i]], series]).cumsum()[1:]
        
        result[self.target_col] = series
        
        return result


class TimeSeriesFeatureUnion(BaseEstimator, TransformerMixin):
    """
    Combine multiple time series transformers.
    
    Similar to sklearn's FeatureUnion but handles the index alignment
    issues that arise with time series transformations.
    
    Parameters
    ----------
    transformers : list of (name, transformer) tuples
        List of transformer objects to apply.
    drop_na : bool, default=True
        Drop rows with any NaN after all transformations.
    """
    
    def __init__(
        self,
        transformers: List[Tuple[str, BaseEstimator]],
        drop_na: bool = True,
    ):
        self.transformers = transformers
        self.drop_na = drop_na
    
    def fit(self, X: pd.DataFrame, y=None) -> "TimeSeriesFeatureUnion":
        """Fit all transformers."""
        for name, transformer in self.transformers:
            transformer.fit(X, y)
        return self
    
    def transform(self, X: pd.DataFrame) -> pd.DataFrame:
        """Apply all transformers and concatenate results."""
        if isinstance(X, np.ndarray):
            X = pd.DataFrame(X)
        
        result = X.copy()
        
        for name, transformer in self.transformers:
            transformed = transformer.transform(X)
            
            # Get new columns only
            new_cols = [c for c in transformed.columns if c not in result.columns]
            
            # Align indices
            result = result.join(transformed[new_cols], how='left')
        
        if self.drop_na:
            result = result.dropna()
        
        return result
    
    def fit_transform(self, X: pd.DataFrame, y=None) -> pd.DataFrame:
        """Fit and transform."""
        return self.fit(X, y).transform(X)
    
    def get_feature_names_out(self, input_features=None) -> List[str]:
        """Get all output feature names."""
        names = []
        for name, transformer in self.transformers:
            if hasattr(transformer, 'get_feature_names_out'):
                names.extend(transformer.get_feature_names_out())
        return names
    
    @property
    def min_samples_required(self) -> int:
        """Minimum samples required across all transformers."""
        return max(
            getattr(t, 'min_samples_required', 1)
            for _, t in self.transformers
        )


def create_ts_feature_pipeline(
    target_col: str,
    datetime_col: Optional[str] = None,
    lags: List[int] = [1, 7, 14],
    rolling_windows: List[int] = [7, 14, 30],
    rolling_functions: List[str] = ['mean', 'std'],
    include_datetime: bool = True,
    include_decomposition: bool = False,
    seasonal_period: Optional[int] = None,
) -> TimeSeriesFeatureUnion:
    """
    Create a standard time series feature engineering pipeline.
    
    Parameters
    ----------
    target_col : str
        Target column for lag/rolling features.
    datetime_col : str, optional
        Datetime column for calendar features.
    lags : list of int
        Lag values to create.
    rolling_windows : list of int
        Window sizes for rolling stats.
    rolling_functions : list of str
        Rolling functions to apply.
    include_datetime : bool
        Whether to include datetime features.
    include_decomposition : bool
        Whether to include seasonal decomposition.
    seasonal_period : int, optional
        Period for decomposition.
    
    Returns
    -------
    TimeSeriesFeatureUnion
        Configured feature pipeline.
    """
    transformers = []
    
    # Lag features
    if lags:
        transformers.append((
            'lags',
            LagFeatureTransformer(lags=lags, target_col=target_col, drop_na=False)
        ))
    
    # Rolling features
    if rolling_windows:
        transformers.append((
            'rolling',
            RollingWindowTransformer(
                windows=rolling_windows,
                functions=rolling_functions,
                target_col=target_col,
                drop_na=False
            )
        ))
    
    # DateTime features
    if include_datetime and datetime_col:
        transformers.append((
            'datetime',
            DateTimeFeatureTransformer(datetime_col=datetime_col, drop_original=False)
        ))
    
    # Seasonal decomposition
    if include_decomposition:
        transformers.append((
            'decomp',
            SeasonalDecomposer(target_col=target_col, period=seasonal_period)
        ))
    
    # Don't drop NaN here - let the caller handle NaN removal with y alignment
    return TimeSeriesFeatureUnion(transformers=transformers, drop_na=False)

