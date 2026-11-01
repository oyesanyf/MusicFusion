# AutoML Integration Guide

ACDSO can work as a **unified decision layer** on top of other AutoML tools. This guide explains how to integrate with various AutoML frameworks.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ACDSOMetaOptimizer                       │
│              (Unified Decision Layer)                       │
├─────────────────────────────────────────────────────────────┤
│  1. Collect candidates from all AutoML tools                │
│  2. Apply ACDSO risk diagnostics                            │
│  3. Enforce production constraints                          │
│  4. Build unified Pareto front                              │
│  5. Select best via knee-point                              │
│  6. Generate audit trail                                    │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │ Candidates
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
   auto-sklearn            TPOT/FLAML         H2O/AutoGluon
   (sklearn-native)     (sklearn-native)      (external)
```

## Quick Start

```python
from acdso.integrations import ACDSOMetaOptimizer, FLAMLAdapter

# Create adapter for FLAML
adapter = FLAMLAdapter(time_budget=60, top_k=5)

# Create meta optimizer
meta = ACDSOMetaOptimizer(
    adapters=[adapter],
    include_acdso_candidates=True,
    scorer="f1",
    max_latency_ms_per_row=10.0,
    verbose=1,
)

# Fit and select best model
meta.fit(X_train, y_train)
predictions = meta.predict(X_test)

print(f"Selected: {meta.best_source_}")
print(meta.audit_trail_)
```

## Adapter Reference

### FLAMLAdapter (Microsoft FLAML)

**Install:** `pip install acdso[flaml]`

```python
from acdso.integrations import FLAMLAdapter

adapter = FLAMLAdapter(
    time_budget=60,           # Seconds
    metric=None,              # Auto-detected
    estimator_list=None,      # Default estimators
    n_splits=5,               # CV folds
    top_k=5,                  # Return top 5 candidates
)
```

### AutoSklearnAdapter

**Install:** `pip install auto-sklearn`

```python
from acdso.integrations import AutoSklearnAdapter

adapter = AutoSklearnAdapter(
    time_budget=120,          # Seconds
    per_run_time_limit=30,    # Per candidate
    memory_limit=4096,        # MB
    ensemble_size=10,
    top_k=5,
    include_estimators=None,  # e.g., ["random_forest", "extra_trees"]
    exclude_estimators=None,
)
```

### TPOTAdapter

**Install:** `pip install tpot`

```python
from acdso.integrations import TPOTAdapter

adapter = TPOTAdapter(
    generations=5,
    population_size=50,
    mutation_rate=0.9,
    crossover_rate=0.1,
    cv=5,
    max_time_mins=None,
    max_eval_time_mins=5,
    top_k=5,
)
```

### H2OAdapter

**Install:** `pip install h2o`

```python
from acdso.integrations import H2OAdapter

adapter = H2OAdapter(
    max_runtime_secs=120,
    max_models=20,
    nfolds=5,
    seed=42,
    include_algos=None,       # e.g., ["GBM", "RF", "XGBoost"]
    exclude_algos=None,
    top_k=5,
)
```

### AutoGluonAdapter

**Install:** `pip install autogluon`

```python
from acdso.integrations import AutoGluonAdapter

adapter = AutoGluonAdapter(
    time_limit=120,
    presets="medium_quality",  # "best_quality", "high_quality", etc.
    eval_metric=None,          # Auto-detected
    hyperparameters=None,      # Custom hyperparameters
    top_k=5,
)
```

## Multiple Adapters

Combine candidates from multiple AutoML tools:

```python
from acdso.integrations import (
    ACDSOMetaOptimizer,
    AutoSklearnAdapter,
    TPOTAdapter,
    FLAMLAdapter,
)

adapters = [
    AutoSklearnAdapter(time_budget=120, top_k=5),
    TPOTAdapter(generations=5, top_k=5),
    FLAMLAdapter(time_budget=60, top_k=5),
]

meta = ACDSOMetaOptimizer(
    adapters=adapters,
    include_acdso_candidates=True,  # Also include ACDSO's own models
    scorer="f1",
    verbose=1,
)

meta.fit(X_train, y_train)

# See which tool produced the best model
print(f"Winner: {meta.best_source_}")

# Summary by source
for source, stats in meta.get_source_summary().items():
    print(f"{source}: {stats['total']} candidates, "
          f"{stats['in_pareto']} in Pareto")
```

## Wrapping Custom Models

Wrap any pre-trained model for ACDSO evaluation:

```python
from acdso.integrations import GenericModelWrapper, CandidateSource

# Train your model
my_model = train_my_custom_model(X_train, y_train)

# Wrap it
wrapper = GenericModelWrapper(
    model=my_model,
    predict_method="predict",
    predict_proba_method="predict_proba",
    preprocess_fn=lambda X: X.values,  # Optional preprocessing
    name="MyCustomModel",
    source="custom",
)

# Create a custom adapter
class CustomAdapter:
    def __init__(self, candidates):
        self.name = "custom"
        self.candidates = candidates
        self.top_k = len(candidates)

    def is_available(self):
        return True

    def generate_candidates(self, X, y, task):
        return self.candidates

    def get_generation_stats(self):
        return {"adapter": self.name}

# Use with meta optimizer
candidate = CandidateSource(
    name="MyCustomModel",
    source="custom",
    estimator=wrapper,
)

meta = ACDSOMetaOptimizer(
    adapters=[CustomAdapter([candidate])],
    include_acdso_candidates=True,
)
```

## Wrapping External Models (H2O, AutoGluon)

For non-sklearn models, use `ExternalModelWrapper`:

```python
from acdso.integrations import ExternalModelWrapper

# H2O model example
def h2o_predict(X):
    h2o_frame = h2o.H2OFrame(X)
    return h2o_model.predict(h2o_frame).as_data_frame()['predict'].values

wrapper = ExternalModelWrapper(
    predict_fn=h2o_predict,
    predict_proba_fn=None,  # Optional
    name="H2O_GBM",
    source="h2o",
)
```

## Production Constraints

Constraints are applied uniformly across all sources:

```python
meta = ACDSOMetaOptimizer(
    adapters=adapters,
    max_latency_ms_per_row=10.0,  # All candidates must meet this
    max_peak_mem_mb=512.0,
    max_train_time_s=60.0,
)

# Candidates violating any constraint are pruned
# Pruned candidates appear in audit_trail_["pruned_or_failed"]
```

## Time Budget Management

Control total optimization time:

```python
meta = ACDSOMetaOptimizer(
    adapters=[
        FLAMLAdapter(time_budget=60),   # 1 min for FLAML
        TPOTAdapter(max_time_mins=2),   # 2 min for TPOT
    ],
    time_budget_s=300,  # 5 min total for meta optimization
)
```

## Audit Trail

The meta optimizer produces a comprehensive audit trail:

```python
import json

meta.fit(X_train, y_train)

audit = meta.audit_trail_
print(json.dumps(audit, indent=2))

# Contents:
# {
#   "task": "classification",
#   "total_time_s": 123.45,
#   "risk_report": {...},
#   "constraints": {...},
#   "adapter_stats": [...],
#   "candidates_evaluated": [...],
#   "pareto_front": [...],
#   "pruned_or_failed": [...],
#   "selected": {
#     "name": "flaml_classification_lgbm",
#     "source": "FLAML",
#     "metric": 0.95,
#     "latency_ms_per_row": 0.5,
#     ...
#   }
# }
```

## Checking Adapter Availability

```python
from acdso.integrations import FLAMLAdapter, TPOTAdapter, H2OAdapter

for AdapterClass in [FLAMLAdapter, TPOTAdapter, H2OAdapter]:
    adapter = AdapterClass()
    status = "✓ Available" if adapter.is_available() else "✗ Not installed"
    print(f"{adapter.name}: {status}")
```

## Best Practices

1. **Start with FLAML**: It's fast and lightweight
2. **Use time budgets**: Control how long each tool runs
3. **Enable ACDSO candidates**: `include_acdso_candidates=True`
4. **Set realistic constraints**: Based on production requirements
5. **Review the audit trail**: Understand why models were selected/rejected
6. **Cache LLM results**: If using LLM advisor across multiple runs

## Complete Workflow Example

Combine Auto EDA with AutoML integration for a complete ML workflow:

```python
from acdso import ACDSOAutoEDA
from acdso.eda import validate_data, get_preprocessing_recommendations
from acdso.integrations import ACDSOMetaOptimizer, FLAMLAdapter

# Step 1: EDA
eda = ACDSOAutoEDA(verbose=1)
report = eda.analyze(df)
print(f"Quality Score: {eda.data_quality_.quality_score}/100")

# Step 2: Validate
result = validate_data(df, {'max_missing_pct': 0.1})
assert result['passed'], "Data validation failed!"

# Step 3: Get recommendations
recs = get_preprocessing_recommendations(df, target='target')
for rec in recs:
    print(f"{rec['column']}: {rec['action']}")

# Step 4: AutoML with meta optimizer
meta = ACDSOMetaOptimizer(
    adapters=[FLAMLAdapter(time_budget=60)],
    include_acdso_candidates=True,
    scorer="f1",
    max_latency_ms_per_row=10.0,
    verbose=1,
)

meta.fit(X_train, y_train)

# Step 5: Evaluate
print(f"Best: {meta.best_source_} - {meta.audit_trail_['selected']['name']}")
print(f"Test score: {meta.score(X_test, y_test):.4f}")

# Step 6: Export artifacts
import json
with open("audit_trail.json", "w") as f:
    json.dump(meta.audit_trail_, f, indent=2)
eda.to_json("eda_report.json")
```

## Related Documentation

- [Quick Start Guide](quickstart.md) - Basic ACDSO usage
- [Auto EDA Guide](auto_eda.md) - EDA features and integrations
- [LLM Configuration](llm_configuration.md) - LLM advisor setup
- [API Reference](api_reference.md) - Complete API documentation

