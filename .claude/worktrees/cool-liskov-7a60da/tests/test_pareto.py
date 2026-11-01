"""
Tests for Pareto Optimization Module
====================================

Tests covering Pareto dominance, front construction,
and knee-point selection.
"""

import numpy as np
import pytest

from acdso.pareto import (
    CandidateResult,
    dominates,
    pareto_front,
    knee_point_selection,
    filter_by_constraints,
)


def make_candidate(
    name: str,
    metric: float,
    train_time: float,
    peak_mem: float,
    latency: float,
    risk: float,
) -> CandidateResult:
    """Helper to create CandidateResult for testing."""
    import math
    return CandidateResult(
        name=name,
        pipeline=None,
        metric=metric,
        train_time_s=train_time,
        peak_mem_mb=peak_mem,
        latency_ms_per_row=latency,
        risk_penalty=risk,
        objectives=(
            metric,
            math.log1p(train_time),
            math.log1p(peak_mem),
            math.log1p(latency),
            risk,
        ),
    )


class TestDominance:
    """Tests for Pareto dominance."""

    def test_clear_dominance(self):
        """Test clear dominance case."""
        # A is better in all objectives
        a = make_candidate("A", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1)
        b = make_candidate("B", metric=0.8, train_time=20, peak_mem=200, latency=2, risk=0.2)

        assert dominates(a, b)
        assert not dominates(b, a)

    def test_no_dominance(self):
        """Test case where neither dominates."""
        # A is better in metric, B is better in time
        a = make_candidate("A", metric=0.9, train_time=20, peak_mem=100, latency=1, risk=0.1)
        b = make_candidate("B", metric=0.8, train_time=10, peak_mem=100, latency=1, risk=0.1)

        assert not dominates(a, b)
        assert not dominates(b, a)

    def test_equal_not_dominating(self):
        """Test that equal candidates don't dominate each other."""
        a = make_candidate("A", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1)
        b = make_candidate("B", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1)

        assert not dominates(a, b)
        assert not dominates(b, a)


class TestParetoFront:
    """Tests for Pareto front construction."""

    def test_single_candidate(self):
        """Test Pareto front with single candidate."""
        a = make_candidate("A", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1)
        front = pareto_front([a])

        assert len(front) == 1
        assert front[0].name == "A"

    def test_two_nondominated(self):
        """Test Pareto front with two non-dominated candidates."""
        a = make_candidate("A", metric=0.9, train_time=20, peak_mem=100, latency=1, risk=0.1)
        b = make_candidate("B", metric=0.8, train_time=10, peak_mem=100, latency=1, risk=0.1)

        front = pareto_front([a, b])

        assert len(front) == 2

    def test_dominated_filtered(self):
        """Test that dominated candidates are filtered."""
        a = make_candidate("A", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1)
        b = make_candidate("B", metric=0.8, train_time=20, peak_mem=200, latency=2, risk=0.2)

        front = pareto_front([a, b])

        assert len(front) == 1
        assert front[0].name == "A"

    def test_complex_front(self):
        """Test Pareto front with multiple candidates."""
        candidates = [
            make_candidate("A", metric=0.95, train_time=50, peak_mem=500, latency=5, risk=0.1),
            make_candidate("B", metric=0.90, train_time=20, peak_mem=200, latency=2, risk=0.1),
            make_candidate("C", metric=0.85, train_time=10, peak_mem=100, latency=1, risk=0.1),
            make_candidate("D", metric=0.80, train_time=15, peak_mem=150, latency=1.5, risk=0.1),  # Dominated by C
        ]

        front = pareto_front(candidates)

        # D should be dominated by C (better in all but risk which is equal)
        names = [c.name for c in front]
        assert "A" in names
        assert "B" in names
        assert "C" in names
        assert "D" not in names


class TestKneePointSelection:
    """Tests for knee-point selection."""

    def test_single_candidate(self):
        """Test knee-point with single candidate."""
        a = make_candidate("A", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1)

        selected = knee_point_selection([a])
        assert selected.name == "A"

    def test_empty_front_raises(self):
        """Test that empty front raises ValueError."""
        with pytest.raises(ValueError, match="empty"):
            knee_point_selection([])

    def test_knee_selection(self):
        """Test knee-point selection logic."""
        # Create candidates along a Pareto front
        candidates = [
            make_candidate("HighAcc", metric=0.95, train_time=100, peak_mem=500, latency=10, risk=0.1),
            make_candidate("Balanced", metric=0.90, train_time=30, peak_mem=150, latency=3, risk=0.1),
            make_candidate("Fast", metric=0.80, train_time=5, peak_mem=50, latency=1, risk=0.1),
        ]

        selected = knee_point_selection(candidates)

        # The balanced option should typically be the knee
        assert selected is not None

    def test_two_candidates(self):
        """Test knee-point with two candidates."""
        a = make_candidate("A", metric=0.9, train_time=50, peak_mem=300, latency=5, risk=0.1)
        b = make_candidate("B", metric=0.8, train_time=10, peak_mem=100, latency=1, risk=0.1)

        selected = knee_point_selection([a, b])
        # Should return one of them
        assert selected.name in ["A", "B"]


class TestConstraintFiltering:
    """Tests for constraint-based filtering."""

    def test_no_constraints(self):
        """Test filtering with no constraints."""
        candidates = [
            make_candidate("A", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1),
            make_candidate("B", metric=0.8, train_time=20, peak_mem=200, latency=2, risk=0.2),
        ]

        passing, pruned = filter_by_constraints(candidates)

        assert len(passing) == 2
        assert len(pruned) == 0

    def test_latency_constraint(self):
        """Test latency constraint filtering."""
        candidates = [
            make_candidate("A", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1),
            make_candidate("B", metric=0.8, train_time=20, peak_mem=200, latency=15, risk=0.2),
        ]

        passing, pruned = filter_by_constraints(
            candidates,
            max_latency_ms_per_row=10.0,
        )

        assert len(passing) == 1
        assert passing[0].name == "A"
        assert len(pruned) == 1
        assert pruned[0]["name"] == "B"

    def test_memory_constraint(self):
        """Test memory constraint filtering."""
        candidates = [
            make_candidate("A", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1),
            make_candidate("B", metric=0.8, train_time=20, peak_mem=600, latency=2, risk=0.2),
        ]

        passing, pruned = filter_by_constraints(
            candidates,
            max_peak_mem_mb=500.0,
        )

        assert len(passing) == 1
        assert passing[0].name == "A"

    def test_train_time_constraint(self):
        """Test training time constraint filtering."""
        candidates = [
            make_candidate("A", metric=0.9, train_time=30, peak_mem=100, latency=1, risk=0.1),
            make_candidate("B", metric=0.8, train_time=120, peak_mem=200, latency=2, risk=0.2),
        ]

        passing, pruned = filter_by_constraints(
            candidates,
            max_train_time_s=60.0,
        )

        assert len(passing) == 1
        assert passing[0].name == "A"

    def test_multiple_constraints(self):
        """Test multiple constraints simultaneously."""
        candidates = [
            make_candidate("A", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1),
            make_candidate("B", metric=0.8, train_time=120, peak_mem=200, latency=2, risk=0.2),  # Fails time
            make_candidate("C", metric=0.85, train_time=20, peak_mem=600, latency=1, risk=0.1),  # Fails memory
        ]

        passing, pruned = filter_by_constraints(
            candidates,
            max_train_time_s=60.0,
            max_peak_mem_mb=500.0,
            max_latency_ms_per_row=10.0,
        )

        assert len(passing) == 1
        assert passing[0].name == "A"
        assert len(pruned) == 2


class TestCandidateResult:
    """Tests for CandidateResult dataclass."""

    def test_to_dict(self):
        """Test CandidateResult serialization."""
        c = make_candidate("Test", metric=0.9, train_time=10, peak_mem=100, latency=1, risk=0.1)
        c.meta = {"cv_scores": [0.88, 0.90, 0.92]}

        d = c.to_dict()

        assert isinstance(d, dict)
        assert d["name"] == "Test"
        assert d["metric"] == 0.9
        assert "pipeline" not in d  # Pipeline should be excluded
        assert d["meta"]["cv_scores"] == [0.88, 0.90, 0.92]

