"""
Tests for Integration Module
============================

Tests for AutoML integration adapters and meta optimizer.
"""

import numpy as np
import pandas as pd
import pytest
from sklearn.datasets import make_classification, make_regression
from sklearn.model_selection import train_test_split
from sklearn.ensemble import RandomForestClassifier

from acdso.integrations.base import (
    BaseAutoMLAdapter,
    ExternalModelWrapper,
    CandidateSource,
    SklearnNativeAdapter,
)
from acdso.integrations.external import GenericModelWrapper
from acdso.integrations.meta_optimizer import ACDSOMetaOptimizer


class TestExternalModelWrapper:
    """Tests for ExternalModelWrapper."""

    def test_basic_wrapper(self):
        """Test basic wrapper functionality."""
        # Create a simple prediction function
        def predict_fn(X):
            return np.ones(len(X))

        wrapper = ExternalModelWrapper(
            predict_fn=predict_fn,
            name="TestModel",
            source="test",
        )

        X = np.random.randn(10, 5)
        preds = wrapper.predict(X)

        assert len(preds) == 10
        assert np.all(preds == 1)

    def test_wrapper_with_proba(self):
        """Test wrapper with probability prediction."""
        def predict_fn(X):
            return np.zeros(len(X))

        def predict_proba_fn(X):
            n = len(X)
            return np.column_stack([np.ones(n) * 0.3, np.ones(n) * 0.7])

        wrapper = ExternalModelWrapper(
            predict_fn=predict_fn,
            predict_proba_fn=predict_proba_fn,
            name="TestModel",
            source="test",
        )

        X = np.random.randn(10, 5)
        proba = wrapper.predict_proba(X)

        assert proba.shape == (10, 2)
        assert np.allclose(proba[:, 0], 0.3)
        assert np.allclose(proba[:, 1], 0.7)

    def test_wrapper_no_proba_raises(self):
        """Test that missing predict_proba raises AttributeError."""
        wrapper = ExternalModelWrapper(
            predict_fn=lambda X: np.zeros(len(X)),
            name="TestModel",
            source="test",
        )

        X = np.random.randn(10, 5)

        with pytest.raises(AttributeError, match="predict_proba"):
            wrapper.predict_proba(X)


class TestGenericModelWrapper:
    """Tests for GenericModelWrapper."""

    def test_wrap_sklearn_model(self):
        """Test wrapping a sklearn model."""
        X, y = make_classification(n_samples=100, n_features=5, random_state=42)
        X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=42)

        # Train a model
        model = RandomForestClassifier(n_estimators=10, random_state=42)
        model.fit(X_train, y_train)

        # Wrap it
        wrapper = GenericModelWrapper(
            model=model,
            predict_method="predict",
            predict_proba_method="predict_proba",
            name="WrappedRF",
            source="sklearn",
        )

        # Test predictions
        preds = wrapper.predict(X_test)
        proba = wrapper.predict_proba(X_test)

        assert len(preds) == len(y_test)
        assert proba.shape[0] == len(y_test)
        assert proba.shape[1] == 2

    def test_wrap_with_preprocessing(self):
        """Test wrapper with preprocessing function."""
        model = RandomForestClassifier(n_estimators=10, random_state=42)
        X, y = make_classification(n_samples=100, n_features=5, random_state=42)
        model.fit(X, y)

        # Wrapper that converts DataFrame to array
        wrapper = GenericModelWrapper(
            model=model,
            predict_method="predict",
            preprocess_fn=lambda X: np.asarray(X),
            name="WrappedRF",
            source="sklearn",
        )

        # Test with DataFrame input
        X_df = pd.DataFrame(X[:10])
        preds = wrapper.predict(X_df)

        assert len(preds) == 10


class TestCandidateSource:
    """Tests for CandidateSource dataclass."""

    def test_creation(self):
        """Test CandidateSource creation."""
        source = CandidateSource(
            name="TestCandidate",
            source="test_tool",
            estimator=None,
            metadata={"rank": 1, "score": 0.95},
        )

        assert source.name == "TestCandidate"
        assert source.source == "test_tool"
        assert source.metadata["rank"] == 1


class MockAdapter(SklearnNativeAdapter):
    """Mock adapter for testing."""

    def __init__(self, models, **kwargs):
        super().__init__(name="MockAdapter", **kwargs)
        self.models = models

    def is_available(self):
        return True

    def generate_candidates(self, X, y, task):
        candidates = []
        for i, model in enumerate(self.models):
            model.fit(X, y)
            candidates.append(self.wrap_estimator(
                estimator=model,
                name=f"mock_model_{i}",
                rank=i,
            ))
        return candidates


class TestACDSOMetaOptimizer:
    """Tests for ACDSOMetaOptimizer."""

    @pytest.fixture
    def classification_data(self):
        """Generate classification data."""
        X, y = make_classification(
            n_samples=200,
            n_features=10,
            n_informative=5,
            random_state=42,
        )
        return pd.DataFrame(X), y

    @pytest.fixture
    def regression_data(self):
        """Generate regression data."""
        X, y = make_regression(
            n_samples=200,
            n_features=10,
            n_informative=5,
            random_state=42,
        )
        return pd.DataFrame(X), y

    def test_fit_with_native_candidates_only(self, classification_data):
        """Test meta optimizer with only ACDSO native candidates."""
        X, y = classification_data
        X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=42)

        meta = ACDSOMetaOptimizer(
            adapters=[],
            include_acdso_candidates=True,
            scorer="accuracy",
            cv=3,
            verbose=0,
        )

        meta.fit(X_train, y_train)

        assert meta.best_estimator_ is not None
        assert meta.best_source_ == "ACDSO"
        assert meta.audit_trail_ is not None

        predictions = meta.predict(X_test)
        assert len(predictions) == len(y_test)

    def test_fit_with_mock_adapter(self, classification_data):
        """Test meta optimizer with mock adapter."""
        from sklearn.linear_model import LogisticRegression
        from sklearn.ensemble import RandomForestClassifier

        X, y = classification_data
        X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=42)

        models = [
            LogisticRegression(max_iter=1000, random_state=42),
            RandomForestClassifier(n_estimators=10, random_state=42),
        ]

        adapter = MockAdapter(models, top_k=2)

        meta = ACDSOMetaOptimizer(
            adapters=[adapter],
            include_acdso_candidates=False,
            scorer="accuracy",
            cv=3,
            verbose=0,
        )

        meta.fit(X_train, y_train)

        assert meta.best_estimator_ is not None
        assert "mock_model" in meta.audit_trail_["selected"]["name"]

    def test_constraint_filtering(self, classification_data):
        """Test that constraints filter candidates."""
        X, y = classification_data

        meta = ACDSOMetaOptimizer(
            adapters=[],
            include_acdso_candidates=True,
            scorer="accuracy",
            cv=3,
            max_latency_ms_per_row=0.001,  # Impossibly strict
            verbose=0,
        )

        # This should either work with one candidate or fail
        try:
            meta.fit(X, y)
            # If it succeeds, check that something was selected
            assert meta.best_estimator_ is not None
        except RuntimeError as e:
            # Expected if all candidates are pruned
            assert "No candidates passed" in str(e)

    def test_audit_trail_structure(self, classification_data):
        """Test audit trail has expected structure."""
        X, y = classification_data

        meta = ACDSOMetaOptimizer(
            adapters=[],
            include_acdso_candidates=True,
            scorer="accuracy",
            cv=3,
            verbose=0,
        )

        meta.fit(X, y)

        audit = meta.audit_trail_

        assert "task" in audit
        assert "total_time_s" in audit
        assert "risk_report" in audit
        assert "constraints" in audit
        assert "adapter_stats" in audit
        assert "candidates_evaluated" in audit
        assert "pareto_front" in audit
        assert "selected" in audit

    def test_source_summary(self, classification_data):
        """Test get_source_summary method."""
        X, y = classification_data

        meta = ACDSOMetaOptimizer(
            adapters=[],
            include_acdso_candidates=True,
            scorer="accuracy",
            cv=3,
            verbose=0,
        )

        meta.fit(X, y)

        summary = meta.get_source_summary()

        assert "ACDSO" in summary
        assert summary["ACDSO"]["total"] > 0
        assert summary["ACDSO"]["selected"] == True

    def test_predict_proba(self, classification_data):
        """Test predict_proba method."""
        X, y = classification_data
        X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=42)

        meta = ACDSOMetaOptimizer(
            adapters=[],
            include_acdso_candidates=True,
            scorer="accuracy",
            cv=3,
            verbose=0,
        )

        meta.fit(X_train, y_train)

        # This may or may not work depending on selected model
        try:
            proba = meta.predict_proba(X_test)
            assert proba.shape[0] == len(y_test)
        except AttributeError:
            pass  # Some models don't support predict_proba

    def test_score(self, classification_data):
        """Test score method."""
        X, y = classification_data
        X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=42)

        meta = ACDSOMetaOptimizer(
            adapters=[],
            include_acdso_candidates=True,
            scorer="accuracy",
            cv=3,
            verbose=0,
        )

        meta.fit(X_train, y_train)

        score = meta.score(X_test, y_test)
        assert 0.0 <= score <= 1.0

    def test_unfitted_raises_error(self, classification_data):
        """Test that unfitted optimizer raises error."""
        X, y = classification_data

        meta = ACDSOMetaOptimizer(adapters=[])

        with pytest.raises(RuntimeError, match="not fitted"):
            meta.predict(X)

        with pytest.raises(RuntimeError, match="not fitted"):
            meta.score(X, y)

    def test_regression_task(self, regression_data):
        """Test meta optimizer with regression task."""
        X, y = regression_data
        X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=42)

        meta = ACDSOMetaOptimizer(
            adapters=[],
            include_acdso_candidates=True,
            scorer="r2",
            cv=3,
            verbose=0,
        )

        meta.fit(X_train, y_train)

        assert meta.audit_trail_["task"] == "regression"

        predictions = meta.predict(X_test)
        assert len(predictions) == len(y_test)


class TestAdapterAvailability:
    """Tests for adapter availability checks."""

    def test_autosklearn_availability(self):
        """Test auto-sklearn availability check."""
        from acdso.integrations import AutoSklearnAdapter

        adapter = AutoSklearnAdapter()
        # Just check it doesn't crash
        _ = adapter.is_available()

    def test_tpot_availability(self):
        """Test TPOT availability check."""
        from acdso.integrations import TPOTAdapter

        adapter = TPOTAdapter()
        _ = adapter.is_available()

    def test_flaml_availability(self):
        """Test FLAML availability check."""
        from acdso.integrations import FLAMLAdapter

        adapter = FLAMLAdapter()
        _ = adapter.is_available()

    def test_h2o_availability(self):
        """Test H2O availability check."""
        from acdso.integrations import H2OAdapter

        adapter = H2OAdapter()
        _ = adapter.is_available()

    def test_autogluon_availability(self):
        """Test AutoGluon availability check."""
        from acdso.integrations import AutoGluonAdapter

        adapter = AutoGluonAdapter()
        _ = adapter.is_available()

