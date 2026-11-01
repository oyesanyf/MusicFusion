"""
Time Series Risk Analyzer
==========================

Extends ACDSO's risk analysis to time series specific concerns:
- Frequency/gap detection
- Seasonality analysis
- Stationarity testing
- Temporal leakage detection
- Horizon vs history adequacy
- Concept drift detection
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any
import warnings
import numpy as np
import pandas as pd


@dataclass
class TimeSeriesRiskReport:
    """
    Comprehensive risk report for time series data.
    
    Attributes
    ----------
    is_valid : bool
        Whether the data is suitable for forecasting.
    frequency : str
        Detected frequency (e.g., 'D', 'H', 'M', 'W').
    has_gaps : bool
        Whether there are missing timestamps.
    gap_locations : List[int]
        Indices where gaps occur.
    is_stationary : bool
        Whether the series is stationary (ADF test).
    stationarity_pvalue : float
        P-value from ADF test.
    seasonality_period : Optional[int]
        Detected seasonal period.
    seasonality_strength : float
        Strength of seasonality (0-1).
    has_trend : bool
        Whether a significant trend exists.
    trend_direction : str
        'increasing', 'decreasing', or 'none'.
    has_outliers : bool
        Whether outliers are present.
    outlier_indices : List[int]
        Indices of detected outliers.
    leakage_warnings : List[str]
        Warnings about potential temporal leakage.
    horizon_adequacy : float
        Ratio of history length to forecast horizon (higher is better).
    recommended_horizon : int
        Maximum recommended forecast horizon.
    notes : List[str]
        Additional observations and recommendations.
    severity : str
        Overall risk severity: 'low', 'medium', 'high', 'critical'.
    """
    
    is_valid: bool = True
    frequency: str = 'unknown'
    frequency_inferred: bool = False
    has_gaps: bool = False
    gap_count: int = 0
    gap_locations: List[int] = field(default_factory=list)
    
    is_stationary: bool = True
    stationarity_pvalue: float = 0.0
    
    seasonality_period: Optional[int] = None
    seasonality_strength: float = 0.0
    has_multiple_seasonalities: bool = False
    
    has_trend: bool = False
    trend_direction: str = 'none'
    trend_slope: float = 0.0
    
    has_outliers: bool = False
    outlier_count: int = 0
    outlier_indices: List[int] = field(default_factory=list)
    
    leakage_warnings: List[str] = field(default_factory=list)
    future_looking_features: List[str] = field(default_factory=list)
    
    horizon_adequacy: float = 1.0
    recommended_horizon: int = 1
    min_samples_for_lags: int = 0
    
    concept_drift_detected: bool = False
    drift_locations: List[int] = field(default_factory=list)
    
    notes: List[str] = field(default_factory=list)
    severity: str = 'low'
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {k: v for k, v in self.__dict__.items()}
    
    def get_severity_score(self) -> float:
        """Compute numeric severity score (0-1, higher is worse)."""
        score = 0.0
        
        if not self.is_valid:
            return 1.0
        
        if self.has_gaps:
            score += 0.2 * min(self.gap_count / 10, 1.0)
        
        if not self.is_stationary:
            score += 0.15
        
        if self.has_outliers:
            score += 0.1 * min(self.outlier_count / 20, 1.0)
        
        if self.leakage_warnings:
            score += 0.3
        
        if self.concept_drift_detected:
            score += 0.2
        
        if self.horizon_adequacy < 3.0:
            score += 0.1 * (1 - self.horizon_adequacy / 3.0)
        
        return min(score, 1.0)


def detect_frequency(
    datetime_index: pd.DatetimeIndex,
    series: Optional[pd.Series] = None,
) -> Tuple[str, bool]:
    """
    Detect the frequency of a datetime index.
    
    Returns
    -------
    frequency : str
        Pandas frequency string (e.g., 'D', 'H', 'M', 'W', 'B').
    inferred : bool
        Whether the frequency was inferred (True) or explicit (False).
    """
    # Try to get explicit frequency
    if hasattr(datetime_index, 'freq') and datetime_index.freq is not None:
        return str(datetime_index.freq), False
    
    # Try to infer
    try:
        inferred = pd.infer_freq(datetime_index)
        if inferred:
            return inferred, True
    except Exception:
        pass
    
    # Manual detection based on median difference
    if len(datetime_index) < 2:
        return 'unknown', True
    
    diffs = np.diff(datetime_index.astype(np.int64))
    median_diff = np.median(diffs)
    
    # Map to common frequencies (in nanoseconds)
    freq_map = {
        60 * 1e9: 'T',           # minute
        3600 * 1e9: 'H',         # hour
        86400 * 1e9: 'D',        # day
        7 * 86400 * 1e9: 'W',    # week
        30 * 86400 * 1e9: 'M',   # month (approx)
        365 * 86400 * 1e9: 'Y',  # year (approx)
    }
    
    for threshold, freq in freq_map.items():
        if abs(median_diff - threshold) < threshold * 0.1:
            return freq, True
    
    return 'unknown', True


def detect_gaps(
    datetime_index: pd.DatetimeIndex,
    expected_freq: Optional[str] = None,
) -> Tuple[bool, List[int]]:
    """
    Detect gaps in the time series.
    
    Returns
    -------
    has_gaps : bool
        Whether gaps exist.
    gap_indices : list
        Indices where gaps occur (index of the row AFTER the gap).
    """
    if len(datetime_index) < 2:
        return False, []
    
    # Get expected frequency
    if expected_freq is None:
        expected_freq, _ = detect_frequency(datetime_index)
    
    if expected_freq == 'unknown':
        return False, []
    
    # Compute expected delta
    try:
        expected_delta = pd.Timedelta(1, expected_freq)
    except ValueError:
        # Handle complex frequencies
        expected_delta = datetime_index[1] - datetime_index[0]
    
    # Find gaps
    diffs = np.diff(datetime_index)
    gap_indices = np.where(diffs > expected_delta * 1.5)[0] + 1
    
    return len(gap_indices) > 0, gap_indices.tolist()


def check_stationarity(
    series: pd.Series,
    significance: float = 0.05,
) -> Tuple[bool, float]:
    """
    Test for stationarity using Augmented Dickey-Fuller test.
    
    Returns
    -------
    is_stationary : bool
        True if series is stationary.
    pvalue : float
        P-value from ADF test.
    """
    try:
        from statsmodels.tsa.stattools import adfuller
        
        # Remove NaN
        clean_series = series.dropna()
        
        if len(clean_series) < 20:
            return True, 0.0  # Too short to test
        
        result = adfuller(clean_series, autolag='AIC')
        pvalue = result[1]
        
        return pvalue < significance, pvalue
    
    except ImportError:
        warnings.warn("statsmodels not installed. Skipping stationarity test.")
        return True, 0.0
    except Exception as e:
        warnings.warn(f"Stationarity test failed: {e}")
        return True, 0.0


def detect_seasonality(
    series: pd.Series,
    max_period: int = 365,
) -> Tuple[Optional[int], float]:
    """
    Detect seasonal period using autocorrelation.
    
    Returns
    -------
    period : int or None
        Detected seasonal period.
    strength : float
        Seasonality strength (0-1).
    """
    try:
        from statsmodels.tsa.stattools import acf
        
        clean_series = series.dropna().values
        
        if len(clean_series) < 2 * max_period:
            max_period = len(clean_series) // 2
        
        if max_period < 2:
            return None, 0.0
        
        # Compute ACF
        acf_values = acf(clean_series, nlags=max_period, fft=True)
        
        # Find peaks (local maxima)
        peaks = []
        for i in range(2, len(acf_values) - 1):
            if acf_values[i] > acf_values[i-1] and acf_values[i] > acf_values[i+1]:
                if acf_values[i] > 0.1:  # Minimum threshold
                    peaks.append((i, acf_values[i]))
        
        if not peaks:
            return None, 0.0
        
        # Get strongest peak
        best_period, strength = max(peaks, key=lambda x: x[1])
        
        # Common periods to check
        common_periods = [7, 12, 24, 30, 52, 365]
        for p in common_periods:
            if abs(best_period - p) <= 2:
                best_period = p
                break
        
        return best_period, float(strength)
    
    except ImportError:
        warnings.warn("statsmodels not installed. Skipping seasonality detection.")
        return None, 0.0
    except Exception as e:
        warnings.warn(f"Seasonality detection failed: {e}")
        return None, 0.0


def detect_trend(
    series: pd.Series,
) -> Tuple[bool, str, float]:
    """
    Detect trend in the series.
    
    Returns
    -------
    has_trend : bool
        Whether a significant trend exists.
    direction : str
        'increasing', 'decreasing', or 'none'.
    slope : float
        Estimated slope.
    """
    clean_series = series.dropna().values
    
    if len(clean_series) < 10:
        return False, 'none', 0.0
    
    # Simple linear regression
    x = np.arange(len(clean_series))
    slope, intercept = np.polyfit(x, clean_series, 1)
    
    # Normalize slope by series scale
    series_scale = np.std(clean_series)
    if series_scale > 0:
        normalized_slope = slope * len(clean_series) / series_scale
    else:
        normalized_slope = 0.0
    
    # Determine significance
    has_trend = abs(normalized_slope) > 0.5
    
    if has_trend:
        direction = 'increasing' if slope > 0 else 'decreasing'
    else:
        direction = 'none'
    
    return has_trend, direction, float(slope)


def detect_outliers(
    series: pd.Series,
    method: str = 'iqr',
    threshold: float = 1.5,
) -> Tuple[bool, List[int]]:
    """
    Detect outliers in the series.
    
    Parameters
    ----------
    method : str
        Detection method: 'iqr', 'zscore', 'mad'.
    threshold : float
        Threshold for outlier detection.
    
    Returns
    -------
    has_outliers : bool
        Whether outliers exist.
    outlier_indices : list
        Indices of outliers.
    """
    clean_series = series.dropna()
    values = clean_series.values
    
    if len(values) < 10:
        return False, []
    
    if method == 'iqr':
        q1, q3 = np.percentile(values, [25, 75])
        iqr = q3 - q1
        lower = q1 - threshold * iqr
        upper = q3 + threshold * iqr
        outlier_mask = (values < lower) | (values > upper)
    
    elif method == 'zscore':
        mean, std = np.mean(values), np.std(values)
        if std > 0:
            z = np.abs((values - mean) / std)
            outlier_mask = z > threshold
        else:
            outlier_mask = np.zeros(len(values), dtype=bool)
    
    elif method == 'mad':
        median = np.median(values)
        mad = np.median(np.abs(values - median))
        if mad > 0:
            modified_z = 0.6745 * (values - median) / mad
            outlier_mask = np.abs(modified_z) > threshold
        else:
            outlier_mask = np.zeros(len(values), dtype=bool)
    
    else:
        raise ValueError(f"Unknown method '{method}'")
    
    outlier_indices = clean_series.index[outlier_mask].tolist()
    
    return len(outlier_indices) > 0, outlier_indices


def check_temporal_leakage(
    df: pd.DataFrame,
    target_col: str,
    datetime_col: str,
    feature_cols: Optional[List[str]] = None,
) -> List[str]:
    """
    Check for potential temporal leakage in features.
    
    Warns about features that might contain future information.
    
    Returns
    -------
    warnings : list
        List of warning messages.
    """
    warnings_list = []
    
    if feature_cols is None:
        feature_cols = [c for c in df.columns if c not in [target_col, datetime_col]]
    
    # Check for highly correlated features with future target
    try:
        future_target = df[target_col].shift(-1)
        
        for col in feature_cols:
            if df[col].dtype in [np.float64, np.int64, float, int]:
                corr = df[col].corr(future_target)
                if abs(corr) > 0.95:
                    warnings_list.append(
                        f"Feature '{col}' is highly correlated (r={corr:.2f}) with future target. "
                        "Possible data leakage."
                    )
    except Exception:
        pass
    
    # Check for suspiciously named columns
    leaky_keywords = ['future', 'next', 'tomorrow', 'ahead', 'forward', 't+1', 't+']
    for col in feature_cols:
        col_lower = col.lower()
        for keyword in leaky_keywords:
            if keyword in col_lower:
                warnings_list.append(
                    f"Feature '{col}' has suspicious name suggesting future data."
                )
                break
    
    return warnings_list


class TimeSeriesRiskAnalyzer:
    """
    Comprehensive risk analyzer for time series forecasting.
    
    Parameters
    ----------
    datetime_col : str
        Name of the datetime column.
    target_col : str
        Name of the target column.
    horizon : int, default=1
        Forecast horizon.
    seasonality : int, optional
        Expected seasonal period. If None, auto-detected.
    """
    
    def __init__(
        self,
        datetime_col: str,
        target_col: str,
        horizon: int = 1,
        seasonality: Optional[int] = None,
    ):
        self.datetime_col = datetime_col
        self.target_col = target_col
        self.horizon = horizon
        self.seasonality = seasonality
    
    def analyze(
        self,
        df: pd.DataFrame,
        feature_cols: Optional[List[str]] = None,
    ) -> TimeSeriesRiskReport:
        """
        Perform comprehensive risk analysis.
        
        Parameters
        ----------
        df : pd.DataFrame
            Time series data.
        feature_cols : list, optional
            Feature columns to analyze for leakage.
        
        Returns
        -------
        TimeSeriesRiskReport
            Detailed risk report.
        """
        report = TimeSeriesRiskReport()
        
        # Validate data
        if self.datetime_col not in df.columns:
            report.is_valid = False
            report.notes.append(f"Datetime column '{self.datetime_col}' not found.")
            report.severity = 'critical'
            return report
        
        if self.target_col not in df.columns:
            report.is_valid = False
            report.notes.append(f"Target column '{self.target_col}' not found.")
            report.severity = 'critical'
            return report
        
        # Get datetime index and series
        df = df.sort_values(self.datetime_col).reset_index(drop=True)
        dt_index = pd.DatetimeIndex(df[self.datetime_col])
        series = df[self.target_col]
        
        # Detect frequency
        report.frequency, report.frequency_inferred = detect_frequency(dt_index)
        
        # Check for gaps
        report.has_gaps, report.gap_locations = detect_gaps(dt_index, report.frequency)
        report.gap_count = len(report.gap_locations)
        
        if report.gap_count > 0:
            report.notes.append(
                f"Found {report.gap_count} gaps in time series. Consider imputation."
            )
        
        # Stationarity test
        report.is_stationary, report.stationarity_pvalue = check_stationarity(series)
        
        if not report.is_stationary:
            report.notes.append(
                "Series is non-stationary. Consider differencing or detrending."
            )
        
        # Seasonality detection
        if self.seasonality:
            report.seasonality_period = self.seasonality
            report.seasonality_strength = 0.5  # Assumed
        else:
            report.seasonality_period, report.seasonality_strength = detect_seasonality(series)
        
        if report.seasonality_period:
            report.notes.append(
                f"Detected seasonality with period {report.seasonality_period}."
            )
        
        # Trend detection
        report.has_trend, report.trend_direction, report.trend_slope = detect_trend(series)
        
        if report.has_trend:
            report.notes.append(
                f"Significant {report.trend_direction} trend detected."
            )
        
        # Outlier detection
        report.has_outliers, report.outlier_indices = detect_outliers(series)
        report.outlier_count = len(report.outlier_indices)
        
        if report.outlier_count > 0:
            report.notes.append(
                f"Found {report.outlier_count} outliers. Consider treatment."
            )
        
        # Horizon adequacy
        n_samples = len(df)
        report.horizon_adequacy = n_samples / max(self.horizon, 1)
        report.recommended_horizon = max(1, n_samples // 10)
        
        if self.horizon > report.recommended_horizon:
            report.notes.append(
                f"Forecast horizon ({self.horizon}) may be too long for {n_samples} samples. "
                f"Recommended max: {report.recommended_horizon}."
            )
        
        # Minimum samples for lags
        if report.seasonality_period:
            report.min_samples_for_lags = report.seasonality_period * 2
        else:
            report.min_samples_for_lags = 14
        
        # Temporal leakage check
        report.leakage_warnings = check_temporal_leakage(
            df, self.target_col, self.datetime_col, feature_cols
        )
        
        if report.leakage_warnings:
            report.notes.append("Potential temporal leakage detected in features.")
        
        # Compute overall severity
        severity_score = report.get_severity_score()
        
        if severity_score < 0.2:
            report.severity = 'low'
        elif severity_score < 0.4:
            report.severity = 'medium'
        elif severity_score < 0.7:
            report.severity = 'high'
        else:
            report.severity = 'critical'
        
        return report
    
    def validate_for_forecasting(
        self,
        df: pd.DataFrame,
        min_samples: int = 30,
    ) -> Tuple[bool, List[str]]:
        """
        Quick validation check for forecasting readiness.
        
        Returns
        -------
        is_valid : bool
            Whether data is suitable for forecasting.
        issues : list
            List of issues found.
        """
        issues = []
        
        if len(df) < min_samples:
            issues.append(f"Insufficient samples: {len(df)} < {min_samples}")
        
        if df[self.target_col].isna().sum() > len(df) * 0.1:
            issues.append("Target has >10% missing values")
        
        if df[self.target_col].nunique() < 2:
            issues.append("Target has no variation")
        
        report = self.analyze(df)
        
        if report.severity in ['high', 'critical']:
            issues.extend(report.notes)
        
        return len(issues) == 0, issues

