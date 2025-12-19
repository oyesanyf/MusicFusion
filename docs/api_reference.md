# ACDSO API Reference

## Core Estimators

### ACDSOClassifier

```python
class ACDSOClassifier(BaseEstimator, ClassifierMixin)
```

Risk-optimized AutoML classifier compatible with scikit-learn.

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `scorer` | str | "accuracy" | Scoring metric. Options: "accuracy", "f1", "f1_macro", "f1_weighted", "roc_auc", "precision", "recall" |
| `cv` | int | 5 | Number of cross-validation folds |
| `preference` | str | "balanced" | Risk preference: "safe" (higher leakage weight) or "balanced" |
| `time_budget_s` | float | None | Maximum total time budget in seconds |
| `max_candidates` | int | 4 | Maximum candidate pipelines to evaluate |
| `random_state` | int | 42 | Random seed for reproducibility |
| `max_train_time_s` | float | None | Maximum training time per candidate (constraint) |
| `max_peak_mem_mb` | float | None | Maximum peak memory usage (constraint) |
| `max_latency_ms_per_row` | float | None | Maximum inference latency per row (constraint) |
| `enable_feature_selection` | bool | True | Enable automatic feature selection for high-VIF data |
| `llm_advisor` | callable | None | LLM advisor function for semantic analysis |
| `verbose` | int | 0 | Verbosity level (0=silent, 1+=progress) |

#### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `best_estimator_` | Pipeline | Selected best pipeline after fitting |
| `risk_report_` | dict | Comprehensive risk analysis report |
| `pareto_front_` | list | List of Pareto-optimal candidates |
| `audit_trail_` | dict | JSON-serializable audit artifact |

#### Methods

##### fit(X, y)
Fit the ACDSO classifier.

```python
clf.fit(X_train, y_train)
```

##### predict(X)
Predict class labels.

```python
predictions = clf.predict(X_test)
```

##### predict_proba(X)
Predict class probabilities.

```python
probabilities = clf.predict_proba(X_test)
```

##### score(X, y)
Return score on test data.

```python
score = clf.score(X_test, y_test)
```

---

### ACDSORegressor

```python
class ACDSORegressor(BaseEstimator, RegressorMixin)
```

Risk-optimized AutoML regressor compatible with scikit-learn.

#### Parameters

Same as ACDSOClassifier, except:
- `scorer` default is "r2"
- Supports regression metrics: "r2", "neg_mean_squared_error", "neg_mean_absolute_error"

#### Methods

Same as ACDSOClassifier, except no `predict_proba()`.

---

## Risk Analysis

### RiskReport

```python
@dataclass
class RiskReport
```

Comprehensive risk report for a dataset.

#### Fields

| Field | Type | Description |
|-------|------|-------------|
| `task` | str | "classification" or "regression" |
| `target_type` | str | "binary", "multiclass", or "regression" |
| `n_rows` | int | Number of rows |
| `n_cols` | int | Number of columns |
| `n_numeric` | int | Number of numeric columns |
| `n_categorical` | int | Number of categorical columns |
| `missing_ratio` | float | Ratio of missing values (0-1) |
| `leakage_proxy_features` | List[str] | Features suspected of leakage |
| `leakage_proxy_score_max` | float | Maximum univariate predictive power |
| `temporal_leakage_gap` | float | Score gap (shuffled - chronological) |
| `class_imbalance_max_ratio` | float | Maximum class ratio |
| `imbalance_severity` | float | Severity score (0-1) |
| `vif_max` | float | Maximum VIF value |
| `vif_features` | List[str] | Features with high VIF (>=10) |
| `cv_score_std` | float | CV score standard deviation |
| `notes` | List[str] | Human-readable warnings |

### analyze_risk()

```python
def analyze_risk(
    X: Any,
    y: Any,
    task: str,
    scorer: str = "accuracy",
    cv: int = 5,
    random_state: int = 42,
    llm_advisor: Optional[Callable] = None,
) -> RiskReport
```

Perform comprehensive risk analysis on a dataset.

### compute_risk_penalty()

```python
def compute_risk_penalty(
    rr: RiskReport,
    cv_score_std: float,
    preference: str = "balanced",
) -> float
```

Compute continuous, severity-weighted risk penalty (0-1).

---

## Pareto Optimization

### CandidateResult

```python
@dataclass
class CandidateResult
```

Result of evaluating a candidate pipeline.

#### Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | str | Candidate identifier |
| `pipeline` | Any | Fitted sklearn Pipeline |
| `metric` | float | Primary evaluation metric |
| `train_time_s` | float | Training time in seconds |
| `peak_mem_mb` | float | Peak memory in MB |
| `latency_ms_per_row` | float | Inference latency in ms/row |
| `risk_penalty` | float | Risk penalty (0-1) |
| `objectives` | Tuple | 5-dimensional objective vector |
| `meta` | Dict | Additional metadata |

### pareto_front()

```python
def pareto_front(candidates: List[CandidateResult]) -> List[CandidateResult]
```

Extract Pareto front from candidates.

### knee_point_selection()

```python
def knee_point_selection(front: List[CandidateResult]) -> CandidateResult
```

Select knee point from Pareto front.

---

## LLM Advisors

### LLMAdvisor

```python
class LLMAdvisor
```

Production-ready LLM advisor.

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `provider` | BaseLLMProvider | required | LLM provider instance |
| `config` | LLMConfig | None | Configuration options |
| `prompt_template` | PromptTemplate | None | Custom prompts |
| `fallback_provider` | BaseLLMProvider | None | Fallback provider |

#### Methods

##### advise(payload)
Analyze dataset and return advisory.

```python
result = advisor.advise({
    "columns": ["col1", "col2"],
    "task": "classification",
    "target_type": "binary",
})
```

### LLMConfig

```python
@dataclass
class LLMConfig
```

Configuration for LLM advisor.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_tokens` | int | 500 | Max response tokens |
| `temperature` | float | 0.3 | Sampling temperature |
| `timeout_s` | float | 30.0 | Request timeout |
| `max_retries` | int | 3 | Max retry attempts |
| `cache_enabled` | bool | True | Enable caching |
| `cache_ttl_s` | int | 3600 | Cache TTL in seconds |

### Providers

#### OpenAIProvider

```python
OpenAIProvider(
    api_key: str = None,      # Uses OPENAI_API_KEY env var
    model: str = "gpt-4o-mini",
    base_url: str = None,
    organization: str = None,
)
```

#### AnthropicProvider

```python
AnthropicProvider(
    api_key: str = None,      # Uses ANTHROPIC_API_KEY env var
    model: str = "claude-3-haiku-20240307",
)
```

#### AzureOpenAIProvider

```python
AzureOpenAIProvider(
    api_key: str = None,
    endpoint: str = None,
    deployment_name: str = "gpt-4",
    api_version: str = "2024-02-15-preview",
)
```

#### OllamaProvider

```python
OllamaProvider(
    model: str = "llama3.2",
    base_url: str = "http://localhost:11434",
)
```

#### LiteLLMProvider

```python
LiteLLMProvider(
    model: str,               # Any LiteLLM-supported model
    api_key: str = None,
    api_base: str = None,
)
```

### create_advisor()

```python
def create_advisor(
    provider: str = "openai",
    model: str = None,
    api_key: str = None,
    **kwargs,
) -> LLMAdvisor
```

Factory function to create an LLM advisor.

```python
advisor = create_advisor("openai", model="gpt-4o")
advisor = create_advisor("anthropic", model="claude-3-sonnet-20240229")
advisor = create_advisor("ollama", model="llama3")
```

---

## Integrations

### ACDSOMetaOptimizer

```python
class ACDSOMetaOptimizer(BaseEstimator)
```

Unified decision layer for multiple AutoML tools.

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `adapters` | List[BaseAutoMLAdapter] | [] | AutoML adapters |
| `include_acdso_candidates` | bool | True | Include ACDSO's own candidates |
| `scorer` | str | "accuracy" | Scoring metric |
| `cv` | int | 5 | Cross-validation folds |
| `preference` | str | "balanced" | Risk preference |
| `max_train_time_s` | float | None | Training time constraint |
| `max_peak_mem_mb` | float | None | Memory constraint |
| `max_latency_ms_per_row` | float | None | Latency constraint |
| `time_budget_s` | float | None | Total time budget |
| `verbose` | int | 0 | Verbosity level |

### Adapters

#### AutoSklearnAdapter

```python
AutoSklearnAdapter(
    time_budget: int = 120,
    top_k: int = 5,
)
```

#### TPOTAdapter

```python
TPOTAdapter(
    generations: int = 5,
    population_size: int = 50,
    top_k: int = 5,
)
```

#### FLAMLAdapter

```python
FLAMLAdapter(
    time_budget: int = 60,
    top_k: int = 5,
)
```

#### H2OAdapter

```python
H2OAdapter(
    max_runtime_secs: int = 120,
    max_models: int = 20,
    top_k: int = 5,
)
```

#### AutoGluonAdapter

```python
AutoGluonAdapter(
    time_limit: int = 120,
    presets: str = "medium_quality",
    top_k: int = 5,
)
```

### GenericModelWrapper

```python
GenericModelWrapper(
    model: Any,
    predict_method: str = "predict",
    predict_proba_method: str = None,
    name: str = "GenericModel",
)
```

Wrap any model for ACDSO evaluation.

