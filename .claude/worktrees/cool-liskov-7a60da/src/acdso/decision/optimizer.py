"""
Action Optimization Module (OR-Tools Integration)
=================================================

Provides constraint-aware action optimization:
- Budget-constrained targeting
- Capacity constraints
- Fairness constraints
- Multi-objective action selection
- Knapsack and Integer Linear Programming

This module turns predictions/uplift into actionable, constraint-aware decisions.

Usage:
    from acdso.decision import ActionOptimizer, BudgetConstraint
    
    optimizer = ActionOptimizer(
        value_col='expected_benefit',
        cost_col='incentive_cost',
    )
    
    optimizer.add_constraint(BudgetConstraint(max_budget=10000))
    
    result = optimizer.optimize(df)
    print(f"Units to treat: {result.n_selected}")
    print(f"Expected value: {result.total_value}")
"""

from __future__ import annotations

import warnings
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any, Union, Callable
import numpy as np
import pandas as pd


@dataclass
class Constraint(ABC):
    """Base class for optimization constraints."""
    
    name: str = 'constraint'
    
    @abstractmethod
    def is_feasible(self, selected: np.ndarray, df: pd.DataFrame) -> bool:
        """Check if selection satisfies constraint."""
        pass
    
    @abstractmethod
    def get_violation(self, selected: np.ndarray, df: pd.DataFrame) -> float:
        """Get constraint violation amount (0 if satisfied)."""
        pass
    
    @abstractmethod
    def add_to_solver(self, solver, variables, df: pd.DataFrame):
        """Add constraint to OR-Tools solver."""
        pass


@dataclass
class BudgetConstraint(Constraint):
    """
    Budget constraint: total cost <= max_budget.
    
    Parameters
    ----------
    max_budget : float
        Maximum total budget.
    cost_col : str, default='cost'
        Column containing per-unit costs.
    """
    
    max_budget: float = 10000.0
    cost_col: str = 'cost'
    name: str = 'budget'
    
    def is_feasible(self, selected: np.ndarray, df: pd.DataFrame) -> bool:
        costs = df[self.cost_col].values
        return float(np.sum(costs * selected)) <= self.max_budget
    
    def get_violation(self, selected: np.ndarray, df: pd.DataFrame) -> float:
        costs = df[self.cost_col].values
        total = float(np.sum(costs * selected))
        return max(0, total - self.max_budget)
    
    def add_to_solver(self, solver, variables, df: pd.DataFrame):
        costs = df[self.cost_col].values
        solver.Add(
            solver.Sum(float(costs[i]) * variables[i] for i in range(len(variables)))
            <= self.max_budget
        )


@dataclass
class CapacityConstraint(Constraint):
    """
    Capacity constraint: max number of units to select.
    
    Parameters
    ----------
    max_count : int
        Maximum number of units to select.
    min_count : int, optional
        Minimum number to select.
    """
    
    max_count: int = 1000
    min_count: int = 0
    name: str = 'capacity'
    
    def is_feasible(self, selected: np.ndarray, df: pd.DataFrame) -> bool:
        count = int(np.sum(selected))
        return self.min_count <= count <= self.max_count
    
    def get_violation(self, selected: np.ndarray, df: pd.DataFrame) -> float:
        count = int(np.sum(selected))
        if count < self.min_count:
            return float(self.min_count - count)
        if count > self.max_count:
            return float(count - self.max_count)
        return 0.0
    
    def add_to_solver(self, solver, variables, df: pd.DataFrame):
        solver.Add(solver.Sum(variables) <= self.max_count)
        if self.min_count > 0:
            solver.Add(solver.Sum(variables) >= self.min_count)


@dataclass
class FairnessConstraint(Constraint):
    """
    Fairness constraint: ensure selection rates are similar across groups.
    
    Parameters
    ----------
    group_col : str
        Column defining groups.
    max_disparity : float, default=0.1
        Maximum allowed difference in selection rates between groups.
    min_per_group : int, default=0
        Minimum selections per group.
    """
    
    group_col: str = 'group'
    max_disparity: float = 0.1
    min_per_group: int = 0
    name: str = 'fairness'
    
    def is_feasible(self, selected: np.ndarray, df: pd.DataFrame) -> bool:
        groups = df[self.group_col].values
        unique_groups = np.unique(groups)
        
        rates = []
        for g in unique_groups:
            mask = groups == g
            if mask.sum() > 0:
                rate = selected[mask].sum() / mask.sum()
                rates.append(rate)
                
                if self.min_per_group > 0 and selected[mask].sum() < self.min_per_group:
                    return False
        
        if len(rates) < 2:
            return True
        
        return max(rates) - min(rates) <= self.max_disparity
    
    def get_violation(self, selected: np.ndarray, df: pd.DataFrame) -> float:
        groups = df[self.group_col].values
        unique_groups = np.unique(groups)
        
        rates = []
        min_violations = 0
        
        for g in unique_groups:
            mask = groups == g
            if mask.sum() > 0:
                rate = selected[mask].sum() / mask.sum()
                rates.append(rate)
                
                if self.min_per_group > 0:
                    min_violations += max(0, self.min_per_group - selected[mask].sum())
        
        disparity = max(rates) - min(rates) if len(rates) >= 2 else 0.0
        disparity_violation = max(0, disparity - self.max_disparity)
        
        return disparity_violation + min_violations
    
    def add_to_solver(self, solver, variables, df: pd.DataFrame):
        groups = df[self.group_col].values
        unique_groups = np.unique(groups)
        
        # This is an approximation - exact fairness is more complex
        for g in unique_groups:
            mask = groups == g
            group_vars = [variables[i] for i in range(len(variables)) if mask[i]]
            
            if self.min_per_group > 0 and len(group_vars) > 0:
                solver.Add(solver.Sum(group_vars) >= self.min_per_group)


@dataclass
class ROIConstraint(Constraint):
    """
    ROI constraint: ensure minimum return on investment.
    
    Parameters
    ----------
    min_roi : float
        Minimum ROI (value / cost).
    value_col : str
        Column with expected value.
    cost_col : str
        Column with cost.
    """
    
    min_roi: float = 1.0
    value_col: str = 'value'
    cost_col: str = 'cost'
    name: str = 'roi'
    
    def is_feasible(self, selected: np.ndarray, df: pd.DataFrame) -> bool:
        values = df[self.value_col].values
        costs = df[self.cost_col].values
        
        total_value = float(np.sum(values * selected))
        total_cost = float(np.sum(costs * selected))
        
        if total_cost == 0:
            return True
        
        return total_value / total_cost >= self.min_roi
    
    def get_violation(self, selected: np.ndarray, df: pd.DataFrame) -> float:
        values = df[self.value_col].values
        costs = df[self.cost_col].values
        
        total_value = float(np.sum(values * selected))
        total_cost = float(np.sum(costs * selected))
        
        if total_cost == 0:
            return 0.0
        
        roi = total_value / total_cost
        return max(0, self.min_roi - roi)
    
    def add_to_solver(self, solver, variables, df: pd.DataFrame):
        # ROI constraint is non-linear, approximate with value >= min_roi * cost
        values = df[self.value_col].values
        costs = df[self.cost_col].values
        
        solver.Add(
            solver.Sum(float(values[i]) * variables[i] for i in range(len(variables)))
            >= self.min_roi * solver.Sum(float(costs[i]) * variables[i] for i in range(len(variables)))
        )


@dataclass
class OptimizationResult:
    """
    Result of action optimization.
    
    Attributes
    ----------
    selected : np.ndarray
        Binary array indicating selected units.
    selected_indices : np.ndarray
        Indices of selected units.
    n_selected : int
        Number of units selected.
    total_value : float
        Total expected value of selection.
    total_cost : float
        Total cost of selection.
    roi : float
        Return on investment.
    constraints_satisfied : Dict[str, bool]
        Whether each constraint is satisfied.
    constraint_violations : Dict[str, float]
        Violation amount for each constraint.
    solver_status : str
        Optimization solver status.
    solve_time : float
        Time to solve in seconds.
    """
    
    selected: np.ndarray
    selected_indices: np.ndarray = field(default_factory=lambda: np.array([]))
    n_selected: int = 0
    total_value: float = 0.0
    total_cost: float = 0.0
    roi: float = 0.0
    constraints_satisfied: Dict[str, bool] = field(default_factory=dict)
    constraint_violations: Dict[str, float] = field(default_factory=dict)
    solver_status: str = 'unknown'
    solve_time: float = 0.0
    
    def __post_init__(self):
        self.selected_indices = np.where(self.selected == 1)[0]
        self.n_selected = int(np.sum(self.selected))
    
    def summary(self) -> str:
        """Generate summary string."""
        lines = [
            "Action Optimization Results",
            "=" * 40,
            f"Units Selected: {self.n_selected}",
            f"Total Value: {self.total_value:.2f}",
            f"Total Cost: {self.total_cost:.2f}",
            f"ROI: {self.roi:.2f}",
            f"Solver Status: {self.solver_status}",
            "",
            "Constraints:",
        ]
        
        for name, satisfied in self.constraints_satisfied.items():
            status = "OK" if satisfied else f"VIOLATED ({self.constraint_violations.get(name, 0):.2f})"
            lines.append(f"  {name}: {status}")
        
        return "\n".join(lines)


class ActionOptimizer:
    """
    Constraint-aware action optimizer using OR-Tools.
    
    Solves: maximize sum(value_i * x_i) subject to constraints
    
    Parameters
    ----------
    value_col : str
        Column containing expected value/benefit of treating each unit.
    cost_col : str, optional
        Column containing cost per unit.
    solver : str, default='CBC'
        Solver to use: 'CBC', 'SCIP', 'GLOP'.
    
    Example
    -------
    >>> optimizer = ActionOptimizer(
    ...     value_col='expected_benefit',
    ...     cost_col='incentive_cost',
    ... )
    >>> optimizer.add_constraint(BudgetConstraint(max_budget=10000))
    >>> optimizer.add_constraint(CapacityConstraint(max_count=500))
    >>> result = optimizer.optimize(df)
    >>> print(f"Selected {result.n_selected} units for ROI {result.roi:.2f}")
    """
    
    def __init__(
        self,
        value_col: str,
        cost_col: Optional[str] = None,
        solver: str = 'CBC',
    ):
        self.value_col = value_col
        self.cost_col = cost_col
        self.solver = solver
        
        self._constraints: List[Constraint] = []
        self._ortools_available = self._check_ortools()
    
    def _check_ortools(self) -> bool:
        """Check if OR-Tools is available."""
        try:
            from ortools.linear_solver import pywraplp
            return True
        except ImportError:
            return False
    
    def add_constraint(self, constraint: Constraint):
        """Add a constraint to the optimization."""
        self._constraints.append(constraint)
        return self
    
    def clear_constraints(self):
        """Remove all constraints."""
        self._constraints = []
        return self
    
    def optimize(
        self,
        df: pd.DataFrame,
        verbose: bool = False,
    ) -> OptimizationResult:
        """
        Optimize action selection.
        
        Parameters
        ----------
        df : pd.DataFrame
            Data with value and cost columns.
        verbose : bool, default=False
            Whether to print progress.
        
        Returns
        -------
        OptimizationResult
            Optimization results.
        """
        if self._ortools_available:
            return self._optimize_ortools(df, verbose)
        else:
            return self._optimize_greedy(df, verbose)
    
    def _optimize_ortools(
        self,
        df: pd.DataFrame,
        verbose: bool,
    ) -> OptimizationResult:
        """Optimize using OR-Tools."""
        import time
        from ortools.linear_solver import pywraplp
        
        start_time = time.time()
        
        # Create solver
        solver = pywraplp.Solver.CreateSolver(self.solver)
        if solver is None:
            warnings.warn(f"Solver {self.solver} not available, trying CBC")
            solver = pywraplp.Solver.CreateSolver('CBC')
        
        if solver is None:
            return self._optimize_greedy(df, verbose)
        
        n = len(df)
        values = df[self.value_col].values
        
        # Create binary decision variables
        x = [solver.IntVar(0, 1, f'x_{i}') for i in range(n)]
        
        # Objective: maximize total value
        solver.Maximize(
            solver.Sum(float(values[i]) * x[i] for i in range(n))
        )
        
        # Add constraints
        for constraint in self._constraints:
            constraint.add_to_solver(solver, x, df)
        
        # Solve
        status = solver.Solve()
        
        solve_time = time.time() - start_time
        
        # Extract solution
        if status == pywraplp.Solver.OPTIMAL or status == pywraplp.Solver.FEASIBLE:
            selected = np.array([int(x[i].solution_value()) for i in range(n)])
            solver_status = 'optimal' if status == pywraplp.Solver.OPTIMAL else 'feasible'
        else:
            selected = np.zeros(n, dtype=int)
            solver_status = 'infeasible'
        
        # Compute metrics
        total_value = float(np.sum(values * selected))
        
        if self.cost_col:
            costs = df[self.cost_col].values
            total_cost = float(np.sum(costs * selected))
            roi = total_value / total_cost if total_cost > 0 else 0.0
        else:
            total_cost = 0.0
            roi = 0.0
        
        # Check constraints
        constraints_satisfied = {}
        constraint_violations = {}
        
        for constraint in self._constraints:
            constraints_satisfied[constraint.name] = constraint.is_feasible(selected, df)
            constraint_violations[constraint.name] = constraint.get_violation(selected, df)
        
        result = OptimizationResult(
            selected=selected,
            total_value=total_value,
            total_cost=total_cost,
            roi=roi,
            constraints_satisfied=constraints_satisfied,
            constraint_violations=constraint_violations,
            solver_status=solver_status,
            solve_time=solve_time,
        )
        
        if verbose:
            print(result.summary())
        
        return result
    
    def _optimize_greedy(
        self,
        df: pd.DataFrame,
        verbose: bool,
    ) -> OptimizationResult:
        """Fallback greedy optimization when OR-Tools unavailable."""
        import time
        
        start_time = time.time()
        
        n = len(df)
        values = df[self.value_col].values
        
        if self.cost_col:
            costs = df[self.cost_col].values
            # Sort by value/cost ratio (ROI)
            with np.errstate(divide='ignore', invalid='ignore'):
                ratios = values / (costs + 1e-10)
            order = np.argsort(-ratios)
        else:
            # Sort by value
            order = np.argsort(-values)
        
        selected = np.zeros(n, dtype=int)
        
        # Greedily add units while constraints satisfied
        for idx in order:
            selected[idx] = 1
            
            # Check all constraints
            feasible = True
            for constraint in self._constraints:
                if not constraint.is_feasible(selected, df):
                    feasible = False
                    break
            
            if not feasible:
                selected[idx] = 0
        
        solve_time = time.time() - start_time
        
        # Compute metrics
        total_value = float(np.sum(values * selected))
        
        if self.cost_col:
            costs = df[self.cost_col].values
            total_cost = float(np.sum(costs * selected))
            roi = total_value / total_cost if total_cost > 0 else 0.0
        else:
            total_cost = 0.0
            roi = 0.0
        
        # Check constraints
        constraints_satisfied = {}
        constraint_violations = {}
        
        for constraint in self._constraints:
            constraints_satisfied[constraint.name] = constraint.is_feasible(selected, df)
            constraint_violations[constraint.name] = constraint.get_violation(selected, df)
        
        result = OptimizationResult(
            selected=selected,
            total_value=total_value,
            total_cost=total_cost,
            roi=roi,
            constraints_satisfied=constraints_satisfied,
            constraint_violations=constraint_violations,
            solver_status='greedy',
            solve_time=solve_time,
        )
        
        if verbose:
            print(result.summary())
        
        return result


def optimize_actions(
    df: pd.DataFrame,
    value_col: str,
    cost_col: Optional[str] = None,
    budget: Optional[float] = None,
    max_count: Optional[int] = None,
) -> OptimizationResult:
    """
    Convenience function for quick action optimization.
    
    Parameters
    ----------
    df : pd.DataFrame
        Data.
    value_col : str
        Value column.
    cost_col : str, optional
        Cost column.
    budget : float, optional
        Budget constraint.
    max_count : int, optional
        Capacity constraint.
    
    Returns
    -------
    OptimizationResult
        Optimization results.
    """
    optimizer = ActionOptimizer(value_col=value_col, cost_col=cost_col)
    
    if budget is not None and cost_col is not None:
        optimizer.add_constraint(BudgetConstraint(max_budget=budget, cost_col=cost_col))
    
    if max_count is not None:
        optimizer.add_constraint(CapacityConstraint(max_count=max_count))
    
    return optimizer.optimize(df)


def knapsack_optimize(
    values: np.ndarray,
    weights: np.ndarray,
    capacity: float,
) -> Tuple[np.ndarray, float]:
    """
    Classic 0/1 knapsack optimization.
    
    Parameters
    ----------
    values : np.ndarray
        Value of each item.
    weights : np.ndarray
        Weight of each item.
    capacity : float
        Knapsack capacity.
    
    Returns
    -------
    selected : np.ndarray
        Binary selection array.
    total_value : float
        Total value of selected items.
    """
    try:
        from ortools.linear_solver import pywraplp
        
        solver = pywraplp.Solver.CreateSolver('CBC')
        n = len(values)
        
        x = [solver.IntVar(0, 1, f'x_{i}') for i in range(n)]
        
        solver.Maximize(solver.Sum(float(values[i]) * x[i] for i in range(n)))
        solver.Add(solver.Sum(float(weights[i]) * x[i] for i in range(n)) <= capacity)
        
        status = solver.Solve()
        
        if status == pywraplp.Solver.OPTIMAL:
            selected = np.array([int(x[i].solution_value()) for i in range(n)])
        else:
            selected = np.zeros(n, dtype=int)
        
        return selected, float(np.sum(values * selected))
        
    except ImportError:
        # Greedy fallback
        ratios = values / (weights + 1e-10)
        order = np.argsort(-ratios)
        
        selected = np.zeros(len(values), dtype=int)
        current_weight = 0.0
        
        for idx in order:
            if current_weight + weights[idx] <= capacity:
                selected[idx] = 1
                current_weight += weights[idx]
        
        return selected, float(np.sum(values * selected))


def ilp_optimize(
    c: np.ndarray,
    A_ub: Optional[np.ndarray] = None,
    b_ub: Optional[np.ndarray] = None,
    A_eq: Optional[np.ndarray] = None,
    b_eq: Optional[np.ndarray] = None,
    bounds: Optional[List[Tuple[float, float]]] = None,
    maximize: bool = True,
) -> Tuple[np.ndarray, float]:
    """
    General Integer Linear Programming solver.
    
    Solves: max/min c'x subject to A_ub x <= b_ub, A_eq x = b_eq
    
    Parameters
    ----------
    c : np.ndarray
        Objective coefficients.
    A_ub : np.ndarray, optional
        Inequality constraint matrix.
    b_ub : np.ndarray, optional
        Inequality constraint bounds.
    A_eq : np.ndarray, optional
        Equality constraint matrix.
    b_eq : np.ndarray, optional
        Equality constraint bounds.
    bounds : list of tuples, optional
        Variable bounds.
    maximize : bool, default=True
        Whether to maximize (True) or minimize (False).
    
    Returns
    -------
    x : np.ndarray
        Optimal solution.
    obj : float
        Optimal objective value.
    """
    try:
        from ortools.linear_solver import pywraplp
        
        solver = pywraplp.Solver.CreateSolver('CBC')
        n = len(c)
        
        # Variables
        if bounds is None:
            bounds = [(0, 1) for _ in range(n)]
        
        x = [solver.IntVar(bounds[i][0], bounds[i][1], f'x_{i}') for i in range(n)]
        
        # Objective
        obj_expr = solver.Sum(float(c[i]) * x[i] for i in range(n))
        if maximize:
            solver.Maximize(obj_expr)
        else:
            solver.Minimize(obj_expr)
        
        # Inequality constraints
        if A_ub is not None and b_ub is not None:
            for j in range(len(b_ub)):
                solver.Add(
                    solver.Sum(float(A_ub[j, i]) * x[i] for i in range(n)) <= float(b_ub[j])
                )
        
        # Equality constraints
        if A_eq is not None and b_eq is not None:
            for j in range(len(b_eq)):
                solver.Add(
                    solver.Sum(float(A_eq[j, i]) * x[i] for i in range(n)) == float(b_eq[j])
                )
        
        status = solver.Solve()
        
        if status == pywraplp.Solver.OPTIMAL:
            solution = np.array([x[i].solution_value() for i in range(n)])
            obj_value = float(solver.Objective().Value())
        else:
            solution = np.zeros(n)
            obj_value = 0.0
        
        return solution, obj_value
        
    except ImportError:
        warnings.warn("OR-Tools not available. ILP optimization requires OR-Tools.")
        return np.zeros(len(c)), 0.0

