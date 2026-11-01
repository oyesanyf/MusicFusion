"""
Uplift Modeling / Individual Treatment Effects (EconML Integration)
====================================================================

Provides heterogeneous treatment effect estimation for personalized decisions:
- Individual Treatment Effects (ITE) estimation
- Multiple meta-learners (T-Learner, S-Learner, X-Learner, DR-Learner)
- Causal Forest
- Uplift evaluation metrics (Qini, AUUC)
- Uplift curves and visualization

This module bridges prediction and decision: "For whom does the treatment work best?"

Usage:
    from acdso.decision import UpliftModeler
    
    modeler = UpliftModeler(
        treatment_col='offered_incentive',
        outcome_col='churned',
    )
    
    result = modeler.fit(df)
    
    # Get individual treatment effects
    ite = result.ite
    
    # Target customers with highest expected benefit
    df['expected_benefit'] = -ite  # Negative because outcome is churn (bad)
    top_targets = df.nlargest(100, 'expected_benefit')
"""

from __future__ import annotations

import warnings
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any, Union, Callable
import numpy as np
import pandas as pd
from sklearn.base import BaseEstimator, clone
from sklearn.model_selection import cross_val_predict
from sklearn.ensemble import RandomForestClassifier, RandomForestRegressor, GradientBoostingRegressor
from sklearn.linear_model import LogisticRegression, Ridge


@dataclass
class UpliftCandidate:
    """
    A candidate uplift model.
    
    Attributes
    ----------
    name : str
        Model name.
    model : BaseEstimator
        The uplift model.
    family : str
        Model family: 't_learner', 's_learner', 'x_learner', 'dr_learner', 'causal_forest'.
    complexity : str
        Complexity level.
    hyperparams : dict
        Hyperparameters.
    """
    
    name: str
    model: Any
    family: str
    complexity: str = 'medium'
    hyperparams: Dict[str, Any] = field(default_factory=dict)


@dataclass
class UpliftResult:
    """
    Result of uplift modeling.
    
    Attributes
    ----------
    ite : np.ndarray
        Individual Treatment Effects for each unit.
    ate : float
        Average Treatment Effect.
    att : float
        Average Treatment Effect on Treated.
    atc : float
        Average Treatment Effect on Control.
    qini_score : float
        Qini coefficient.
    auuc : float
        Area Under Uplift Curve.
    best_model : str
        Name of best model.
    all_models : Dict[str, float]
        All models and their scores.
    feature_importance : Dict[str, float]
        Feature importance for treatment effect heterogeneity.
    percentile_effects : Dict[int, float]
        Treatment effect by percentile of predicted ITE.
    policy_value : float
        Value of treating top decile vs random.
    """
    
    ite: np.ndarray
    ate: float = 0.0
    att: float = 0.0
    atc: float = 0.0
    qini_score: float = 0.0
    auuc: float = 0.0
    best_model: str = ''
    all_models: Dict[str, float] = field(default_factory=dict)
    feature_importance: Dict[str, float] = field(default_factory=dict)
    percentile_effects: Dict[int, float] = field(default_factory=dict)
    policy_value: float = 0.0
    
    def get_targeting_order(self) -> np.ndarray:
        """Get indices sorted by ITE (highest benefit first)."""
        return np.argsort(-self.ite)
    
    def get_top_k_targets(self, k: int) -> np.ndarray:
        """Get indices of top-k targets."""
        return self.get_targeting_order()[:k]
    
    def summary(self) -> str:
        """Generate summary string."""
        return (
            f"Uplift Model Results\n"
            f"{'='*40}\n"
            f"Best Model: {self.best_model}\n"
            f"ATE: {self.ate:.4f}\n"
            f"Qini Score: {self.qini_score:.4f}\n"
            f"AUUC: {self.auuc:.4f}\n"
            f"Policy Value: {self.policy_value:.4f}\n"
        )


class TLearner(BaseEstimator):
    """
    T-Learner (Two-Model Learner) for uplift estimation.
    
    Fits separate models for treatment and control groups,
    then estimates ITE as the difference in predictions.
    
    ITE = E[Y|X, T=1] - E[Y|X, T=0]
    
    Parameters
    ----------
    base_model : BaseEstimator
        Base model class for outcome prediction.
    """
    
    def __init__(self, base_model=None):
        self.base_model = base_model or GradientBoostingRegressor(
            n_estimators=100, max_depth=5, random_state=42
        )
        self._model_t1 = None
        self._model_t0 = None
    
    def fit(self, X: np.ndarray, y: np.ndarray, treatment: np.ndarray):
        """Fit separate models for treated and control."""
        self._model_t1 = clone(self.base_model)
        self._model_t0 = clone(self.base_model)
        
        mask_t1 = treatment == 1
        mask_t0 = treatment == 0
        
        if mask_t1.sum() > 0:
            self._model_t1.fit(X[mask_t1], y[mask_t1])
        if mask_t0.sum() > 0:
            self._model_t0.fit(X[mask_t0], y[mask_t0])
        
        return self
    
    def effect(self, X: np.ndarray) -> np.ndarray:
        """Estimate individual treatment effects."""
        pred_t1 = self._model_t1.predict(X)
        pred_t0 = self._model_t0.predict(X)
        return pred_t1 - pred_t0
    
    def predict(self, X: np.ndarray) -> np.ndarray:
        """Alias for effect()."""
        return self.effect(X)


class SLearner(BaseEstimator):
    """
    S-Learner (Single-Model Learner) for uplift estimation.
    
    Fits a single model with treatment as a feature,
    then estimates ITE by comparing predictions with T=1 vs T=0.
    
    ITE = f(X, T=1) - f(X, T=0)
    """
    
    def __init__(self, base_model=None):
        self.base_model = base_model or GradientBoostingRegressor(
            n_estimators=100, max_depth=5, random_state=42
        )
        self._model = None
    
    def fit(self, X: np.ndarray, y: np.ndarray, treatment: np.ndarray):
        """Fit single model with treatment as feature."""
        # Add treatment as feature
        X_with_t = np.column_stack([X, treatment])
        
        self._model = clone(self.base_model)
        self._model.fit(X_with_t, y)
        
        return self
    
    def effect(self, X: np.ndarray) -> np.ndarray:
        """Estimate individual treatment effects."""
        n = len(X)
        
        # Predict with T=1
        X_t1 = np.column_stack([X, np.ones(n)])
        pred_t1 = self._model.predict(X_t1)
        
        # Predict with T=0
        X_t0 = np.column_stack([X, np.zeros(n)])
        pred_t0 = self._model.predict(X_t0)
        
        return pred_t1 - pred_t0
    
    def predict(self, X: np.ndarray) -> np.ndarray:
        return self.effect(X)


class XLearner(BaseEstimator):
    """
    X-Learner for uplift estimation.
    
    More efficient when treatment groups are imbalanced.
    Uses imputed treatment effects and propensity weighting.
    """
    
    def __init__(self, base_model=None, propensity_model=None):
        self.base_model = base_model or GradientBoostingRegressor(
            n_estimators=100, max_depth=5, random_state=42
        )
        self.propensity_model = propensity_model or LogisticRegression(max_iter=1000)
        
        self._model_t1 = None
        self._model_t0 = None
        self._model_tau_t1 = None
        self._model_tau_t0 = None
        self._propensity = None
    
    def fit(self, X: np.ndarray, y: np.ndarray, treatment: np.ndarray):
        """Fit X-Learner."""
        # Step 1: Fit outcome models
        self._model_t1 = clone(self.base_model)
        self._model_t0 = clone(self.base_model)
        
        mask_t1 = treatment == 1
        mask_t0 = treatment == 0
        
        self._model_t1.fit(X[mask_t1], y[mask_t1])
        self._model_t0.fit(X[mask_t0], y[mask_t0])
        
        # Step 2: Compute imputed treatment effects
        # For treated: D_1 = Y - mu_0(X)
        D_1 = y[mask_t1] - self._model_t0.predict(X[mask_t1])
        
        # For control: D_0 = mu_1(X) - Y
        D_0 = self._model_t1.predict(X[mask_t0]) - y[mask_t0]
        
        # Step 3: Fit CATE models on imputed effects
        self._model_tau_t1 = clone(self.base_model)
        self._model_tau_t0 = clone(self.base_model)
        
        self._model_tau_t1.fit(X[mask_t1], D_1)
        self._model_tau_t0.fit(X[mask_t0], D_0)
        
        # Step 4: Fit propensity model
        self._propensity = clone(self.propensity_model)
        self._propensity.fit(X, treatment)
        
        return self
    
    def effect(self, X: np.ndarray) -> np.ndarray:
        """Estimate individual treatment effects."""
        # Get propensity scores
        ps = self._propensity.predict_proba(X)[:, 1]
        
        # Get CATE estimates from both models
        tau_1 = self._model_tau_t1.predict(X)
        tau_0 = self._model_tau_t0.predict(X)
        
        # Weighted average using propensity
        # Weight towards the model trained on more similar units
        tau = ps * tau_0 + (1 - ps) * tau_1
        
        return tau
    
    def predict(self, X: np.ndarray) -> np.ndarray:
        return self.effect(X)


class DRLearner(BaseEstimator):
    """
    Doubly Robust Learner for uplift estimation.
    
    Combines outcome modeling and propensity weighting for
    robustness to model misspecification.
    
    If using EconML, wraps their DRLearner. Otherwise, implements
    a simplified version.
    """
    
    def __init__(
        self,
        model_y=None,
        model_t=None,
        model_final=None,
        use_econml: bool = True,
    ):
        self.model_y = model_y
        self.model_t = model_t
        self.model_final = model_final
        self.use_econml = use_econml
        
        self._econml_model = None
        self._fallback_model = None
    
    def fit(self, X: np.ndarray, y: np.ndarray, treatment: np.ndarray):
        """Fit DR Learner."""
        if self.use_econml:
            try:
                from econml.dr import DRLearner as EconMLDRLearner
                
                self._econml_model = EconMLDRLearner(
                    model_regression=self.model_y or RandomForestRegressor(
                        n_estimators=100, random_state=42
                    ),
                    model_propensity=self.model_t or LogisticRegression(max_iter=1000),
                    model_final=self.model_final or Ridge(),
                    random_state=42,
                )
                
                self._econml_model.fit(y, treatment, X=X)
                return self
                
            except ImportError:
                pass
        
        # Fallback to X-Learner if EconML unavailable
        self._fallback_model = XLearner(
            base_model=self.model_y or GradientBoostingRegressor(
                n_estimators=100, max_depth=5, random_state=42
            ),
            propensity_model=self.model_t or LogisticRegression(max_iter=1000),
        )
        self._fallback_model.fit(X, y, treatment)
        
        return self
    
    def effect(self, X: np.ndarray) -> np.ndarray:
        """Estimate individual treatment effects."""
        if self._econml_model is not None:
            return self._econml_model.effect(X).flatten()
        return self._fallback_model.effect(X)
    
    def predict(self, X: np.ndarray) -> np.ndarray:
        return self.effect(X)


class CausalForestWrapper(BaseEstimator):
    """
    Causal Forest wrapper for uplift estimation.
    
    Uses EconML's CausalForestDML if available, otherwise
    falls back to a custom implementation.
    """
    
    def __init__(
        self,
        n_estimators: int = 100,
        max_depth: Optional[int] = None,
        min_samples_leaf: int = 5,
    ):
        self.n_estimators = n_estimators
        self.max_depth = max_depth
        self.min_samples_leaf = min_samples_leaf
        
        self._econml_model = None
        self._fallback_model = None
    
    def fit(self, X: np.ndarray, y: np.ndarray, treatment: np.ndarray):
        """Fit Causal Forest."""
        try:
            from econml.dml import CausalForestDML
            
            self._econml_model = CausalForestDML(
                model_y=RandomForestRegressor(n_estimators=100, random_state=42),
                model_t=LogisticRegression(max_iter=1000),
                n_estimators=self.n_estimators,
                max_depth=self.max_depth,
                min_samples_leaf=self.min_samples_leaf,
                random_state=42,
            )
            
            self._econml_model.fit(y, treatment, X=X)
            
        except ImportError:
            # Fallback to T-Learner with RF
            self._fallback_model = TLearner(
                base_model=RandomForestRegressor(
                    n_estimators=self.n_estimators,
                    max_depth=self.max_depth,
                    min_samples_leaf=self.min_samples_leaf,
                    random_state=42,
                )
            )
            self._fallback_model.fit(X, y, treatment)
        
        return self
    
    def effect(self, X: np.ndarray) -> np.ndarray:
        """Estimate individual treatment effects."""
        if self._econml_model is not None:
            return self._econml_model.effect(X).flatten()
        return self._fallback_model.effect(X)
    
    def predict(self, X: np.ndarray) -> np.ndarray:
        return self.effect(X)


def qini_score(
    y_true: np.ndarray,
    uplift_pred: np.ndarray,
    treatment: np.ndarray,
) -> float:
    """
    Compute Qini coefficient for uplift model evaluation.
    
    Qini measures how well the model ranks units by treatment effect.
    Higher is better. Range: [-1, 1] theoretically, typically [0, 0.5].
    
    Parameters
    ----------
    y_true : np.ndarray
        True outcomes.
    uplift_pred : np.ndarray
        Predicted individual treatment effects.
    treatment : np.ndarray
        Treatment indicators (0/1).
    
    Returns
    -------
    float
        Qini coefficient.
    """
    # Sort by predicted uplift (descending)
    order = np.argsort(-uplift_pred)
    
    y_sorted = y_true[order]
    t_sorted = treatment[order]
    
    n = len(y_true)
    n_t = treatment.sum()
    n_c = n - n_t
    
    if n_t == 0 or n_c == 0:
        return 0.0
    
    # Cumulative outcomes
    cum_y_t = np.cumsum(y_sorted * t_sorted)
    cum_y_c = np.cumsum(y_sorted * (1 - t_sorted))
    cum_n_t = np.cumsum(t_sorted)
    cum_n_c = np.cumsum(1 - t_sorted)
    
    # Qini curve: cumulative uplift
    with np.errstate(divide='ignore', invalid='ignore'):
        qini_curve = cum_y_t - cum_y_c * (cum_n_t / (cum_n_c + 1e-10))
    
    # Compute area under Qini curve (normalized)
    qini_area = np.trapz(qini_curve) / n
    
    # Random targeting baseline
    random_area = (cum_y_t[-1] - cum_y_c[-1] * (n_t / n_c)) * 0.5
    
    # Qini coefficient
    if random_area == 0:
        return 0.0
    
    return float(qini_area / (n * 0.5))


def auuc_score(
    y_true: np.ndarray,
    uplift_pred: np.ndarray,
    treatment: np.ndarray,
) -> float:
    """
    Compute Area Under Uplift Curve (AUUC).
    
    Similar to Qini but measures cumulative incremental gains.
    
    Parameters
    ----------
    y_true : np.ndarray
        True outcomes.
    uplift_pred : np.ndarray
        Predicted individual treatment effects.
    treatment : np.ndarray
        Treatment indicators (0/1).
    
    Returns
    -------
    float
        AUUC score.
    """
    # Sort by predicted uplift (descending)
    order = np.argsort(-uplift_pred)
    
    y_sorted = y_true[order]
    t_sorted = treatment[order]
    
    n = len(y_true)
    
    # Compute uplift curve points
    cum_treated = np.cumsum(t_sorted)
    cum_control = np.cumsum(1 - t_sorted)
    cum_resp_treated = np.cumsum(y_sorted * t_sorted)
    cum_resp_control = np.cumsum(y_sorted * (1 - t_sorted))
    
    with np.errstate(divide='ignore', invalid='ignore'):
        treated_rate = cum_resp_treated / (cum_treated + 1e-10)
        control_rate = cum_resp_control / (cum_control + 1e-10)
        uplift_curve_vals = treated_rate - control_rate
    
    # Area under curve
    auuc = np.trapz(uplift_curve_vals) / n
    
    return float(auuc)


def uplift_curve(
    y_true: np.ndarray,
    uplift_pred: np.ndarray,
    treatment: np.ndarray,
    n_bins: int = 10,
) -> Tuple[np.ndarray, np.ndarray]:
    """
    Compute uplift curve for visualization.
    
    Returns
    -------
    percentiles : np.ndarray
        Percentile thresholds.
    uplift_values : np.ndarray
        Uplift at each percentile.
    """
    # Sort by predicted uplift (descending)
    order = np.argsort(-uplift_pred)
    
    y_sorted = y_true[order]
    t_sorted = treatment[order]
    
    n = len(y_true)
    bin_size = n // n_bins
    
    percentiles = []
    uplift_values = []
    
    for i in range(n_bins):
        end_idx = (i + 1) * bin_size
        
        y_bin = y_sorted[:end_idx]
        t_bin = t_sorted[:end_idx]
        
        n_t = t_bin.sum()
        n_c = len(t_bin) - n_t
        
        if n_t > 0 and n_c > 0:
            treated_rate = (y_bin * t_bin).sum() / n_t
            control_rate = (y_bin * (1 - t_bin)).sum() / n_c
            uplift = treated_rate - control_rate
        else:
            uplift = 0.0
        
        percentiles.append((i + 1) * 100 // n_bins)
        uplift_values.append(uplift)
    
    return np.array(percentiles), np.array(uplift_values)


class UpliftModeler:
    """
    Comprehensive uplift modeling for personalized targeting.
    
    Estimates Individual Treatment Effects (ITE) using multiple
    methods and selects the best one based on Qini/AUUC.
    
    Parameters
    ----------
    treatment_col : str
        Treatment column name.
    outcome_col : str
        Outcome column name.
    feature_cols : List[str], optional
        Feature columns. If None, uses all other columns.
    models : List[str], optional
        Models to try: 't_learner', 's_learner', 'x_learner', 'dr_learner', 'causal_forest'.
    cv : int, default=5
        Cross-validation folds for evaluation.
    
    Example
    -------
    >>> modeler = UpliftModeler(
    ...     treatment_col='offered_incentive',
    ...     outcome_col='churned',
    ... )
    >>> result = modeler.fit(df)
    >>> print(f"Best model: {result.best_model}")
    >>> print(f"Qini: {result.qini_score:.4f}")
    >>> 
    >>> # Get top targets
    >>> top_100 = result.get_top_k_targets(100)
    """
    
    AVAILABLE_MODELS = ['t_learner', 's_learner', 'x_learner', 'dr_learner', 'causal_forest']
    
    def __init__(
        self,
        treatment_col: str,
        outcome_col: str,
        feature_cols: Optional[List[str]] = None,
        models: Optional[List[str]] = None,
        cv: int = 5,
    ):
        self.treatment_col = treatment_col
        self.outcome_col = outcome_col
        self.feature_cols = feature_cols
        self.models = models or self.AVAILABLE_MODELS
        self.cv = cv
        
        self._result: Optional[UpliftResult] = None
        self._best_model: Optional[BaseEstimator] = None
    
    def fit(
        self,
        df: pd.DataFrame,
        verbose: bool = True,
    ) -> UpliftResult:
        """
        Fit uplift models and select best.
        
        Parameters
        ----------
        df : pd.DataFrame
            Data with treatment, outcome, and features.
        verbose : bool, default=True
            Whether to print progress.
        
        Returns
        -------
        UpliftResult
            Uplift modeling results.
        """
        df = df.copy()
        
        # Determine features
        if self.feature_cols is None:
            self.feature_cols = [
                c for c in df.columns
                if c not in [self.treatment_col, self.outcome_col]
                and df[c].dtype in [np.float64, np.int64, float, int, bool]
            ]
        
        X = df[self.feature_cols].values
        y = df[self.outcome_col].values
        treatment = df[self.treatment_col].values
        
        # Handle NaN
        mask = ~np.isnan(X).any(axis=1) & ~np.isnan(y) & ~np.isnan(treatment)
        X, y, treatment = X[mask], y[mask], treatment[mask]
        
        if verbose:
            print("=" * 50)
            print("ACDSO Uplift Modeling")
            print("=" * 50)
            print(f"Samples: {len(X)}")
            print(f"Features: {len(self.feature_cols)}")
            print(f"Treatment rate: {treatment.mean():.1%}")
            print(f"Models to evaluate: {self.models}")
        
        # Evaluate each model
        model_scores = {}
        model_ites = {}
        
        for model_name in self.models:
            try:
                if verbose:
                    print(f"\nEvaluating {model_name}...")
                
                model = self._create_model(model_name)
                model.fit(X, y, treatment)
                
                ite = model.effect(X)
                
                # Evaluate with Qini and AUUC
                qini = qini_score(y, ite, treatment)
                auuc = auuc_score(y, ite, treatment)
                
                model_scores[model_name] = (qini + auuc) / 2
                model_ites[model_name] = ite
                
                if verbose:
                    print(f"  Qini: {qini:.4f}, AUUC: {auuc:.4f}")
                
            except Exception as e:
                if verbose:
                    print(f"  Failed: {str(e)[:50]}")
                model_scores[model_name] = 0.0
        
        # Select best model
        best_model_name = max(model_scores.keys(), key=lambda k: model_scores[k])
        best_ite = model_ites.get(best_model_name, np.zeros(len(X)))
        
        # Refit best model on full data
        self._best_model = self._create_model(best_model_name)
        self._best_model.fit(X, y, treatment)
        
        # Compute final metrics
        final_qini = qini_score(y, best_ite, treatment)
        final_auuc = auuc_score(y, best_ite, treatment)
        
        # Compute ATE, ATT, ATC
        ate = float(np.mean(best_ite))
        att = float(np.mean(best_ite[treatment == 1])) if treatment.sum() > 0 else ate
        atc = float(np.mean(best_ite[treatment == 0])) if (1 - treatment).sum() > 0 else ate
        
        # Compute percentile effects
        percentile_effects = {}
        for p in [10, 25, 50, 75, 90]:
            threshold = np.percentile(best_ite, 100 - p)
            mask_top = best_ite >= threshold
            if mask_top.sum() > 0:
                percentile_effects[p] = float(np.mean(best_ite[mask_top]))
        
        # Compute policy value (top decile vs random)
        top_10_idx = best_ite >= np.percentile(best_ite, 90)
        if top_10_idx.sum() > 0:
            policy_value = float(np.mean(best_ite[top_10_idx])) - ate
        else:
            policy_value = 0.0
        
        # Feature importance (if available)
        feature_importance = self._extract_feature_importance(self._best_model)
        
        self._result = UpliftResult(
            ite=best_ite,
            ate=ate,
            att=att,
            atc=atc,
            qini_score=final_qini,
            auuc=final_auuc,
            best_model=best_model_name,
            all_models={k: float(v) for k, v in model_scores.items()},
            feature_importance=feature_importance,
            percentile_effects=percentile_effects,
            policy_value=policy_value,
        )
        
        if verbose:
            print("\n" + self._result.summary())
        
        return self._result
    
    def _create_model(self, model_name: str) -> BaseEstimator:
        """Create uplift model by name."""
        models = {
            't_learner': TLearner(),
            's_learner': SLearner(),
            'x_learner': XLearner(),
            'dr_learner': DRLearner(),
            'causal_forest': CausalForestWrapper(),
        }
        
        if model_name not in models:
            raise ValueError(f"Unknown model: {model_name}")
        
        return models[model_name]
    
    def _extract_feature_importance(self, model: BaseEstimator) -> Dict[str, float]:
        """Extract feature importance from model."""
        importance = {}
        
        # Try different attributes
        for attr in ['feature_importances_', 'coef_']:
            if hasattr(model, attr):
                values = getattr(model, attr)
                if isinstance(values, np.ndarray):
                    values = np.abs(values).flatten()
                    for i, v in enumerate(values[:len(self.feature_cols)]):
                        importance[self.feature_cols[i]] = float(v)
                break
        
        # For T-Learner, average importances from both models
        if hasattr(model, '_model_t1') and hasattr(model._model_t1, 'feature_importances_'):
            imp_t1 = model._model_t1.feature_importances_
            imp_t0 = model._model_t0.feature_importances_
            avg_imp = (imp_t1 + imp_t0) / 2
            for i, v in enumerate(avg_imp[:len(self.feature_cols)]):
                importance[self.feature_cols[i]] = float(v)
        
        # Sort by importance
        importance = dict(sorted(importance.items(), key=lambda x: -x[1]))
        
        return importance
    
    def predict(self, df: pd.DataFrame) -> np.ndarray:
        """Predict ITE for new data."""
        if self._best_model is None:
            raise RuntimeError("Must call fit() before predict()")
        
        X = df[self.feature_cols].values
        return self._best_model.effect(X)
    
    @property
    def result(self) -> Optional[UpliftResult]:
        return self._result


def get_uplift_candidates() -> List[UpliftCandidate]:
    """Get all available uplift model candidates."""
    return [
        UpliftCandidate(
            name='T-Learner (GBM)',
            model=TLearner(GradientBoostingRegressor(n_estimators=100, max_depth=5)),
            family='t_learner',
            complexity='medium',
        ),
        UpliftCandidate(
            name='T-Learner (RF)',
            model=TLearner(RandomForestRegressor(n_estimators=100)),
            family='t_learner',
            complexity='medium',
        ),
        UpliftCandidate(
            name='S-Learner (GBM)',
            model=SLearner(GradientBoostingRegressor(n_estimators=100, max_depth=5)),
            family='s_learner',
            complexity='simple',
        ),
        UpliftCandidate(
            name='X-Learner',
            model=XLearner(),
            family='x_learner',
            complexity='complex',
        ),
        UpliftCandidate(
            name='DR-Learner',
            model=DRLearner(),
            family='dr_learner',
            complexity='complex',
        ),
        UpliftCandidate(
            name='Causal Forest',
            model=CausalForestWrapper(n_estimators=100),
            family='causal_forest',
            complexity='complex',
        ),
    ]

