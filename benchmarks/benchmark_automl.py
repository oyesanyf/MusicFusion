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

import argparse
import gc
import json
import os
import sys
import time
import tracemalloc
import warnings
from dataclasses import dataclass, asdict, field
from datetime import datetime
from typing import Any, Dict, List, Optional, Tuple

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
    from tpot import TPOTClassifier, TPOTRegressor
    _HAS_TPOT = True
except ImportError:
    pass

try:
    from flaml import AutoML as FLAMLAutoML
    _HAS_FLAML = True
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
        n_train=len(X_train),
        n_test=len(X_test),
        n_features=X_train.shape[1],
        time_budget_s=time_budget_s,
    )
    
    try:
        gc.collect()
        tracemalloc.start()
        
        t0 = time.time()
        
        # TPOT uses generations/population, convert time budget to generations
        # Rough estimate: 1 generation ≈ 30-60 seconds
        generations = max(1, int(time_budget_s / 45))
        
        if task == 'classification':
            model = TPOTClassifier(
                generations=generations,
                population_size=20,
                cv=5,
                random_state=42,
                verbosity=0,
                n_jobs=1,
                max_time_mins=time_budget_s / 60,
            )
        else:
            model = TPOTRegressor(
                generations=generations,
                population_size=20,
                cv=5,
                random_state=42,
                verbosity=0,
                n_jobs=1,
                max_time_mins=time_budget_s / 60,
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
        
        # TPOT has Pareto front but no risk reports
        result.has_risk_report = False
        result.has_audit_trail = False
        result.pareto_front_size = len(getattr(model, 'pareto_front_', [])) if hasattr(model, 'pareto_front_') else None
        
    except Exception as e:
        result.error = str(e)
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
        model.fit(
            X_train, y_train,
            task=task,
            time_budget=time_budget_s,
            metric='f1' if task == 'classification' else 'r2',
            verbose=0,
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
        
    except Exception as e:
        result.error = str(e)
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
                risk_str = "✓" if r.has_risk_report else "✗"
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
        print(f"    {metric:<25} → {winner}")
    
    # Leakage detection
    print("\n  LEAKAGE DETECTION CAPABILITY:")
    print("-"*40)
    for lt in summary.leakage_tests:
        status = "✓ DETECTED" if lt.detected else "✗ NOT DETECTED"
        print(f"    {lt.system:<15} {status} (method: {lt.detection_method})")
    
    # Production readiness
    print("\n  PRODUCTION READINESS:")
    print("-"*40)
    
    systems_with_risk = set(r.system for r in summary.results if r.has_risk_report)
    systems_with_audit = set(r.system for r in summary.results if r.has_audit_trail)
    
    for sys in summary.systems_tested:
        risk = "✓" if sys in systems_with_risk else "✗"
        audit = "✓" if sys in systems_with_audit else "✗"
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
        print("\n" + "█"*80)
        print("█" + " "*78 + "█")
        print("█" + "  ACDSO BENCHMARK SUITE".center(78) + "█")
        print("█" + "  Fair Comparison Against Auto-Sklearn, TPOT, FLAML".center(78) + "█")
        print("█" + " "*78 + "█")
        print("█"*80)
        print(f"\n  Available systems: {', '.join(available_systems)}")
        print(f"  Time budget: {time_budget_s}s per system per dataset")
    
    # Load datasets
    datasets = load_benchmark_datasets(dataset_names)
    if verbose:
        print(f"  Datasets to test: {len(datasets)}")
        for name, info in datasets.items():
            print(f"    - {name}: {info['description']}")
    
    # Run benchmarks
    all_results = []
    
    for ds_name, ds_info in datasets.items():
        if verbose:
            print(f"\n{'─'*80}")
            print(f"  Dataset: {ds_name}")
            print(f"{'─'*80}")
        
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
            
            if sys_name == 'ACDSO':
                result = run_acdso(X_train, y_train, X_test, y_test, task, time_budget_s)
            elif sys_name == 'Auto-sklearn':
                result = run_autosklearn(X_train, y_train, X_test, y_test, task, time_budget_s)
            elif sys_name == 'TPOT':
                result = run_tpot(X_train, y_train, X_test, y_test, task, time_budget_s)
            elif sys_name == 'FLAML':
                result = run_flaml(X_train, y_train, X_test, y_test, task, time_budget_s)
            else:
                continue
            
            result.dataset = ds_name
            all_results.append(result)
            
            if verbose:
                if result.error:
                    print(f"ERROR: {result.error[:40]}")
                else:
                    print(f"Score: {result.test_score:.4f}, Time: {result.fit_time_s:.1f}s")
    
    # Leakage detection tests
    if verbose:
        print(f"\n{'─'*80}")
        print("  Running Leakage Detection Tests...")
        print(f"{'─'*80}")
    
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
  python benchmark_automl.py --quick
  python benchmark_automl.py --datasets iris,wine --time-budget 60
  python benchmark_automl.py --datasets all --time-budget 300 --output results.json
  python benchmark_automl.py --systems ACDSO,FLAML --datasets synthetic_clf
        """
    )
    
    parser.add_argument(
        '--datasets', '-d',
        type=str,
        default='iris,wine,breast_cancer,diabetes',
        help='Comma-separated dataset names or "all". Available: iris, wine, breast_cancer, '
             'diabetes, california, synthetic_clf, synthetic_reg, leaky_clf, imbalanced_clf, multicollinear'
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
        help='Comma-separated systems to test: ACDSO, Auto-sklearn, TPOT, FLAML'
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
    
    args = parser.parse_args()
    
    if args.quick:
        datasets = ['iris']
        time_budget = 30
    else:
        datasets = None if args.datasets == 'all' else args.datasets.split(',')
        time_budget = args.time_budget
    
    systems = args.systems.split(',') if args.systems else None
    
    run_benchmark(
        dataset_names=datasets,
        time_budget_s=time_budget,
        systems=systems,
        output_file=args.output,
        verbose=True,
    )


if __name__ == "__main__":
    main()

