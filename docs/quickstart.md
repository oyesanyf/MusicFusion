# ACDSO Quick Start Guide

This guide will get you up and running with ACDSO in 5 minutes.

## Installation

```bash
pip install acdso
```

## Try the Demo (Easiest Way)

The fastest way to see ACDSO in action is to run the demo on your own data:

```bash
# Run on your CSV file
python demo.py --file your_data.csv

# Specify target column
python demo.py --file data.csv --target price --task regression

# Run all built-in demos
python demo.py
```

### Demo Arguments

| Argument | Short | Description |
|----------|-------|-------------|
| `--file` | `-f` | Path to your data file (CSV, Excel, Parquet, JSON) |
| `--target` | `-t` | Target column name (default: last column) |
| `--task` | | `auto`, `classification`, or `regression` |

The demo will automatically:
1. Load and preview your data
2. Run Auto EDA (quality assessment, correlations, anomalies)
3. Handle categorical encoding
4. Run risk-aware AutoML
5. Show evaluation metrics
6. Export audit trail and EDA report

## Basic Classification

```python
from acdso import ACDSOClassifier
from sklearn.datasets import make_classification
from sklearn.model_selection import train_test_split

# Generate sample data
X, y = make_classification(n_samples=1000, n_features=20, random_state=42)
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2)

# Create and fit classifier
clf = ACDSOClassifier(
    scorer="f1",
    cv=5,
    verbose=1,
)
clf.fit(X_train, y_train)

# Make predictions
predictions = clf.predict(X_test)
score = clf.score(X_test, y_test)
print(f"Test F1 Score: {score:.4f}")
```

## Basic Regression

```python
from acdso import ACDSORegressor
from sklearn.datasets import make_regression

X, y = make_regression(n_samples=1000, n_features=20, random_state=42)
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2)

reg = ACDSORegressor(
    scorer="r2",
    cv=5,
    verbose=1,
)
reg.fit(X_train, y_train)

predictions = reg.predict(X_test)
score = reg.score(X_test, y_test)
print(f"Test R² Score: {score:.4f}")
```

## With Production Constraints

```python
clf = ACDSOClassifier(
    scorer="f1",
    cv=5,
    max_latency_ms_per_row=10.0,  # Max 10ms per prediction
    max_peak_mem_mb=500.0,         # Max 500MB memory
    max_train_time_s=60.0,         # Max 60s per candidate
    preference="safe",             # Risk-averse selection
    verbose=1,
)
```

## Understanding the Results

### Risk Report

```python
clf.fit(X_train, y_train)

print("Risk Analysis:")
print(f"  Imbalance severity: {clf.risk_report_['imbalance_severity']:.2f}")
print(f"  VIF max: {clf.risk_report_['vif_max']:.2f}")
print(f"  Notes: {clf.risk_report_['notes']}")
```

### Pareto Front

```python
print("\nPareto-Optimal Models:")
for model in clf.pareto_front_:
    print(f"  {model['name']}: F1={model['metric']:.3f}, "
          f"latency={model['latency_ms_per_row']:.2f}ms")
```

### Audit Trail

```python
import json

# Save full audit trail
with open("audit.json", "w") as f:
    json.dump(clf.audit_trail_, f, indent=2)

# Key information
print(f"\nSelected Model: {clf.audit_trail_['selected']['name']}")
print(f"Total Candidates: {len(clf.audit_trail_['candidates_evaluated'])}")
```

## Adding LLM Analysis

```python
from acdso import ACDSOClassifier, create_advisor

# Create LLM advisor (uses OPENAI_API_KEY env var)
advisor = create_advisor("openai", model="gpt-4o-mini")

clf = ACDSOClassifier(
    llm_advisor=advisor,
    verbose=1,
)
clf.fit(X_train, y_train)

# LLM insights are included in risk_report_.notes
```

## Auto EDA

ACDSO also includes powerful Auto EDA capabilities:

```python
from acdso import ACDSOAutoEDA

# Create EDA analyzer
eda = ACDSOAutoEDA(verbose=1)

# Run analysis
report = eda.analyze(df, dataset_name="my_data")

# Print summary
eda.print_summary()

# Access results
print(f"Quality Score: {eda.data_quality_.quality_score}/100")
print(f"Recommendations: {report.recommendations}")

# Export
eda.to_html("eda_report.html")
eda.to_json("eda_report.json")
```

### Standalone Utility Functions

```python
from acdso.eda import (
    quick_profile,
    compare_datasets,
    validate_data,
    get_preprocessing_recommendations,
)

# Quick profiling
profiles = quick_profile(df)
print(profiles['age'].mean)

# Compare datasets
comparison = compare_datasets(train_df, test_df)
print(comparison['distribution_drift'])

# Validate data
result = validate_data(df, {'max_missing_pct': 0.1})
print(result['passed'])

# Get preprocessing recommendations
recs = get_preprocessing_recommendations(df, target='price')
for rec in recs:
    print(f"{rec['column']}: {rec['action']}")
```

## Next Steps

- See `examples/` for more detailed examples
- Read the full API reference in README.md
- Try the AutoML integration with `ACDSOMetaOptimizer`
- Explore Auto EDA features in `docs/auto_eda.md`

