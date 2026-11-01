# ACDSO Auto EDA Guide

## Overview

ACDSO v2.0 includes a powerful **Auto EDA** (Automatic Exploratory Data Analysis) module that applies the same multi-objective optimization approach as the AutoML components. It provides:

- **Statistical Profiling**: Distributions, correlations, outliers, skewness
- **Data Quality Assessment**: Missing values, duplicates, cardinality analysis
- **Anomaly Detection**: Z-score and multivariate outlier detection
- **Pareto Optimization**: Balance thoroughness, speed, and insight quality
- **LLM Insights**: AI-powered analysis (optional)
- **Integration with Popular EDA Tools**: ydata-profiling, sweetviz, dataprep, D-Tale

## Installation

```bash
# Basic (native Auto EDA)
pip install acdso

# With ydata-profiling
pip install acdso[ydata-profiling]

# With sweetviz
pip install acdso[sweetviz]

# With all EDA tools
pip install acdso[eda]
```

## Quick Start

### Command-Line Demo (Easiest)

The fastest way to run Auto EDA on your data is via the demo:

```bash
# Run Auto EDA + AutoML on your file
python demo.py --file your_data.csv

# Specify target column (Auto EDA runs on features only)
python demo.py --file data.csv --target price
```

This will automatically:
1. Load your data (CSV, Excel, Parquet, JSON)
2. Run full Auto EDA analysis
3. Run risk-aware AutoML
4. Export `*_eda_report.json` and `*_audit_trail.json`

### Python API Usage

```python
from acdso import ACDSOAutoEDA
import pandas as pd

# Load your data
df = pd.read_csv("your_data.csv")

# Create EDA analyzer
eda = ACDSOAutoEDA(verbose=1)

# Run analysis
report = eda.analyze(df, dataset_name="My Dataset")

# Print summary to console
eda.print_summary()

# Access results
print(f"Quality Score: {eda.data_quality_.quality_score}/100")
print(f"Issues: {eda.data_quality_.issues}")
print(f"Recommendations: {report.recommendations}")
```

### Export Options

```python
# Export as JSON (for programmatic use)
eda.to_json("eda_report.json")

# Export as HTML (for sharing/viewing)
eda.to_html("eda_report.html")

# Export as Markdown (for documentation)
eda.to_markdown("eda_report.md")

# Get as dictionary
report_dict = eda.to_dict()
```

## Configuration Options

### ACDSOAutoEDA Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `correlation_threshold` | float | 0.7 | Minimum correlation to flag as high |
| `outlier_method` | str | "iqr" | Method for outlier detection |
| `max_columns` | int | 100 | Maximum columns to profile |
| `max_correlations` | int | 50 | Maximum correlations to compute |
| `max_anomalies` | int | 100 | Maximum anomalies to return |
| `time_budget_s` | float | None | Maximum time budget for analysis |
| `llm_advisor` | callable | None | LLM advisor for insights |
| `verbose` | int | 0 | Verbosity level (0=silent, 1=progress) |

### Example with Custom Configuration

```python
from acdso import ACDSOAutoEDA, create_advisor

# With LLM insights
advisor = create_advisor("openai", model="gpt-4o-mini")

eda = ACDSOAutoEDA(
    correlation_threshold=0.8,      # Higher threshold
    max_columns=200,                # Profile more columns
    max_anomalies=50,               # Limit anomalies
    llm_advisor=advisor,            # Enable LLM insights
    verbose=1,
)

report = eda.analyze(df)
```

## Understanding Results

### Data Quality Score

The quality score (0-100) is calculated based on:

| Factor | Penalty | Description |
|--------|---------|-------------|
| Missing Data | Up to -30 | High missing percentages reduce score |
| Duplicates | Up to -20 | Duplicate rows reduce score |
| Constants | Up to -15 | Constant columns are penalized |
| ID-like Columns | Up to -10 | Columns that look like IDs |

### Column Profiles

Each column is profiled with:

**Numeric Columns:**
- Basic stats: mean, std, min, max, median, quartiles
- Distribution: skewness, kurtosis
- Quality flags: is_constant, is_id_like, is_highly_skewed, has_outliers
- Outliers: IQR and Z-score based counts

**Categorical Columns:**
- Cardinality: n_unique, cardinality_ratio
- Top value: most frequent value and percentage
- Quality flags: is_constant, is_id_like

### Accessing Specific Profiles

```python
# Get profile for a specific column
age_profile = eda.get_column_profile("age")

print(f"Mean: {age_profile.mean}")
print(f"Std: {age_profile.std}")
print(f"Skewness: {age_profile.skewness}")
print(f"Has Outliers: {age_profile.has_outliers}")
print(f"N Outliers (IQR): {age_profile.n_outliers_iqr}")
```

## Integration with Popular EDA Tools

### Check Available Backends

```python
from acdso.eda import get_available_backends

backends = get_available_backends()
print(backends)
# {'native': True, 'ydata-profiling': True, 'sweetviz': False, ...}
```

### ydata-profiling (pandas-profiling)

Full statistical profiling with interactive HTML reports.

```python
from acdso.eda import YDataProfilingAdapter

# Create adapter
adapter = YDataProfilingAdapter(
    minimal=False,      # Full analysis
    explorative=True,   # Extra details
    title="My EDA Report",
)

# Run analysis
adapter.analyze(df)

# Export HTML report
adapter.to_html("profile_report.html")

# For Jupyter notebooks
adapter.to_widgets()
```

### Sweetviz

Beautiful visualizations and dataset comparison.

```python
from acdso.eda import SweetvizAdapter

# Basic analysis
adapter = SweetvizAdapter(target_feat="target")
adapter.analyze(df)
adapter.to_html("sweetviz_report.html")

# Compare two datasets
adapter = SweetvizAdapter(
    target_feat="target",
    compare_df=test_df,  # Compare against test set
)
adapter.analyze(train_df)
adapter.to_html("comparison_report.html")
```

### Dataprep

Fast EDA with interactive visualizations.

```python
from acdso.eda import DataprepAdapter

adapter = DataprepAdapter()
adapter.analyze(df)
adapter.to_html("dataprep_report.html")
```

### D-Tale

Interactive browser-based exploration.

```python
from acdso.eda import DTaleAdapter

adapter = DTaleAdapter()
adapter.analyze(df)

# Opens interactive UI in browser
adapter.open_browser()
```

## Standalone Utility Functions

### Quick Profiling

Profile columns without running full EDA:

```python
from acdso.eda import quick_profile

profiles = quick_profile(df)

# Access profiles
for col, profile in profiles.items():
    print(f"{col}: mean={profile.mean}, missing={profile.missing_pct}%")
```

### Dataset Comparison

Compare two datasets for schema and distribution differences:

```python
from acdso.eda import compare_datasets

comparison = compare_datasets(
    train_df, test_df,
    name1="Train",
    name2="Test",
)

# Schema differences
print(comparison['schema_differences'])

# Distribution drift
for drift in comparison['distribution_drift']:
    print(f"{drift['column']}: KS statistic={drift['ks_statistic']}")

# Summary
print(f"Compatible: {comparison['summary']['compatible']}")
print(f"Columns with drift: {comparison['summary']['columns_with_drift']}")
```

### Data Validation

Validate data against quality rules:

```python
from acdso.eda import validate_data

rules = {
    'max_missing_pct': 0.1,         # Max 10% missing per column
    'max_duplicate_pct': 0.05,      # Max 5% duplicates
    'required_columns': ['id', 'value', 'timestamp'],
    'column_types': {
        'id': 'int',
        'value': 'float',
    },
    'value_ranges': {
        'age': (0, 120),
        'price': (0, 1000000),
    },
}

result = validate_data(df, rules)

print(f"Passed: {result['passed']}")
print(f"Checks: {result['checks_passed']}/{result['checks_run']}")

for failure in result['failures']:
    print(f"Failed: {failure}")
```

### Infer Data Types

Infer semantic data types for columns:

```python
from acdso.eda import infer_data_types

types = infer_data_types(df)

for col, info in types.items():
    print(f"{col}:")
    print(f"  Pandas dtype: {info['pandas_dtype']}")
    print(f"  Semantic type: {info['semantic_type']}")
    print(f"  Suggested dtype: {info['suggested_dtype']}")
```

Recognized semantic types:
- `numeric_continuous`, `numeric_discrete`, `categorical_numeric`
- `categorical`, `text`, `identifier`
- `datetime`
- `email`, `url`, `phone`, `ip_address`, `uuid`, `date`

### Preprocessing Recommendations

Get preprocessing recommendations for modeling:

```python
from acdso.eda import get_preprocessing_recommendations

recs = get_preprocessing_recommendations(df, target='price')

for rec in recs:
    print(f"{rec['column']}: {rec['action']}")
    print(f"  Reason: {rec['reason']}")
    print(f"  Priority: {rec['priority']}")
```

Recommendation actions:
- `drop_column`: Remove column (constant, ID-like, too many missing)
- `impute_median`, `impute_mode`: Handle missing values
- `log_transform`: Transform skewed features
- `reduce_cardinality`: Handle high-cardinality categorical
- `clip_outliers`: Handle extreme outliers

## Pareto Optimization

Like ACDSO AutoML, the Auto EDA uses Pareto optimization to balance multiple objectives:

| Objective | Description |
|-----------|-------------|
| Quality Score | How thorough/accurate the analysis is (0-1) |
| Coverage | Fraction of data analyzed (0-1) |
| Insight Count | Number of actionable insights found |
| Speed | Analysis time (minimize) |

Access the Pareto front:

```python
# All analysis candidates
for candidate in eda.all_candidates_:
    print(f"{candidate.name}: quality={candidate.quality_score}, "
          f"insights={candidate.insight_count}, time={candidate.compute_time_s}s")

# Pareto-optimal analyses
for candidate in eda.pareto_front_:
    print(f"Pareto: {candidate.name}")
```

## LLM-Powered Insights

Enable AI-powered analysis for deeper insights:

```python
from acdso import ACDSOAutoEDA, create_advisor

# Create LLM advisor
advisor = create_advisor("openai", model="gpt-4o-mini")

eda = ACDSOAutoEDA(llm_advisor=advisor, verbose=1)
report = eda.analyze(df)

# LLM insights are labeled with [AI Information]
for insight in report.insights:
    print(insight)
```

For keyword-based analysis without API:

```python
from acdso import ACDSOAutoEDA, llm_advisor_stub

eda = ACDSOAutoEDA(llm_advisor=llm_advisor_stub)
report = eda.analyze(df)
```

## Best Practices

### 1. Start with Basic Analysis

```python
eda = ACDSOAutoEDA(verbose=1)
report = eda.analyze(df)
eda.print_summary()
```

### 2. Check Quality Score First

```python
if eda.data_quality_.quality_score < 70:
    print("Data quality issues detected!")
    for issue in eda.data_quality_.issues:
        print(f"  - {issue}")
```

### 3. Address Issues Before Modeling

```python
recs = get_preprocessing_recommendations(df, target='target')
high_priority = [r for r in recs if r['priority'] == 'high']

for rec in high_priority:
    print(f"IMPORTANT: {rec['column']} - {rec['action']}")
```

### 4. Compare Train/Test Distributions

```python
comparison = compare_datasets(train_df, test_df)

if comparison['distribution_drift']:
    print("WARNING: Distribution drift detected!")
    for drift in comparison['distribution_drift']:
        print(f"  - {drift['column']}: p-value={drift['p_value']}")
```

### 5. Validate Data Before Production

```python
result = validate_data(df, your_rules)
if not result['passed']:
    raise ValueError(f"Data validation failed: {result['failures']}")
```

## API Reference

### Classes

- `ACDSOAutoEDA`: Main Auto EDA class
- `ColumnProfile`: Profile for a single column
- `DataQualityReport`: Overall data quality assessment
- `EDAReport`: Complete EDA report
- `EDACandidate`: Analysis candidate with metrics
- `CorrelationResult`: Correlation between features
- `AnomalyResult`: Detected anomaly

### Backend Adapters

- `YDataProfilingAdapter`: ydata-profiling integration
- `SweetvizAdapter`: Sweetviz integration
- `DataprepAdapter`: Dataprep integration
- `DTaleAdapter`: D-Tale integration

### Functions

- `quick_profile(df)`: Quick column profiling
- `compare_datasets(df1, df2)`: Compare two datasets
- `validate_data(df, rules)`: Validate against rules
- `infer_data_types(df)`: Infer semantic types
- `get_preprocessing_recommendations(df)`: Get preprocessing suggestions
- `get_available_backends()`: Check available EDA backends
- `get_backend_adapter(backend)`: Get adapter for backend

## Examples

See the `demo.py` file for comprehensive examples:

```bash
python demo.py
```

This runs 18 demos including:

### Auto EDA Demos
- Demo 9: Basic Auto EDA
- Demo 10: Data Quality Analysis
- Demo 11: Pareto Optimization
- Demo 12: LLM Insights
- Demo 13: Export and Reporting
- Demo 16: EDA Tool Integrations (ydata-profiling, sweetviz, etc.)
- Demo 17: EDA Utility Functions (quick_profile, validate_data, etc.)

### AutoML Integration Demos
- Demo 14: Meta Optimizer Architecture
- Demo 15: FLAML Integration
- Demo 18: Complete ML Workflow (EDA + AutoML)

## Related Documentation

- [Quick Start Guide](quickstart.md) - Basic ACDSO usage
- [AutoML Integration](automl_integration.md) - Meta optimizer and adapters
- [LLM Configuration](llm_configuration.md) - LLM advisor setup

