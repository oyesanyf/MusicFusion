"""
ACDSO Decision Intelligence Core Engine
=======================================

Unified interface for decision-grade analytics:
1. Causal Analysis (DoWhy) - "Does treatment cause outcome?"
2. Uplift Modeling (EconML) - "For whom does treatment work best?"
3. Action Optimization (OR-Tools) - "Who should we treat under constraints?"
4. Policy Learning (RLlib) - "How to decide over time?"

The Decision Intelligence layer sits ON TOP of ACDSO's predictive engine.

Usage:
    from acdso.decision import DecisionIntelligence
    
    di = DecisionIntelligence(
        treatment_col='offered_incentive',
        outcome_col='churned',
        cost_col='incentive_cost',
    )
    
    result = di.fit(df, budget=10000)
    
    # Get recommendations
    recommendations = result.get_recommendations()
    print(f"ATE: {result.causal.ate}")
    print(f"Target {result.optimization.n_selected} customers")
"""

from __future__ import annotations

import time
import warnings
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any, Union
import numpy as np
import pandas as pd

from .causal import (
    CausalAnalyzer,
    CausalModel,
    CausalEstimate,
    CausalReadinessReport,
)
from .uplift import (
    UpliftModeler,
    UpliftResult,
    qini_score,
)
from .optimizer import (
    ActionOptimizer,
    OptimizationResult,
    BudgetConstraint,
    CapacityConstraint,
    FairnessConstraint,
)
from .policy import (
    PolicyLearner,
    PolicyResult,
    ContextualBanditEnv,
)


@dataclass
class DecisionAuditTrail:
    """
    Complete audit trail for decision intelligence pipeline.
    
    Attributes
    ----------
    timestamp : str
        When the analysis was run.
    data_shape : Tuple[int, int]
        Shape of input data.
    treatment_col : str
        Treatment column.
    outcome_col : str
        Outcome column.
    causal_summary : Dict
        Summary of causal analysis.
    uplift_summary : Dict
        Summary of uplift modeling.
    optimization_summary : Dict
        Summary of action optimization.
    policy_summary : Dict
        Summary of policy learning (if run).
    recommendations : Dict
        Final recommendations.
    warnings : List[str]
        Warnings generated during analysis.
    total_time : float
        Total analysis time.
    """
    
    timestamp: str = ''
    data_shape: Tuple[int, int] = (0, 0)
    treatment_col: str = ''
    outcome_col: str = ''
    
    causal_summary: Dict[str, Any] = field(default_factory=dict)
    uplift_summary: Dict[str, Any] = field(default_factory=dict)
    optimization_summary: Dict[str, Any] = field(default_factory=dict)
    policy_summary: Dict[str, Any] = field(default_factory=dict)
    
    recommendations: Dict[str, Any] = field(default_factory=dict)
    warnings: List[str] = field(default_factory=list)
    total_time: float = 0.0
    
    def to_dict(self) -> Dict[str, Any]:
        return {k: v for k, v in self.__dict__.items()}
    
    def to_json(self) -> str:
        import json
        return json.dumps(self.to_dict(), indent=2, default=str)


@dataclass
class DecisionResult:
    """
    Complete result of decision intelligence analysis.
    
    Attributes
    ----------
    causal : CausalModel
        Causal analysis results.
    uplift : UpliftResult
        Uplift modeling results.
    optimization : OptimizationResult
        Action optimization results.
    policy : PolicyResult, optional
        Policy learning results (if run).
    audit_trail : DecisionAuditTrail
        Complete audit trail.
    recommendations_df : pd.DataFrame
        DataFrame with recommendations per unit.
    """
    
    causal: Optional[CausalModel] = None
    uplift: Optional[UpliftResult] = None
    optimization: Optional[OptimizationResult] = None
    policy: Optional[PolicyResult] = None
    audit_trail: DecisionAuditTrail = field(default_factory=DecisionAuditTrail)
    recommendations_df: Optional[pd.DataFrame] = None
    
    def get_recommendations(self, top_n: Optional[int] = None) -> pd.DataFrame:
        """
        Get recommended actions.
        
        Parameters
        ----------
        top_n : int, optional
            Return only top N recommendations.
        
        Returns
        -------
        pd.DataFrame
            Recommendations with columns: index, recommended_action, 
            expected_benefit, confidence.
        """
        if self.recommendations_df is None:
            return pd.DataFrame()
        
        df = self.recommendations_df.copy()
        
        if top_n is not None:
            df = df.nlargest(top_n, 'expected_benefit')
        
        return df
    
    def summary(self) -> str:
        """Generate comprehensive summary."""
        lines = [
            "=" * 60,
            "ACDSO Decision Intelligence Results",
            "=" * 60,
        ]
        
        # Causal Summary
        if self.causal:
            lines.extend([
                "",
                "CAUSAL ANALYSIS",
                "-" * 40,
                f"Average Treatment Effect (ATE): {self.causal.ate.effect:.4f}",
                f"Interpretation: {self.causal.ate.interpretation}",
                f"Decision Ready: {'Yes' if self.causal.readiness.decision_ready else 'NO - CHECK WARNINGS'}",
            ])
            
            if not self.causal.readiness.decision_ready:
                for w in self.causal.readiness.causal_warnings[:3]:
                    lines.append(f"  WARNING: {w}")
        
        # Uplift Summary
        if self.uplift:
            lines.extend([
                "",
                "UPLIFT MODELING",
                "-" * 40,
                f"Best Model: {self.uplift.best_model}",
                f"Qini Score: {self.uplift.qini_score:.4f}",
                f"Policy Value (top 10% vs random): {self.uplift.policy_value:.4f}",
            ])
        
        # Optimization Summary
        if self.optimization:
            lines.extend([
                "",
                "ACTION OPTIMIZATION",
                "-" * 40,
                f"Units to Target: {self.optimization.n_selected}",
                f"Total Expected Value: {self.optimization.total_value:.2f}",
                f"Total Cost: {self.optimization.total_cost:.2f}",
                f"ROI: {self.optimization.roi:.2f}",
            ])
        
        # Policy Summary
        if self.policy:
            lines.extend([
                "",
                "POLICY LEARNING",
                "-" * 40,
                f"Mean Reward: {self.policy.mean_reward:.4f}",
                f"Action Distribution: {self.policy.action_distribution}",
            ])
        
        # Final Recommendations
        lines.extend([
            "",
            "=" * 60,
            "RECOMMENDATIONS",
            "=" * 60,
        ])
        
        if self.recommendations_df is not None:
            n_treat = (self.recommendations_df['recommended_action'] == 1).sum()
            lines.append(f"Recommend treating {n_treat} units")
            
            if 'expected_benefit' in self.recommendations_df.columns:
                total_benefit = self.recommendations_df.loc[
                    self.recommendations_df['recommended_action'] == 1,
                    'expected_benefit'
                ].sum()
                lines.append(f"Total expected benefit: {total_benefit:.2f}")
        
        return "\n".join(lines)


class DecisionIntelligence:
    """
    ACDSO Decision Intelligence Engine.
    
    Unified interface for:
    1. Causal effect estimation (DoWhy)
    2. Individual treatment effects (EconML)
    3. Constrained action optimization (OR-Tools)
    4. Sequential policy learning (RLlib)
    
    Parameters
    ----------
    treatment_col : str
        Name of the treatment column (binary 0/1).
    outcome_col : str
        Name of the outcome column.
    cost_col : str, optional
        Name of the cost column (for optimization).
    value_col : str, optional
        Name of the value/benefit column. If None, uses -outcome for 
        minimization outcomes (like churn) or outcome for maximization.
    confounders : List[str], optional
        Confounder column names for causal analysis.
    feature_cols : List[str], optional
        Feature columns for uplift modeling.
    outcome_type : str, default='minimize'
        Whether to minimize ('minimize', e.g., churn) or maximize ('maximize', e.g., conversion).
    
    Example
    -------
    >>> di = DecisionIntelligence(
    ...     treatment_col='offered_incentive',
    ...     outcome_col='churned',
    ...     cost_col='incentive_cost',
    ...     outcome_type='minimize',  # We want to minimize churn
    ... )
    >>> 
    >>> result = di.fit(df, budget=10000)
    >>> print(result.summary())
    >>> 
    >>> # Get specific recommendations
    >>> top_100 = result.get_recommendations(top_n=100)
    """
    
    def __init__(
        self,
        treatment_col: str,
        outcome_col: str,
        cost_col: Optional[str] = None,
        value_col: Optional[str] = None,
        confounders: Optional[List[str]] = None,
        feature_cols: Optional[List[str]] = None,
        outcome_type: str = 'minimize',
    ):
        self.treatment_col = treatment_col
        self.outcome_col = outcome_col
        self.cost_col = cost_col
        self.value_col = value_col
        self.confounders = confounders
        self.feature_cols = feature_cols
        self.outcome_type = outcome_type
        
        self._result: Optional[DecisionResult] = None
    
    def fit(
        self,
        df: pd.DataFrame,
        budget: Optional[float] = None,
        max_count: Optional[int] = None,
        fairness_col: Optional[str] = None,
        run_causal: bool = True,
        run_uplift: bool = True,
        run_optimization: bool = True,
        run_policy: bool = False,
        policy_iterations: int = 50,
        verbose: bool = True,
    ) -> DecisionResult:
        """
        Run the complete decision intelligence pipeline.
        
        Parameters
        ----------
        df : pd.DataFrame
            Data with treatment, outcome, and features.
        budget : float, optional
            Budget constraint for optimization.
        max_count : int, optional
            Maximum units to select.
        fairness_col : str, optional
            Column for fairness constraint.
        run_causal : bool, default=True
            Whether to run causal analysis.
        run_uplift : bool, default=True
            Whether to run uplift modeling.
        run_optimization : bool, default=True
            Whether to run action optimization.
        run_policy : bool, default=False
            Whether to run policy learning (can be slow).
        policy_iterations : int, default=50
            Iterations for policy learning.
        verbose : bool, default=True
            Whether to print progress.
        
        Returns
        -------
        DecisionResult
            Complete results.
        """
        start_time = time.time()
        
        audit_trail = DecisionAuditTrail(
            timestamp=pd.Timestamp.now().isoformat(),
            data_shape=df.shape,
            treatment_col=self.treatment_col,
            outcome_col=self.outcome_col,
        )
        
        if verbose:
            print("=" * 60)
            print("ACDSO Decision Intelligence Pipeline")
            print("=" * 60)
            print(f"Samples: {len(df)}")
            print(f"Treatment: {self.treatment_col}")
            print(f"Outcome: {self.outcome_col}")
            print(f"Outcome type: {self.outcome_type}")
        
        df = df.copy()
        
        # Auto-detect features if not provided
        if self.feature_cols is None:
            self.feature_cols = [
                c for c in df.columns
                if c not in [self.treatment_col, self.outcome_col, self.cost_col, self.value_col]
                and df[c].dtype in [np.float64, np.int64, float, int, bool]
            ]
        
        causal_model = None
        uplift_result = None
        optimization_result = None
        policy_result = None
        
        # Step 1: Causal Analysis
        if run_causal:
            if verbose:
                print("\n[1/4] Running Causal Analysis (DoWhy)...")
            
            causal_model = self._run_causal(df, verbose)
            
            audit_trail.causal_summary = {
                'ate': causal_model.ate.effect if causal_model.ate else None,
                'att': causal_model.att.effect if causal_model.att else None,
                'decision_ready': causal_model.readiness.decision_ready if causal_model.readiness else True,
                'warnings': causal_model.readiness.causal_warnings if causal_model.readiness else [],
            }
            
            if causal_model.readiness and not causal_model.readiness.decision_ready:
                audit_trail.warnings.extend(causal_model.readiness.causal_warnings)
        
        # Step 2: Uplift Modeling
        if run_uplift:
            if verbose:
                print("\n[2/4] Running Uplift Modeling (EconML)...")
            
            uplift_result = self._run_uplift(df, verbose)
            
            audit_trail.uplift_summary = {
                'best_model': uplift_result.best_model,
                'qini_score': uplift_result.qini_score,
                'auuc': uplift_result.auuc,
                'policy_value': uplift_result.policy_value,
            }
            
            # Add ITE to dataframe for optimization
            df['_ite'] = uplift_result.ite
            
            # Compute expected benefit
            if self.outcome_type == 'minimize':
                df['_expected_benefit'] = -uplift_result.ite  # Negative ITE = reduction in bad outcome
            else:
                df['_expected_benefit'] = uplift_result.ite
        
        # Step 3: Action Optimization
        if run_optimization and (uplift_result is not None or self.value_col is not None):
            if verbose:
                print("\n[3/4] Running Action Optimization (OR-Tools)...")
            
            optimization_result = self._run_optimization(
                df, budget, max_count, fairness_col, verbose
            )
            
            audit_trail.optimization_summary = {
                'n_selected': optimization_result.n_selected,
                'total_value': optimization_result.total_value,
                'total_cost': optimization_result.total_cost,
                'roi': optimization_result.roi,
                'constraints_satisfied': optimization_result.constraints_satisfied,
            }
        
        # Step 4: Policy Learning (optional)
        if run_policy:
            if verbose:
                print("\n[4/4] Running Policy Learning (RLlib)...")
            
            policy_result = self._run_policy(df, policy_iterations, verbose)
            
            if policy_result:
                audit_trail.policy_summary = {
                    'mean_reward': policy_result.mean_reward,
                    'action_distribution': policy_result.action_distribution,
                }
        
        # Build recommendations
        recommendations_df = self._build_recommendations(df, optimization_result)
        
        audit_trail.recommendations = {
            'n_to_treat': int((recommendations_df['recommended_action'] == 1).sum()) if recommendations_df is not None else 0,
            'total_expected_benefit': float(recommendations_df.loc[
                recommendations_df['recommended_action'] == 1, 'expected_benefit'
            ].sum()) if recommendations_df is not None and 'expected_benefit' in recommendations_df.columns else 0,
        }
        
        audit_trail.total_time = time.time() - start_time
        
        self._result = DecisionResult(
            causal=causal_model,
            uplift=uplift_result,
            optimization=optimization_result,
            policy=policy_result,
            audit_trail=audit_trail,
            recommendations_df=recommendations_df,
        )
        
        if verbose:
            print("\n" + self._result.summary())
        
        return self._result
    
    def _run_causal(self, df: pd.DataFrame, verbose: bool) -> CausalModel:
        """Run causal analysis."""
        analyzer = CausalAnalyzer(
            treatment_col=self.treatment_col,
            outcome_col=self.outcome_col,
            confounders=self.confounders or self.feature_cols,
        )
        
        return analyzer.fit(df, run_refutation=True, verbose=verbose)
    
    def _run_uplift(self, df: pd.DataFrame, verbose: bool) -> UpliftResult:
        """Run uplift modeling."""
        modeler = UpliftModeler(
            treatment_col=self.treatment_col,
            outcome_col=self.outcome_col,
            feature_cols=self.feature_cols,
        )
        
        return modeler.fit(df, verbose=verbose)
    
    def _run_optimization(
        self,
        df: pd.DataFrame,
        budget: Optional[float],
        max_count: Optional[int],
        fairness_col: Optional[str],
        verbose: bool,
    ) -> OptimizationResult:
        """Run action optimization."""
        # Determine value column
        if self.value_col:
            value_col = self.value_col
        elif '_expected_benefit' in df.columns:
            value_col = '_expected_benefit'
        else:
            raise ValueError("No value column for optimization. Run uplift first or provide value_col.")
        
        optimizer = ActionOptimizer(
            value_col=value_col,
            cost_col=self.cost_col,
        )
        
        # Add constraints
        if budget is not None and self.cost_col is not None:
            optimizer.add_constraint(BudgetConstraint(
                max_budget=budget,
                cost_col=self.cost_col,
            ))
        
        if max_count is not None:
            optimizer.add_constraint(CapacityConstraint(max_count=max_count))
        
        if fairness_col is not None:
            optimizer.add_constraint(FairnessConstraint(
                group_col=fairness_col,
                max_disparity=0.1,
            ))
        
        return optimizer.optimize(df, verbose=verbose)
    
    def _run_policy(
        self,
        df: pd.DataFrame,
        n_iterations: int,
        verbose: bool,
    ) -> Optional[PolicyResult]:
        """Run policy learning."""
        try:
            # Create bandit environment from uplift results
            if '_ite' not in df.columns:
                if verbose:
                    print("  Skipping policy learning: no ITE available")
                return None
            
            # Build reward matrix
            X = df[self.feature_cols].values.astype(np.float32)
            ite = df['_ite'].values
            
            if self.cost_col:
                costs = df[self.cost_col].values
            else:
                costs = np.zeros(len(df))
            
            # Reward for not treating: 0
            # Reward for treating: benefit - cost
            if self.outcome_type == 'minimize':
                benefit = -ite  # Negative outcome reduction is positive benefit
            else:
                benefit = ite
            
            rewards = np.column_stack([
                np.zeros(len(df)),  # No action
                benefit - costs,  # Treat
            ])
            
            env = ContextualBanditEnv(
                X=X,
                rewards=rewards,
                feature_names=self.feature_cols,
                action_names=['no_action', 'treat'],
            )
            
            learner = PolicyLearner(env, algorithm='PPO')
            return learner.train(n_iterations=n_iterations, verbose=verbose)
            
        except Exception as e:
            if verbose:
                print(f"  Policy learning failed: {str(e)[:50]}")
            return None
    
    def _build_recommendations(
        self,
        df: pd.DataFrame,
        optimization: Optional[OptimizationResult],
    ) -> pd.DataFrame:
        """Build recommendations DataFrame."""
        result = pd.DataFrame(index=df.index)
        
        if optimization is not None:
            result['recommended_action'] = optimization.selected
        else:
            result['recommended_action'] = 0
        
        if '_expected_benefit' in df.columns:
            result['expected_benefit'] = df['_expected_benefit']
        
        if '_ite' in df.columns:
            result['individual_treatment_effect'] = df['_ite']
        
        if self.cost_col and self.cost_col in df.columns:
            result['cost'] = df[self.cost_col]
        
        # Add confidence based on ITE magnitude
        if '_ite' in df.columns:
            ite = df['_ite'].values
            ite_std = np.std(ite)
            if ite_std > 0:
                result['confidence'] = np.clip(np.abs(ite) / (2 * ite_std), 0, 1)
            else:
                result['confidence'] = 0.5
        
        return result
    
    @property
    def result(self) -> Optional[DecisionResult]:
        return self._result


def auto_decision(
    df: pd.DataFrame,
    treatment_col: str,
    outcome_col: str,
    cost_col: Optional[str] = None,
    budget: Optional[float] = None,
    outcome_type: str = 'minimize',
    **kwargs,
) -> DecisionResult:
    """
    Convenience function for quick decision intelligence.
    
    Parameters
    ----------
    df : pd.DataFrame
        Data.
    treatment_col : str
        Treatment column.
    outcome_col : str
        Outcome column.
    cost_col : str, optional
        Cost column.
    budget : float, optional
        Budget constraint.
    outcome_type : str, default='minimize'
        Whether to minimize or maximize outcome.
    **kwargs
        Additional arguments for DecisionIntelligence.fit().
    
    Returns
    -------
    DecisionResult
        Decision intelligence results.
    """
    di = DecisionIntelligence(
        treatment_col=treatment_col,
        outcome_col=outcome_col,
        cost_col=cost_col,
        outcome_type=outcome_type,
    )
    
    return di.fit(df, budget=budget, **kwargs)

