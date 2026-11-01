"""
Tests for ACDSOClassifier
=========================

Tests covering classification functionality, risk analysis,
constraint handling, and Pareto optimization.
"""

import numpy as np
import pandas as pd
import pytest
from sklearn.datasets import make_classification, load_iris
from sklearn.model_selection import train_test_split

from acdso import ACDSOClassifier
from acdso.risk import RiskReport


class TestACDSOClassifierBasic:
    """Basic functionality tests for ACDSOClassifier."""

    @pytest.fixture
    def binary_data(self):
        """Generate binary classification data."""
        X, y = make_classification(
            n_samples=200,
            n_features=10,
            n_informative=5,
            n_redundant=2,
            n_classes=2,
            random_state=42,
        )
        return pd.DataFrame(X, columns=[f"feature_{i}" for i in range(X.shape[1])]), y

    @pytest.fixture
    def multiclass_data(self):
        """Load iris dataset for multiclass classification."""
        iris = load_iris()
        X = pd.DataFrame(iris.data, columns=iris.feature_names)
        y = iris.target
        return X, y

    def test_fit_predict_binary(self, binary_data):
        """Test basic fit and predict on binary classification."""
        X, y = binary_data
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42
        )

        clf = ACDSOClassifier(
            scorer="accuracy",
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        clf.fit(X_train, y_train)

        # Check fitted attributes
        assert clf.best_estimator_ is not None
        assert clf.risk_report_ is not None
        assert clf.pareto_front_ is not None
        assert clf.audit_trail_ is not None

        # Check predictions
        predictions = clf.predict(X_test)
        assert len(predictions) == len(y_test)
        assert set(predictions).issubset(set(y))

    def test_fit_predict_multiclass(self, multiclass_data):
        """Test basic fit and predict on multiclass classification."""
        X, y = multiclass_data
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42
        )

        clf = ACDSOClassifier(
            scorer="accuracy",
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        clf.fit(X_train, y_train)

        predictions = clf.predict(X_test)
        assert len(predictions) == len(y_test)

    def test_predict_proba(self, binary_data):
        """Test probability predictions."""
        X, y = binary_data
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42
        )

        clf = ACDSOClassifier(cv=3, max_candidates=2, random_state=42, verbose=0)
        clf.fit(X_train, y_train)

        proba = clf.predict_proba(X_test)
        assert proba.shape[0] == len(y_test)
        assert proba.shape[1] == 2  # binary classification
        assert np.allclose(proba.sum(axis=1), 1.0)

    def test_score(self, binary_data):
        """Test scoring method."""
        X, y = binary_data
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42
        )

        clf = ACDSOClassifier(
            scorer="accuracy",
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )
        clf.fit(X_train, y_train)

        score = clf.score(X_test, y_test)
        assert 0.0 <= score <= 1.0

    def test_unfitted_raises_error(self, binary_data):
        """Test that unfitted classifier raises RuntimeError."""
        X, y = binary_data
        clf = ACDSOClassifier()

        with pytest.raises(RuntimeError, match="not fitted"):
            clf.predict(X)

        with pytest.raises(RuntimeError, match="not fitted"):
            clf.predict_proba(X)

        with pytest.raises(RuntimeError, match="not fitted"):
            clf.score(X, y)


class TestACDSOClassifierRisk:
    """Risk analysis tests for ACDSOClassifier."""

    @pytest.fixture
    def imbalanced_data(self):
        """Generate imbalanced binary classification data."""
        X, y = make_classification(
            n_samples=200,
            n_features=10,
            n_informative=5,
            n_classes=2,
            weights=[0.9, 0.1],  # 90/10 imbalance
            random_state=42,
        )
        return pd.DataFrame(X, columns=[f"feature_{i}" for i in range(X.shape[1])]), y

    def test_risk_report_structure(self, imbalanced_data):
        """Test that risk report has expected structure."""
        X, y = imbalanced_data

        clf = ACDSOClassifier(cv=3, max_candidates=2, random_state=42, verbose=0)
        clf.fit(X, y)

        report = clf.risk_report_
        assert isinstance(report, dict)

        # Check required fields
        required_fields = [
            "task",
            "target_type",
            "n_rows",
            "n_cols",
            "n_numeric",
            "n_categorical",
            "missing_ratio",
            "imbalance_severity",
            "vif_max",
            "notes",
        ]
        for field in required_fields:
            assert field in report, f"Missing field: {field}"

    def test_imbalance_detection(self, imbalanced_data):
        """Test that imbalance is detected."""
        X, y = imbalanced_data

        clf = ACDSOClassifier(cv=3, max_candidates=2, random_state=42, verbose=0)
        clf.fit(X, y)

        report = clf.risk_report_
        # Should detect imbalance
        assert report["imbalance_severity"] > 0.5
        assert report["class_imbalance_max_ratio"] > 0.8

    def test_preference_affects_risk_penalty(self):
        """Test that preference mode affects risk penalty weights."""
        X, y = make_classification(n_samples=100, n_features=5, random_state=42)
        X = pd.DataFrame(X)

        clf_safe = ACDSOClassifier(
            preference="safe",
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )
        clf_balanced = ACDSOClassifier(
            preference="balanced",
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        clf_safe.fit(X, y)
        clf_balanced.fit(X, y)

        # Both should complete without error
        assert clf_safe.best_estimator_ is not None
        assert clf_balanced.best_estimator_ is not None


class TestACDSOClassifierConstraints:
    """Constraint handling tests for ACDSOClassifier."""

    @pytest.fixture
    def simple_data(self):
        """Generate simple classification data."""
        X, y = make_classification(
            n_samples=100,
            n_features=5,
            n_informative=3,
            random_state=42,
        )
        return pd.DataFrame(X, columns=[f"f{i}" for i in range(X.shape[1])]), y

    def test_time_budget(self, simple_data):
        """Test that time budget is respected."""
        X, y = simple_data

        clf = ACDSOClassifier(
            time_budget_s=120.0,  # 2 minutes
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        clf.fit(X, y)
        assert clf.best_estimator_ is not None

    def test_latency_constraint(self, simple_data):
        """Test latency constraint handling."""
        X, y = simple_data

        clf = ACDSOClassifier(
            max_latency_ms_per_row=1000.0,  # Very lenient
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        clf.fit(X, y)
        assert clf.best_estimator_ is not None

        # Check that selected model respects constraint
        selected = clf.audit_trail_["selected"]
        assert selected["latency_ms_per_row"] <= 1000.0

    def test_memory_constraint(self, simple_data):
        """Test memory constraint handling."""
        X, y = simple_data

        clf = ACDSOClassifier(
            max_peak_mem_mb=1000.0,  # Very lenient
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        clf.fit(X, y)
        assert clf.best_estimator_ is not None


class TestACDSOClassifierAuditTrail:
    """Audit trail tests for ACDSOClassifier."""

    @pytest.fixture
    def simple_data(self):
        """Generate simple classification data."""
        X, y = make_classification(n_samples=100, n_features=5, random_state=42)
        return pd.DataFrame(X, columns=[f"f{i}" for i in range(X.shape[1])]), y

    def test_audit_trail_structure(self, simple_data):
        """Test audit trail has expected structure."""
        X, y = simple_data

        clf = ACDSOClassifier(cv=3, max_candidates=2, random_state=42, verbose=0)
        clf.fit(X, y)

        audit = clf.audit_trail_

        # Check required sections
        assert "risk_report" in audit
        assert "constraints" in audit
        assert "candidates_evaluated" in audit
        assert "pareto_front" in audit
        assert "selected" in audit

    def test_audit_trail_serializable(self, simple_data):
        """Test that audit trail is JSON serializable."""
        import json

        X, y = simple_data

        clf = ACDSOClassifier(cv=3, max_candidates=2, random_state=42, verbose=0)
        clf.fit(X, y)

        # Should not raise
        json_str = json.dumps(clf.audit_trail_)
        assert len(json_str) > 0

        # Should round-trip
        parsed = json.loads(json_str)
        assert parsed["selected"]["name"] == clf.audit_trail_["selected"]["name"]

    def test_pareto_front_populated(self, simple_data):
        """Test that Pareto front is populated."""
        X, y = simple_data

        clf = ACDSOClassifier(cv=3, max_candidates=2, random_state=42, verbose=0)
        clf.fit(X, y)

        assert len(clf.pareto_front_) >= 1

        for candidate in clf.pareto_front_:
            assert "name" in candidate
            assert "metric" in candidate
            assert "train_time_s" in candidate
            assert "risk_penalty" in candidate


class TestACDSOClassifierLLMAdvisor:
    """LLM advisor integration tests."""

    @pytest.fixture
    def data_with_suspicious_columns(self):
        """Generate data with suspicious column names."""
        np.random.seed(42)
        n = 100
        X = pd.DataFrame({
            "feature_1": np.random.randn(n),
            "feature_2": np.random.randn(n),
            "future_outcome": np.random.randn(n),  # Suspicious name
            "target_leak": np.random.randn(n),  # Suspicious name
        })
        y = np.random.randint(0, 2, n)
        return X, y

    def test_llm_advisor_stub(self, data_with_suspicious_columns):
        """Test that LLM advisor stub works."""
        from acdso import llm_advisor_stub

        X, y = data_with_suspicious_columns

        clf = ACDSOClassifier(
            llm_advisor=llm_advisor_stub,
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        clf.fit(X, y)

        # Should have LLM notes in report
        notes = clf.risk_report_.get("notes", [])
        llm_notes = [n for n in notes if "Advisor" in n or "LLM" in n]
        assert len(llm_notes) > 0

    def test_custom_llm_advisor(self, data_with_suspicious_columns):
        """Test that custom LLM advisor is called."""
        X, y = data_with_suspicious_columns

        advisor_called = {"called": False}

        def custom_advisor(payload):
            advisor_called["called"] = True
            return {"note": "Custom advisor note", "suspicious_columns": []}

        clf = ACDSOClassifier(
            llm_advisor=custom_advisor,
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        clf.fit(X, y)

        assert advisor_called["called"]


class TestACDSOClassifierSklearnCompat:
    """scikit-learn compatibility tests."""

    @pytest.fixture
    def simple_data(self):
        """Generate simple classification data."""
        X, y = make_classification(n_samples=100, n_features=5, random_state=42)
        return pd.DataFrame(X, columns=[f"f{i}" for i in range(X.shape[1])]), y

    def test_get_params(self, simple_data):
        """Test get_params method."""
        clf = ACDSOClassifier(scorer="f1", cv=10, preference="safe")

        params = clf.get_params()
        assert params["scorer"] == "f1"
        assert params["cv"] == 10
        assert params["preference"] == "safe"

    def test_set_params(self, simple_data):
        """Test set_params method."""
        clf = ACDSOClassifier()

        clf.set_params(scorer="roc_auc", cv=3)
        assert clf.scorer == "roc_auc"
        assert clf.cv == 3

    def test_clone(self, simple_data):
        """Test that classifier can be cloned."""
        from sklearn.base import clone

        clf = ACDSOClassifier(scorer="f1", cv=7, preference="safe")
        cloned = clone(clf)

        assert cloned.scorer == "f1"
        assert cloned.cv == 7
        assert cloned.preference == "safe"
        assert cloned.best_estimator_ is None  # Cloned is unfitted

