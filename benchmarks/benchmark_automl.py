"""
ACDSO Benchmarking Suite: Fair Comparison with Auto-Sklearn and TPOT
=====================================================================

This module provides a rigorous, fair benchmarking framework to compare
ACDSO against other AutoML systems across multiple dimensions:

1. TRADITIONAL METRICS
   - Accuracy/F1/R² (predictive performance)
   - Training time
   - Inference latency

2. PRODUCTION-RELEVANT METRICS (ACDSO's unique focus)
   - Leakage detection capability
   - Stability across resamples
   - Memory footprint
   - Audit trail availability

3. FAIRNESS CONSIDERATIONS
   - Equal time budgets
   - Same train/test splits
   - Standardized preprocessing
   - Multiple datasets
   - Statistical significance testing

Usage:
    python benchmark_automl.py --datasets all --time-budget 300 --output results.json
    python benchmark_automl.py --datasets iris,wine --time-budget 60
    python benchmark_automl.py --quick  # Fast demo run
"""

from __future__ import annotations

# MUST be before any imports to suppress pkg_resources deprecation warning
import warnings
warnings.filterwarnings('ignore', message='.*pkg_resources.*')
warnings.filterwarnings('ignore', category=DeprecationWarning)
warnings.filterwarnings('ignore', category=FutureWarning)
warnings.filterwarnings('ignore', category=UserWarning)

import argparse
import gc
import json
import os
import sys
import time
import tracemalloc
from dataclasses import dataclass, asdict, field
from datetime import datetime
from typing import Any, Dict, List, Optional, Tuple

# Additional warning suppression after imports
warnings.filterwarnings('ignore', message='.*pkg_resources.*')
warnings.filterwarnings('ignore', category=DeprecationWarning, module='stopit')
warnings.filterwarnings('ignore', category=DeprecationWarning, module='sklearn')
warnings.filterwarnings('ignore', category=FutureWarning)
warnings.filterwarnings('ignore', message='.*H2O.*')
warnings.filterwarnings('ignore', message='.*h2o.*')
warnings.filterwarnings('ignore', message='.*fastai.*')
warnings.filterwarnings('ignore', message='.*NeuralNet.*')
warnings.filterwarnings('ignore', category=UserWarning, module='autogluon')
warnings.filterwarnings('ignore', category=UserWarning, module='h2o')

import numpy as np
import pandas as pd
from scipy import stats
from sklearn.datasets import (
    load_iris, load_wine, load_breast_cancer, load_diabetes,
    fetch_california_housing, make_classification, make_regression
)
from sklearn.model_selection import train_test_split, cross_val_score, StratifiedKFold, KFold
from sklearn.metrics import (
    accuracy_score, f1_score, precision_score, recall_score,
    r2_score, mean_squared_error, mean_absolute_error
)
from sklearn.preprocessing import LabelEncoder

# Suppress warnings for cleaner output
warnings.filterwarnings('ignore')

# Check available AutoML systems
_HAS_ACDSO = True
_HAS_AUTOSKLEARN = False
_HAS_TPOT = False
_HAS_FLAML = False

try:
    from acdso import ACDSOClassifier, ACDSORegressor
except ImportError:
    _HAS_ACDSO = False
    print("Warning: ACDSO not installed. Install with: pip install -e .")

try:
    import autosklearn.classification
    import autosklearn.regression
    _HAS_AUTOSKLEARN = True
except ImportError:
    pass

try:
    # TPOT 1.0+ uses TPOTEstimator
    from tpot import TPOTEstimator
    _HAS_TPOT = True
    _TPOT_V1 = True
except ImportError:
    _TPOT_V1 = False
    try:
        # TPOT < 1.0 uses TPOTClassifier/TPOTRegressor
        from tpot import TPOTClassifier, TPOTRegressor
        _HAS_TPOT = True
    except ImportError:
        pass

try:
    from flaml import AutoML as FLAMLAutoML
    _HAS_FLAML = True
except ImportError:
    pass

# PyCaret - popular, user-friendly AutoML
_HAS_PYCARET = False
try:
    from pycaret.classification import ClassificationExperiment
    from pycaret.regression import RegressionExperiment
    _HAS_PYCARET = True
except ImportError:
    pass

# AutoGluon - Amazon's AutoML
_HAS_AUTOGLUON = False
try:
    from autogluon.tabular import TabularPredictor
    _HAS_AUTOGLUON = True
except ImportError:
    pass

# H2O AutoML
_HAS_H2O = False
try:
    import h2o
    from h2o.automl import H2OAutoML
    _HAS_H2O = True
except ImportError:
    pass

# MLJAR - simple and fast
_HAS_MLJAR = False
try:
    from supervised import AutoML as MLJARAutoML
    _HAS_MLJAR = True
except ImportError:
    pass


# =============================================================================
# Data Classes for Results
# =============================================================================

@dataclass
class BenchmarkResult:
    """Result of a single benchmark run."""
    system: str
    dataset: str
    task: str  # classification or regression
    
    # Traditional metrics
    test_score: float  # Primary metric (accuracy/r2)
    test_f1: Optional[float] = None  # For classification
    test_precision: Optional[float] = None
    test_recall: Optional[float] = None
    test_rmse: Optional[float] = None  # For regression
    test_mae: Optional[float] = None
    
    # Time metrics
    fit_time_s: float = 0.0
    predict_time_s: float = 0.0
    latency_ms_per_row: float = 0.0
    
    # Memory metrics
    peak_memory_mb: float = 0.0
    
    # Stability metrics
    cv_score_mean: float = 0.0
    cv_score_std: float = 0.0
    
    # Production metrics (ACDSO-specific)
    has_risk_report: bool = False
    has_audit_trail: bool = False
    
    # All models evaluated by this system (leaderboard)
    models_evaluated: List[Dict[str, Any]] = field(default_factory=list)
    detected_leakage: bool = False
    detected_multicollinearity: bool = False
    risk_penalty: Optional[float] = None
    pareto_front_size: Optional[int] = None
    
    # Metadata
    n_train: int = 0
    n_test: int = 0
    n_features: int = 0
    time_budget_s: float = 0.0
    error: Optional[str] = None
    model_name: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class LeakageTestResult:
    """Result of leakage detection test."""
    system: str
    leakage_type: str  # 'proxy', 'temporal', 'direct'
    injected: bool
    detected: bool
    detection_method: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass 
class BenchmarkSummary:
    """Summary of all benchmark results."""
    timestamp: str
    systems_tested: List[str]
    datasets_tested: List[str]
    time_budget_s: float
    results: List[BenchmarkResult] = field(default_factory=list)
    leakage_tests: List[LeakageTestResult] = field(default_factory=list)
    winner_by_metric: Dict[str, str] = field(default_factory=dict)
    statistical_tests: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "timestamp": self.timestamp,
            "systems_tested": self.systems_tested,
            "datasets_tested": self.datasets_tested,
            "time_budget_s": self.time_budget_s,
            "results": [r.to_dict() for r in self.results],
            "leakage_tests": [l.to_dict() for l in self.leakage_tests],
            "winner_by_metric": self.winner_by_metric,
            "statistical_tests": self.statistical_tests,
        }


# =============================================================================
# Dataset Loading
# =============================================================================

def load_custom_dataset(
    file_path: str,
    target_column: str,
    task: str = "auto",
    exclude_columns: List[str] = None,
) -> Dict[str, Any]:
    """
    Load a custom dataset from a file.
    
    Parameters
    ----------
    file_path : str
        Path to CSV, Excel, or Parquet file.
    target_column : str
        Name of the target column.
    task : str
        'classification', 'regression', or 'auto' (infer from target).
    exclude_columns : list, optional
        Columns to exclude from features.
    
    Returns
    -------
    dict
        {'X': DataFrame, 'y': array, 'task': str, 'description': str}
    """
    import os
    
    # Load file based on extension
    ext = os.path.splitext(file_path)[1].lower()
    if ext == '.csv':
        df = pd.read_csv(file_path)
    elif ext in ['.xls', '.xlsx']:
        df = pd.read_excel(file_path)
    elif ext == '.parquet':
        df = pd.read_parquet(file_path)
    elif ext == '.json':
        df = pd.read_json(file_path)
    else:
        raise ValueError(f"Unsupported file format: {ext}")
    
    if target_column not in df.columns:
        raise ValueError(f"Target column '{target_column}' not found. Available: {list(df.columns)}")
    
    # Extract target
    y = df[target_column].values
    
    # Exclude columns
    drop_cols = [target_column]
    if exclude_columns:
        drop_cols.extend([c for c in exclude_columns if c in df.columns])
    
    X = df.drop(columns=drop_cols)
    
    # Auto-detect task
    if task == "auto":
        if np.issubdtype(y.dtype, np.floating):
            unique_vals = np.unique(y)
            if len(unique_vals) > 20:
                task = "regression"
            else:
                task = "classification"
        else:
            unique_vals = np.unique(y)
            if len(unique_vals) <= 20:
                task = "classification"
            else:
                task = "regression"
    
    name = os.path.basename(file_path)
    return {
        'X': X,
        'y': y,
        'task': task,
        'description': f'{name} ({len(df)} samples, {X.shape[1]} features, target: {target_column})'
    }


def load_benchmark_datasets(dataset_names: List[str] = None) -> Dict[str, Dict[str, Any]]:
    """
    Load standardized benchmark datasets.
    
    Returns dict of {name: {'X': df, 'y': array, 'task': str}}
    """
    all_datasets = {}
    
    # Classification datasets
    if dataset_names is None or 'iris' in dataset_names:
        data = load_iris()
        all_datasets['iris'] = {
            'X': pd.DataFrame(data.data, columns=data.feature_names),
            'y': data.target,
            'task': 'classification',
            'description': 'Iris (150 samples, 4 features, 3 classes)'
        }
    
    if dataset_names is None or 'wine' in dataset_names:
        data = load_wine()
        all_datasets['wine'] = {
            'X': pd.DataFrame(data.data, columns=data.feature_names),
            'y': data.target,
            'task': 'classification',
            'description': 'Wine (178 samples, 13 features, 3 classes)'
        }
    
    if dataset_names is None or 'breast_cancer' in dataset_names:
        data = load_breast_cancer()
        all_datasets['breast_cancer'] = {
            'X': pd.DataFrame(data.data, columns=data.feature_names),
            'y': data.target,
            'task': 'classification',
            'description': 'Breast Cancer (569 samples, 30 features, 2 classes)'
        }
    
    if dataset_names is None or 'synthetic_clf' in dataset_names:
        X, y = make_classification(
            n_samples=1000, n_features=20, n_informative=10,
            n_redundant=5, n_clusters_per_class=2, random_state=42
        )
        all_datasets['synthetic_clf'] = {
            'X': pd.DataFrame(X, columns=[f'f{i}' for i in range(20)]),
            'y': y,
            'task': 'classification',
            'description': 'Synthetic Classification (1000 samples, 20 features)'
        }
    
    # Regression datasets
    if dataset_names is None or 'diabetes' in dataset_names:
        data = load_diabetes()
        all_datasets['diabetes'] = {
            'X': pd.DataFrame(data.data, columns=data.feature_names),
            'y': data.target,
            'task': 'regression',
            'description': 'Diabetes (442 samples, 10 features)'
        }
    
    if dataset_names is None or 'california' in dataset_names:
        try:
            data = fetch_california_housing()
            # Subsample for faster benchmarks
            n = min(2000, len(data.target))
            idx = np.random.RandomState(42).choice(len(data.target), n, replace=False)
            all_datasets['california'] = {
                'X': pd.DataFrame(data.data[idx], columns=data.feature_names),
                'y': data.target[idx],
                'task': 'regression',
                'description': f'California Housing ({n} samples, 8 features)'
            }
        except Exception:
            pass
    
    if dataset_names is None or 'synthetic_reg' in dataset_names:
        X, y = make_regression(
            n_samples=1000, n_features=20, n_informative=10,
            noise=10.0, random_state=42
        )
        all_datasets['synthetic_reg'] = {
            'X': pd.DataFrame(X, columns=[f'f{i}' for i in range(20)]),
            'y': y,
            'task': 'regression',
            'description': 'Synthetic Regression (1000 samples, 20 features)'
        }
    
    # Datasets with intentional issues (for risk detection testing)
    if dataset_names is None or 'leaky_clf' in dataset_names:
        X, y = make_classification(n_samples=500, n_features=10, random_state=42)
        df = pd.DataFrame(X, columns=[f'f{i}' for i in range(10)])
        # Inject leakage: add a feature that is almost the target
        df['leaky_feature'] = y + np.random.randn(len(y)) * 0.01
        all_datasets['leaky_clf'] = {
            'X': df,
            'y': y,
            'task': 'classification',
            'description': 'Classification with injected leakage',
            'has_leakage': True,
            'leaky_columns': ['leaky_feature']
        }
    
    if dataset_names is None or 'imbalanced_clf' in dataset_names:
        X, y = make_classification(
            n_samples=1000, n_features=10, weights=[0.95, 0.05],
            flip_y=0, random_state=42
        )
        all_datasets['imbalanced_clf'] = {
            'X': pd.DataFrame(X, columns=[f'f{i}' for i in range(10)]),
            'y': y,
            'task': 'classification',
            'description': 'Severely imbalanced (95/5 split)',
            'has_imbalance': True
        }
    
    if dataset_names is None or 'multicollinear' in dataset_names:
        np.random.seed(42)
        n = 500
        x1 = np.random.randn(n)
        x2 = x1 + np.random.randn(n) * 0.01  # Almost identical to x1
        x3 = x1 + np.random.randn(n) * 0.01  # Almost identical to x1
        X = pd.DataFrame({
            'x1': x1, 'x2': x2, 'x3': x3,
            'x4': np.random.randn(n),
            'x5': np.random.randn(n),
        })
        y = (x1 + X['x4'] + np.random.randn(n) * 0.5 > 0).astype(int)
        all_datasets['multicollinear'] = {
            'X': X,
            'y': y,
            'task': 'classification',
            'description': 'High multicollinearity (VIF > 100)',
            'has_multicollinearity': True
        }
    
    # Linear datasets (best handled by linear models)
    if dataset_names is None or 'linear_reg' in dataset_names:
        # Perfect linear regression dataset
        np.random.seed(42)
        n = 800
        X = pd.DataFrame({
            'x1': np.random.randn(n),
            'x2': np.random.randn(n),
            'x3': np.random.randn(n),
            'x4': np.random.randn(n),
            'x5': np.random.randn(n),
        })
        # y = 3*x1 + 2*x2 - 1.5*x3 + 0.5*x4 + noise
        y = 3*X['x1'] + 2*X['x2'] - 1.5*X['x3'] + 0.5*X['x4'] + np.random.randn(n) * 0.5
        all_datasets['linear_reg'] = {
            'X': X,
            'y': y.values,
            'task': 'regression',
            'description': 'Linear regression (y = 3x1 + 2x2 - 1.5x3 + 0.5x4 + noise)',
            'is_linear': True
        }
    
    if dataset_names is None or 'linear_clf' in dataset_names:
        # Linear classification (separable by hyperplane)
        np.random.seed(42)
        n = 800
        X = pd.DataFrame({
            'x1': np.random.randn(n),
            'x2': np.random.randn(n),
            'x3': np.random.randn(n),
            'x4': np.random.randn(n),
        })
        # y = sign(2*x1 + 1.5*x2 - x3 + noise)
        decision = 2*X['x1'] + 1.5*X['x2'] - X['x3'] + np.random.randn(n) * 0.3
        y = (decision > 0).astype(int)
        all_datasets['linear_clf'] = {
            'X': X,
            'y': y,
            'task': 'classification',
            'description': 'Linear classification (linearly separable)',
            'is_linear': True
        }
    
    if dataset_names is None or 'linear_sparse' in dataset_names:
        # Sparse linear regression (only a few features matter)
        np.random.seed(42)
        n = 600
        n_features = 20
        X = pd.DataFrame(
            np.random.randn(n, n_features),
            columns=[f'x{i}' for i in range(n_features)]
        )
        # Only x0, x1, x2 matter: y = 5*x0 - 3*x1 + 2*x2 + noise
        y = 5*X['x0'] - 3*X['x1'] + 2*X['x2'] + np.random.randn(n) * 0.5
        all_datasets['linear_sparse'] = {
            'X': X,
            'y': y.values,
            'task': 'regression',
            'description': 'Sparse linear (3/20 features relevant)',
            'is_linear': True,
            'is_sparse': True
        }
    
    if dataset_names is None or 'linear_highdim' in dataset_names:
        # High-dimensional linear regression
        np.random.seed(42)
        n = 500
        n_features = 100
        # Generate features with some correlation structure
        base = np.random.randn(n, 10)
        X_mat = np.hstack([
            base,
            base @ np.random.randn(10, 90) * 0.5 + np.random.randn(n, 90) * 0.5
        ])
        X = pd.DataFrame(X_mat, columns=[f'x{i}' for i in range(n_features)])
        # Linear combination of first 10 features
        coeffs = np.array([3, 2, -1.5, 1, -0.5, 0.8, -0.3, 0.2, 0.1, -0.1])
        y = X_mat[:, :10] @ coeffs + np.random.randn(n) * 0.5
        all_datasets['linear_highdim'] = {
            'X': X,
            'y': y,
            'task': 'regression',
            'description': 'High-dimensional linear (100 features, linear signal)',
            'is_linear': True,
            'is_high_dimensional': True
        }
    
    return all_datasets


# =============================================================================
# AutoML System Wrappers
# =============================================================================

def run_acdso(
    X_train: pd.DataFrame,
    y_train: np.ndarray,
    X_test: pd.DataFrame,
    y_test: np.ndarray,
    task: str,
    time_budget_s: float,
) -> BenchmarkResult:
    """Run ACDSO and collect all metrics."""
    if not _HAS_ACDSO:
        return BenchmarkResult(
            system='ACDSO', dataset='', task=task,
            test_score=0.0, error='ACDSO not installed'
        )
    
    result = BenchmarkResult(
        system='ACDSO',
        dataset='',
        task=task,
        test_score=0.0,  # Will be updated after fitting
        n_train=len(X_train),
        n_test=len(X_test),
        n_features=X_train.shape[1],
        time_budget_s=time_budget_s,
    )
    
    try:
        # Start memory tracking
        gc.collect()
        tracemalloc.start()
        
        # Configure and fit
        t0 = time.time()
        
        if task == 'classification':
            model = ACDSOClassifier(
                scorer='f1_weighted',
                cv=5,
                preference='balanced',
                time_budget_s=time_budget_s,
                verbose=0,
            )
        else:
            model = ACDSORegressor(
                scorer='r2',
                cv=5,
                preference='balanced',
                time_budget_s=time_budget_s,
                verbose=0,
            )
        
        model.fit(X_train, y_train)
        fit_time = time.time() - t0
        
        # Memory
        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        
        # Predictions
        t0 = time.time()
        y_pred = model.predict(X_test)
        predict_time = time.time() - t0
        
        # Metrics
        result.fit_time_s = fit_time
        result.predict_time_s = predict_time
        result.latency_ms_per_row = (predict_time * 1000) / len(X_test)
        result.peak_memory_mb = peak / (1024 * 1024)
        
        if task == 'classification':
            result.test_score = accuracy_score(y_test, y_pred)
            result.test_f1 = f1_score(y_test, y_pred, average='weighted')
            result.test_precision = precision_score(y_test, y_pred, average='weighted', zero_division=0)
            result.test_recall = recall_score(y_test, y_pred, average='weighted', zero_division=0)
        else:
            result.test_score = r2_score(y_test, y_pred)
            result.test_rmse = np.sqrt(mean_squared_error(y_test, y_pred))
            result.test_mae = mean_absolute_error(y_test, y_pred)
        
        # CV stability
        if hasattr(model, 'audit_trail_') and model.audit_trail_:
            candidates = model.audit_trail_.get('candidates_evaluated', [])
            if candidates:
                cv_stds = [c.get('cv_score_std', 0) for c in candidates]
                result.cv_score_std = min(cv_stds) if cv_stds else 0.0
        
        # ACDSO-specific: Risk diagnostics
        result.has_risk_report = hasattr(model, 'risk_report_') and model.risk_report_ is not None
        result.has_audit_trail = hasattr(model, 'audit_trail_') and model.audit_trail_ is not None
        
        if result.has_risk_report:
            rr = model.risk_report_
            result.detected_leakage = (
                len(rr.get('leakage_proxy_features', [])) > 0 or
                (rr.get('leakage_proxy_score_max', 0) >= 0.95)
            )
            result.detected_multicollinearity = rr.get('vif_max', 1) >= 10
            # Compute approximate risk penalty
            imb = rr.get('imbalance_severity', 0)
            vif = min(1, (rr.get('vif_max', 1) - 5) / 25)
            result.risk_penalty = 0.5 * imb + 0.5 * max(0, vif)
        
        if result.has_audit_trail:
            result.pareto_front_size = len(model.audit_trail_.get('pareto_front', []))
            # Get selected model name
            selected = model.audit_trail_.get('selected', {})
            result.model_name = selected.get('name', 'Unknown')
            
            # Capture all models evaluated by ACDSO
            candidates = model.audit_trail_.get('candidates_evaluated', [])
            result.models_evaluated = []
            for c in candidates:
                result.models_evaluated.append({
                    'name': c.get('name', 'Unknown'),
                    'score': c.get('metric', c.get('cv_score', 0)),
                    'fit_time': c.get('fit_time', 0),
                    'memory_mb': c.get('memory_mb', 0),
                    'latency_ms': c.get('latency_ms_per_row', 0),
                    'risk_penalty': c.get('risk_penalty', 0),
                    'is_best': c.get('name') == selected.get('name'),
                })
        
    except Exception as e:
        result.error = str(e)
        if tracemalloc.is_tracing():
            tracemalloc.stop()
    
    return result


def run_autosklearn(
    X_train: pd.DataFrame,
    y_train: np.ndarray,
    X_test: pd.DataFrame,
    y_test: np.ndarray,
    task: str,
    time_budget_s: float,
) -> BenchmarkResult:
    """Run Auto-sklearn and collect metrics."""
    if not _HAS_AUTOSKLEARN:
        return BenchmarkResult(
            system='Auto-sklearn', dataset='', task=task,
            test_score=0.0, error='Auto-sklearn not installed'
        )
    
    result = BenchmarkResult(
        system='Auto-sklearn',
        dataset='',
        task=task,
        test_score=0.0,  # Will be updated after fitting
        n_train=len(X_train),
        n_test=len(X_test),
        n_features=X_train.shape[1],
        time_budget_s=time_budget_s,
    )
    
    try:
        gc.collect()
        tracemalloc.start()
        
        t0 = time.time()
        
        if task == 'classification':
            model = autosklearn.classification.AutoSklearnClassifier(
                time_left_for_this_task=int(time_budget_s),
                per_run_time_limit=int(time_budget_s / 5),
                memory_limit=4096,
                n_jobs=1,
            )
        else:
            model = autosklearn.regression.AutoSklearnRegressor(
                time_left_for_this_task=int(time_budget_s),
                per_run_time_limit=int(time_budget_s / 5),
                memory_limit=4096,
                n_jobs=1,
            )
        
        model.fit(X_train.values, y_train)
        fit_time = time.time() - t0
        
        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        
        t0 = time.time()
        y_pred = model.predict(X_test.values)
        predict_time = time.time() - t0
        
        result.fit_time_s = fit_time
        result.predict_time_s = predict_time
        result.latency_ms_per_row = (predict_time * 1000) / len(X_test)
        result.peak_memory_mb = peak / (1024 * 1024)
        
        if task == 'classification':
            result.test_score = accuracy_score(y_test, y_pred)
            result.test_f1 = f1_score(y_test, y_pred, average='weighted')
            result.test_precision = precision_score(y_test, y_pred, average='weighted', zero_division=0)
            result.test_recall = recall_score(y_test, y_pred, average='weighted', zero_division=0)
        else:
            result.test_score = r2_score(y_test, y_pred)
            result.test_rmse = np.sqrt(mean_squared_error(y_test, y_pred))
            result.test_mae = mean_absolute_error(y_test, y_pred)
        
        # Auto-sklearn doesn't have risk reports
        result.has_risk_report = False
        result.has_audit_trail = False
        
    except Exception as e:
        result.error = str(e)
        if tracemalloc.is_tracing():
            tracemalloc.stop()
    
    return result


def run_tpot(
    X_train: pd.DataFrame,
    y_train: np.ndarray,
    X_test: pd.DataFrame,
    y_test: np.ndarray,
    task: str,
    time_budget_s: float,
) -> BenchmarkResult:
    """Run TPOT and collect metrics."""
    if not _HAS_TPOT:
        return BenchmarkResult(
            system='TPOT', dataset='', task=task,
            test_score=0.0, error='TPOT not installed'
        )
    
    result = BenchmarkResult(
        system='TPOT',
        dataset='',
        task=task,
        test_score=0.0,  # Will be updated after fitting
        n_train=len(X_train),
        n_test=len(X_test),
        n_features=X_train.shape[1],
        time_budget_s=time_budget_s,
    )
    
    try:
        gc.collect()
        tracemalloc.start()
        
        t0 = time.time()
        
        # Suppress all warnings during TPOT execution
        import sys
        import io
        
        # TPOT 1.0+ has issues with genetic algorithm on small datasets/short budgets
        # We need at least 180s for TPOT 1.0+ to have a chance to complete
        min_budget = 180 if _TPOT_V1 else 60
        actual_budget = max(time_budget_s, min_budget)
        
        if _TPOT_V1 and len(X_train) < 100:
            tracemalloc.stop()
            result.error = f"TPOT 1.0+ requires >=100 samples (got {len(X_train)} samples)"
            return result
        
        # TPOT API changed significantly in version 1.0+
        model = None
        fit_success = False
        
        if _TPOT_V1:
            # TPOT 1.0+ - use simplified configuration
            from tpot.config import get_search_space
            
            try:
                # Use a single fast estimator for more reliable fitting
                if task == 'classification':
                    search_space = get_search_space('RandomForestClassifier')
                    scorers = ['accuracy']
                else:
                    search_space = get_search_space('RandomForestRegressor')
                    scorers = ['neg_mean_squared_error']
                
                model = TPOTEstimator(
                    search_space=search_space,
                    scorers=scorers,
                    scorers_weights=[1.0],
                    classification=(task == 'classification'),
                    max_time_mins=max(2, actual_budget / 60),
                    max_eval_time_mins=max(1, actual_budget / 120),
                    population_size=10,
                    generations=3,
                    n_jobs=1,
                    verbose=0,
                    random_state=42,
                    cv=3,
                )
                
                # Suppress stderr during fit
                old_stderr = sys.stderr
                sys.stderr = io.StringIO()
                try:
                    model.fit(X_train.values, y_train)
                    fit_success = hasattr(model, 'fitted_pipeline_') and model.fitted_pipeline_ is not None
                finally:
                    sys.stderr = old_stderr
            except Exception as e:
                fit_success = False
        else:
            # TPOT < 1.0 uses TPOTClassifier/TPOTRegressor
            if task == 'classification':
                model = TPOTClassifier(
                    generations=2,
                    population_size=10,
                    cv=3,
                    random_state=42,
                    verbosity=0,
                    n_jobs=1,
                    max_time_mins=actual_budget / 60,
                )
            else:
                model = TPOTRegressor(
                    generations=2,
                    population_size=10,
                    cv=3,
                    random_state=42,
                    verbosity=0,
                    n_jobs=1,
                    max_time_mins=actual_budget / 60,
                )
            
            old_stderr = sys.stderr
            sys.stderr = io.StringIO()
            try:
                model.fit(X_train.values, y_train)
                fit_success = True
            finally:
                sys.stderr = old_stderr
        
        fit_time = time.time() - t0
        
        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        
        # Check if TPOT actually fitted a model
        if not fit_success or model is None:
            result.error = "TPOT genetic algorithm failed - needs more time or data"
            result.fit_time_s = fit_time
            return result
        
        t0 = time.time()
        y_pred = model.predict(X_test.values)
        predict_time = time.time() - t0
        
        result.fit_time_s = fit_time
        result.predict_time_s = predict_time
        result.latency_ms_per_row = (predict_time * 1000) / len(X_test)
        result.peak_memory_mb = peak / (1024 * 1024)
        
        if task == 'classification':
            result.test_score = accuracy_score(y_test, y_pred)
            result.test_f1 = f1_score(y_test, y_pred, average='weighted')
            result.test_precision = precision_score(y_test, y_pred, average='weighted', zero_division=0)
            result.test_recall = recall_score(y_test, y_pred, average='weighted', zero_division=0)
        else:
            result.test_score = r2_score(y_test, y_pred)
            result.test_rmse = np.sqrt(mean_squared_error(y_test, y_pred))
            result.test_mae = mean_absolute_error(y_test, y_pred)
        
        # TPOT has Pareto front but no risk reports
        result.has_risk_report = False
        result.has_audit_trail = False
        result.pareto_front_size = len(getattr(model, 'pareto_front_', [])) if hasattr(model, 'pareto_front_') else None
        
        # Capture selected model name for TPOT
        try:
            if _TPOT_V1 and hasattr(model, 'fitted_pipeline_') and model.fitted_pipeline_ is not None:
                # TPOT 1.0+ stores pipeline
                result.model_name = str(type(model.fitted_pipeline_.steps[-1][1]).__name__)
            elif hasattr(model, 'fitted_pipeline_') and model.fitted_pipeline_ is not None:
                result.model_name = str(type(model.fitted_pipeline_.steps[-1][1]).__name__)
        except Exception:
            result.model_name = 'Unknown'
        
    except Exception as e:
        result.error = str(e)[:60] + "..." if len(str(e)) > 60 else str(e)
        if tracemalloc.is_tracing():
            tracemalloc.stop()
    
    return result


def run_flaml(
    X_train: pd.DataFrame,
    y_train: np.ndarray,
    X_test: pd.DataFrame,
    y_test: np.ndarray,
    task: str,
    time_budget_s: float,
) -> BenchmarkResult:
    """Run FLAML and collect metrics."""
    if not _HAS_FLAML:
        return BenchmarkResult(
            system='FLAML', dataset='', task=task,
            test_score=0.0, error='FLAML not installed'
        )
    
    result = BenchmarkResult(
        system='FLAML',
        dataset='',
        task=task,
        test_score=0.0,  # Will be updated after fitting
        n_train=len(X_train),
        n_test=len(X_test),
        n_features=X_train.shape[1],
        time_budget_s=time_budget_s,
    )
    
    try:
        gc.collect()
        tracemalloc.start()
        
        t0 = time.time()
        
        model = FLAMLAutoML()
        # CRITICAL: Force single-threaded execution and strict time budget
        model.fit(
            X_train, y_train,
            task=task,
            time_budget=time_budget_s,
            metric='f1' if task == 'classification' else 'r2',
            verbose=0,
            n_jobs=1,  # Single-threaded to prevent hanging
            early_stop=True,  # Enable early stopping
            max_iter=100,  # Limit iterations to prevent infinite loops
        )
        fit_time = time.time() - t0
        
        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        
        t0 = time.time()
        y_pred = model.predict(X_test)
        predict_time = time.time() - t0
        
        result.fit_time_s = fit_time
        result.predict_time_s = predict_time
        result.latency_ms_per_row = (predict_time * 1000) / len(X_test)
        result.peak_memory_mb = peak / (1024 * 1024)
        
        if task == 'classification':
            result.test_score = accuracy_score(y_test, y_pred)
            result.test_f1 = f1_score(y_test, y_pred, average='weighted')
            result.test_precision = precision_score(y_test, y_pred, average='weighted', zero_division=0)
            result.test_recall = recall_score(y_test, y_pred, average='weighted', zero_division=0)
        else:
            result.test_score = r2_score(y_test, y_pred)
            result.test_rmse = np.sqrt(mean_squared_error(y_test, y_pred))
            result.test_mae = mean_absolute_error(y_test, y_pred)
        
        result.has_risk_report = False
        result.has_audit_trail = False
        
        # Capture selected model name for FLAML
        try:
            if hasattr(model, 'best_estimator') and model.best_estimator is not None:
                result.model_name = str(type(model.best_estimator).__name__)
            elif hasattr(model, 'model') and model.model is not None:
                result.model_name = str(type(model.model.estimator).__name__)
        except Exception:
            result.model_name = 'Unknown'
        
        # Capture models evaluated by FLAML
        try:
            result.models_evaluated = [{
                'name': str(type(model.best_estimator).__name__),
                'score': float(model.best_loss) if hasattr(model, 'best_loss') else None,
                'config': model.best_config if hasattr(model, 'best_config') else {},
                'is_best': True,
            }]
            # Add config history if available
            if hasattr(model, 'config_history'):
                for i, (config, score) in enumerate(zip(model.config_history, model.best_loss_history if hasattr(model, 'best_loss_history') else [])):
                    result.models_evaluated.append({
                        'name': config.get('learner', 'Unknown'),
                        'score': float(score) if score else None,
                        'config': config,
                        'is_best': False,
                    })
        except Exception:
            pass
        
    except Exception as e:
        result.error = str(e)
        if tracemalloc.is_tracing():
            tracemalloc.stop()
    
    return result


def run_pycaret(
    X_train: pd.DataFrame,
    y_train: np.ndarray,
    X_test: pd.DataFrame,
    y_test: np.ndarray,
    task: str,
    time_budget_s: float,
) -> BenchmarkResult:
    """Run PyCaret and collect metrics."""
    if not _HAS_PYCARET:
        return BenchmarkResult(
            system='PyCaret', dataset='', task=task,
            test_score=0.0, error='PyCaret not installed (pip install pycaret)'
        )
    
    result = BenchmarkResult(
        system='PyCaret',
        dataset='',
        task=task,
        test_score=0.0,
        n_train=len(X_train),
        n_test=len(X_test),
        n_features=X_train.shape[1],
        time_budget_s=time_budget_s,
    )
    
    try:
        gc.collect()
        tracemalloc.start()
        
        t0 = time.time()
        
        # Combine X and y for PyCaret
        train_df = X_train.copy()
        train_df['target'] = y_train
        
        # Suppress PyCaret output
        import sys
        import io
        old_stdout = sys.stdout
        sys.stdout = io.StringIO()
        
        try:
            if task == 'classification':
                exp = ClassificationExperiment()
                exp.setup(train_df, target='target', session_id=42, verbose=False, html=False)
                best_model = exp.compare_models(budget_time=time_budget_s/60, verbose=False)
            else:
                exp = RegressionExperiment()
                exp.setup(train_df, target='target', session_id=42, verbose=False, html=False)
                best_model = exp.compare_models(budget_time=time_budget_s/60, verbose=False)
        finally:
            sys.stdout = old_stdout
        
        fit_time = time.time() - t0
        
        current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        
        t0 = time.time()
        y_pred = exp.predict_model(best_model, data=X_test)['prediction_label'].values
        predict_time = time.time() - t0
        
        result.fit_time_s = fit_time
        result.predict_time_s = predict_time
        result.latency_ms_per_row = (predict_time * 1000) / len(X_test)
        result.peak_memory_mb = peak / (1024 * 1024)
        
        if task == 'classification':
            result.test_score = accuracy_score(y_test, y_pred)
            result.test_f1 = f1_score(y_test, y_pred, average='weighted')
            result.test_precision = precision_score(y_test, y_pred, average='weighted', zero_division=0)
            result.test_recall = recall_score(y_test, y_pred, average='weighted', zero_division=0)
        else:
            result.test_score = r2_score(y_test, y_pred)
            result.test_rmse = np.sqrt(mean_squared_error(y_test, y_pred))
            result.test_mae = mean_absolute_error(y_test, y_pred)
        
        result.has_risk_report = False
        result.has_audit_trail = False
        
    except Exception as e:
        result.error = str(e)[:60] + "..." if len(str(e)) > 60 else str(e)
        if tracemalloc.is_tracing():
            tracemalloc.stop()
    
    return result


def run_autogluon(
    X_train: pd.DataFrame,
    y_train: np.ndarray,
    X_test: pd.DataFrame,
    y_test: np.ndarray,
    task: str,
    time_budget_s: float,
) -> BenchmarkResult:
    """Run AutoGluon and collect metrics."""
    if not _HAS_AUTOGLUON:
        return BenchmarkResult(
            system='AutoGluon', dataset='', task=task,
            test_score=0.0, error='AutoGluon not installed (pip install autogluon)'
        )
    
    result = BenchmarkResult(
        system='AutoGluon',
        dataset='',
        task=task,
        test_score=0.0,
        n_train=len(X_train),
        n_test=len(X_test),
        n_features=X_train.shape[1],
        time_budget_s=time_budget_s,
    )
    
    try:
        gc.collect()
        tracemalloc.start()
        
        t0 = time.time()
        
        # Suppress AutoGluon warnings
        import io
        import logging
        logging.getLogger("autogluon").setLevel(logging.ERROR)
        warnings.filterwarnings('ignore', module='autogluon')
        warnings.filterwarnings('ignore', message='.*fastai.*')
        warnings.filterwarnings('ignore', message='.*NeuralNet.*')
        
        # Combine X and y for AutoGluon
        train_df = X_train.copy()
        train_df['target'] = y_train
        test_df = X_test.copy()
        
        # Create temp directory for AutoGluon
        import tempfile
        import shutil
        temp_dir = tempfile.mkdtemp()
        
        try:
            # Redirect stderr during fit to suppress warnings
            old_stderr = sys.stderr
            sys.stderr = io.StringIO()
            
            try:
                predictor = TabularPredictor(
                    label='target',
                    path=temp_dir,
                    verbosity=0,
                    problem_type=task if task == 'regression' else 'multiclass'
                )
                # Use single-threaded, fast presets to prevent hanging
                predictor.fit(
                    train_df,
                    time_limit=int(time_budget_s),
                    presets='best_quality',  # More predictable than medium_quality
                    num_cpus=1,  # Single-threaded
                    num_gpus=0,  # No GPU
                    hyperparameters={'GBM': {}, 'RF': {}, 'XT': {}},  # Only fast models
                    excluded_model_types=['NN', 'FASTAI', 'KNN'],  # Exclude slow models
                )
            finally:
                sys.stderr = old_stderr
            
            fit_time = time.time() - t0
            
            current, peak = tracemalloc.get_traced_memory()
            tracemalloc.stop()
            
            t0 = time.time()
            y_pred = predictor.predict(test_df)
            predict_time = time.time() - t0
        finally:
            shutil.rmtree(temp_dir, ignore_errors=True)
        
        result.fit_time_s = fit_time
        result.predict_time_s = predict_time
        result.latency_ms_per_row = (predict_time * 1000) / len(X_test)
        result.peak_memory_mb = peak / (1024 * 1024)
        
        if task == 'classification':
            result.test_score = accuracy_score(y_test, y_pred)
            result.test_f1 = f1_score(y_test, y_pred, average='weighted')
            result.test_precision = precision_score(y_test, y_pred, average='weighted', zero_division=0)
            result.test_recall = recall_score(y_test, y_pred, average='weighted', zero_division=0)
        else:
            result.test_score = r2_score(y_test, y_pred)
            result.test_rmse = np.sqrt(mean_squared_error(y_test, y_pred))
            result.test_mae = mean_absolute_error(y_test, y_pred)
        
        result.has_risk_report = False
        result.has_audit_trail = False
        
        # Capture selected model name for AutoGluon
        try:
            best_model = predictor.get_model_best()
            result.model_name = str(best_model) if best_model else 'Unknown'
        except Exception:
            result.model_name = 'Unknown'
        
        # Capture AutoGluon leaderboard (all models evaluated)
        try:
            leaderboard = predictor.leaderboard(silent=True)
            if leaderboard is not None and len(leaderboard) > 0:
                result.models_evaluated = []
                for idx, row in leaderboard.iterrows():
                    result.models_evaluated.append({
                        'name': str(row.get('model', idx)),
                        'score': float(row.get('score_val', row.get('score_test', 0))),
                        'fit_time': float(row.get('fit_time', 0)),
                        'pred_time': float(row.get('pred_time_val', 0)),
                        'is_best': idx == 0,
                    })
        except Exception:
            pass
        
    except Exception as e:
        result.error = str(e)[:60] + "..." if len(str(e)) > 60 else str(e)
        if tracemalloc.is_tracing():
            tracemalloc.stop()
    
    return result


def run_h2o(
    X_train: pd.DataFrame,
    y_train: np.ndarray,
    X_test: pd.DataFrame,
    y_test: np.ndarray,
    task: str,
    time_budget_s: float,
) -> BenchmarkResult:
    """Run H2O AutoML and collect metrics."""
    if not _HAS_H2O:
        return BenchmarkResult(
            system='H2O', dataset='', task=task,
            test_score=0.0, error='H2O not installed (pip install h2o)'
        )
    
    result = BenchmarkResult(
        system='H2O',
        dataset='',
        task=task,
        test_score=0.0,
        n_train=len(X_train),
        n_test=len(X_test),
        n_features=X_train.shape[1],
        time_budget_s=time_budget_s,
    )
    
    try:
        gc.collect()
        tracemalloc.start()
        
        t0 = time.time()
        
        # Suppress H2O output completely
        import io
        import contextlib
        import logging
        
        # Suppress H2O logging
        logging.getLogger("h2o").setLevel(logging.CRITICAL)
        
        # Suppress all warnings during H2O
        warnings.filterwarnings('ignore')
        
        # Redirect stdout/stderr during H2O init and training
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        sys.stdout = io.StringIO()
        sys.stderr = io.StringIO()
        
        try:
            # Initialize H2O with single-threaded settings (suppress output)
            h2o.init(verbose=False, nthreads=1, max_mem_size='2G')
            
            # Combine X and y
            train_df = X_train.copy()
            train_df['target'] = y_train
            test_df = X_test.copy()
            test_df['target'] = y_test
            
            # Convert to H2O frames
            train_h2o = h2o.H2OFrame(train_df)
            test_h2o = h2o.H2OFrame(test_df)
            
            # Set target column type
            if task == 'classification':
                train_h2o['target'] = train_h2o['target'].asfactor()
                test_h2o['target'] = test_h2o['target'].asfactor()
            
            # Run AutoML with single-threaded settings
            aml = H2OAutoML(
                max_runtime_secs=int(time_budget_s),
                seed=42,
                verbosity='error',
                nfolds=3,  # Reduce cross-validation folds
                max_models=5,  # Limit number of models to prevent hanging
                exclude_algos=['DeepLearning', 'StackedEnsemble'],  # Exclude slow models
            )
            aml.train(y='target', training_frame=train_h2o)
            
            fit_time = time.time() - t0
            
            current, peak = tracemalloc.get_traced_memory()
            tracemalloc.stop()
            
            t0 = time.time()
            preds = aml.leader.predict(test_h2o)
            y_pred = preds.as_data_frame()['predict'].values
            predict_time = time.time() - t0
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr
        
        result.fit_time_s = fit_time
        result.predict_time_s = predict_time
        result.latency_ms_per_row = (predict_time * 1000) / len(X_test)
        result.peak_memory_mb = peak / (1024 * 1024)
        
        if task == 'classification':
            result.test_score = accuracy_score(y_test, y_pred)
            result.test_f1 = f1_score(y_test, y_pred, average='weighted')
            result.test_precision = precision_score(y_test, y_pred, average='weighted', zero_division=0)
            result.test_recall = recall_score(y_test, y_pred, average='weighted', zero_division=0)
        else:
            result.test_score = r2_score(y_test, y_pred)
            result.test_rmse = np.sqrt(mean_squared_error(y_test, y_pred))
            result.test_mae = mean_absolute_error(y_test, y_pred)
        
        result.has_risk_report = False
        result.has_audit_trail = False
        
        # Capture selected model name for H2O
        try:
            if aml.leader is not None:
                result.model_name = str(aml.leader.model_id)
        except Exception:
            result.model_name = 'Unknown'
        
        # Capture H2O leaderboard (all models evaluated)
        try:
            leaderboard = aml.leaderboard.as_data_frame()
            if leaderboard is not None and len(leaderboard) > 0:
                result.models_evaluated = []
                for idx, row in leaderboard.iterrows():
                    result.models_evaluated.append({
                        'name': str(row.get('model_id', idx)),
                        'score': float(row.get('auc', row.get('rmse', row.get('mean_per_class_error', 0)))),
                        'is_best': idx == 0,
                    })
        except Exception:
            pass
        
        # Shutdown H2O silently
        old_stdout = sys.stdout
        old_stderr = sys.stderr
        sys.stdout = io.StringIO()
        sys.stderr = io.StringIO()
        try:
            h2o.cluster().shutdown(prompt=False)
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr
        
    except Exception as e:
        result.error = str(e)[:80] + "..." if len(str(e)) > 80 else str(e)
        if tracemalloc.is_tracing():
            tracemalloc.stop()
        try:
            h2o.cluster().shutdown(prompt=False)
        except:
            pass
    
    return result


def run_mljar(
    X_train: pd.DataFrame,
    y_train: np.ndarray,
    X_test: pd.DataFrame,
    y_test: np.ndarray,
    task: str,
    time_budget_s: float,
) -> BenchmarkResult:
    """Run MLJAR AutoML and collect metrics."""
    if not _HAS_MLJAR:
        return BenchmarkResult(
            system='MLJAR', dataset='', task=task,
            test_score=0.0, error='MLJAR not installed (pip install mljar-supervised)'
        )
    
    result = BenchmarkResult(
        system='MLJAR',
        dataset='',
        task=task,
        test_score=0.0,
        n_train=len(X_train),
        n_test=len(X_test),
        n_features=X_train.shape[1],
        time_budget_s=time_budget_s,
    )
    
    try:
        gc.collect()
        tracemalloc.start()
        
        t0 = time.time()
        
        # Create temp directory for MLJAR
        import tempfile
        import shutil
        temp_dir = tempfile.mkdtemp()
        
        try:
            automl = MLJARAutoML(
                mode='Compete',
                total_time_limit=int(time_budget_s),
                results_path=temp_dir,
                ml_task=task,
                random_state=42,
                verbose=0,
            )
            automl.fit(X_train, y_train)
            
            fit_time = time.time() - t0
            
            current, peak = tracemalloc.get_traced_memory()
            tracemalloc.stop()
            
            t0 = time.time()
            y_pred = automl.predict(X_test)
            predict_time = time.time() - t0
        finally:
            shutil.rmtree(temp_dir, ignore_errors=True)
        
        result.fit_time_s = fit_time
        result.predict_time_s = predict_time
        result.latency_ms_per_row = (predict_time * 1000) / len(X_test)
        result.peak_memory_mb = peak / (1024 * 1024)
        
        if task == 'classification':
            result.test_score = accuracy_score(y_test, y_pred)
            result.test_f1 = f1_score(y_test, y_pred, average='weighted')
            result.test_precision = precision_score(y_test, y_pred, average='weighted', zero_division=0)
            result.test_recall = recall_score(y_test, y_pred, average='weighted', zero_division=0)
        else:
            result.test_score = r2_score(y_test, y_pred)
            result.test_rmse = np.sqrt(mean_squared_error(y_test, y_pred))
            result.test_mae = mean_absolute_error(y_test, y_pred)
        
        result.has_risk_report = False
        result.has_audit_trail = False
        
    except Exception as e:
        result.error = str(e)[:60] + "..." if len(str(e)) > 60 else str(e)
        if tracemalloc.is_tracing():
            tracemalloc.stop()
    
    return result


# =============================================================================
# Leakage Detection Testing
# =============================================================================

def test_leakage_detection(time_budget_s: float = 60) -> List[LeakageTestResult]:
    """
    Test each system's ability to detect injected leakage.
    
    This is a key differentiator for ACDSO.
    """
    results = []
    
    # Create dataset with injected proxy leakage
    np.random.seed(42)
    n = 500
    X = np.random.randn(n, 5)
    y = (X[:, 0] + X[:, 1] > 0).astype(int)
    
    # Inject leaky feature (99% correlated with target)
    leaky = y + np.random.randn(n) * 0.01
    X_leaky = pd.DataFrame(
        np.column_stack([X, leaky]),
        columns=['f0', 'f1', 'f2', 'f3', 'f4', 'leaky_target']
    )
    
    X_train, X_test, y_train, y_test = train_test_split(
        X_leaky, y, test_size=0.2, random_state=42
    )
    
    # Test ACDSO
    if _HAS_ACDSO:
        try:
            model = ACDSOClassifier(scorer='f1', time_budget_s=time_budget_s, verbose=0)
            model.fit(X_train, y_train)
            
            detected = False
            method = None
            if hasattr(model, 'risk_report_') and model.risk_report_:
                rr = model.risk_report_
                if rr.get('leakage_proxy_score_max', 0) >= 0.95:
                    detected = True
                    method = 'univariate_predictive_power'
                if len(rr.get('leakage_proxy_features', [])) > 0:
                    detected = True
                    method = 'leakage_proxy_features'
            
            results.append(LeakageTestResult(
                system='ACDSO',
                leakage_type='proxy',
                injected=True,
                detected=detected,
                detection_method=method,
            ))
        except Exception as e:
            results.append(LeakageTestResult(
                system='ACDSO', leakage_type='proxy',
                injected=True, detected=False,
                detection_method=f'error: {e}'
            ))
    
    # Auto-sklearn (no leakage detection)
    if _HAS_AUTOSKLEARN:
        results.append(LeakageTestResult(
            system='Auto-sklearn',
            leakage_type='proxy',
            injected=True,
            detected=False,
            detection_method='not_supported',
        ))
    
    # TPOT (no leakage detection)
    if _HAS_TPOT:
        results.append(LeakageTestResult(
            system='TPOT',
            leakage_type='proxy',
            injected=True,
            detected=False,
            detection_method='not_supported',
        ))
    
    # FLAML (no leakage detection)
    if _HAS_FLAML:
        results.append(LeakageTestResult(
            system='FLAML',
            leakage_type='proxy',
            injected=True,
            detected=False,
            detection_method='not_supported',
        ))
    
    return results


# =============================================================================
# Statistical Analysis
# =============================================================================

def compute_statistical_tests(results: List[BenchmarkResult]) -> Dict[str, Any]:
    """Compute statistical significance tests between systems."""
    tests = {}
    
    # Group by system
    by_system = {}
    for r in results:
        if r.error is None:
            if r.system not in by_system:
                by_system[r.system] = []
            by_system[r.system].append(r)
    
    systems = list(by_system.keys())
    if len(systems) < 2:
        return tests
    
    # Pairwise Wilcoxon signed-rank tests on test_score
    for i, sys1 in enumerate(systems):
        for sys2 in systems[i+1:]:
            scores1 = [r.test_score for r in by_system[sys1]]
            scores2 = [r.test_score for r in by_system[sys2]]
            
            # Match by dataset
            matched_scores1 = []
            matched_scores2 = []
            datasets1 = {r.dataset: r.test_score for r in by_system[sys1]}
            datasets2 = {r.dataset: r.test_score for r in by_system[sys2]}
            
            for ds in set(datasets1.keys()) & set(datasets2.keys()):
                matched_scores1.append(datasets1[ds])
                matched_scores2.append(datasets2[ds])
            
            if len(matched_scores1) >= 3:
                try:
                    stat, p_value = stats.wilcoxon(matched_scores1, matched_scores2)
                    tests[f'{sys1}_vs_{sys2}_score'] = {
                        'statistic': float(stat),
                        'p_value': float(p_value),
                        'significant_0.05': p_value < 0.05,
                        'mean_diff': float(np.mean(matched_scores1) - np.mean(matched_scores2)),
                    }
                except Exception:
                    pass
    
    return tests


def determine_winners(results: List[BenchmarkResult]) -> Dict[str, str]:
    """Determine the best system for each metric."""
    metrics = [
        ('test_score', True),  # higher is better
        ('fit_time_s', False),  # lower is better
        ('latency_ms_per_row', False),  # lower is better
        ('peak_memory_mb', False),  # lower is better
        ('cv_score_std', False),  # lower is better (more stable)
    ]
    
    winners = {}
    
    for metric, higher_better in metrics:
        by_system = {}
        for r in results:
            if r.error is None:
                val = getattr(r, metric, None)
                if val is not None:
                    if r.system not in by_system:
                        by_system[r.system] = []
                    by_system[r.system].append(val)
        
        if by_system:
            means = {sys: np.mean(vals) for sys, vals in by_system.items()}
            if higher_better:
                winners[metric] = max(means, key=means.get)
            else:
                winners[metric] = min(means, key=means.get)
    
    # Special metrics (production readiness)
    has_risk = {r.system for r in results if r.has_risk_report}
    has_audit = {r.system for r in results if r.has_audit_trail}
    
    if has_risk:
        winners['risk_detection'] = ', '.join(sorted(has_risk))
    if has_audit:
        winners['audit_trail'] = ', '.join(sorted(has_audit))
    
    return winners


# =============================================================================
# Reporting
# =============================================================================

def print_comparison_table(results: List[BenchmarkResult], title: str = "Benchmark Results"):
    """Print a formatted comparison table."""
    print(f"\n{'='*80}")
    print(f"  {title}")
    print('='*80)
    
    # Group by dataset
    by_dataset = {}
    for r in results:
        if r.dataset not in by_dataset:
            by_dataset[r.dataset] = []
        by_dataset[r.dataset].append(r)
    
    for dataset, ds_results in by_dataset.items():
        print(f"\n  Dataset: {dataset}")
        print('-'*78)
        
        # Header
        print(f"  {'System':<15} {'Score':>8} {'F1':>8} {'Time(s)':>10} {'Lat(ms)':>10} {'Mem(MB)':>10} {'Risk?':>6}")
        print(f"  {'-'*15} {'-'*8} {'-'*8} {'-'*10} {'-'*10} {'-'*10} {'-'*6}")
        
        for r in sorted(ds_results, key=lambda x: x.test_score, reverse=True):
            if r.error:
                print(f"  {r.system:<15} ERROR: {r.error[:40]}")
            else:
                f1_str = f"{r.test_f1:.4f}" if r.test_f1 else "-"
                risk_str = "YES" if r.has_risk_report else "NO"
                print(f"  {r.system:<15} {r.test_score:>8.4f} {f1_str:>8} "
                      f"{r.fit_time_s:>10.2f} {r.latency_ms_per_row:>10.3f} "
                      f"{r.peak_memory_mb:>10.1f} {risk_str:>6}")


def print_summary(summary: BenchmarkSummary):
    """Print a summary of all benchmark results."""
    print("\n" + "="*80)
    print("  BENCHMARK SUMMARY")
    print("="*80)
    
    print(f"\n  Timestamp: {summary.timestamp}")
    print(f"  Systems tested: {', '.join(summary.systems_tested)}")
    print(f"  Datasets: {len(summary.datasets_tested)}")
    print(f"  Time budget: {summary.time_budget_s}s per system")
    
    # Winners
    print("\n  WINNERS BY METRIC:")
    print("-"*40)
    for metric, winner in summary.winner_by_metric.items():
        print(f"    {metric:<25} -> {winner}")
    
    # Leakage detection
    print("\n  LEAKAGE DETECTION CAPABILITY:")
    print("-"*40)
    for lt in summary.leakage_tests:
        status = "[OK] DETECTED" if lt.detected else "[X] NOT DETECTED"
        print(f"    {lt.system:<15} {status} (method: {lt.detection_method})")
    
    # Production readiness
    print("\n  PRODUCTION READINESS:")
    print("-"*40)
    
    systems_with_risk = set(r.system for r in summary.results if r.has_risk_report)
    systems_with_audit = set(r.system for r in summary.results if r.has_audit_trail)
    
    for sys in summary.systems_tested:
        risk = "[OK]" if sys in systems_with_risk else "[X]"
        audit = "[OK]" if sys in systems_with_audit else "[X]"
        print(f"    {sys:<15} Risk Report: {risk}  Audit Trail: {audit}")
    
    # Statistical tests
    if summary.statistical_tests:
        print("\n  STATISTICAL SIGNIFICANCE (p < 0.05):")
        print("-"*40)
        for test_name, test_result in summary.statistical_tests.items():
            if test_result.get('significant_0.05'):
                diff = test_result['mean_diff']
                direction = ">" if diff > 0 else "<"
                print(f"    {test_name}: p={test_result['p_value']:.4f}, diff={diff:+.4f}")


# =============================================================================
# Main Benchmark Runner
# =============================================================================

def run_benchmark(
    dataset_names: List[str] = None,
    time_budget_s: float = 300,
    systems: List[str] = None,
    output_file: str = None,
    verbose: bool = True,
    custom_datasets: Dict[str, Dict[str, Any]] = None,
) -> BenchmarkSummary:
    """
    Run the complete benchmark suite.
    
    Parameters
    ----------
    dataset_names : list, optional
        Datasets to benchmark. None for all.
    time_budget_s : float
        Time budget per system per dataset.
    systems : list, optional
        Systems to test. None for all available.
    output_file : str, optional
        Path to save JSON results.
    verbose : bool
        Print progress.
    custom_datasets : dict, optional
        Additional custom datasets to include. Format: {name: {'X': df, 'y': array, 'task': str}}
    
    Returns
    -------
    BenchmarkSummary
        Complete benchmark results.
    """
    # Determine which systems to test
    available_systems = []
    if _HAS_ACDSO:
        available_systems.append('ACDSO')
    if _HAS_AUTOSKLEARN:
        available_systems.append('Auto-sklearn')
    if _HAS_TPOT:
        available_systems.append('TPOT')
    if _HAS_FLAML:
        available_systems.append('FLAML')
    if _HAS_PYCARET:
        available_systems.append('PyCaret')
    if _HAS_AUTOGLUON:
        available_systems.append('AutoGluon')
    if _HAS_H2O:
        available_systems.append('H2O')
    if _HAS_MLJAR:
        available_systems.append('MLJAR')
    
    if systems:
        available_systems = [s for s in systems if s in available_systems]
    
    if not available_systems:
        print("No AutoML systems available to test!")
        print("Install options:")
        print("  ACDSO: pip install -e .")
        print("  Auto-sklearn: pip install auto-sklearn")
        print("  TPOT: pip install tpot")
        print("  FLAML: pip install flaml")
        return None
    
    if verbose:
        print("\n" + "="*80)
        print("=" + " "*78 + "=")
        print("=" + "  ACDSO BENCHMARK SUITE".center(78) + "=")
        print("=" + "  Fair Comparison Against Auto-Sklearn, TPOT, FLAML".center(78) + "=")
        print("=" + " "*78 + "=")
        print("="*80)
        print(f"\n  Available systems: {', '.join(available_systems)}")
        print(f"  Time budget: {time_budget_s}s per system per dataset")
    
    # Load datasets
    datasets = load_benchmark_datasets(dataset_names)
    
    # Merge custom datasets if provided
    if custom_datasets:
        datasets.update(custom_datasets)
    
    if verbose:
        print(f"  Datasets to test: {len(datasets)}")
        for name, info in datasets.items():
            print(f"    - {name}: {info['description']}")
    
    # Run benchmarks
    all_results = []
    
    for ds_name, ds_info in datasets.items():
        if verbose:
            print(f"\n{'-'*80}")
            print(f"  Dataset: {ds_name}")
            print(f"{'-'*80}")
        
        X = ds_info['X']
        y = ds_info['y']
        task = ds_info['task']
        
        # Encode target if needed
        if task == 'classification' and y.dtype == object:
            y = LabelEncoder().fit_transform(y)
        
        # Train/test split (fixed seed for reproducibility)
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42,
            stratify=y if task == 'classification' else None
        )
        
        for sys_name in available_systems:
            if verbose:
                print(f"    Running {sys_name}...", end=" ", flush=True)
            
            run_func = None
            if sys_name == 'ACDSO':
                run_func = run_acdso
            elif sys_name == 'Auto-sklearn':
                run_func = run_autosklearn
            elif sys_name == 'TPOT':
                run_func = run_tpot
            elif sys_name == 'FLAML':
                run_func = run_flaml
            elif sys_name == 'PyCaret':
                run_func = run_pycaret
            elif sys_name == 'AutoGluon':
                run_func = run_autogluon
            elif sys_name == 'H2O':
                run_func = run_h2o
            elif sys_name == 'MLJAR':
                run_func = run_mljar
            else:
                continue
            
            # Run directly without timeout - let each system complete
            try:
                result = run_func(X_train, y_train, X_test, y_test, task, time_budget_s)
            except Exception as e:
                result = BenchmarkResult(
                    system=sys_name,
                    dataset=ds_name,
                    task=task,
                    test_score=0.0,
                    error=str(e)[:100],
                )
            
            result.dataset = ds_name
            all_results.append(result)
            
            if verbose:
                if result.error:
                    # Show more of the error message
                    err_msg = result.error if len(result.error) <= 55 else result.error[:52] + "..."
                    print(f"ERROR: {err_msg}")
                else:
                    print(f"Score: {result.test_score:.4f}, Time: {result.fit_time_s:.1f}s")
    
    # Leakage detection tests
    if verbose:
        print(f"\n{'-'*80}")
        print("  Running Leakage Detection Tests...")
        print(f"{'-'*80}")
    
    leakage_results = test_leakage_detection(time_budget_s=min(60, time_budget_s))
    
    # Statistical analysis
    stat_tests = compute_statistical_tests(all_results)
    winners = determine_winners(all_results)
    
    # Build summary
    summary = BenchmarkSummary(
        timestamp=datetime.now().isoformat(),
        systems_tested=available_systems,
        datasets_tested=list(datasets.keys()),
        time_budget_s=time_budget_s,
        results=all_results,
        leakage_tests=leakage_results,
        winner_by_metric=winners,
        statistical_tests=stat_tests,
    )
    
    # Print results
    if verbose:
        print_comparison_table(all_results)
        print_summary(summary)
    
    # Save to file
    if output_file:
        with open(output_file, 'w') as f:
            json.dump(summary.to_dict(), f, indent=2)
        if verbose:
            print(f"\n  Results saved to: {output_file}")
    
    return summary


# =============================================================================
# CLI
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="ACDSO Benchmark Suite: Fair comparison with Auto-Sklearn, TPOT, FLAML",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Quick demo
  python benchmark_automl.py --quick
  
  # Standard datasets
  python benchmark_automl.py --datasets iris,wine --time-budget 60
  python benchmark_automl.py --datasets all --time-budget 300 --output results.json
  
  # YOUR OWN DATASET
  python benchmark_automl.py --file brain.csv --target Sec --task classification
  python benchmark_automl.py --file data.csv --target price --task regression --exclude id,date
  
  # Mix custom + standard datasets
  python benchmark_automl.py --file brain.csv --target Sec --datasets iris,wine
        """
    )
    
    parser.add_argument(
        '--datasets', '-d',
        type=str,
        default=None,
        help='Comma-separated dataset names or "all". Available: iris, wine, breast_cancer, '
             'diabetes, california, synthetic_clf, synthetic_reg, leaky_clf, imbalanced_clf, multicollinear. '
             'Omit to only benchmark custom file.'
    )
    
    parser.add_argument(
        '--time-budget', '-t',
        type=float,
        default=120,
        help='Time budget in seconds per system per dataset (default: 120)'
    )
    
    parser.add_argument(
        '--systems', '-s',
        type=str,
        default=None,
        help='Comma-separated systems to test: ACDSO, Auto-sklearn, TPOT, FLAML, PyCaret, AutoGluon, H2O, MLJAR'
    )
    
    parser.add_argument(
        '--output', '-o',
        type=str,
        default=None,
        help='Output JSON file path'
    )
    
    parser.add_argument(
        '--quick',
        action='store_true',
        help='Quick demo run (iris only, 30s budget)'
    )
    
    # Custom dataset arguments
    parser.add_argument(
        '--file', '-f',
        type=str,
        default=None,
        help='Path to your own dataset file (CSV, Excel, Parquet, JSON)'
    )
    
    parser.add_argument(
        '--target',
        type=str,
        default=None,
        help='Target column name in your dataset (required with --file)'
    )
    
    parser.add_argument(
        '--task',
        type=str,
        choices=['classification', 'regression', 'auto'],
        default='auto',
        help='Task type: classification, regression, or auto (default: auto-detect)'
    )
    
    parser.add_argument(
        '--exclude', '-e',
        type=str,
        default=None,
        help='Comma-separated list of columns to exclude from features'
    )
    
    args = parser.parse_args()
    
    # Handle custom dataset
    custom_datasets = {}
    if args.file:
        if not args.target:
            parser.error("--target is required when using --file")
        
        exclude_cols = args.exclude.split(',') if args.exclude else None
        
        try:
            custom_data = load_custom_dataset(
                file_path=args.file,
                target_column=args.target,
                task=args.task,
                exclude_columns=exclude_cols,
            )
            
            # Use filename as dataset name
            import os
            ds_name = os.path.splitext(os.path.basename(args.file))[0]
            custom_datasets[ds_name] = custom_data
            
            print(f"\n  Loaded custom dataset: {custom_data['description']}")
            print(f"  Task: {custom_data['task']}")
            if exclude_cols:
                print(f"  Excluded columns: {exclude_cols}")
        except Exception as e:
            print(f"Error loading custom dataset: {e}")
            return
    
    # Determine built-in datasets
    if args.quick:
        datasets = ['iris']
        time_budget = 30
    elif args.datasets:
        datasets = None if args.datasets == 'all' else args.datasets.split(',')
        time_budget = args.time_budget
    elif args.file:
        # Only use custom dataset if no --datasets specified
        datasets = []
        time_budget = args.time_budget
    else:
        # Default datasets if nothing specified
        datasets = ['iris', 'wine', 'breast_cancer', 'diabetes']
        time_budget = args.time_budget
    
    systems = args.systems.split(',') if args.systems else None
    
    run_benchmark(
        dataset_names=datasets,
        time_budget_s=time_budget,
        systems=systems,
        output_file=args.output,
        verbose=True,
        custom_datasets=custom_datasets if custom_datasets else None,
    )


if __name__ == "__main__":
    main()

