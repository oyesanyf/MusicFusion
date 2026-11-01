"""
ACDSO Auto Time Series Module
==============================

Extends ACDSO's AutoML capabilities to time series forecasting with:
- Time-aware cross-validation (no data leakage)
- Automatic lag/rolling feature generation
- Time series specific metrics (MAE, RMSE, MAPE, MASE)
- Forecasting pipeline optimization
- Time series risk analysis
- Benchmarking against Prophet, AutoTS, StatsForecast, etc.

Usage:
    from acdso.timeseries import AutoTS, TimeSeriesRiskAnalyzer
    
    ts = AutoTS(datetime_col="date", target_col="sales", horizon=7)
    result = ts.fit(df)
    forecast = result.best_pipeline.predict(df_future)
    
Benchmarking:
    from acdso.timeseries import TimeSeriesBenchmark, run_ts_benchmark
    
    benchmark = TimeSeriesBenchmark(
        datetime_col="date",
        target_col="sales",
        horizon=7,
    )
    results = benchmark.run(df)
    benchmark.generate_report("ts_benchmark.html")
"""

from .core import AutoTS, AutoTSResult, ForecastingPipeline, auto_forecast
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
    SeasonalDecomposer,
    DifferencingTransformer,
    TimeSeriesFeatureUnion,
    create_ts_feature_pipeline,
    ColumnDropper,
    DropNaNRowsTransformer,
)
from .metrics import (
    mae,
    rmse,
    mape,
    smape,
    mase,
    wape,
    get_ts_scorer,
    TimeSeriesScorer,
    evaluate_forecast,
    compare_forecasts,
)
from .risk import (
    TimeSeriesRiskAnalyzer,
    TimeSeriesRiskReport,
    detect_frequency,
    detect_seasonality,
    check_stationarity,
    detect_outliers,
    detect_trend,
)
from .candidates import (
    TimeSeriesCandidateGenerator,
    ForecastingCandidate,
    NaiveForecaster,
    SeasonalNaiveForecaster,
    MovingAverageForecaster,
    ExponentialSmoothingForecaster,
    TimeSeriesRegressor,
    create_ts_pipeline,
    get_default_candidates,
)
from .benchmark import (
    TimeSeriesBenchmark,
    TSBenchmarkResult,
    TSBenchmarkSummary,
    run_ts_benchmark,
)

__all__ = [
    # Core
    "AutoTS",
    "AutoTSResult",
    "ForecastingPipeline",
    "auto_forecast",
    # Splits
    "TimeSeriesSplitter",
    "ExpandingWindowSplit",
    "RollingWindowSplit",
    "WalkForwardSplit",
    "create_time_series_cv",
    # Transformers
    "LagFeatureTransformer",
    "RollingWindowTransformer",
    "DateTimeFeatureTransformer",
    "SeasonalDecomposer",
    "DifferencingTransformer",
    "TimeSeriesFeatureUnion",
    "create_ts_feature_pipeline",
    "ColumnDropper",
    "DropNaNRowsTransformer",
    # Metrics
    "mae",
    "rmse",
    "mape",
    "smape",
    "mase",
    "wape",
    "get_ts_scorer",
    "TimeSeriesScorer",
    "evaluate_forecast",
    "compare_forecasts",
    # Risk
    "TimeSeriesRiskAnalyzer",
    "TimeSeriesRiskReport",
    "detect_frequency",
    "detect_seasonality",
    "check_stationarity",
    "detect_outliers",
    "detect_trend",
    # Candidates
    "TimeSeriesCandidateGenerator",
    "ForecastingCandidate",
    "NaiveForecaster",
    "SeasonalNaiveForecaster",
    "MovingAverageForecaster",
    "ExponentialSmoothingForecaster",
    "TimeSeriesRegressor",
    "create_ts_pipeline",
    "get_default_candidates",
    # Benchmark
    "TimeSeriesBenchmark",
    "TSBenchmarkResult",
    "TSBenchmarkSummary",
    "run_ts_benchmark",
]

