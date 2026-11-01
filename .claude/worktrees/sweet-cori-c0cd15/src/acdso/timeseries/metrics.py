"""
Time Series Forecasting Metrics
================================

Specialized metrics for evaluating time series forecasts:
- MAE: Mean Absolute Error
- RMSE: Root Mean Squared Error
- MAPE: Mean Absolute Percentage Error
- sMAPE: Symmetric MAPE
- MASE: Mean Absolute Scaled Error (excellent for benchmarking)
- WAPE: Weighted Absolute Percentage Error
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Dict, Optional, Union
import numpy as np
import pandas as pd
from sklearn.metrics import make_scorer


def mae(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    """
    Mean Absolute Error.
    
    MAE = (1/n) * sum(|y_true - y_pred|)
    
    Interpretation: Average absolute deviation from actual values.
    Lower is better. Same scale as target variable.
    """
    y_true, y_pred = np.asarray(y_true), np.asarray(y_pred)
    return float(np.mean(np.abs(y_true - y_pred)))


def rmse(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    """
    Root Mean Squared Error.
    
    RMSE = sqrt((1/n) * sum((y_true - y_pred)^2))
    
    Interpretation: Penalizes large errors more than MAE.
    Lower is better. Same scale as target variable.
    """
    y_true, y_pred = np.asarray(y_true), np.asarray(y_pred)
    return float(np.sqrt(np.mean((y_true - y_pred) ** 2)))


def mape(y_true: np.ndarray, y_pred: np.ndarray, epsilon: float = 1e-10) -> float:
    """
    Mean Absolute Percentage Error.
    
    MAPE = (100/n) * sum(|y_true - y_pred| / |y_true|)
    
    Interpretation: Average percentage error. Scale-independent.
    Lower is better. Can be undefined when y_true contains zeros.
    
    Parameters
    ----------
    epsilon : float
        Small value added to denominator to avoid division by zero.
    
    Returns
    -------
    float
        MAPE as a percentage (0-100+)
    """
    y_true, y_pred = np.asarray(y_true), np.asarray(y_pred)
    
    # Handle zeros in y_true
    mask = np.abs(y_true) > epsilon
    if not np.any(mask):
        return np.inf
    
    return float(100.0 * np.mean(np.abs((y_true[mask] - y_pred[mask]) / (np.abs(y_true[mask]) + epsilon))))


def smape(y_true: np.ndarray, y_pred: np.ndarray, epsilon: float = 1e-10) -> float:
    """
    Symmetric Mean Absolute Percentage Error.
    
    sMAPE = (100/n) * sum(2 * |y_true - y_pred| / (|y_true| + |y_pred|))
    
    Interpretation: Symmetric version of MAPE. Bounded [0, 200].
    Lower is better. More robust to zeros than MAPE.
    """
    y_true, y_pred = np.asarray(y_true), np.asarray(y_pred)
    
    numerator = 2.0 * np.abs(y_true - y_pred)
    denominator = np.abs(y_true) + np.abs(y_pred) + epsilon
    
    return float(100.0 * np.mean(numerator / denominator))


def mase(
    y_true: np.ndarray,
    y_pred: np.ndarray,
    y_train: Optional[np.ndarray] = None,
    seasonality: int = 1,
) -> float:
    """
    Mean Absolute Scaled Error.
    
    MASE = MAE / MAE_naive
    
    Where MAE_naive is the MAE of the naive seasonal forecast on training data.
    
    Interpretation:
    - MASE < 1: Model is better than naive baseline
    - MASE = 1: Model is equivalent to naive baseline
    - MASE > 1: Model is worse than naive baseline
    
    This is the recommended metric for comparing forecasting methods
    as it's scale-independent and interpretable.
    
    Parameters
    ----------
    y_true : array
        Actual test values.
    y_pred : array
        Predicted values.
    y_train : array, optional
        Training series for computing naive baseline MAE.
        If None, uses y_true for in-sample MASE.
    seasonality : int, default=1
        Seasonal period for naive forecast (1 = non-seasonal naive).
    
    Returns
    -------
    float
        MASE value.
    """
    y_true, y_pred = np.asarray(y_true), np.asarray(y_pred)
    
    # Compute forecast MAE
    forecast_mae = mae(y_true, y_pred)
    
    # Compute naive baseline MAE
    if y_train is not None:
        y_train = np.asarray(y_train)
        naive_errors = np.abs(y_train[seasonality:] - y_train[:-seasonality])
    else:
        # In-sample MASE: use y_true
        naive_errors = np.abs(y_true[seasonality:] - y_true[:-seasonality])
    
    naive_mae = np.mean(naive_errors)
    
    if naive_mae < 1e-10:
        return np.inf
    
    return float(forecast_mae / naive_mae)


def wape(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    """
    Weighted Absolute Percentage Error.
    
    WAPE = sum(|y_true - y_pred|) / sum(|y_true|) * 100
    
    Interpretation: Total absolute error as percentage of total actual values.
    Better for intermittent demand with many zeros.
    """
    y_true, y_pred = np.asarray(y_true), np.asarray(y_pred)
    
    total_actual = np.sum(np.abs(y_true))
    if total_actual < 1e-10:
        return np.inf
    
    return float(100.0 * np.sum(np.abs(y_true - y_pred)) / total_actual)


def coverage(
    y_true: np.ndarray,
    y_pred_lower: np.ndarray,
    y_pred_upper: np.ndarray,
) -> float:
    """
    Prediction Interval Coverage.
    
    Measures what fraction of actual values fall within the prediction interval.
    
    Interpretation: Should match the confidence level (e.g., 95% interval
    should have ~95% coverage). Under-coverage means intervals are too narrow.
    """
    y_true = np.asarray(y_true)
    y_pred_lower = np.asarray(y_pred_lower)
    y_pred_upper = np.asarray(y_pred_upper)
    
    in_interval = (y_true >= y_pred_lower) & (y_true <= y_pred_upper)
    return float(np.mean(in_interval))


def pinball_loss(y_true: np.ndarray, y_pred: np.ndarray, quantile: float = 0.5) -> float:
    """
    Pinball (Quantile) Loss.
    
    Used for evaluating quantile forecasts.
    
    Parameters
    ----------
    quantile : float
        Quantile being evaluated (0-1).
    """
    y_true, y_pred = np.asarray(y_true), np.asarray(y_pred)
    
    error = y_true - y_pred
    loss = np.where(error >= 0, quantile * error, (quantile - 1) * error)
    
    return float(np.mean(loss))


@dataclass
class TimeSeriesScorer:
    """
    Scorer wrapper for time series metrics.
    
    Compatible with sklearn's cross-validation framework.
    
    Parameters
    ----------
    metric : str or callable
        Metric name ('mae', 'rmse', 'mape', 'smape', 'mase') or custom function.
    greater_is_better : bool
        Whether higher scores are better.
    y_train : array, optional
        Training data for MASE calculation.
    seasonality : int
        Seasonal period for MASE.
    """
    
    metric: Union[str, Callable]
    greater_is_better: bool = False
    y_train: Optional[np.ndarray] = None
    seasonality: int = 1
    
    METRICS: Dict[str, Callable] = None
    
    def __post_init__(self):
        self.METRICS = {
            'mae': mae,
            'rmse': rmse,
            'mape': mape,
            'smape': smape,
            'mase': lambda y_t, y_p: mase(y_t, y_p, self.y_train, self.seasonality),
            'wape': wape,
        }
    
    def __call__(self, y_true: np.ndarray, y_pred: np.ndarray) -> float:
        if callable(self.metric):
            return self.metric(y_true, y_pred)
        
        if self.metric not in self.METRICS:
            raise ValueError(f"Unknown metric '{self.metric}'. Available: {list(self.METRICS.keys())}")
        
        return self.METRICS[self.metric](y_true, y_pred)
    
    def get_sklearn_scorer(self):
        """Get an sklearn-compatible scorer."""
        sign = 1 if self.greater_is_better else -1
        return make_scorer(self, greater_is_better=self.greater_is_better)


def get_ts_scorer(
    metric: str = 'mae',
    y_train: Optional[np.ndarray] = None,
    seasonality: int = 1,
) -> TimeSeriesScorer:
    """
    Factory function to create a time series scorer.
    
    Parameters
    ----------
    metric : str
        One of: 'mae', 'rmse', 'mape', 'smape', 'mase', 'wape'
    y_train : array, optional
        Training data (required for MASE).
    seasonality : int
        Seasonal period for MASE calculation.
    
    Returns
    -------
    TimeSeriesScorer
        Configured scorer instance.
    """
    return TimeSeriesScorer(
        metric=metric,
        greater_is_better=False,  # All forecast metrics are "lower is better"
        y_train=y_train,
        seasonality=seasonality,
    )


def evaluate_forecast(
    y_true: np.ndarray,
    y_pred: np.ndarray,
    y_train: Optional[np.ndarray] = None,
    seasonality: int = 1,
    metrics: Optional[list] = None,
) -> Dict[str, float]:
    """
    Evaluate a forecast using multiple metrics.
    
    Parameters
    ----------
    y_true : array
        Actual values.
    y_pred : array
        Predicted values.
    y_train : array, optional
        Training data for MASE.
    seasonality : int
        Seasonal period.
    metrics : list, optional
        Metrics to compute. Default: all available.
    
    Returns
    -------
    dict
        Dictionary of metric names to values.
    """
    metrics = metrics or ['mae', 'rmse', 'mape', 'smape', 'mase', 'wape']
    
    results = {}
    for m in metrics:
        try:
            scorer = get_ts_scorer(m, y_train=y_train, seasonality=seasonality)
            results[m] = scorer(y_true, y_pred)
        except Exception as e:
            results[m] = np.nan
    
    return results


def compare_forecasts(
    y_true: np.ndarray,
    forecasts: Dict[str, np.ndarray],
    y_train: Optional[np.ndarray] = None,
    seasonality: int = 1,
) -> pd.DataFrame:
    """
    Compare multiple forecasts using all metrics.
    
    Parameters
    ----------
    y_true : array
        Actual values.
    forecasts : dict
        Dictionary mapping model name to predictions.
    y_train : array, optional
        Training data for MASE.
    seasonality : int
        Seasonal period.
    
    Returns
    -------
    pd.DataFrame
        Comparison table with models as rows, metrics as columns.
    """
    results = []
    
    for name, y_pred in forecasts.items():
        metrics = evaluate_forecast(y_true, y_pred, y_train, seasonality)
        metrics['model'] = name
        results.append(metrics)
    
    df = pd.DataFrame(results).set_index('model')
    
    # Add rank columns
    for col in df.columns:
        df[f'{col}_rank'] = df[col].rank(method='min')
    
    return df

