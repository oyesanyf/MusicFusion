"""
ACDSO v2.0: Risk-Optimized AutoML for Production Systems
=========================================================

A production-oriented AutoML library that unifies contextual risk diagnostics,
adaptive pipeline synthesis, and multi-objective optimization into a single
closed-loop optimization procedure.

ACDSO optimizes predictive performance alongside:
- Training cost
- Inference latency
- Memory footprint
- Quantifiable data/model risk (semantic leakage, temporal leakage,
  multicollinearity, instability)

Key Features:
- Continuous, severity-aware risk functions (not binary flags)
- Constraint-based pruning for production guardrails
- Pareto knee-point selection (not naive "best accuracy")
- JSON-serializable Risk Audit Trail for MLOps integration
- Extensible hooks for meta-learning and LLM-based advisory

Basic Usage:
    >>> from acdso import ACDSOClassifier, ACDSORegressor
    >>>
    >>> # Classification
    >>> clf = ACDSOClassifier(scorer="f1", cv=5, preference="safe")
    >>> clf.fit(X_train, y_train)
    >>> predictions = clf.predict(X_test)
    >>> print(clf.risk_report_)
    >>> print(clf.audit_trail_)
    >>>
    >>> # Regression
    >>> reg = ACDSORegressor(scorer="r2", cv=5)
    >>> reg.fit(X_train, y_train)
    >>> predictions = reg.predict(X_test)

Authors: DS-TUI Development Team
Version: 2.0.0 (Production-Ready)
License: MIT
"""

from __future__ import annotations

__version__ = "2.0.0"
__author__ = "DS-TUI Development Team"
__license__ = "MIT"

# Core estimators
from acdso.core import (
    ACDSOClassifier,
    ACDSORegressor,
)

# Auto EDA
from acdso.eda import (
    # Main class
    ACDSOAutoEDA,
    # Data classes
    ColumnProfile,
    DataQualityReport,
    EDAReport,
    EDACandidate,
    CorrelationResult,
    AnomalyResult,
    # Backend adapters
    EDABackend,
    get_available_backends,
    get_backend_adapter,
    YDataProfilingAdapter,
    SweetvizAdapter,
    DataprepAdapter,
    DTaleAdapter,
    # Standalone functions
    quick_profile,
    compare_datasets,
    validate_data,
    infer_data_types,
    get_preprocessing_recommendations,
)

# Risk analysis components
from acdso.risk import (
    RiskReport,
    analyze_risk,
    compute_risk_penalty,
)

# Utility functions
from acdso.utils import (
    build_preprocessor,
    detect_datetime_column,
    infer_target_type,
    split_columns,
)

# Pareto optimization
from acdso.pareto import (
    CandidateResult,
    pareto_front,
    knee_point_selection,
)

# LLM advisors
from acdso.advisors import (
    # Main advisor class
    LLMAdvisor,
    LLMConfig,
    PromptTemplate,
    # Providers
    BaseLLMProvider,
    OpenAIProvider,
    AnthropicProvider,
    AzureOpenAIProvider,
    OllamaProvider,
    LiteLLMProvider,
    # Factory and utilities
    create_advisor,
    llm_advisor_stub,
    BaseLLMAdvisor,
    StubLLMAdvisor,
)

# Integrations (lazy import to avoid requiring all dependencies)
def __getattr__(name):
    """Lazy import for integration modules."""
    if name == "integrations":
        from acdso import integrations
        return integrations
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")

__all__ = [
    # Version info
    "__version__",
    "__author__",
    "__license__",
    # Core estimators
    "ACDSOClassifier",
    "ACDSORegressor",
    # Auto EDA
    "ACDSOAutoEDA",
    "ColumnProfile",
    "DataQualityReport",
    "EDAReport",
    "EDACandidate",
    "CorrelationResult",
    "AnomalyResult",
    "EDABackend",
    "get_available_backends",
    "get_backend_adapter",
    "YDataProfilingAdapter",
    "SweetvizAdapter",
    "DataprepAdapter",
    "DTaleAdapter",
    "quick_profile",
    "compare_datasets",
    "validate_data",
    "infer_data_types",
    "get_preprocessing_recommendations",
    # Risk components
    "RiskReport",
    "analyze_risk",
    "compute_risk_penalty",
    # Utilities
    "build_preprocessor",
    "detect_datetime_column",
    "infer_target_type",
    "split_columns",
    # Pareto optimization
    "CandidateResult",
    "pareto_front",
    "knee_point_selection",
    # Advisors
    "LLMAdvisor",
    "LLMConfig",
    "PromptTemplate",
    "BaseLLMProvider",
    "OpenAIProvider",
    "AnthropicProvider",
    "AzureOpenAIProvider",
    "OllamaProvider",
    "LiteLLMProvider",
    "create_advisor",
    "llm_advisor_stub",
    "BaseLLMAdvisor",
    "StubLLMAdvisor",
    # Integrations submodule
    "integrations",
]

