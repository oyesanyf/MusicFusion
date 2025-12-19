# Changelog

All notable changes to ACDSO will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2025-12-18

### Added

- **ACDSOClassifier**: Production-ready scikit-learn compatible classifier
- **ACDSORegressor**: Production-ready scikit-learn compatible regressor
- **Risk Analysis Module**:
  - Continuous, severity-aware risk penalties
  - Leakage proxy detection via univariate predictive power
  - Temporal leakage detection (chronological vs shuffled CV)
  - VIF-based multicollinearity detection
  - Class imbalance severity scoring
  - Instability risk via CV score variance
- **Multi-Objective Optimization**:
  - 5-dimensional objective vector (metric, time, memory, latency, risk)
  - Log-scaled operational metrics
  - Pareto front construction
  - Knee-point selection algorithm
- **Constraint-Based Pruning**:
  - `max_train_time_s`: Maximum training time per candidate
  - `max_peak_mem_mb`: Maximum peak memory usage
  - `max_latency_ms_per_row`: Maximum inference latency
- **Audit Trail**:
  - JSON-serializable `risk_report_` attribute
  - JSON-serializable `pareto_front_` attribute
  - JSON-serializable `audit_trail_` with full evaluation details
- **LLM Advisory Integration**:
  - `BaseLLMAdvisor` abstract base class
  - `StubLLMAdvisor` for testing/development
  - `OpenAIAdvisorTemplate` for OpenAI integration
  - `llm_advisor_stub` functional wrapper
- **Adaptive Candidate Generation**:
  - LogisticRegression / Ridge for linear baselines
  - RandomForest for ensemble methods
  - XGBoost / LightGBM for large datasets (optional)
  - Automatic class weight balancing for imbalanced data
  - Automatic feature selection for high multicollinearity

### Changed

- Complete rewrite from proof-of-concept to production-grade implementation
- Risk penalties now use continuous functions instead of binary flags
- Model selection uses Pareto knee-point instead of "best accuracy"

### Fixed

- Memory measurement now uses tracemalloc for accurate peak memory tracking
- Latency measurement uses actual inference timing

## [1.0.0] - 2025-01-01

### Added

- Initial proof-of-concept implementation
- Basic risk heuristics
- Simple candidate pool

