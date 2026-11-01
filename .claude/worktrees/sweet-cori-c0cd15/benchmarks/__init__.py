"""
ACDSO Benchmarking Suite
========================

Fair comparison of ACDSO against other AutoML systems.

Usage:
    from benchmarks import run_benchmark
    
    summary = run_benchmark(
        dataset_names=['iris', 'wine'],
        time_budget_s=120,
        systems=['ACDSO', 'FLAML'],
    )
"""

from benchmarks.benchmark_automl import (
    run_benchmark,
    BenchmarkResult,
    BenchmarkSummary,
    LeakageTestResult,
    load_benchmark_datasets,
    test_leakage_detection,
)

__all__ = [
    'run_benchmark',
    'BenchmarkResult',
    'BenchmarkSummary',
    'LeakageTestResult',
    'load_benchmark_datasets',
    'test_leakage_detection',
]

