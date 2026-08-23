# ACDSO v2.0: Risk-Optimized AutoML for Production Systems

[![PyPI version](https://badge.fury.io/py/acdso.svg)](https://badge.fury.io/py/acdso)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![scikit-learn compatible](https://img.shields.io/badge/scikit--learn-compatible-orange.svg)](https://scikit-learn.org/)

**ACDSO** (Adaptive Contextual Data Science Optimization) is a production-oriented AutoML library that goes beyond accuracy optimization. It unifies **contextual risk diagnostics**, **adaptive pipeline synthesis**, and **multi-objective optimization** into a single framework.

Unlike traditional AutoML that only maximizes accuracy, ACDSO optimizes across **five dimensions**:
- 🎯 **Predictive Performance** (accuracy, F1, R², etc.)
- ⏱️ **Training Cost** (time to train)
- 💾 **Memory Footprint** (peak memory usage)
- ⚡ **Inference Latency** (prediction speed)
- ⚠️ **Risk Penalty** (leakage, instability, imbalance)

---

## 📦 Installation

### Basic Installation

```bash
pip install acdso
```

### With Optional Dependencies

```bash
# Boosting algorithms (XGBoost, LightGBM)
pip install acdso[boosters]

# LLM-based semantic analysis
pip install acdso[openai]      # OpenAI GPT
pip install acdso[anthropic]   # Anthropic Claude
pip install acdso[llm]         # Both providers
pip install acdso[litellm]     # 100+ providers via LiteLLM

# AutoML integrations
pip install acdso[flaml]       # Microsoft FLAML
pip install acdso[automl]      # All AutoML integrations

# Everything
pip install acdso[all]

# Development
pip install acdso[dev]
```

### From Source

```bash
git clone https://github.com/ds-tui/acdso.git
cd acdso
pip install -e ".[all]"
```

---

## 🖥️ Web GUI

```bash
pip install flask
python start.py
```

Opens `http://127.0.0.1:5000` — drop in a CSV / Excel / Parquet / JSON /
Feather / Pickle file, pick the target column(s) from the detected schema,
toggle any `run_acdso.py` flag, and watch the pipeline stream its output
live. Reports, predictions and model files written to `reports/` are listed
for viewing or download when the run finishes.

Options: `--port 8000`, `--host 0.0.0.0` (binds beyond localhost — anyone who
can reach the port can upload data and start runs), `--no-browser`, `--debug`.

The GUI runs the same `run_acdso.py` command line you would type by hand and
shows it before starting, so there is no separate execution path. One run at
a time: ACDSO writes artifacts under deterministic dataset-derived names, so
concurrent runs would overwrite each other. Served in production by
[waitress](https://docs.pylonsproject.org/projects/waitress/) by default
(`--debug` switches to Flask's dev server for editing the GUI itself).

Also included: a **Forecast** card for "how much next period" questions
(separate from the AutoML predict path — see docs below for why), live
**column-selection validation** (rule-based plus an optional LLM check) that
blocks a run before it starts if two roles are assigned the same column, and
a **Report History** panel listing every file ever written to `reports/`,
not just the current run's.

For what every single flag and GUI field actually does — including which
ones are currently no-ops in `run_acdso.py`'s dispatch logic — see
**[docs/web_gui.md](docs/web_gui.md)**.

---

## 🚀 Quick Start

### Classification

```python
from acdso import ACDSOClassifier

# Create classifier with production constraints
clf = ACDSOClassifier(
    scorer="f1",                      # Optimize for F1 score
    cv=5,                             # 5-fold cross-validation
    preference="safe",                # Risk-averse selection
    max_latency_ms_per_row=10.0,      # Max 10ms per prediction
    max_peak_mem_mb=500.0,            # Max 500MB memory
    verbose=1,
)

# Fit the model
clf.fit(X_train, y_train)

# Make predictions
predictions = clf.predict(X_test)
probabilities = clf.predict_proba(X_test)

# Access diagnostics
print(clf.risk_report_)    # Risk analysis
print(clf.pareto_front_)   # All optimal candidates
print(clf.audit_trail_)    # Full evaluation log
```

### Regression

```python
from acdso import ACDSORegressor

reg = ACDSORegressor(
    scorer="r2",
    cv=5,
    max_latency_ms_per_row=5.0,
    verbose=1,
)

reg.fit(X_train, y_train)
predictions = reg.predict(X_test)
```

---

## 🔍 Key Features

### 1. Multi-Objective Optimization

Each candidate is evaluated on a 5-dimensional objective vector:

```
J(pipeline) = [
    1 - Error,              # Maximize accuracy
    log(1 + TrainTime),     # Minimize training time
    log(1 + PeakMemory),    # Minimize memory usage
    log(1 + Latency),       # Minimize inference latency
    RiskPenalty             # Minimize data/model risk
]
```

### 2. Advanced Risk Detection

ACDSO detects issues that other AutoML tools miss:

| Risk Type | Detection Method |
|-----------|------------------|
| **Leakage Proxy** | Univariate predictive power analysis |
| **Temporal Leakage** | Chronological vs shuffled CV comparison |
| **Multicollinearity** | Variance Inflation Factor (VIF) |
| **Instability** | CV score variance across folds |
| **Class Imbalance** | Continuous severity scoring |
| **Semantic Issues** | LLM-based column name analysis |

### 3. Production Constraints

Enforce hard limits that candidates must meet:

```python
clf = ACDSOClassifier(
    max_train_time_s=60.0,        # Max 60s training
    max_peak_mem_mb=512.0,        # Max 512MB memory
    max_latency_ms_per_row=5.0,   # Max 5ms per prediction
)
```

Candidates violating any constraint are **automatically pruned**.

### 4. Pareto Selection

Instead of picking the "best accuracy" model, ACDSO:
1. Constructs a **Pareto front** of non-dominated solutions
2. Selects the **knee-point** - optimal tradeoff between accuracy and cost/risk

```python
# Access the full Pareto front
for candidate in clf.pareto_front_:
    print(f"{candidate['name']}: "
          f"metric={candidate['metric']:.3f}, "
          f"latency={candidate['latency_ms_per_row']:.2f}ms, "
          f"risk={candidate['risk_penalty']:.2f}")
```

### 5. Audit Trail

Every run produces a JSON-serializable audit artifact for MLOps:

```python
import json

# Save audit trail
with open("audit.json", "w") as f:
    json.dump(clf.audit_trail_, f, indent=2)

# Contents:
# - risk_report: Full risk analysis
# - constraints: Applied guardrails
# - candidates_evaluated: All candidates with metrics
# - pareto_front: Non-dominated solutions
# - pruned_or_failed: Rejected candidates and reasons
# - selected: Final model details
```

---

## 🤖 LLM Integration

ACDSO supports LLM-based semantic analysis to detect issues that statistical methods miss.

### Quick Setup

```python
from acdso import ACDSOClassifier, create_advisor

# OpenAI (uses OPENAI_API_KEY env var)
advisor = create_advisor("openai", model="gpt-4o-mini")

# Anthropic (uses ANTHROPIC_API_KEY env var)
advisor = create_advisor("anthropic", model="claude-3-haiku-20240307")

# Ollama (free, local)
advisor = create_advisor("ollama", model="llama3.2")

# Use with classifier
clf = ACDSOClassifier(llm_advisor=advisor)
```

### Supported Providers

| Provider | Install | Environment Variable |
|----------|---------|---------------------|
| OpenAI | `pip install acdso[openai]` | `OPENAI_API_KEY` |
| Anthropic | `pip install acdso[anthropic]` | `ANTHROPIC_API_KEY` |
| Azure OpenAI | `pip install acdso[openai]` | `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT` |
| Ollama | - | - (local) |
| LiteLLM | `pip install acdso[litellm]` | varies by provider |

### Advanced Configuration

```python
from acdso import (
    LLMAdvisor,
    LLMConfig,
    PromptTemplate,
    OpenAIProvider,
    OllamaProvider,
)

# Custom configuration
config = LLMConfig(
    max_tokens=1000,
    temperature=0.2,
    cache_enabled=True,      # Cache responses
    cache_ttl_s=3600,        # 1 hour TTL
    max_retries=3,           # Retry on failure
)

# Custom prompt
prompt = PromptTemplate(
    system_prompt="You are a data quality expert...",
    user_prompt_template="Analyze columns: {columns}...",
)

# Primary + fallback provider
advisor = LLMAdvisor(
    provider=OpenAIProvider(model="gpt-4o"),
    fallback_provider=OllamaProvider(model="llama3"),
    config=config,
    prompt_template=prompt,
)

clf = ACDSOClassifier(llm_advisor=advisor)
```

### Without LLM (Keyword Matching)

```python
from acdso import llm_advisor_stub

clf = ACDSOClassifier(llm_advisor=llm_advisor_stub)
```

---

## 📊 Auto EDA

ACDSO v2.0 includes a powerful Auto EDA module that uses the same multi-objective optimization approach as the AutoML components.

### Basic Usage

```python
from acdso import ACDSOAutoEDA

# Create EDA analyzer
eda = ACDSOAutoEDA(verbose=1)

# Run analysis
report = eda.analyze(df, dataset_name="my_data")

# Print summary
eda.print_summary()

# Export results
eda.to_json("eda_report.json")
eda.to_html("eda_report.html")
eda.to_markdown("eda_report.md")
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Column Profiling** | Statistics, distributions, outliers, skewness |
| **Data Quality Score** | 0-100 score based on missing data, duplicates, constants |
| **Correlation Analysis** | High correlation detection with p-values |
| **Anomaly Detection** | Z-score and multivariate outlier detection |
| **LLM Insights** | AI-powered analysis (optional) |
| **Pareto Optimization** | Balance thoroughness, speed, and insight quality |

### Integration with Popular EDA Tools

ACDSO Auto EDA integrates with popular EDA libraries:

```python
from acdso import get_available_backends

# Check what's available
print(get_available_backends())
# {'native': True, 'ydata-profiling': True, 'sweetviz': True, ...}
```

#### ydata-profiling (pandas-profiling)

```python
from acdso.eda import YDataProfilingAdapter

adapter = YDataProfilingAdapter(minimal=False, explorative=True)
adapter.analyze(df)
adapter.to_html("profile_report.html")
```

#### Sweetviz

```python
from acdso.eda import SweetvizAdapter

# Compare datasets
adapter = SweetvizAdapter(target_feat="target", compare_df=test_df)
adapter.analyze(train_df)
adapter.to_html("comparison.html")
```

#### D-Tale Interactive

```python
from acdso.eda import DTaleAdapter

adapter = DTaleAdapter()
adapter.analyze(df)
adapter.open_browser()  # Opens interactive UI
```

### Standalone Utility Functions

```python
from acdso.eda import (
    quick_profile,
    compare_datasets,
    validate_data,
    infer_data_types,
    get_preprocessing_recommendations,
)

# Quick column profiling
profiles = quick_profile(df)
print(profiles['age'].mean)
print(profiles['age'].has_outliers)

# Compare train/test datasets
comparison = compare_datasets(train_df, test_df, "Train", "Test")
print(comparison['distribution_drift'])

# Validate against rules
result = validate_data(df, {
    'max_missing_pct': 0.1,
    'required_columns': ['id', 'value'],
    'value_ranges': {'age': (0, 120)},
})
print(result['passed'])

# Infer semantic types
types = infer_data_types(df)
print(types['email']['semantic_type'])  # 'email'

# Get preprocessing recommendations
recs = get_preprocessing_recommendations(df, target='price')
for rec in recs:
    print(f"{rec['column']}: {rec['action']}")
```

### EDA Installation Options

```bash
# Basic Auto EDA (native only)
pip install acdso

# With ydata-profiling integration
pip install acdso[ydata-profiling]

# With sweetviz integration
pip install acdso[sweetviz]

# With all EDA tools
pip install acdso[eda]
```

---

## 🔗 AutoML Integration

ACDSO works as a **unified decision layer** on top of other AutoML tools.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ACDSOMetaOptimizer                       │
│              (Unified Decision Layer)                       │
├─────────────────────────────────────────────────────────────┤
│  Risk Analysis → Constraint Pruning → Pareto → Knee-Point  │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │ Candidates
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
   auto-sklearn            TPOT/FLAML           H2O/AutoGluon
```

### Usage

```python
from acdso.integrations import (
    ACDSOMetaOptimizer,
    AutoSklearnAdapter,
    TPOTAdapter,
    FLAMLAdapter,
    H2OAdapter,
)

# Create adapters for different AutoML tools
adapters = [
    AutoSklearnAdapter(time_budget=120, top_k=5),
    TPOTAdapter(generations=5, top_k=5),
    FLAMLAdapter(time_budget=60, top_k=5),
]

# ACDSO as unified decision layer
meta = ACDSOMetaOptimizer(
    adapters=adapters,
    include_acdso_candidates=True,
    scorer="f1",
    max_latency_ms_per_row=10.0,
    preference="safe",
    verbose=1,
)

meta.fit(X_train, y_train)

print(f"Selected: {meta.best_source_} - {meta.audit_trail_['selected']['name']}")
print(meta.get_source_summary())
```

### Supported Integrations

| Tool | Adapter | Type |
|------|---------|------|
| auto-sklearn | `AutoSklearnAdapter` | sklearn-native |
| TPOT | `TPOTAdapter` | sklearn-native |
| FLAML | `FLAMLAdapter` | sklearn-friendly |
| H2O AutoML | `H2OAdapter` | external (wrapped) |
| AutoGluon | `AutoGluonAdapter` | external (wrapped) |

### Wrapping Custom Models

```python
from acdso.integrations import GenericModelWrapper, CandidateSource

# Wrap any pre-trained model
wrapper = GenericModelWrapper(
    model=my_model,
    predict_method="predict",
    predict_proba_method="predict_proba",
    name="MyCustomModel",
    source="custom",
)
```

---

## 📊 API Reference

### ACDSOClassifier / ACDSORegressor

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `scorer` | str | "accuracy"/"r2" | Scoring metric |
| `cv` | int | 5 | Cross-validation folds |
| `preference` | str | "balanced" | Risk preference ("safe" or "balanced") |
| `time_budget_s` | float | None | Total time budget |
| `max_candidates` | int | 4 | Maximum candidates to evaluate |
| `random_state` | int | 42 | Random seed |
| `max_train_time_s` | float | None | Max training time constraint |
| `max_peak_mem_mb` | float | None | Max memory constraint |
| `max_latency_ms_per_row` | float | None | Max latency constraint |
| `enable_feature_selection` | bool | True | Auto feature selection |
| `llm_advisor` | callable | None | LLM advisor function |
| `verbose` | int | 0 | Verbosity level |

### Fitted Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `best_estimator_` | Pipeline | Selected best pipeline |
| `risk_report_` | dict | Risk analysis report |
| `pareto_front_` | list | Pareto-optimal candidates |
| `audit_trail_` | dict | Full audit artifact |

### Risk Report Fields

| Field | Description |
|-------|-------------|
| `leakage_proxy_features` | Features suspected of leakage |
| `leakage_proxy_score_max` | Max univariate predictive power |
| `temporal_leakage_gap` | Score gap (shuffled - chronological) |
| `class_imbalance_max_ratio` | Maximum class ratio |
| `imbalance_severity` | Imbalance severity (0-1) |
| `vif_max` | Maximum VIF value |
| `vif_features` | Features with high VIF |
| `cv_score_std` | CV score standard deviation |
| `notes` | Human-readable warnings |

---

## 🆚 Comparison with Other AutoML

| Feature | ACDSO | auto-sklearn | TPOT | H2O | FLAML |
|---------|-------|--------------|------|-----|-------|
| Risk-aware selection | ✅ | ❌ | ❌ | ❌ | ❌ |
| Leakage detection | ✅ | ❌ | ❌ | ❌ | ❌ |
| Constraint pruning | ✅ | ❌ | ❌ | ✅ | ❌ |
| Multi-objective Pareto | ✅ | ❌ | ✅ | ❌ | ❌ |
| Knee-point selection | ✅ | ❌ | ❌ | ❌ | ❌ |
| Audit trail | ✅ | ❌ | ❌ | ✅ | ❌ |
| LLM integration | ✅ | ❌ | ❌ | ❌ | ❌ |
| Unified decision layer | ✅ | ❌ | ❌ | ❌ | ❌ |
| scikit-learn compatible | ✅ | ✅ | ✅ | ❌ | ✅ |

---

## 📋 Requirements

### Core Dependencies
- Python 3.8+
- numpy >= 1.21.0
- pandas >= 1.3.0
- scikit-learn >= 1.0.0

### Optional Dependencies
- xgboost >= 1.5.0 (boosting)
- lightgbm >= 3.3.0 (boosting)
- openai >= 1.0.0 (LLM)
- anthropic >= 0.18.0 (LLM)
- litellm >= 1.0.0 (LLM)
- flaml >= 2.0.0 (AutoML integration)

---

## 🎮 Demo Suite

ACDSO includes a comprehensive demo suite that showcases all features with **automated report generation**.

### Quick Reference

```bash
# Most common: Run everything on your data
python run_acdso.py --autonomous --file data.csv

# Complete workflow: Train + Predict
python run_acdso.py --autonomous --predict price --file data.csv

# Just make predictions (model must exist)
python run_acdso.py --predict price --file data.csv

# Time series forecasting
python run_acdso.py --autonomous --file sales.csv --target revenue --datetime-col date

# Benchmark your dataset
python run_acdso.py --file data.csv --target price --benchmark
```

### Autonomous Mode (Recommended)

Run everything automatically with a single command - generates a comprehensive combined HTML report:

```bash
# Run ALL analyses on your data (EDA, Risk, AutoML, Time Series, Benchmarks)
python run_acdso.py --autonomous --file data.csv

# With custom target
python run_acdso.py --autonomous --file data.csv --target price

# Short form
python run_acdso.py -a -f data.csv -t price
```

**What Autonomous Mode Does:**
1. **Auto EDA** - Complete exploratory data analysis with quality scores
2. **Risk Analysis** - Leakage detection, multicollinearity, imbalance analysis
3. **AutoML** - Risk-aware model selection with full audit trail
4. **Time Series** - Forecasting (if datetime column detected)
5. **Benchmarks** - Comparison against other AutoML systems
6. **Combined Report** - Single HTML file with everything embedded

### Complete Workflow: Train + Predict

Train a model and make predictions in one command:

```bash
# Train model and make predictions on full dataset
python run_acdso.py --autonomous --predict price --file data.csv

# This will:
# 1. Run all analyses (EDA, Risk, AutoML, Time Series, Benchmarks)
# 2. Save the best trained model
# 3. Make predictions on ALL rows in the dataset
# 4. Generate ONE combined HTML report with everything
```

**Output Files:**
- `reports/autonomous_{filename}_report.html` - **Combined HTML report** (EDA, Time Series, AutoML, Predictions)
- `reports/autonomous_{filename}_combined.json` - JSON report with all data
- `reports/model_{filename}.pkl` - Saved model for future predictions
- `reports/predictions_{filename}_{target}_full_dataset.csv` - Predictions on full dataset

### Run Built-in Demos

```bash
# Run all 18 demos using the Seaborn tips dataset
python run_acdso.py

# Run specific demos
python run_acdso.py --v3              # v3 innovations demo
python run_acdso.py --timeseries     # Time series forecasting
python run_acdso.py --decision       # Decision intelligence
python run_acdso.py --benchmark      # Benchmark comparison
```

### Run on Your Own Data

```bash
# Basic usage - uses last column as target
python run_acdso.py --file data.csv

# Specify target column
python run_acdso.py --file data.csv --target price

# Specify task type
python run_acdso.py --file data.csv --target churn --task classification

# Time series with datetime column
python run_acdso.py --file sales.csv --target revenue --datetime-col date --timeseries

# Short form
python run_acdso.py -f customers.xlsx -t label
```

### Make Predictions (After Training)

```bash
# Make predictions using a previously trained model
python run_acdso.py --predict price --file data.csv

# Predictions are made on ALL rows in the dataset
# Output: reports/predictions_{filename}_{target}_full_dataset.csv
```

### Combined HTML Reports

All modes generate detailed **combined HTML reports** that include:

- **EDA Section**: Complete exploratory data analysis with visualizations
- **Risk Analysis**: Leakage detection, multicollinearity, data quality scores
- **AutoML Results**: All models evaluated with scores, selected model details
- **Time Series**: Forecasting results and benchmark comparisons (if applicable)
- **Benchmark Results**: Comparison against other AutoML systems with model names
- **Predictions**: Prediction statistics and distributions (if --predict used)
- **Console Output**: Full execution log
- **Audit Trails**: Complete model selection history

**Report Location:** `reports/autonomous_{filename}_report.html`

### Supported File Formats

| Format | Extensions |
|--------|------------|
| CSV | `.csv` |
| Excel | `.xlsx`, `.xls` |
| Parquet | `.parquet` |
| JSON | `.json` |
| Feather | `.feather` |
| Pickle | `.pkl`, `.pickle` |

### Demo Arguments

| Argument | Short | Description |
|----------|-------|-------------|
| `--autonomous` | `-a` | Run ALL analyses automatically (EDA, Risk, AutoML, Time Series, Benchmarks) |
| `--file` | `-f` | Path to data file |
| `--target` | `-t` | Target column name (default: last column) |
| `--predict` | `-p` | Make predictions using best trained model (requires --file) |
| `--task` | | `auto`, `classification`, or `regression` |
| `--timeseries` | `-ts` | Run time series forecasting demo |
| `--datetime-col` | | Datetime column for time series |
| `--horizon` | | Forecast horizon (default: 7) |
| `--benchmark` | `-b` | Compare against other AutoML systems |
| `--decision` | `-d` | Run decision intelligence demo |
| `--exclude` | `-e` | Comma-separated columns to exclude |
| `--no-auto-exclude` | | Disable automatic column exclusion |
| `--all` | | Run all built-in demos |

---

## 📁 Examples

See the `examples/` directory for complete examples:

- `basic_usage.py` - Classification and regression basics
- `llm_advisor_usage.py` - LLM configuration examples
- `meta_optimizer_usage.py` - AutoML integration examples

---

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Setup development environment
git clone https://github.com/ds-tui/acdso.git
cd acdso
pip install -e ".[dev]"

# Run tests
pytest tests/ -v

# Format code
black src/ tests/
isort src/ tests/
```

---

## 📜 License

MIT License - see [LICENSE](LICENSE) for details.

---

## 📚 Citation

If you use ACDSO in your research, please cite:

```bibtex
@software{acdso2025,
  title = {ACDSO v2.0: Risk-Optimized AutoML for Production Systems},
  author = {DS-TUI Development Team},
  year = {2025},
  version = {2.0.0},
  url = {https://github.com/ds-tui/acdso}
}
```

---

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/ds-tui/acdso/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ds-tui/acdso/discussions)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)
