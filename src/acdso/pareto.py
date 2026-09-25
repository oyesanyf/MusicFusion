"""
ACDSO v3.0 - Pareto Optimization Module
========================================

Multi-objective optimization components including:
- Pareto dominance checking (6+ objectives supported)
- Pareto front construction
- Knee-point selection for optimal tradeoffs

v3 Changes:
- Extended to support 6 objectives (added robustness_drop)
- Explicit objective direction handling (maximize vs minimize)
- Improved dominance checking for variable-length objectives
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Tuple, Sequence

import math
import numpy as np


# Objective directions: True = maximize, False = minimize
# Index: 0=metric, 1=train_time, 2=memory, 3=latency, 4=risk, 5=robustness_drop
OBJECTIVE_DIRECTIONS: Tuple[bool, ...] = (
    True,   # 0: metric (higher is better)
    False,  # 1: log(train_time) (lower is better)
    False,  # 2: log(peak_mem) (lower is better)
    False,  # 3: log(latency) (lower is better)
    False,  # 4: risk_penalty (lower is better)
    False,  # 5: robustness_drop (lower is better) - v3 addition
)

OBJECTIVE_NAMES: Tuple[str, ...] = (
    "metric",
    "log_train_time",
    "log_peak_mem", 
    "log_latency",
    "risk_penalty",
    "robustness_drop",
)


@dataclass
class CandidateResult:
    """
    Result of evaluating a candidate pipeline (v3).

    Contains all metrics needed for multi-objective optimization
    and constraint checking.

    Attributes
    ----------
    name : str
        Name/identifier of the candidate.
    pipeline : Any
        Fitted sklearn Pipeline.
    metric : float
        Primary evaluation metric (e.g., accuracy, F1).
    train_time_s : float
        Training time in seconds.
    peak_mem_mb : float
        Peak memory usage in megabytes.
    latency_ms_per_row : float
        Inference latency in milliseconds per row.
    risk_penalty : float
        Computed risk penalty (0.0 to 1.0).
    objectives : Tuple[float, ...]
        6-objective vector (v3):
        [0] metric         - primary score (MAXIMIZE)
        [1] log_train_time - training cost (minimize)
        [2] log_peak_mem   - memory cost (minimize)
        [3] log_latency    - inference cost (minimize)
        [4] risk_penalty   - data risk (minimize)
        [5] robustness_drop - worst-case perf drop under stress (minimize)
    meta : Dict[str, Any]
        Additional metadata (e.g., cv_scores, cv_score_std, stress_tests).
    """

    name: str
    pipeline: Any
    metric: float
    train_time_s: float
    peak_mem_mb: float
    latency_ms_per_row: float
    risk_penalty: float
    objectives: Tuple[float, ...]
    meta: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary (excluding pipeline) for JSON serialization."""
        result = {
            "name": self.name,
            "metric": self.metric,
            "train_time_s": self.train_time_s,
            "peak_mem_mb": self.peak_mem_mb,
            "latency_ms_per_row": self.latency_ms_per_row,
            "risk_penalty": self.risk_penalty,
            "objectives": list(self.objectives),
            "meta": self.meta,
        }
        # Include robustness_drop if available in objectives
        if len(self.objectives) >= 6:
            result["robustness_drop"] = self.objectives[5]
        return result
    
    @property
    def robustness_drop(self) -> float:
        """Get robustness drop (6th objective) if available."""
        return self.objectives[5] if len(self.objectives) >= 6 else 0.0


def dominates(a: CandidateResult, b: CandidateResult) -> bool:
    """
    Check if candidate 'a' Pareto-dominates candidate 'b'.

    A dominates B if A is at least as good as B in all objectives
    and strictly better in at least one.

    Objectives (v3 - 6 objectives):
    - objectives[0]: metric (MAXIMIZE - higher is better)
    - objectives[1]: log(train_time) (minimize - lower is better)
    - objectives[2]: log(peak_mem) (minimize - lower is better)
    - objectives[3]: log(latency) (minimize - lower is better)
    - objectives[4]: risk_penalty (minimize - lower is better)
    - objectives[5]: robustness_drop (minimize - lower is better)

    Parameters
    ----------
    a : CandidateResult
        First candidate.
    b : CandidateResult
        Second candidate.

    Returns
    -------
    bool
        True if a dominates b.
    """
    obj_a = a.objectives
    obj_b = b.objectives
    
    # Handle variable-length objectives (5 or 6)
    n_obj = min(len(obj_a), len(obj_b), len(OBJECTIVE_DIRECTIONS))
    
    EPS = 1e-9
    better_or_equal = True
    strictly_better = False

    for i in range(n_obj):
        maximize = OBJECTIVE_DIRECTIONS[i]
        va, vb = obj_a[i], obj_b[i]
        diff = va - vb
        
        if abs(diff) <= EPS:
            continue
        
        if maximize:
            if diff > EPS:
                strictly_better = True
            else:
                better_or_equal = False
                break
        else:
            if diff < -EPS:
                strictly_better = True
            else:
                better_or_equal = False
                break

    return better_or_equal and strictly_better


def pareto_front(candidates: List[CandidateResult]) -> List[CandidateResult]:
    """
    Extract the Pareto front from a list of candidates.

    The Pareto front contains all non-dominated solutions, i.e.,
    solutions where no other solution is better in all objectives.

    Parameters
    ----------
    candidates : List[CandidateResult]
        List of evaluated candidates.

    Returns
    -------
    List[CandidateResult]
        Pareto-optimal candidates.
    """
    front: List[CandidateResult] = []
    for c in candidates:
        # Check if any other candidate dominates this one
        is_dominated = any(dominates(o, c) for o in candidates if o is not c)
        if not is_dominated:
            front.append(c)
    return front


def knee_point_selection(front: List[CandidateResult]) -> CandidateResult:
    """
    Select the knee point from the Pareto front.

    The knee point represents the solution where additional improvement
    in the primary metric requires disproportionately higher costs/risks.
    It is found by maximizing distance from the line connecting the
    extreme points on the normalized Pareto front.

    v3: Now includes robustness_drop in the cost/risk aggregation.

    Parameters
    ----------
    front : List[CandidateResult]
        Pareto-optimal candidates.

    Returns
    -------
    CandidateResult
        Selected candidate at the knee point.

    Raises
    ------
    ValueError
        If the front is empty.
    """
    if not front:
        raise ValueError("Pareto front is empty; cannot select knee point.")

    if len(front) == 1:
        return front[0]

    # Extract objective components
    m = np.array([c.metric for c in front], dtype=float)
    t = np.array([c.train_time_s for c in front], dtype=float)
    mem = np.array([c.peak_mem_mb for c in front], dtype=float)
    lat = np.array([c.latency_ms_per_row for c in front], dtype=float)
    r = np.array([c.risk_penalty for c in front], dtype=float)
    rob = np.array([c.robustness_drop for c in front], dtype=float)

    def normalize(x: np.ndarray, higher_better: bool) -> np.ndarray:
        x_min, x_max = float(np.min(x)), float(np.max(x))
        if math.isclose(x_min, x_max, abs_tol=1e-12):
            return np.ones_like(x)
        z = (x - x_min) / (x_max - x_min)
        return z if higher_better else (1.0 - z)

    mb = normalize(m, higher_better=True)
    tb = normalize(t, higher_better=False)
    memb = normalize(mem, higher_better=False)
    latb = normalize(lat, higher_better=False)
    rb = normalize(r, higher_better=False)
    robb = normalize(rob, higher_better=False)

    benefit = mb
    costrisk = 1.0 - (0.25 * tb + 0.15 * memb + 0.15 * latb + 0.20 * rb + 0.25 * robb)

    pts = np.vstack([benefit, costrisk]).T

    idx_a = int(np.argmax(benefit))
    idx_b = int(np.argmin(costrisk))

    # If one candidate is both best benefit and lowest cost/risk, it is strictly superior
    if idx_a == idx_b:
        return front[idx_a]

    a = pts[idx_a]
    b = pts[idx_b]
    ab = b - a
    ab_norm = float(np.linalg.norm(ab))

    if ab_norm < 1e-9:
        return front[idx_a]

    # Utopia point is (1.0, 0.0) -> maximum benefit, zero cost/risk
    utopia_dists = np.sqrt((1.0 - benefit) ** 2 + costrisk ** 2)

    if len(front) == 2:
        return front[int(np.argmin(utopia_dists))]

    # Signed orthogonal distance towards utopia (1.0, 0.0)
    # cross product: (b_x - a_x) * (costrisk - a_y) - (b_y - a_y) * (benefit - a_x)
    signed_dists = ((b[0] - a[0]) * (costrisk - a[1]) - (b[1] - a[1]) * (benefit - a[0])) / ab_norm

    if float(np.max(signed_dists)) > 1e-6:
        return front[int(np.argmax(signed_dists))]
    else:
        return front[int(np.argmin(utopia_dists))]


def filter_by_constraints(
    candidates: List[CandidateResult],
    max_train_time_s: float | None = None,
    max_peak_mem_mb: float | None = None,
    max_latency_ms_per_row: float | None = None,
) -> Tuple[List[CandidateResult], List[Dict[str, Any]]]:
    """
    Filter candidates by constraint violations.

    Parameters
    ----------
    candidates : List[CandidateResult]
        List of evaluated candidates.
    max_train_time_s : float, optional
        Maximum allowed training time.
    max_peak_mem_mb : float, optional
        Maximum allowed peak memory.
    max_latency_ms_per_row : float, optional
        Maximum allowed inference latency.

    Returns
    -------
    Tuple[List[CandidateResult], List[Dict[str, Any]]]
        Tuple of (passing_candidates, pruned_info).
    """
    passing = []
    pruned = []

    for c in candidates:
        violations = []

        if max_train_time_s is not None and c.train_time_s > max_train_time_s:
            violations.append(f"train_time={c.train_time_s:.2f}s > {max_train_time_s}s")

        if max_peak_mem_mb is not None and c.peak_mem_mb > max_peak_mem_mb:
            violations.append(f"peak_mem={c.peak_mem_mb:.1f}MB > {max_peak_mem_mb}MB")

        if max_latency_ms_per_row is not None and c.latency_ms_per_row > max_latency_ms_per_row:
            violations.append(f"latency={c.latency_ms_per_row:.2f}ms > {max_latency_ms_per_row}ms")

        if violations:
            pruned.append({
                "name": c.name,
                "reason": "constraints",
                "violations": violations,
                "metrics": {
                    "train_time_s": c.train_time_s,
                    "peak_mem_mb": c.peak_mem_mb,
                    "latency_ms_per_row": c.latency_ms_per_row,
                    "metric": c.metric,
                    "risk_penalty": c.risk_penalty,
                },
            })
        else:
            passing.append(c)

    return passing, pruned

