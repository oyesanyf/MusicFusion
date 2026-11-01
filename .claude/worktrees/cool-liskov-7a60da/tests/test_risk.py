"""
Tests for Risk Analysis Module
==============================

Tests covering risk detection, VIF computation, leakage detection,
and temporal leakage checks.
"""

import numpy as np
import pandas as pd
import pytest

from acdso.risk import (
    RiskReport,
    analyze_risk,
    compute_risk_penalty,
    _vif_numeric,
    _univariate_predictive_power,
)
from acdso.utils import class_imbalance, infer_target_type


class TestRiskReport:
    """Tests for RiskReport dataclass."""

    def test_creation(self):
        """Test RiskReport creation."""
        report = RiskReport(
            task="classification",
            target_type="binary",
            n_rows=1000,
            n_cols=10,
            n_numeric=8,
            n_categorical=2,
            missing_ratio=0.05,
        )

        assert report.task == "classification"
        assert report.n_rows == 1000
        assert report.missing_ratio == 0.05

    def test_to_dict(self):
        """Test RiskReport serialization."""
        report = RiskReport(
            task="regression",
            target_type="regression",
            n_rows=500,
            n_cols=5,
            n_numeric=5,
            n_categorical=0,
            missing_ratio=0.0,
            notes=["Test note"],
        )

        d = report.to_dict()
        assert isinstance(d, dict)
        assert d["task"] == "regression"
        assert "Test note" in d["notes"]


class TestVIF:
    """Tests for VIF computation."""

    def test_vif_independent_features(self):
        """Test VIF for independent features."""
        np.random.seed(42)
        n = 200
        X = pd.DataFrame({
            "a": np.random.randn(n),
            "b": np.random.randn(n),
            "c": np.random.randn(n),
        })

        vif_max, high_vif = _vif_numeric(X)

        # Independent features should have low VIF (close to 1)
        assert vif_max < 5.0
        assert len(high_vif) == 0

    def test_vif_collinear_features(self):
        """Test VIF for collinear features."""
        np.random.seed(42)
        n = 200
        x1 = np.random.randn(n)
        x2 = x1 + 0.01 * np.random.randn(n)  # Highly correlated
        x3 = np.random.randn(n)

        X = pd.DataFrame({"x1": x1, "x2": x2, "x3": x3})

        vif_max, high_vif = _vif_numeric(X)

        # Should detect high VIF due to collinearity
        assert vif_max > 10.0
        assert len(high_vif) > 0

    def test_vif_single_column(self):
        """Test VIF with single column."""
        X = pd.DataFrame({"a": np.random.randn(100)})
        vif_max, high_vif = _vif_numeric(X)

        # Single column should have VIF = 1
        assert vif_max == 1.0
        assert len(high_vif) == 0


class TestClassImbalance:
    """Tests for class imbalance detection."""

    def test_balanced_classes(self):
        """Test balanced class detection."""
        y = np.array([0] * 50 + [1] * 50)
        max_ratio, severity = class_imbalance(y)

        assert max_ratio == 0.5
        assert severity == 0.0

    def test_imbalanced_classes(self):
        """Test imbalanced class detection."""
        y = np.array([0] * 90 + [1] * 10)
        max_ratio, severity = class_imbalance(y)

        assert max_ratio == 0.9
        assert severity == 0.8  # (0.9 - 0.5) / 0.5 = 0.8

    def test_extreme_imbalance(self):
        """Test extreme imbalance detection."""
        y = np.array([0] * 99 + [1] * 1)
        max_ratio, severity = class_imbalance(y)

        assert max_ratio == 0.99
        assert severity >= 0.98


class TestTargetTypeInference:
    """Tests for target type inference."""

    def test_binary_classification(self):
        """Test binary classification detection."""
        y = np.array([0, 1, 0, 1, 1, 0])
        assert infer_target_type(y) == "binary"

        y = np.array(["a", "b", "a", "b"])
        assert infer_target_type(y) == "binary"

    def test_multiclass_classification(self):
        """Test multiclass classification detection."""
        y = np.array([0, 1, 2, 0, 1, 2])
        assert infer_target_type(y) == "multiclass"

        y = np.array(["a", "b", "c", "a"])
        assert infer_target_type(y) == "multiclass"

    def test_regression(self):
        """Test regression detection."""
        y = np.array([1.5, 2.3, 0.7, 4.2])
        assert infer_target_type(y) == "regression"


class TestAnalyzeRisk:
    """Tests for the main analyze_risk function."""

    def test_basic_analysis(self):
        """Test basic risk analysis."""
        np.random.seed(42)
        X = pd.DataFrame({
            "a": np.random.randn(100),
            "b": np.random.randn(100),
        })
        y = np.random.randint(0, 2, 100)

        report = analyze_risk(X, y, task="classification")

        assert isinstance(report, RiskReport)
        assert report.task == "classification"
        assert report.n_rows == 100
        assert report.n_cols == 2

    def test_missing_data_detection(self):
        """Test missing data detection."""
        X = pd.DataFrame({
            "a": [1.0, 2.0, np.nan, 4.0, 5.0] * 20,
            "b": [1.0, np.nan, np.nan, 4.0, 5.0] * 20,
        })
        y = np.random.randint(0, 2, 100)

        report = analyze_risk(X, y, task="classification")

        # Should detect significant missing data
        assert report.missing_ratio > 0.1

    def test_with_llm_advisor(self):
        """Test with custom LLM advisor."""
        X = pd.DataFrame({
            "feature_1": np.random.randn(50),
            "target_leak": np.random.randn(50),  # Suspicious name
        })
        y = np.random.randint(0, 2, 50)

        def mock_advisor(payload):
            return {
                "note": "Test advisor note",
                "suspicious_columns": ["target_leak"],
            }

        report = analyze_risk(
            X, y,
            task="classification",
            llm_advisor=mock_advisor,
        )

        # Should have advisor note
        llm_notes = [n for n in report.notes if "Test advisor" in n]
        assert len(llm_notes) > 0


class TestComputeRiskPenalty:
    """Tests for risk penalty computation."""

    def test_low_risk_penalty(self):
        """Test penalty for low-risk data."""
        report = RiskReport(
            task="classification",
            target_type="binary",
            n_rows=1000,
            n_cols=10,
            n_numeric=10,
            n_categorical=0,
            missing_ratio=0.01,
            imbalance_severity=0.0,
            vif_max=2.0,
            leakage_proxy_score_max=0.6,
        )

        penalty = compute_risk_penalty(report, cv_score_std=0.01)
        assert penalty < 0.2

    def test_high_risk_penalty(self):
        """Test penalty for high-risk data."""
        report = RiskReport(
            task="classification",
            target_type="binary",
            n_rows=100,
            n_cols=5,
            n_numeric=5,
            n_categorical=0,
            missing_ratio=0.4,
            imbalance_severity=0.8,
            vif_max=50.0,
            leakage_proxy_score_max=0.99,
            temporal_leakage_gap=0.15,
        )

        penalty = compute_risk_penalty(report, cv_score_std=0.1)
        assert penalty > 0.5

    def test_safe_preference_weights_leakage_higher(self):
        """Test that safe preference weights leakage higher."""
        report = RiskReport(
            task="classification",
            target_type="binary",
            n_rows=100,
            n_cols=5,
            n_numeric=5,
            n_categorical=0,
            missing_ratio=0.0,
            imbalance_severity=0.0,
            vif_max=1.0,
            leakage_proxy_score_max=0.98,  # High leakage signal
        )

        penalty_safe = compute_risk_penalty(
            report, cv_score_std=0.01, preference="safe"
        )
        penalty_balanced = compute_risk_penalty(
            report, cv_score_std=0.01, preference="balanced"
        )

        # Safe preference should penalize leakage more
        assert penalty_safe >= penalty_balanced

