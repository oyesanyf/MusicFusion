"""
ACDSO Decision Intelligence Module
===================================

Extends ACDSO's AutoML capabilities with decision-grade analytics:
- Causal Effect Estimation (DoWhy)
- Uplift Modeling / Individual Treatment Effects (EconML)
- Action Optimization Under Constraints (OR-Tools)
- Policy Learning for Sequential Decisions (RLlib)

The Decision Intelligence layer sits ON TOP of ACDSO's predictive engine:
    ACDSO AutoML → Best predictive model under constraints
    Decision Intelligence → Best action under constraints (budget, policy, ROI)

Usage:
    from acdso.decision import DecisionIntelligence, CausalAnalyzer, UpliftModeler
    
    # Full pipeline
    di = DecisionIntelligence(
        treatment_col='offered_incentive',
        outcome_col='churned',
        cost_col='incentive_cost',
    )
    result = di.fit(df)
    actions = result.get_optimal_actions(budget=10000)
    
    # Or individual components
    causal = CausalAnalyzer(treatment='incentive', outcome='churn')
    ate = causal.estimate_ate(df)
    
    uplift = UpliftModeler(treatment='incentive', outcome='churn')
    ite = uplift.estimate_ite(df)
"""

from .causal import (
    CausalAnalyzer,
    CausalModel,
    CausalEstimate,
    CausalReadinessReport,
    estimate_ate,
    estimate_att,
    check_causal_assumptions,
    refute_estimate,
)
from .uplift import (
    UpliftModeler,
    UpliftResult,
    UpliftCandidate,
    TLearner,
    SLearner,
    XLearner,
    DRLearner,
    CausalForestWrapper,
    qini_score,
    auuc_score,
    uplift_curve,
    get_uplift_candidates,
)
from .optimizer import (
    ActionOptimizer,
    OptimizationResult,
    Constraint,
    BudgetConstraint,
    CapacityConstraint,
    FairnessConstraint,
    optimize_actions,
    knapsack_optimize,
    ilp_optimize,
)
from .policy import (
    PolicyLearner,
    PolicyResult,
    DecisionEnvironment,
    ContextualBanditEnv,
    SequentialDecisionEnv,
    train_policy,
    evaluate_policy,
    get_policy_action,
)
from .core import (
    DecisionIntelligence,
    DecisionResult,
    DecisionAuditTrail,
    auto_decision,
)

__all__ = [
    # Causal Analysis (DoWhy)
    "CausalAnalyzer",
    "CausalModel",
    "CausalEstimate",
    "CausalReadinessReport",
    "estimate_ate",
    "estimate_att",
    "check_causal_assumptions",
    "refute_estimate",
    # Uplift Modeling (EconML)
    "UpliftModeler",
    "UpliftResult",
    "UpliftCandidate",
    "TLearner",
    "SLearner",
    "XLearner",
    "DRLearner",
    "CausalForestWrapper",
    "qini_score",
    "auuc_score",
    "uplift_curve",
    "get_uplift_candidates",
    # Action Optimization (OR-Tools)
    "ActionOptimizer",
    "OptimizationResult",
    "Constraint",
    "BudgetConstraint",
    "CapacityConstraint",
    "FairnessConstraint",
    "optimize_actions",
    "knapsack_optimize",
    "ilp_optimize",
    # Policy Learning (RLlib)
    "PolicyLearner",
    "PolicyResult",
    "DecisionEnvironment",
    "ContextualBanditEnv",
    "SequentialDecisionEnv",
    "train_policy",
    "evaluate_policy",
    "get_policy_action",
    # Core Decision Intelligence
    "DecisionIntelligence",
    "DecisionResult",
    "DecisionAuditTrail",
    "auto_decision",
]

