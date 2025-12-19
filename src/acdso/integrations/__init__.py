"""
ACDSO v2.0 - Integrations Module
=================================

Adapters and wrappers for integrating ACDSO with other AutoML tools.

ACDSO acts as the unified decision layer that applies:
- Risk diagnostics
- Constraint pruning (latency/memory/time)
- Pareto front construction
- Knee-point selection
- Audit trail generation

Other AutoML tools become either:
- Candidate generators (they propose pipelines/models)
- Sub-optimizers (they search within a candidate family)

While ACDSO does the final multi-objective selection.

Supported Integrations:
- auto-sklearn (sklearn-native)
- TPOT (sklearn-native pipelines)
- FLAML (sklearn-friendly)
- H2O AutoML (adapter required)
- AutoGluon (adapter required)

Example Usage:
    >>> from acdso.integrations import ACDSOMetaOptimizer, AutoSklearnAdapter, TPOTAdapter
    >>>
    >>> # Create adapters for different AutoML tools
    >>> autosklearn_adapter = AutoSklearnAdapter(time_budget=120)
    >>> tpot_adapter = TPOTAdapter(generations=5)
    >>>
    >>> # Create meta-optimizer
    >>> meta = ACDSOMetaOptimizer(
    ...     adapters=[autosklearn_adapter, tpot_adapter],
    ...     max_latency_ms_per_row=10.0,
    ...     preference="safe",
    ... )
    >>>
    >>> # Fit and select best model
    >>> meta.fit(X_train, y_train)
    >>> print(meta.audit_trail_)
"""

from acdso.integrations.base import (
    BaseAutoMLAdapter,
    ExternalModelWrapper,
    CandidateSource,
)

from acdso.integrations.meta_optimizer import ACDSOMetaOptimizer

from acdso.integrations.sklearn_native import (
    AutoSklearnAdapter,
    TPOTAdapter,
    FLAMLAdapter,
)

from acdso.integrations.external import (
    H2OAdapter,
    AutoGluonAdapter,
    GenericModelWrapper,
)

__all__ = [
    # Base classes
    "BaseAutoMLAdapter",
    "ExternalModelWrapper",
    "CandidateSource",
    # Meta optimizer
    "ACDSOMetaOptimizer",
    # sklearn-native adapters
    "AutoSklearnAdapter",
    "TPOTAdapter",
    "FLAMLAdapter",
    # External adapters
    "H2OAdapter",
    "AutoGluonAdapter",
    "GenericModelWrapper",
]

