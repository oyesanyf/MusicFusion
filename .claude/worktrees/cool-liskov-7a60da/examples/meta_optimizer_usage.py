"""
ACDSO v2.0 - Meta Optimizer Examples
=====================================

This script demonstrates using ACDSO as a unified decision layer
across multiple AutoML tools.

The key architecture:
- Each AutoML tool generates its best candidates
- ACDSO applies risk diagnostics, constraint pruning, Pareto front
- Knee-point selection chooses the optimal tradeoff
- Audit trail documents the entire process
"""

import numpy as np
import pandas as pd
from sklearn.datasets import make_classification, make_regression
from sklearn.model_selection import train_test_split

# Note: These imports will only work if the respective tools are installed


def example_multi_source_classification():
    """
    Example: Classification with multiple AutoML sources.

    This example shows how to combine candidates from multiple
    AutoML tools and let ACDSO select the best one.
    """
    print("=" * 70)
    print("Multi-Source Classification Example")
    print("=" * 70)

    # Generate synthetic data
    X, y = make_classification(
        n_samples=1000,
        n_features=20,
        n_informative=10,
        n_redundant=5,
        n_classes=2,
        random_state=42,
    )
    X = pd.DataFrame(X, columns=[f"feature_{i}" for i in range(X.shape[1])])
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42
    )

    from acdso.integrations import ACDSOMetaOptimizer

    # Check which adapters are available
    adapters = []

    try:
        from acdso.integrations import FLAMLAdapter
        adapters.append(FLAMLAdapter(time_budget=30, top_k=3))
        print("  - FLAML adapter: available")
    except ImportError:
        print("  - FLAML adapter: not installed")

    try:
        from acdso.integrations import TPOTAdapter
        adapters.append(TPOTAdapter(generations=3, population_size=20, top_k=3))
        print("  - TPOT adapter: available")
    except ImportError:
        print("  - TPOT adapter: not installed")

    try:
        from acdso.integrations import AutoSklearnAdapter
        adapters.append(AutoSklearnAdapter(time_budget=60, top_k=3))
        print("  - auto-sklearn adapter: available")
    except ImportError:
        print("  - auto-sklearn adapter: not installed")

    if not adapters:
        print("\nNo AutoML tools installed. Using ACDSO native candidates only.")

    # Create meta optimizer
    meta = ACDSOMetaOptimizer(
        adapters=adapters,
        include_acdso_candidates=True,  # Always include ACDSO's own candidates
        scorer="f1",
        cv=5,
        preference="safe",
        max_latency_ms_per_row=10.0,  # Production constraint
        verbose=1,
    )

    # Fit and select
    print("\nRunning meta optimization...")
    meta.fit(X_train, y_train)

    # Results
    print("\n" + "=" * 50)
    print("RESULTS")
    print("=" * 50)
    print(f"Selected Model: {meta.audit_trail_['selected']['name']}")
    print(f"Source: {meta.best_source_}")
    print(f"Metric (F1): {meta.audit_trail_['selected']['metric']:.4f}")
    print(f"Latency: {meta.audit_trail_['selected']['latency_ms_per_row']:.2f} ms/row")

    # Test performance
    score = meta.score(X_test, y_test)
    print(f"Test F1 Score: {score:.4f}")

    # Source summary
    print("\nSource Summary:")
    summary = meta.get_source_summary()
    for source, stats in summary.items():
        print(f"  {source}: {stats['total']} candidates, "
              f"{stats['in_pareto']} in Pareto, "
              f"{'SELECTED' if stats['selected'] else ''}")

    # Pareto front
    print("\nPareto Front:")
    for candidate in meta.pareto_front_:
        print(f"  - {candidate['name']} ({candidate['source']}): "
              f"metric={candidate['metric']:.4f}, "
              f"latency={candidate['latency_ms_per_row']:.2f}ms")

    return meta


def example_with_custom_candidates():
    """
    Example: Add custom pre-trained models to ACDSO evaluation.

    This shows how to wrap any model (from any source) and have
    ACDSO evaluate it alongside other candidates.
    """
    print("\n" + "=" * 70)
    print("Custom Candidates Example")
    print("=" * 70)

    from sklearn.ensemble import GradientBoostingClassifier, AdaBoostClassifier
    from sklearn.linear_model import LogisticRegression

    from acdso.integrations import (
        ACDSOMetaOptimizer,
        CandidateSource,
        GenericModelWrapper,
    )

    # Generate data
    X, y = make_classification(n_samples=500, n_features=10, random_state=42)
    X = pd.DataFrame(X)
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42
    )

    # Train some custom models
    print("\nTraining custom models...")

    models = [
        ("GradientBoosting_custom", GradientBoostingClassifier(n_estimators=50, random_state=42)),
        ("AdaBoost_custom", AdaBoostClassifier(n_estimators=50, random_state=42)),
        ("LogisticRegression_custom", LogisticRegression(max_iter=1000, random_state=42)),
    ]

    custom_candidates = []
    for name, model in models:
        model.fit(X_train, y_train)

        # Wrap the fitted model
        wrapper = GenericModelWrapper(
            model=model,
            predict_method="predict",
            predict_proba_method="predict_proba",
            name=name,
            source="custom",
        )

        custom_candidates.append(CandidateSource(
            name=name,
            source="custom",
            estimator=wrapper,
            metadata={"type": type(model).__name__},
        ))

        print(f"  - Trained {name}")

    # Create a custom adapter that returns our pre-trained models
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
            return {"adapter": self.name, "candidates_generated": len(self.candidates)}

    # Create meta optimizer with custom adapter
    meta = ACDSOMetaOptimizer(
        adapters=[CustomAdapter(custom_candidates)],
        include_acdso_candidates=True,
        scorer="accuracy",
        cv=3,
        verbose=1,
    )

    print("\nRunning meta optimization with custom candidates...")
    meta.fit(X_train, y_train)

    print("\n" + "=" * 50)
    print("RESULTS")
    print("=" * 50)
    print(f"Selected Model: {meta.audit_trail_['selected']['name']}")
    print(f"Source: {meta.best_source_}")
    print(f"Test Accuracy: {meta.score(X_test, y_test):.4f}")

    return meta


def example_save_audit_trail():
    """
    Example: Save comprehensive audit trail for MLOps.

    The audit trail can be saved to a file, artifact store,
    or experiment tracker for production ML governance.
    """
    print("\n" + "=" * 70)
    print("Audit Trail Export Example")
    print("=" * 70)

    import json

    from acdso.integrations import ACDSOMetaOptimizer

    X, y = make_classification(n_samples=300, n_features=10, random_state=42)
    X = pd.DataFrame(X)

    meta = ACDSOMetaOptimizer(
        adapters=[],  # Just ACDSO native
        include_acdso_candidates=True,
        scorer="f1",
        cv=3,
        verbose=0,
    )
    meta.fit(X, y)

    # Export audit trail
    audit_json = json.dumps(meta.audit_trail_, indent=2)

    print("\nAudit Trail Structure:")
    print("  - task")
    print("  - total_time_s")
    print("  - risk_report")
    print("  - constraints")
    print("  - adapter_stats")
    print("  - candidates_evaluated")
    print("  - pareto_front")
    print("  - pruned_or_failed")
    print("  - selected")

    print("\nSelected Model Details:")
    print(json.dumps(meta.audit_trail_["selected"], indent=2))

    # In production:
    # with open("audit_trail.json", "w") as f:
    #     json.dump(meta.audit_trail_, f, indent=2)

    return meta


def example_constraint_enforcement():
    """
    Example: Enforce production constraints across all sources.

    ACDSO enforces latency, memory, and training time constraints
    regardless of which AutoML tool generated the candidate.
    """
    print("\n" + "=" * 70)
    print("Constraint Enforcement Example")
    print("=" * 70)

    from acdso.integrations import ACDSOMetaOptimizer

    X, y = make_classification(n_samples=500, n_features=15, random_state=42)
    X = pd.DataFrame(X)
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42
    )

    # Strict production constraints
    meta = ACDSOMetaOptimizer(
        adapters=[],
        include_acdso_candidates=True,
        scorer="f1",
        cv=3,
        max_latency_ms_per_row=5.0,   # Very strict: 5ms max
        max_peak_mem_mb=256.0,         # 256MB max
        max_train_time_s=30.0,         # 30s max per candidate
        verbose=1,
    )

    print("\nConstraints:")
    print(f"  - Max Latency: 5.0 ms/row")
    print(f"  - Max Memory: 256.0 MB")
    print(f"  - Max Train Time: 30.0 s")

    meta.fit(X_train, y_train)

    print("\n" + "=" * 50)
    print("RESULTS")
    print("=" * 50)

    # Show which candidates were pruned
    pruned = meta.audit_trail_.get("pruned_or_failed", [])
    if pruned:
        print(f"\nPruned {len(pruned)} candidates:")
        for p in pruned[:5]:
            print(f"  - {p['name']}: {p['reason']}")

    print(f"\nSelected: {meta.audit_trail_['selected']['name']}")
    print(f"Latency: {meta.audit_trail_['selected']['latency_ms_per_row']:.2f} ms/row")
    print(f"Memory: {meta.audit_trail_['selected']['peak_mem_mb']:.1f} MB")

    return meta


if __name__ == "__main__":
    example_multi_source_classification()
    example_with_custom_candidates()
    example_save_audit_trail()
    example_constraint_enforcement()

    print("\n" + "=" * 70)
    print("All examples completed successfully!")
    print("=" * 70)

