# ACDSO Benchmarking Suite

Fair, rigorous comparison of ACDSO against other AutoML systems.

## Quick Start

```bash
# Quick demo (30 seconds)
python benchmark_automl.py --quick

# Standard benchmark (2 minutes per system)
python benchmark_automl.py --datasets iris,wine,breast_cancer --time-budget 120

# Full benchmark with all datasets
python benchmark_automl.py --datasets all --time-budget 300 --output results.json
```

## What's Measured

### Traditional Metrics (All Systems)
| Metric | Description |
|--------|-------------|
| Test Score | Accuracy (classification) or R² (regression) |
| F1 Score | Weighted F1 for classification |
| Training Time | Wall-clock time to fit |
| Latency | Inference time per row (ms) |
| Memory | Peak memory usage (MB) |

### Production Metrics (ACDSO Advantage)
| Metric | ACDSO | Auto-Sklearn | TPOT | FLAML |
|--------|-------|--------------|------|-------|
| Leakage Detection | ✅ | ❌ | ❌ | ❌ |
| Risk Quantification | ✅ | ❌ | ❌ | ❌ |
| Audit Trail | ✅ | ❌ | ❌ | ❌ |
| Multicollinearity Check | ✅ | ❌ | ❌ | ❌ |
| Knee-Point Selection | ✅ | ❌ | Partial | ❌ |

## Fairness Principles

This benchmark follows these principles for fair comparison:

1. **Equal Time Budgets**: Each system gets the same wall-clock time
2. **Same Splits**: Identical train/test splits (fixed random seed)
3. **Standardized Metrics**: Same sklearn metrics for all systems
4. **Multiple Datasets**: Classification + regression, various sizes
5. **Statistical Testing**: Wilcoxon signed-rank tests for significance
6. **Reproducibility**: All random seeds fixed

## Datasets

### Standard Datasets
| Dataset | Task | Samples | Features | Classes |
|---------|------|---------|----------|---------|
| iris | Classification | 150 | 4 | 3 |
| wine | Classification | 178 | 13 | 3 |
| breast_cancer | Classification | 569 | 30 | 2 |
| diabetes | Regression | 442 | 10 | - |
| california | Regression | 2000 | 8 | - |
| synthetic_clf | Classification | 1000 | 20 | 2 |
| synthetic_reg | Regression | 1000 | 20 | - |

### Challenge Datasets (ACDSO Differentiators)
| Dataset | Challenge | ACDSO Advantage |
|---------|-----------|-----------------|
| leaky_clf | Injected target leakage | Detects leaky features |
| imbalanced_clf | 95/5 class imbalance | Quantifies imbalance risk |
| multicollinear | VIF > 100 | Detects multicollinearity |

## Example Output

```
████████████████████████████████████████████████████████████████████████████████
█                          ACDSO BENCHMARK SUITE                               █
█          Fair Comparison Against Auto-Sklearn, TPOT, FLAML                   █
████████████████████████████████████████████████████████████████████████████████

  Available systems: ACDSO, FLAML
  Time budget: 120s per system per dataset
  Datasets to test: 4

────────────────────────────────────────────────────────────────────────────────
  Dataset: iris
────────────────────────────────────────────────────────────────────────────────
    Running ACDSO... Score: 0.9667, Time: 4.2s
    Running FLAML... Score: 0.9667, Time: 3.1s

================================================================================
  BENCHMARK SUMMARY
================================================================================

  WINNERS BY METRIC:
----------------------------------------
    test_score                → ACDSO
    fit_time_s                → FLAML
    latency_ms_per_row        → ACDSO
    risk_detection            → ACDSO
    audit_trail               → ACDSO

  LEAKAGE DETECTION CAPABILITY:
----------------------------------------
    ACDSO           ✓ DETECTED (method: univariate_predictive_power)
    FLAML           ✗ NOT DETECTED (method: not_supported)

  PRODUCTION READINESS:
----------------------------------------
    ACDSO           Risk Report: ✓  Audit Trail: ✓
    FLAML           Risk Report: ✗  Audit Trail: ✗
```

## Installation Requirements

```bash
# Required
pip install -e .  # ACDSO

# Optional (for comparison)
pip install auto-sklearn  # Linux only, requires swig
pip install tpot
pip install flaml
```

## CLI Options

```
usage: benchmark_automl.py [-h] [--datasets DATASETS] [--time-budget TIME_BUDGET]
                           [--systems SYSTEMS] [--output OUTPUT] [--quick]

Options:
  --datasets, -d    Comma-separated dataset names or "all"
  --time-budget, -t Time budget in seconds per system (default: 120)
  --systems, -s     Systems to test: ACDSO, Auto-sklearn, TPOT, FLAML
  --output, -o      Output JSON file path
  --quick           Quick demo (iris only, 30s)
```

## Programmatic Usage

```python
from benchmarks.benchmark_automl import run_benchmark

# Run benchmark
summary = run_benchmark(
    dataset_names=['iris', 'wine', 'breast_cancer'],
    time_budget_s=120,
    systems=['ACDSO', 'FLAML'],
    output_file='results.json',
    verbose=True,
)

# Access results
for result in summary.results:
    print(f"{result.system} on {result.dataset}: {result.test_score:.4f}")
    if result.has_risk_report:
        print(f"  - Detected leakage: {result.detected_leakage}")
        print(f"  - Risk penalty: {result.risk_penalty}")
```

## Expected Results

Based on the unique features of ACDSO, we expect:

1. **Comparable Accuracy**: ACDSO should match or approach other AutoML systems
2. **Faster Training**: Smaller candidate pool with smarter selection
3. **Better Stability**: Lower CV variance due to risk-aware selection
4. **Unique Capabilities**: Only ACDSO provides:
   - Leakage detection
   - Risk quantification
   - Audit trails
   - Knee-point selection

## Contributing

To add new datasets or systems:

1. Add dataset loader in `load_benchmark_datasets()`
2. Add system wrapper function `run_<system>()`
3. Update availability check at top of file
4. Update this README

