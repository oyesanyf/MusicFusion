"""
ACDSO v2.0 - Basic Usage Examples
=================================

This script demonstrates basic usage of ACDSOClassifier and ACDSORegressor
for classification and regression tasks.
"""

import numpy as np
import pandas as pd
from sklearn.datasets import make_classification, make_regression, load_iris
from sklearn.model_selection import train_test_split

from acdso import ACDSOClassifier, ACDSORegressor


def example_binary_classification():
    """Example: Binary classification with production constraints."""
    print("=" * 60)
    print("Binary Classification Example")
    print("=" * 60)

    # Generate synthetic data
    X, y = make_classification(
        n_samples=500,
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

    # Create classifier with production constraints
    clf = ACDSOClassifier(
        scorer="f1",
        cv=5,
        preference="safe",
        max_latency_ms_per_row=10.0,  # Max 10ms per prediction
        max_peak_mem_mb=500.0,        # Max 500MB memory
        time_budget_s=120,            # Max 2 minutes total
        verbose=1,
    )

    # Fit the model
    clf.fit(X_train, y_train)

    # Make predictions
    predictions = clf.predict(X_test)
    score = clf.score(X_test, y_test)

    print(f"\nTest F1 Score: {score:.4f}")
    print(f"Selected Model: {clf.audit_trail_['selected']['name']}")

    # Print risk report highlights
    print("\nRisk Report Highlights:")
    print(f"  - Task: {clf.risk_report_['task']}")
    print(f"  - Target Type: {clf.risk_report_['target_type']}")
    print(f"  - Imbalance Severity: {clf.risk_report_['imbalance_severity']:.2f}")
    print(f"  - VIF Max: {clf.risk_report_['vif_max']:.2f}")
    print(f"  - Notes: {clf.risk_report_['notes']}")

    # Print Pareto front
    print("\nPareto Front:")
    for candidate in clf.pareto_front_:
        print(f"  - {candidate['name']}: metric={candidate['metric']:.4f}, "
              f"latency={candidate['latency_ms_per_row']:.2f}ms/row")

    return clf


def example_multiclass_classification():
    """Example: Multiclass classification with Iris dataset."""
    print("\n" + "=" * 60)
    print("Multiclass Classification Example (Iris)")
    print("=" * 60)

    # Load iris dataset
    iris = load_iris()
    X = pd.DataFrame(iris.data, columns=iris.feature_names)
    y = iris.target

    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42
    )

    # Create classifier
    clf = ACDSOClassifier(
        scorer="accuracy",
        cv=5,
        preference="balanced",
        max_candidates=3,
        verbose=1,
    )

    clf.fit(X_train, y_train)

    predictions = clf.predict(X_test)
    score = clf.score(X_test, y_test)

    print(f"\nTest Accuracy: {score:.4f}")
    print(f"Selected Model: {clf.audit_trail_['selected']['name']}")

    return clf


def example_regression():
    """Example: Regression with production constraints."""
    print("\n" + "=" * 60)
    print("Regression Example")
    print("=" * 60)

    # Generate synthetic regression data
    X, y = make_regression(
        n_samples=500,
        n_features=15,
        n_informative=8,
        noise=20.0,
        random_state=42,
    )
    X = pd.DataFrame(X, columns=[f"feature_{i}" for i in range(X.shape[1])])

    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42
    )

    # Create regressor
    reg = ACDSORegressor(
        scorer="r2",
        cv=5,
        preference="balanced",
        max_latency_ms_per_row=5.0,
        time_budget_s=60,
        verbose=1,
    )

    reg.fit(X_train, y_train)

    predictions = reg.predict(X_test)
    score = reg.score(X_test, y_test)

    print(f"\nTest R2 Score: {score:.4f}")
    print(f"Selected Model: {reg.audit_trail_['selected']['name']}")

    # Print risk report
    print("\nRisk Report Highlights:")
    print(f"  - VIF Max: {reg.risk_report_['vif_max']:.2f}")
    print(f"  - Missing Ratio: {reg.risk_report_['missing_ratio']:.2%}")

    return reg


def example_with_llm_advisor():
    """Example: Using LLM advisor for semantic analysis."""
    print("\n" + "=" * 60)
    print("LLM Advisor Example")
    print("=" * 60)

    from acdso import llm_advisor_stub

    # Create data with suspicious column names
    np.random.seed(42)
    n = 300
    X = pd.DataFrame({
        "feature_1": np.random.randn(n),
        "feature_2": np.random.randn(n),
        "feature_3": np.random.randn(n),
        "future_outcome": np.random.randn(n),  # Suspicious!
        "target_leak": np.random.randn(n),     # Suspicious!
    })
    y = np.random.randint(0, 2, n)

    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42
    )

    # Create classifier with LLM advisor
    clf = ACDSOClassifier(
        scorer="f1",
        cv=3,
        llm_advisor=llm_advisor_stub,  # Use stub advisor
        verbose=1,
    )

    clf.fit(X_train, y_train)

    print("\nRisk Notes (with LLM analysis):")
    for note in clf.risk_report_['notes']:
        print(f"  - {note}")

    return clf


def example_save_audit_trail():
    """Example: Saving audit trail for MLOps integration."""
    print("\n" + "=" * 60)
    print("Audit Trail Export Example")
    print("=" * 60)

    import json

    X, y = make_classification(n_samples=200, n_features=10, random_state=42)
    X = pd.DataFrame(X)

    clf = ACDSOClassifier(cv=3, max_candidates=2, verbose=0)
    clf.fit(X, y)

    # Export audit trail to JSON
    audit_json = json.dumps(clf.audit_trail_, indent=2)

    print("Audit Trail (JSON):")
    print(audit_json[:500] + "...\n")

    # In production, you might save this to a file or artifact store:
    # with open("audit_trail.json", "w") as f:
    #     json.dump(clf.audit_trail_, f, indent=2)

    return clf


if __name__ == "__main__":
    # Run all examples
    example_binary_classification()
    example_multiclass_classification()
    example_regression()
    example_with_llm_advisor()
    example_save_audit_trail()

    print("\n" + "=" * 60)
    print("All examples completed successfully!")
    print("=" * 60)

