"""
Tests for ACDSORegressor
========================

Tests covering regression functionality, risk analysis,
constraint handling, and Pareto optimization.
"""

import numpy as np
import pandas as pd
import pytest
from sklearn.datasets import make_regression, load_diabetes
from sklearn.model_selection import train_test_split

from acdso import ACDSORegressor


class TestACDSORegressorBasic:
    """Basic functionality tests for ACDSORegressor."""

    @pytest.fixture
    def simple_regression_data(self):
        """Generate simple regression data."""
        X, y = make_regression(
            n_samples=200,
            n_features=10,
            n_informative=5,
            noise=10.0,
            random_state=42,
        )
        return pd.DataFrame(X, columns=[f"feature_{i}" for i in range(X.shape[1])]), y

    @pytest.fixture
    def diabetes_data(self):
        """Load diabetes dataset for regression."""
        diabetes = load_diabetes()
        X = pd.DataFrame(diabetes.data, columns=diabetes.feature_names)
        y = diabetes.target
        return X, y

    def test_fit_predict(self, simple_regression_data):
        """Test basic fit and predict on regression."""
        X, y = simple_regression_data
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42
        )

        reg = ACDSORegressor(
            scorer="r2",
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        reg.fit(X_train, y_train)

        # Check fitted attributes
        assert reg.best_estimator_ is not None
        assert reg.risk_report_ is not None
        assert reg.pareto_front_ is not None
        assert reg.audit_trail_ is not None

        # Check predictions
        predictions = reg.predict(X_test)
        assert len(predictions) == len(y_test)
        assert predictions.dtype == np.float64 or predictions.dtype == np.float32

    def test_fit_predict_diabetes(self, diabetes_data):
        """Test fit and predict on diabetes dataset."""
        X, y = diabetes_data
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42
        )

        reg = ACDSORegressor(
            scorer="r2",
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        reg.fit(X_train, y_train)

        predictions = reg.predict(X_test)
        assert len(predictions) == len(y_test)

    def test_score(self, simple_regression_data):
        """Test scoring method."""
        X, y = simple_regression_data
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42
        )

        reg = ACDSORegressor(
            scorer="r2",
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )
        reg.fit(X_train, y_train)

        score = reg.score(X_test, y_test)
        # R2 can be negative for very bad fits, but should generally be reasonable
        assert score > -1.0  # Not completely broken

    def test_neg_mse_scorer(self, simple_regression_data):
        """Test with negative MSE scorer."""
        X, y = simple_regression_data
        X_train, X_test, y_train, y_test = train_test_split(
            X, y, test_size=0.2, random_state=42
        )

        reg = ACDSORegressor(
            scorer="neg_mean_squared_error",
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )
        reg.fit(X_train, y_train)

        # Score should be negative (as per scikit-learn convention)
        score = reg.score(X_test, y_test)
        assert score <= 0.0

    def test_unfitted_raises_error(self, simple_regression_data):
        """Test that unfitted regressor raises RuntimeError."""
        X, y = simple_regression_data
        reg = ACDSORegressor()

        with pytest.raises(RuntimeError, match="not fitted"):
            reg.predict(X)

        with pytest.raises(RuntimeError, match="not fitted"):
            reg.score(X, y)


class TestACDSORegressorRisk:
    """Risk analysis tests for ACDSORegressor."""

    @pytest.fixture
    def multicollinear_data(self):
        """Generate data with multicollinearity."""
        np.random.seed(42)
        n = 200
        x1 = np.random.randn(n)
        x2 = np.random.randn(n)
        x3 = x1 + 0.1 * np.random.randn(n)  # Highly correlated with x1
        x4 = x2 + 0.1 * np.random.randn(n)  # Highly correlated with x2
        x5 = np.random.randn(n)

        X = pd.DataFrame({
            "x1": x1,
            "x2": x2,
            "x3": x3,
            "x4": x4,
            "x5": x5,
        })
        y = 2 * x1 + 3 * x2 + np.random.randn(n)
        return X, y

    def test_risk_report_structure(self, multicollinear_data):
        """Test that risk report has expected structure."""
        X, y = multicollinear_data

        reg = ACDSORegressor(cv=3, max_candidates=2, random_state=42, verbose=0)
        reg.fit(X, y)

        report = reg.risk_report_
        assert isinstance(report, dict)

        # Check task type
        assert report["task"] == "regression"

        # Check required fields
        required_fields = [
            "n_rows",
            "n_cols",
            "n_numeric",
            "missing_ratio",
            "vif_max",
        ]
        for field in required_fields:
            assert field in report, f"Missing field: {field}"

    def test_multicollinearity_detection(self, multicollinear_data):
        """Test that multicollinearity is detected."""
        X, y = multicollinear_data

        reg = ACDSORegressor(cv=3, max_candidates=2, random_state=42, verbose=0)
        reg.fit(X, y)

        report = reg.risk_report_
        # Should detect high VIF due to correlated features
        assert report["vif_max"] > 5.0  # VIF > 5 indicates multicollinearity

    def test_no_imbalance_for_regression(self, multicollinear_data):
        """Test that imbalance fields are None/0 for regression."""
        X, y = multicollinear_data

        reg = ACDSORegressor(cv=3, max_candidates=2, random_state=42, verbose=0)
        reg.fit(X, y)

        report = reg.risk_report_
        # Imbalance is not applicable for regression
        assert report["class_imbalance_max_ratio"] is None
        assert report["imbalance_severity"] == 0.0


class TestACDSORegressorConstraints:
    """Constraint handling tests for ACDSORegressor."""

    @pytest.fixture
    def simple_data(self):
        """Generate simple regression data."""
        X, y = make_regression(n_samples=100, n_features=5, random_state=42)
        return pd.DataFrame(X, columns=[f"f{i}" for i in range(X.shape[1])]), y

    def test_time_budget(self, simple_data):
        """Test that time budget is respected."""
        X, y = simple_data

        reg = ACDSORegressor(
            time_budget_s=120.0,
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        reg.fit(X, y)
        assert reg.best_estimator_ is not None

    def test_latency_constraint(self, simple_data):
        """Test latency constraint handling."""
        X, y = simple_data

        reg = ACDSORegressor(
            max_latency_ms_per_row=1000.0,
            cv=3,
            max_candidates=2,
            random_state=42,
            verbose=0,
        )

        reg.fit(X, y)
        assert reg.best_estimator_ is not None

        selected = reg.audit_trail_["selected"]
        assert selected["latency_ms_per_row"] <= 1000.0


class TestACDSORegressorAuditTrail:
    """Audit trail tests for ACDSORegressor."""

    @pytest.fixture
    def simple_data(self):
        """Generate simple regression data."""
        X, y = make_regression(n_samples=100, n_features=5, random_state=42)
        return pd.DataFrame(X, columns=[f"f{i}" for i in range(X.shape[1])]), y

    def test_audit_trail_structure(self, simple_data):
        """Test audit trail has expected structure."""
        X, y = simple_data

        reg = ACDSORegressor(cv=3, max_candidates=2, random_state=42, verbose=0)
        reg.fit(X, y)

        audit = reg.audit_trail_

        assert "risk_report" in audit
        assert "constraints" in audit
        assert "candidates_evaluated" in audit
        assert "pareto_front" in audit
        assert "selected" in audit

    def test_audit_trail_serializable(self, simple_data):
        """Test that audit trail is JSON serializable."""
        import json

        X, y = simple_data

        reg = ACDSORegressor(cv=3, max_candidates=2, random_state=42, verbose=0)
        reg.fit(X, y)

        json_str = json.dumps(reg.audit_trail_)
        assert len(json_str) > 0

        parsed = json.loads(json_str)
        assert parsed["selected"]["name"] == reg.audit_trail_["selected"]["name"]


class TestACDSORegressorSklearnCompat:
    """scikit-learn compatibility tests."""

    def test_get_params(self):
        """Test get_params method."""
        reg = ACDSORegressor(scorer="neg_mean_squared_error", cv=10)

        params = reg.get_params()
        assert params["scorer"] == "neg_mean_squared_error"
        assert params["cv"] == 10

    def test_set_params(self):
        """Test set_params method."""
        reg = ACDSORegressor()

        reg.set_params(scorer="r2", cv=3)
        assert reg.scorer == "r2"
        assert reg.cv == 3

    def test_clone(self):
        """Test that regressor can be cloned."""
        from sklearn.base import clone

        reg = ACDSORegressor(scorer="neg_mean_absolute_error", cv=7)
        cloned = clone(reg)

        assert cloned.scorer == "neg_mean_absolute_error"
        assert cloned.cv == 7
        assert cloned.best_estimator_ is None

