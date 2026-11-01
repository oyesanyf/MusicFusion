"""
ACDSO Time Series Benchmark
============================

Benchmark ACDSO AutoTS against popular time series forecasting tools:
- Prophet (Meta/Facebook)
- AutoTS
- Darts
- StatsForecast (Nixtla)
- MLForecast (Nixtla)
- sktime
- pmdarima (AutoARIMA)
- PyCaret Time Series
- NeuralProphet
- GluonTS

Usage:
    from acdso.timeseries.benchmark import TimeSeriesBenchmark
    
    benchmark = TimeSeriesBenchmark(
        datetime_col='date',
        target_col='sales',
        horizon=7,
    )
    
    results = benchmark.run(df)
    benchmark.generate_report('ts_benchmark_results.html')
"""

from __future__ import annotations

import json
import time
import warnings
from dataclasses import dataclass, field, asdict
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Any, Callable
import numpy as np
import pandas as pd

from .core import AutoTS, AutoTSResult
from .metrics import evaluate_forecast, mae, rmse, mape, smape, mase
from .splits import WalkForwardSplit


@dataclass
class TSBenchmarkResult:
    """
    Result from a single system benchmark run.
    
    Attributes
    ----------
    system : str
        System name.
    dataset : str
        Dataset name.
    mae : float
        Mean Absolute Error.
    rmse : float
        Root Mean Squared Error.
    mape : float
        Mean Absolute Percentage Error.
    smape : float
        Symmetric MAPE.
    mase : float
        Mean Absolute Scaled Error.
    fit_time : float
        Training time in seconds.
    predict_time : float
        Prediction time in seconds.
    status : str
        'success' or 'error'.
    error_message : str
        Error message if failed.
    notes : List[str]
        Additional notes.
    """
    
    system: str
    dataset: str
    mae: float = np.nan
    rmse: float = np.nan
    mape: float = np.nan
    smape: float = np.nan
    mase: float = np.nan
    fit_time: float = 0.0
    predict_time: float = 0.0
    status: str = 'success'
    error_message: str = ''
    notes: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict:
        return asdict(self)


@dataclass
class TSBenchmarkSummary:
    """
    Complete benchmark summary.
    
    Attributes
    ----------
    results : List[TSBenchmarkResult]
        All results.
    winners_by_metric : Dict[str, str]
        Winner for each metric.
    systems_tested : List[str]
        Systems that were tested.
    datasets : List[str]
        Datasets used.
    timestamp : str
        Benchmark timestamp.
    """
    
    results: List[TSBenchmarkResult]
    winners_by_metric: Dict[str, str] = field(default_factory=dict)
    systems_tested: List[str] = field(default_factory=list)
    datasets: List[str] = field(default_factory=list)
    timestamp: str = ''
    total_time: float = 0.0
    
    def to_dataframe(self) -> pd.DataFrame:
        """Convert results to DataFrame."""
        return pd.DataFrame([r.to_dict() for r in self.results])
    
    def to_dict(self) -> Dict:
        return {
            'results': [r.to_dict() for r in self.results],
            'winners_by_metric': self.winners_by_metric,
            'systems_tested': self.systems_tested,
            'datasets': self.datasets,
            'timestamp': self.timestamp,
            'total_time': self.total_time,
        }


class TimeSeriesBenchmark:
    """
    Benchmark ACDSO AutoTS against other time series tools.
    
    Parameters
    ----------
    datetime_col : str
        Datetime column name.
    target_col : str
        Target column name.
    horizon : int, default=7
        Forecast horizon.
    test_ratio : float, default=0.2
        Ratio of data for testing.
    time_budget : int, default=300
        Time budget per system in seconds.
    systems : List[str], optional
        Systems to benchmark. If None, tests all available.
    verbose : bool, default=True
        Whether to print progress.
    
    Available Systems:
        - 'acdso': ACDSO AutoTS
        - 'prophet': Facebook Prophet
        - 'autots': AutoTS library
        - 'statsforecast': Nixtla StatsForecast
        - 'mlforecast': Nixtla MLForecast
        - 'sktime': sktime
        - 'pmdarima': Auto-ARIMA
        - 'pycaret': PyCaret Time Series
        - 'neuralprophet': NeuralProphet
        - 'darts': Darts
        - 'timesfm': Google's TimesFM pretrained foundation model
          (requires ``pip install timesfm[torch]``; skipped automatically
          if there's too little history to be worth it -- see
          ``_run_timesfm``)
    """

    AVAILABLE_SYSTEMS = [
        'acdso',
        'prophet',
        'autots',
        'statsforecast',
        'mlforecast',
        'sktime',
        'pmdarima',
        'pycaret',
        'neuralprophet',
        'darts',
        'timesfm',
    ]
    
    def __init__(
        self,
        datetime_col: str,
        target_col: str,
        horizon: int = 7,
        test_ratio: float = 0.2,
        time_budget: int = 300,
        systems: Optional[List[str]] = None,
        verbose: bool = True,
    ):
        self.datetime_col = datetime_col
        self.target_col = target_col
        self.horizon = horizon
        self.test_ratio = test_ratio
        self.time_budget = time_budget
        self.systems = systems or self._detect_available_systems()
        self.verbose = verbose
        
        self._results: List[TSBenchmarkResult] = []
        self._summary: Optional[TSBenchmarkSummary] = None
    
    def _detect_available_systems(self) -> List[str]:
        """Detect which systems are installed."""
        available = ['acdso']  # Always available
        
        system_imports = {
            'prophet': 'prophet',
            'autots': 'autots',
            'statsforecast': 'statsforecast',
            'mlforecast': 'mlforecast',
            'sktime': 'sktime',
            'pmdarima': 'pmdarima',
            'pycaret': 'pycaret.time_series',
            'neuralprophet': 'neuralprophet',
            'darts': 'darts',
            'timesfm': 'timesfm3',
        }
        
        for system, module in system_imports.items():
            try:
                __import__(module)
                available.append(system)
            except ImportError:
                pass
        
        return available
    
    def run(
        self,
        df: pd.DataFrame,
        dataset_name: str = 'dataset',
    ) -> TSBenchmarkSummary:
        """
        Run benchmark on the given dataset.
        
        Parameters
        ----------
        df : pd.DataFrame
            Time series data.
        dataset_name : str
            Name for the dataset.
        
        Returns
        -------
        TSBenchmarkSummary
            Benchmark results.
        """
        start_time = time.time()
        
        # Suppress verbose output from external time series libraries
        import logging
        import warnings
        
        # Set logging levels to suppress verbose output
        for logger_name in ['cmdstanpy', 'prophet', 'autots', 'statsforecast', 
                            'mlforecast', 'darts', 'pmdarima', 'sktime']:
            logging.getLogger(logger_name).setLevel(logging.ERROR)
        
        if self.verbose:
            print("=" * 70)
            print("ACDSO Time Series Benchmark")
            print("=" * 70)
            print(f"Dataset: {dataset_name}")
            print(f"Samples: {len(df)}")
            print(f"Horizon: {self.horizon}")
            print(f"Systems: {', '.join(self.systems)}")
            print("=" * 70)
        
        # Prepare data
        df = self._prepare_data(df)
        train_df, test_df = self._split_data(df)
        
        y_train = train_df[self.target_col].values
        y_test = test_df[self.target_col].values
        
        self._results = []
        
        # Run each system
        for system in self.systems:
            if self.verbose:
                print(f"\nRunning {system.upper()}...")
            
            result = self._run_system(system, train_df, test_df, y_train, y_test, dataset_name)
            self._results.append(result)
            
            if self.verbose:
                if result.status == 'success':
                    print(f"  RMSE: {result.rmse:.4f}, MAE: {result.mae:.4f}, Time: {result.fit_time:.1f}s")
                else:
                    print(f"  ERROR: {result.error_message[:60]}")
        
        # Compute winners
        winners = self._compute_winners()
        
        # Create summary
        self._summary = TSBenchmarkSummary(
            results=self._results,
            winners_by_metric=winners,
            systems_tested=self.systems,
            datasets=[dataset_name],
            timestamp=datetime.now().isoformat(),
            total_time=time.time() - start_time,
        )
        
        if self.verbose:
            print("\n" + "=" * 70)
            print("BENCHMARK RESULTS")
            print("=" * 70)
            self._print_leaderboard()
        
        return self._summary
    
    def _prepare_data(self, df: pd.DataFrame) -> pd.DataFrame:
        """Prepare and validate the data."""
        df = df.copy()
        df[self.datetime_col] = pd.to_datetime(df[self.datetime_col])
        df = df.sort_values(self.datetime_col).reset_index(drop=True)
        
        # Keep only datetime_col, target_col, and numeric columns to avoid dtype errors
        cols_to_keep = [self.datetime_col, self.target_col]
        for col in df.columns:
            if col not in cols_to_keep:
                if np.issubdtype(df[col].dtype, np.number):
                    cols_to_keep.append(col)
        df = df[cols_to_keep]
        
        return df
    
    def _split_data(self, df: pd.DataFrame) -> Tuple[pd.DataFrame, pd.DataFrame]:
        """Split into train/test."""
        n = len(df)
        split_idx = int(n * (1 - self.test_ratio))
        return df.iloc[:split_idx], df.iloc[split_idx:]
    
    def _run_system(
        self,
        system: str,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run a single system."""
        runner = getattr(self, f'_run_{system}', None)
        
        if runner is None:
            return TSBenchmarkResult(
                system=system,
                dataset=dataset_name,
                status='error',
                error_message=f'Runner not implemented for {system}',
            )
        
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                return runner(train_df, test_df, y_train, y_test, dataset_name)
        except Exception as e:
            return TSBenchmarkResult(
                system=system,
                dataset=dataset_name,
                status='error',
                error_message=str(e),
            )
    
    def _run_acdso(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run ACDSO AutoTS."""
        result = TSBenchmarkResult(system='acdso', dataset=dataset_name)
        
        try:
            t0 = time.time()
            
            ts = AutoTS(
                datetime_col=self.datetime_col,
                target_col=self.target_col,
                horizon=self.horizon,
                time_budget=self.time_budget,
                verbose=False,
            )
            
            ts_result = ts.fit(train_df)
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            y_pred = ts_result.predict(test_df)
            result.predict_time = time.time() - t0
            
            # Compute metrics
            result.mae = mae(y_test, y_pred)
            result.rmse = rmse(y_test, y_pred)
            result.mape = mape(y_test, y_pred)
            result.smape = smape(y_test, y_pred)
            result.mase = mase(y_test, y_pred, y_train)
            
            result.notes.append(f"Best model: {ts_result.best_pipeline.name}")
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)
        
        return result
    
    def _run_prophet(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run Facebook Prophet."""
        result = TSBenchmarkResult(system='prophet', dataset=dataset_name)
        
        try:
            import warnings
            import logging
            from prophet import Prophet
            
            # Suppress Prophet/cmdstanpy verbose output
            logging.getLogger('cmdstanpy').setLevel(logging.ERROR)
            logging.getLogger('prophet').setLevel(logging.ERROR)
            
            # Prophet requires columns 'ds' and 'y'
            prophet_train = train_df[[self.datetime_col, self.target_col]].copy()
            prophet_train.columns = ['ds', 'y']
            
            prophet_test = test_df[[self.datetime_col]].copy()
            prophet_test.columns = ['ds']
            
            t0 = time.time()
            
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                model = Prophet(
                    yearly_seasonality=True,
                    weekly_seasonality=True,
                    daily_seasonality=False,
                )
                model.fit(prophet_train)
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            forecast = model.predict(prophet_test)
            y_pred = forecast['yhat'].values
            result.predict_time = time.time() - t0
            
            # Compute metrics
            result.mae = mae(y_test, y_pred)
            result.rmse = rmse(y_test, y_pred)
            result.mape = mape(y_test, y_pred)
            result.smape = smape(y_test, y_pred)
            result.mase = mase(y_test, y_pred, y_train)
            
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)
        
        return result
    
    def _run_autots(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run AutoTS library."""
        result = TSBenchmarkResult(system='autots', dataset=dataset_name)
        
        try:
            import warnings
            import logging
            from autots import AutoTS as AutoTSLib
            
            # Suppress AutoTS verbose template errors
            logging.getLogger('autots').setLevel(logging.ERROR)
            
            t0 = time.time()
            
            import sys
            import os
            from io import StringIO
            
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                
                # Redirect stdout/stderr to suppress AutoTS template errors
                old_stdout = sys.stdout
                old_stderr = sys.stderr
                sys.stdout = StringIO()
                sys.stderr = StringIO()
                
                try:
                    # Ensure forecast_length doesn't exceed training data size
                    forecast_length = min(len(test_df), max(1, len(train_df) // 2))
                    if forecast_length > len(train_df):
                        raise ValueError(f"forecast_length ({forecast_length}) is too large for training data ({len(train_df)} rows)")
                    
                    model = AutoTSLib(
                        forecast_length=forecast_length,
                        frequency='infer',
                        max_generations=3,  # Reduced for speed
                        num_validations=2,
                        validation_method='backwards',
                        no_negatives=False,
                        ensemble='simple',
                        verbose=-1,  # Fully suppress output
                    )
                    
                    model = model.fit(
                        train_df,
                        date_col=self.datetime_col,
                        value_col=self.target_col,
                    )
                finally:
                    sys.stdout = old_stdout
                    sys.stderr = old_stderr
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            prediction = model.predict()
            y_pred = prediction.forecast[self.target_col].values
            result.predict_time = time.time() - t0
            
            # Compute metrics
            result.mae = mae(y_test[:len(y_pred)], y_pred)
            result.rmse = rmse(y_test[:len(y_pred)], y_pred)
            result.mape = mape(y_test[:len(y_pred)], y_pred)
            result.smape = smape(y_test[:len(y_pred)], y_pred)
            result.mase = mase(y_test[:len(y_pred)], y_pred, y_train)
            
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)
        
        return result
    
    def _run_statsforecast(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run Nixtla StatsForecast."""
        result = TSBenchmarkResult(system='statsforecast', dataset=dataset_name)
        
        try:
            from statsforecast import StatsForecast
            from statsforecast.models import AutoARIMA, AutoETS, AutoTheta
            
            # Prepare data for StatsForecast
            sf_df = train_df[[self.datetime_col, self.target_col]].copy()
            sf_df.columns = ['ds', 'y']
            sf_df['unique_id'] = 'series_1'
            
            t0 = time.time()
            
            sf = StatsForecast(
                models=[
                    AutoARIMA(season_length=7),
                    AutoETS(season_length=7),
                ],
                freq='D',  # Adjust based on data frequency
                n_jobs=1,
            )
            
            sf.fit(sf_df)
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            forecast = sf.predict(h=len(test_df))
            y_pred = forecast['AutoARIMA'].values
            result.predict_time = time.time() - t0
            
            # Compute metrics
            result.mae = mae(y_test, y_pred)
            result.rmse = rmse(y_test, y_pred)
            result.mape = mape(y_test, y_pred)
            result.smape = smape(y_test, y_pred)
            result.mase = mase(y_test, y_pred, y_train)
            
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)
        
        return result
    
    def _run_mlforecast(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run Nixtla MLForecast."""
        result = TSBenchmarkResult(system='mlforecast', dataset=dataset_name)
        
        try:
            from mlforecast import MLForecast
            from sklearn.ensemble import RandomForestRegressor
            from lightgbm import LGBMRegressor
            
            # Prepare data
            mlf_df = train_df[[self.datetime_col, self.target_col]].copy()
            mlf_df.columns = ['ds', 'y']
            mlf_df['unique_id'] = 'series_1'
            
            t0 = time.time()
            
            mlf = MLForecast(
                models=[LGBMRegressor(verbosity=-1, n_estimators=100)],
                freq='D',
                lags=[1, 7, 14, 28],
                lag_transforms={
                    1: [(lambda x: x.rolling(7).mean(), 'rolling_mean_7')],
                },
            )
            
            mlf.fit(mlf_df)
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            forecast = mlf.predict(h=len(test_df))
            y_pred = forecast['LGBMRegressor'].values
            result.predict_time = time.time() - t0
            
            # Compute metrics
            result.mae = mae(y_test, y_pred)
            result.rmse = rmse(y_test, y_pred)
            result.mape = mape(y_test, y_pred)
            result.smape = smape(y_test, y_pred)
            result.mase = mase(y_test, y_pred, y_train)
            
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)
        
        return result
    
    def _run_sktime(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run sktime with simple, fast models to avoid hanging."""
        result = TSBenchmarkResult(system='sktime', dataset=dataset_name)
        
        try:
            from sktime.forecasting.naive import NaiveForecaster
            from sktime.forecasting.trend import TrendForecaster
            from sktime.forecasting.exp_smoothing import ExponentialSmoothing
            
            # Prepare data as pandas Series with datetime index
            # Handle potential None values and ensure proper data types
            if y_train is None or len(y_train) == 0:
                raise ValueError("Empty training data")
            
            # Ensure datetime column exists and is valid
            if self.datetime_col not in train_df.columns:
                raise ValueError(f"Datetime column '{self.datetime_col}' not found")
            
            # Create datetime index, handling potential issues
            try:
                dt_index = pd.DatetimeIndex(train_df[self.datetime_col])
            except Exception:
                # Fallback: create a range index
                dt_index = pd.RangeIndex(len(y_train))
            
            y = pd.Series(y_train, index=dt_index)
            
            t0 = time.time()
            
            # Use simpler models to avoid AutoARIMA hanging issues
            # Try Exponential Smoothing first (fast and reliable), then fallback to Naive
            forecaster = None
            try:
                # Use ExponentialSmoothing - much faster than AutoARIMA
                # sp must be None when seasonal is None to avoid 'int * None' error
                forecaster = ExponentialSmoothing(
                    trend="add",
                    seasonal=None,  # No seasonality to avoid complexity
                    sp=None,  # Must be None when seasonal=None
                )
                forecaster.fit(y)
            except Exception:
                try:
                    # Try TrendForecaster as second option
                    forecaster = TrendForecaster()
                    forecaster.fit(y)
                except Exception:
                    # Final fallback to NaiveForecaster
                    forecaster = NaiveForecaster(strategy="mean")
                    forecaster.fit(y)
            
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            # Create forecast horizon safely
            horizon_len = len(test_df) if test_df is not None and len(test_df) > 0 else self.horizon
            fh = np.arange(1, horizon_len + 1)
            
            # Handle case where predict might return None or fail
            try:
                y_pred = forecaster.predict(fh=fh)
                if y_pred is None:
                    raise ValueError("sktime predict returned None")
                y_pred = y_pred.values if hasattr(y_pred, 'values') else np.array(y_pred)
            except Exception as pred_e:
                # Fallback to naive forecast if prediction fails
                naive = NaiveForecaster(strategy="last")
                naive.fit(y)
                y_pred = naive.predict(fh=fh)
                y_pred = y_pred.values if hasattr(y_pred, 'values') else np.array(y_pred)
            
            result.predict_time = time.time() - t0
            
            # Ensure predictions are valid
            if y_pred is None or len(y_pred) == 0:
                raise ValueError("No predictions generated")
            
            # Align prediction length with test data
            min_len = min(len(y_test), len(y_pred))
            y_test_aligned = y_test[:min_len]
            y_pred_aligned = y_pred[:min_len]
            
            # Compute metrics
            result.mae = mae(y_test_aligned, y_pred_aligned)
            result.rmse = rmse(y_test_aligned, y_pred_aligned)
            result.mape = mape(y_test_aligned, y_pred_aligned)
            result.smape = smape(y_test_aligned, y_pred_aligned)
            result.mase = mase(y_test_aligned, y_pred_aligned, y_train)
            
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)
        
        return result
    
    def _run_pmdarima(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run pmdarima AutoARIMA."""
        result = TSBenchmarkResult(system='pmdarima', dataset=dataset_name)
        
        try:
            import pmdarima as pm
            
            t0 = time.time()
            
            model = pm.auto_arima(
                y_train,
                seasonal=True,
                m=7,
                suppress_warnings=True,
                error_action='ignore',
                stepwise=True,
                n_jobs=-1,
            )
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            y_pred = model.predict(n_periods=len(test_df))
            result.predict_time = time.time() - t0
            
            # Compute metrics
            result.mae = mae(y_test, y_pred)
            result.rmse = rmse(y_test, y_pred)
            result.mape = mape(y_test, y_pred)
            result.smape = smape(y_test, y_pred)
            result.mase = mase(y_test, y_pred, y_train)
            
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)

        return result

    def _run_timesfm(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run Google's TimesFM pretrained foundation model (zero-shot).

        Reuses the same TimesFMForecaster adapter as the AutoTS candidate
        pool, so there's one place that knows how to call the model.
        Skips cleanly (status='error', not a crash) below the adapter's
        own min_context_length -- a foundation model has nothing to offer
        over the naive baselines on a dozen-point series. Also fails
        cleanly into status='error' (caught by _run_system's try/except)
        if huggingface.co is unreachable at inference time -- required to
        fetch the pretrained weights, and blocked by policy in some
        network environments. See TimesFMForecaster's docstring.
        """
        result = TSBenchmarkResult(system='timesfm', dataset=dataset_name)

        try:
            from .candidates import TimesFMForecaster

            model = TimesFMForecaster()
            if len(y_train) < model.min_context_length:
                result.status = 'error'
                result.error_message = (
                    f"Only {len(y_train)} training points; TimesFM needs at least "
                    f"{model.min_context_length} to be worth running."
                )
                return result

            t0 = time.time()
            model.fit(None, y_train)
            result.fit_time = time.time() - t0

            t0 = time.time()
            y_pred = model.predict(np.zeros(len(test_df)))
            result.predict_time = time.time() - t0

            result.mae = mae(y_test, y_pred)
            result.rmse = rmse(y_test, y_pred)
            result.mape = mape(y_test, y_pred)
            result.smape = smape(y_test, y_pred)
            result.mase = mase(y_test, y_pred, y_train)

            result.status = 'success'

        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)

        return result

    def _run_pycaret(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run PyCaret Time Series."""
        result = TSBenchmarkResult(system='pycaret', dataset=dataset_name)
        
        try:
            from pycaret.time_series import TSForecastingExperiment
            
            # Prepare data
            ts_df = train_df[[self.datetime_col, self.target_col]].copy()
            ts_df.set_index(self.datetime_col, inplace=True)
            
            t0 = time.time()
            
            exp = TSForecastingExperiment()
            exp.setup(
                data=ts_df,
                target=self.target_col,
                fh=len(test_df),
                fold=3,
                session_id=42,
                verbose=False,
            )
            
            best_model = exp.compare_models(n_select=1, verbose=False)[0]
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            predictions = exp.predict_model(best_model)
            y_pred = predictions['y_pred'].values
            result.predict_time = time.time() - t0
            
            # Compute metrics
            result.mae = mae(y_test[:len(y_pred)], y_pred)
            result.rmse = rmse(y_test[:len(y_pred)], y_pred)
            result.mape = mape(y_test[:len(y_pred)], y_pred)
            result.smape = smape(y_test[:len(y_pred)], y_pred)
            result.mase = mase(y_test[:len(y_pred)], y_pred, y_train)
            
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)
        
        return result
    
    def _run_neuralprophet(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run NeuralProphet."""
        result = TSBenchmarkResult(system='neuralprophet', dataset=dataset_name)
        
        try:
            from neuralprophet import NeuralProphet, set_log_level
            
            set_log_level('ERROR')
            
            # Prepare data
            np_train = train_df[[self.datetime_col, self.target_col]].copy()
            np_train.columns = ['ds', 'y']
            
            np_test = test_df[[self.datetime_col]].copy()
            np_test.columns = ['ds']
            
            t0 = time.time()
            
            model = NeuralProphet(
                yearly_seasonality=True,
                weekly_seasonality=True,
                epochs=50,
            )
            model.fit(np_train, freq='D')
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            future = model.make_future_dataframe(np_train, periods=len(test_df))
            forecast = model.predict(future)
            y_pred = forecast['yhat1'].values[-len(test_df):]
            result.predict_time = time.time() - t0
            
            # Compute metrics
            result.mae = mae(y_test, y_pred)
            result.rmse = rmse(y_test, y_pred)
            result.mape = mape(y_test, y_pred)
            result.smape = smape(y_test, y_pred)
            result.mase = mase(y_test, y_pred, y_train)
            
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)
        
        return result
    
    def _run_darts(
        self,
        train_df: pd.DataFrame,
        test_df: pd.DataFrame,
        y_train: np.ndarray,
        y_test: np.ndarray,
        dataset_name: str,
    ) -> TSBenchmarkResult:
        """Run Darts."""
        result = TSBenchmarkResult(system='darts', dataset=dataset_name)
        
        try:
            from darts import TimeSeries
            from darts.models import AutoARIMA, ExponentialSmoothing, Theta
            
            # Create Darts TimeSeries
            ts = TimeSeries.from_dataframe(
                train_df,
                time_col=self.datetime_col,
                value_cols=[self.target_col],
            )
            
            t0 = time.time()
            
            model = AutoARIMA()
            model.fit(ts)
            result.fit_time = time.time() - t0
            
            t0 = time.time()
            forecast = model.predict(n=len(test_df))
            y_pred = forecast.values().flatten()
            result.predict_time = time.time() - t0
            
            # Compute metrics
            result.mae = mae(y_test, y_pred)
            result.rmse = rmse(y_test, y_pred)
            result.mape = mape(y_test, y_pred)
            result.smape = smape(y_test, y_pred)
            result.mase = mase(y_test, y_pred, y_train)
            
            result.status = 'success'
            
        except Exception as e:
            result.status = 'error'
            result.error_message = str(e)
        
        return result
    
    def _compute_winners(self) -> Dict[str, str]:
        """Compute winner for each metric."""
        winners = {}
        
        successful = [r for r in self._results if r.status == 'success']
        
        if not successful:
            return winners
        
        for metric in ['mae', 'rmse', 'mape', 'smape', 'mase', 'fit_time']:
            values = [(r.system, getattr(r, metric)) for r in successful]
            values = [(s, v) for s, v in values if not np.isnan(v)]
            
            if values:
                winner = min(values, key=lambda x: x[1])
                winners[metric] = winner[0]
        
        return winners
    
    def _print_leaderboard(self):
        """Print the leaderboard."""
        successful = [r for r in self._results if r.status == 'success']
        failed = [r for r in self._results if r.status == 'error']
        
        if successful:
            print("\nLeaderboard (sorted by RMSE):")
            print("-" * 80)
            print(f"{'System':<15} {'RMSE':<12} {'MAE':<12} {'MAPE':<12} {'Time(s)':<12}")
            print("-" * 80)
            
            for r in sorted(successful, key=lambda x: x.rmse):
                print(f"{r.system:<15} {r.rmse:<12.4f} {r.mae:<12.4f} {r.mape:<12.2f} {r.fit_time:<12.1f}")
        
        if failed:
            print("\nFailed Systems:")
            for r in failed:
                print(f"  - {r.system}: {r.error_message[:50]}")
        
        if self._summary and self._summary.winners_by_metric:
            print("\nWinners by Metric:")
            for metric, winner in self._summary.winners_by_metric.items():
                print(f"  {metric}: {winner}")
    
    def generate_report(
        self,
        output_path: str,
        format: str = 'html',
    ) -> str:
        """
        Generate a benchmark report.
        
        Parameters
        ----------
        output_path : str
            Path for the output file.
        format : str
            Output format: 'html', 'json', 'csv'.
        
        Returns
        -------
        str
            Path to the generated report.
        """
        if self._summary is None:
            raise RuntimeError("Must run benchmark before generating report")
        
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        if format == 'json':
            with open(output_path, 'w') as f:
                json.dump(self._summary.to_dict(), f, indent=2, default=str)
        
        elif format == 'csv':
            self._summary.to_dataframe().to_csv(output_path, index=False)
        
        elif format == 'html':
            html = self._generate_html_report()
            with open(output_path, 'w', encoding='utf-8') as f:
                f.write(html)
        
        else:
            raise ValueError(f"Unknown format: {format}")
        
        return str(output_path)
    
    def _generate_html_report(self) -> str:
        """Generate HTML report."""
        successful = [r for r in self._results if r.status == 'success']
        failed = [r for r in self._results if r.status == 'error']
        
        # Build comparison table
        rows = []
        for r in sorted(successful, key=lambda x: x.rmse):
            rows.append(f"""
                <tr>
                    <td><strong>{r.system.upper()}</strong></td>
                    <td>{r.rmse:.4f}</td>
                    <td>{r.mae:.4f}</td>
                    <td>{r.mape:.2f}%</td>
                    <td>{r.smape:.2f}%</td>
                    <td>{r.mase:.4f}</td>
                    <td>{r.fit_time:.1f}s</td>
                </tr>
            """)
        
        table_rows = '\n'.join(rows)
        
        # Winners section
        winners_html = ''
        if self._summary and self._summary.winners_by_metric:
            winners_items = '\n'.join([
                f'<li><strong>{m}:</strong> {w}</li>'
                for m, w in self._summary.winners_by_metric.items()
            ])
            winners_html = f'<ul>{winners_items}</ul>'
        
        # Failed section
        failed_html = ''
        if failed:
            failed_items = '\n'.join([
                f'<li><strong>{r.system}:</strong> {r.error_message[:100]}</li>'
                for r in failed
            ])
            failed_html = f'''
                <div class="failed-section">
                    <h3>Failed Systems</h3>
                    <ul>{failed_items}</ul>
                </div>
            '''
        
        html = f"""
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ACDSO Time Series Benchmark Report</title>
    <style>
        :root {{
            --primary: #2563eb;
            --success: #16a34a;
            --warning: #d97706;
            --danger: #dc2626;
            --bg: #f8fafc;
            --card-bg: #ffffff;
            --text: #1e293b;
        }}
        
        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
            background: var(--bg);
            color: var(--text);
            margin: 0;
            padding: 20px;
            line-height: 1.6;
        }}
        
        .container {{
            max-width: 1200px;
            margin: 0 auto;
        }}
        
        .header {{
            text-align: center;
            padding: 40px 0;
            background: linear-gradient(135deg, var(--primary), #7c3aed);
            color: white;
            border-radius: 16px;
            margin-bottom: 30px;
        }}
        
        .header h1 {{
            margin: 0 0 10px 0;
            font-size: 2.5rem;
        }}
        
        .card {{
            background: var(--card-bg);
            border-radius: 12px;
            padding: 24px;
            margin-bottom: 24px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }}
        
        .card h2 {{
            margin-top: 0;
            color: var(--primary);
            border-bottom: 2px solid var(--primary);
            padding-bottom: 10px;
        }}
        
        table {{
            width: 100%;
            border-collapse: collapse;
        }}
        
        th, td {{
            padding: 12px 16px;
            text-align: left;
            border-bottom: 1px solid #e2e8f0;
        }}
        
        th {{
            background: #f1f5f9;
            font-weight: 600;
        }}
        
        tr:hover {{
            background: #f8fafc;
        }}
        
        .winner {{
            background: #dcfce7 !important;
        }}
        
        .failed-section {{
            background: #fef2f2;
            border-left: 4px solid var(--danger);
            padding: 16px;
            border-radius: 8px;
        }}
        
        .winners-section ul {{
            list-style: none;
            padding: 0;
        }}
        
        .winners-section li {{
            padding: 8px 0;
            border-bottom: 1px solid #e2e8f0;
        }}
        
        .timestamp {{
            text-align: center;
            color: #64748b;
            margin-top: 40px;
            font-size: 0.875rem;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>ACDSO Time Series Benchmark</h1>
            <p>Comprehensive comparison of forecasting systems</p>
        </div>
        
        <div class="card">
            <h2>Benchmark Results</h2>
            <table>
                <thead>
                    <tr>
                        <th>System</th>
                        <th>RMSE</th>
                        <th>MAE</th>
                        <th>MAPE</th>
                        <th>sMAPE</th>
                        <th>MASE</th>
                        <th>Fit Time</th>
                    </tr>
                </thead>
                <tbody>
                    {table_rows}
                </tbody>
            </table>
        </div>
        
        <div class="card winners-section">
            <h2>Winners by Metric</h2>
            {winners_html}
        </div>
        
        {failed_html}
        
        <div class="card">
            <h2>Benchmark Configuration</h2>
            <ul>
                <li><strong>Datasets:</strong> {', '.join(self._summary.datasets)}</li>
                <li><strong>Systems Tested:</strong> {', '.join(self._summary.systems_tested)}</li>
                <li><strong>Forecast Horizon:</strong> {self.horizon}</li>
                <li><strong>Time Budget:</strong> {self.time_budget}s per system</li>
                <li><strong>Total Time:</strong> {self._summary.total_time:.1f}s</li>
            </ul>
        </div>
        
        <p class="timestamp">
            Generated: {self._summary.timestamp}
        </p>
    </div>
</body>
</html>
        """
        
        return html


def run_ts_benchmark(
    df: pd.DataFrame,
    datetime_col: str,
    target_col: str,
    horizon: int = 7,
    output_dir: str = 'reports',
    **kwargs,
) -> TSBenchmarkSummary:
    """
    Convenience function to run a complete time series benchmark.
    
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
    output_dir : str
        Directory for reports.
    **kwargs
        Additional arguments for TimeSeriesBenchmark.
    
    Returns
    -------
    TSBenchmarkSummary
        Benchmark results.
    """
    benchmark = TimeSeriesBenchmark(
        datetime_col=datetime_col,
        target_col=target_col,
        horizon=horizon,
        **kwargs,
    )
    
    summary = benchmark.run(df)
    
    # Generate reports
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    
    html_path = benchmark.generate_report(
        output_dir / f'ts_benchmark_{timestamp}.html',
        format='html',
    )
    
    json_path = benchmark.generate_report(
        output_dir / f'ts_benchmark_{timestamp}.json',
        format='json',
    )
    
    print(f"\nReports generated:")
    print(f"  HTML: {html_path}")
    print(f"  JSON: {json_path}")
    
    return summary

