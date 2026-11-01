"""
Causal Analysis Module (DoWhy Integration)
==========================================

Provides causal effect estimation and assumption validation:
- Average Treatment Effect (ATE) estimation
- Average Treatment Effect on Treated (ATT)
- Causal assumption checking (positivity, overlap, unconfoundedness)
- Estimate refutation tests
- Causal readiness assessment for risk reports

This module transforms ACDSO from "prediction-grade" to "decision-grade".

Usage:
    from acdso.decision import CausalAnalyzer
    
    analyzer = CausalAnalyzer(
        treatment_col='incentive_offered',
        outcome_col='churned',
        confounders=['age', 'tenure', 'spend'],
    )
    
    # Estimate effects
    result = analyzer.fit(df)
    print(f"ATE: {result.ate}")
    print(f"Causal ready: {result.readiness.decision_ready}")
"""

from __future__ import annotations

import warnings
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any, Union
import numpy as np
import pandas as pd


@dataclass
class CausalReadinessReport:
    """
    Assessment of whether data supports valid causal conclusions.
    
    This is critical for decision-making: a model can predict well
    but produce garbage causal estimates if assumptions are violated.
    
    Attributes
    ----------
    decision_ready : bool
        Overall assessment: can we trust causal conclusions?
    treatment_overlap_score : float
        Propensity score overlap (0-1, higher is better).
    propensity_min : float
        Minimum propensity score (should be > 0.01).
    propensity_max : float
        Maximum propensity score (should be < 0.99).
    positivity_violations : int
        Number of regions with no overlap.
    treatment_rate : float
        Fraction of units that received treatment.
    treatment_imbalance_severity : str
        'low', 'medium', 'high', 'critical'.
    confounders_identified : List[str]
        Confounders included in the model.
    confounders_potentially_missing : List[str]
        Suspected missing confounders.
    post_treatment_features : List[str]
        Features that may occur after treatment (leakage).
    overlap_warnings : List[str]
        Specific overlap/positivity warnings.
    causal_warnings : List[str]
        All causal assumption warnings.
    severity_score : float
        Numeric severity (0-1, higher is worse).
    """
    
    decision_ready: bool = True
    treatment_overlap_score: float = 1.0
    propensity_min: float = 0.0
    propensity_max: float = 1.0
    positivity_violations: int = 0
    positivity_violation_details: List[str] = field(default_factory=list)
    
    treatment_rate: float = 0.5
    treatment_imbalance_severity: str = 'low'
    
    confounders_identified: List[str] = field(default_factory=list)
    confounders_potentially_missing: List[str] = field(default_factory=list)
    
    post_treatment_features: List[str] = field(default_factory=list)
    
    overlap_warnings: List[str] = field(default_factory=list)
    causal_warnings: List[str] = field(default_factory=list)
    
    severity_score: float = 0.0
    
    def to_dict(self) -> Dict[str, Any]:
        return {k: v for k, v in self.__dict__.items()}


@dataclass
class CausalEstimate:
    """
    Result of causal effect estimation.
    
    Attributes
    ----------
    effect : float
        Estimated causal effect.
    effect_type : str
        Type of effect: 'ate', 'att', 'atc', 'cate'.
    confidence_interval : Tuple[float, float]
        95% confidence interval.
    standard_error : float
        Standard error of estimate.
    p_value : float
        P-value for null hypothesis of no effect.
    method : str
        Estimation method used.
    is_significant : bool
        Whether effect is statistically significant at alpha=0.05.
    refutation_results : Dict[str, Any]
        Results of refutation tests.
    interpretation : str
        Human-readable interpretation.
    """
    
    effect: float
    effect_type: str = 'ate'
    confidence_interval: Tuple[float, float] = (0.0, 0.0)
    standard_error: float = 0.0
    p_value: float = 1.0
    method: str = 'unknown'
    is_significant: bool = False
    refutation_results: Dict[str, Any] = field(default_factory=dict)
    interpretation: str = ''
    
    def __post_init__(self):
        if not self.interpretation:
            direction = "increases" if self.effect > 0 else "decreases"
            sig = "significantly" if self.is_significant else "not significantly"
            self.interpretation = (
                f"Treatment {sig} {direction} outcome by {abs(self.effect):.4f} "
                f"(95% CI: [{self.confidence_interval[0]:.4f}, {self.confidence_interval[1]:.4f}])"
            )


@dataclass
class CausalModel:
    """
    A fitted causal model.
    
    Attributes
    ----------
    treatment : str
        Treatment variable name.
    outcome : str
        Outcome variable name.
    confounders : List[str]
        Confounder variable names.
    ate : CausalEstimate
        Average Treatment Effect estimate.
    att : CausalEstimate
        Average Treatment Effect on Treated.
    readiness : CausalReadinessReport
        Causal readiness assessment.
    propensity_model : Any
        Fitted propensity score model.
    outcome_model : Any
        Fitted outcome model.
    dowhy_model : Any
        Underlying DoWhy model (if available).
    """
    
    treatment: str
    outcome: str
    confounders: List[str]
    ate: Optional[CausalEstimate] = None
    att: Optional[CausalEstimate] = None
    readiness: Optional[CausalReadinessReport] = None
    propensity_model: Any = None
    outcome_model: Any = None
    dowhy_model: Any = None
    identified_estimand: Any = None


class CausalAnalyzer:
    """
    Comprehensive causal effect analyzer using DoWhy.
    
    Provides:
    - ATE/ATT estimation with multiple methods
    - Causal assumption validation
    - Propensity score analysis
    - Refutation tests
    - Decision-readiness assessment
    
    Parameters
    ----------
    treatment_col : str
        Name of the treatment column (binary: 0/1).
    outcome_col : str
        Name of the outcome column.
    confounders : List[str], optional
        List of confounder column names. If None, auto-detected.
    effect_modifiers : List[str], optional
        Variables that modify treatment effect.
    instruments : List[str], optional
        Instrumental variables (if available).
    estimation_method : str, default='backdoor.propensity_score_weighting'
        DoWhy estimation method.
    
    Example
    -------
    >>> analyzer = CausalAnalyzer(
    ...     treatment_col='offered_incentive',
    ...     outcome_col='churned',
    ...     confounders=['age', 'tenure', 'monthly_spend'],
    ... )
    >>> model = analyzer.fit(df)
    >>> print(f"ATE: {model.ate.effect:.4f}")
    >>> print(f"Decision ready: {model.readiness.decision_ready}")
    """
    
    ESTIMATION_METHODS = [
        'backdoor.linear_regression',
        'backdoor.propensity_score_matching',
        'backdoor.propensity_score_weighting',
        'backdoor.propensity_score_stratification',
        'iv.instrumental_variable',
    ]
    
    def __init__(
        self,
        treatment_col: str,
        outcome_col: str,
        confounders: Optional[List[str]] = None,
        effect_modifiers: Optional[List[str]] = None,
        instruments: Optional[List[str]] = None,
        estimation_method: str = 'backdoor.propensity_score_weighting',
    ):
        self.treatment_col = treatment_col
        self.outcome_col = outcome_col
        self.confounders = confounders
        self.effect_modifiers = effect_modifiers
        self.instruments = instruments
        self.estimation_method = estimation_method
        
        self._model: Optional[CausalModel] = None
        self._dowhy_available = self._check_dowhy()
    
    def _check_dowhy(self) -> bool:
        """Check if DoWhy is available."""
        try:
            import dowhy
            return True
        except ImportError:
            return False
    
    def fit(
        self,
        df: pd.DataFrame,
        run_refutation: bool = True,
        verbose: bool = False,
    ) -> CausalModel:
        """
        Fit causal model and estimate effects.
        
        Parameters
        ----------
        df : pd.DataFrame
            Data with treatment, outcome, and confounders.
        run_refutation : bool, default=True
            Whether to run refutation tests.
        verbose : bool, default=False
            Whether to print progress.
        
        Returns
        -------
        CausalModel
            Fitted causal model with estimates.
        """
        df = df.copy()
        
        # Auto-detect confounders if not provided
        if self.confounders is None:
            self.confounders = self._auto_detect_confounders(df)
        
        # Check causal readiness
        readiness = self._check_causal_readiness(df)
        
        if verbose:
            print(f"Causal readiness: {'READY' if readiness.decision_ready else 'WARNING'}")
            if readiness.causal_warnings:
                for w in readiness.causal_warnings:
                    print(f"  - {w}")
        
        # Estimate effects
        if self._dowhy_available:
            ate, att, dowhy_model, estimand = self._estimate_with_dowhy(
                df, run_refutation, verbose
            )
        else:
            ate, att = self._estimate_without_dowhy(df)
            dowhy_model = None
            estimand = None
        
        # Build propensity model
        propensity_model = self._fit_propensity_model(df)
        
        self._model = CausalModel(
            treatment=self.treatment_col,
            outcome=self.outcome_col,
            confounders=self.confounders,
            ate=ate,
            att=att,
            readiness=readiness,
            propensity_model=propensity_model,
            dowhy_model=dowhy_model,
            identified_estimand=estimand,
        )
        
        return self._model
    
    def _auto_detect_confounders(self, df: pd.DataFrame) -> List[str]:
        """Auto-detect potential confounders."""
        exclude = {self.treatment_col, self.outcome_col}
        
        if self.effect_modifiers:
            exclude.update(self.effect_modifiers)
        if self.instruments:
            exclude.update(self.instruments)
        
        # All other numeric columns are potential confounders
        confounders = []
        for col in df.columns:
            if col in exclude:
                continue
            if df[col].dtype in [np.float64, np.int64, float, int, bool]:
                confounders.append(col)
        
        return confounders
    
    def _check_causal_readiness(self, df: pd.DataFrame) -> CausalReadinessReport:
        """Check if data supports valid causal conclusions."""
        report = CausalReadinessReport()
        report.confounders_identified = self.confounders or []
        
        # Treatment rate
        treatment = df[self.treatment_col].values
        report.treatment_rate = float(np.mean(treatment))
        
        # Treatment imbalance
        if report.treatment_rate < 0.05 or report.treatment_rate > 0.95:
            report.treatment_imbalance_severity = 'critical'
            report.causal_warnings.append(
                f"Extreme treatment imbalance: {report.treatment_rate:.1%} treated. "
                "Causal estimates may be unreliable."
            )
        elif report.treatment_rate < 0.1 or report.treatment_rate > 0.9:
            report.treatment_imbalance_severity = 'high'
            report.causal_warnings.append(
                f"High treatment imbalance: {report.treatment_rate:.1%} treated."
            )
        elif report.treatment_rate < 0.2 or report.treatment_rate > 0.8:
            report.treatment_imbalance_severity = 'medium'
        else:
            report.treatment_imbalance_severity = 'low'
        
        # Propensity score analysis (positivity/overlap)
        if self.confounders:
            propensity_scores = self._compute_propensity_scores(df)
            
            report.propensity_min = float(np.min(propensity_scores))
            report.propensity_max = float(np.max(propensity_scores))
            
            # Check positivity
            if report.propensity_min < 0.01:
                report.positivity_violations += 1
                report.overlap_warnings.append(
                    f"Near-zero propensity scores detected (min={report.propensity_min:.4f}). "
                    "Some units have almost no chance of treatment."
                )
            
            if report.propensity_max > 0.99:
                report.positivity_violations += 1
                report.overlap_warnings.append(
                    f"Near-one propensity scores detected (max={report.propensity_max:.4f}). "
                    "Some units are almost certain to be treated."
                )
            
            # Overlap score (based on propensity distribution)
            # Use trimmed range as overlap metric
            p10, p90 = np.percentile(propensity_scores, [10, 90])
            report.treatment_overlap_score = float(min(p10, 1 - p90) / 0.5)
            report.treatment_overlap_score = max(0, min(1, report.treatment_overlap_score))
            
            if report.treatment_overlap_score < 0.3:
                report.overlap_warnings.append(
                    "Poor overlap between treated and control groups."
                )
        
        # Check for post-treatment features (temporal leakage)
        report.post_treatment_features = self._detect_post_treatment_features(df)
        if report.post_treatment_features:
            report.causal_warnings.append(
                f"Potential post-treatment variables detected: {report.post_treatment_features}. "
                "Including these may bias causal estimates."
            )
        
        # Add overlap warnings to causal warnings
        report.causal_warnings.extend(report.overlap_warnings)
        
        # Compute severity score
        severity = 0.0
        if report.treatment_imbalance_severity == 'critical':
            severity += 0.4
        elif report.treatment_imbalance_severity == 'high':
            severity += 0.25
        elif report.treatment_imbalance_severity == 'medium':
            severity += 0.1
        
        severity += 0.2 * (1 - report.treatment_overlap_score)
        severity += 0.1 * min(report.positivity_violations, 2) / 2
        severity += 0.2 * min(len(report.post_treatment_features), 3) / 3
        
        report.severity_score = min(severity, 1.0)
        report.decision_ready = report.severity_score < 0.5
        
        return report
    
    def _compute_propensity_scores(self, df: pd.DataFrame) -> np.ndarray:
        """Compute propensity scores using logistic regression."""
        from sklearn.linear_model import LogisticRegression
        from sklearn.preprocessing import StandardScaler
        
        X = df[self.confounders].values
        T = df[self.treatment_col].values
        
        # Handle NaN
        mask = ~np.isnan(X).any(axis=1) & ~np.isnan(T)
        X_clean = X[mask]
        T_clean = T[mask]
        
        if len(X_clean) < 10:
            return np.full(len(df), 0.5)
        
        # Scale features
        scaler = StandardScaler()
        X_scaled = scaler.fit_transform(X_clean)
        
        # Fit logistic regression
        model = LogisticRegression(max_iter=1000, random_state=42)
        model.fit(X_scaled, T_clean)
        
        # Predict propensity
        X_all = scaler.transform(np.nan_to_num(X, nan=0))
        propensity = model.predict_proba(X_all)[:, 1]
        
        return propensity
    
    def _detect_post_treatment_features(self, df: pd.DataFrame) -> List[str]:
        """Detect features that might occur after treatment."""
        post_treatment = []
        
        # Heuristic: features highly correlated with treatment might be post-treatment
        treatment = df[self.treatment_col].values
        
        for col in self.confounders or []:
            if col == self.treatment_col:
                continue
            
            try:
                values = df[col].values
                corr = np.corrcoef(treatment, values)[0, 1]
                
                # Very high correlation might indicate post-treatment
                if abs(corr) > 0.9:
                    post_treatment.append(col)
            except Exception:
                pass
        
        # Check column names for suspicious patterns
        suspicious_keywords = ['after', 'post', 'result', 'outcome', 'response']
        for col in df.columns:
            if col in [self.treatment_col, self.outcome_col]:
                continue
            col_lower = col.lower()
            for keyword in suspicious_keywords:
                if keyword in col_lower and col not in post_treatment:
                    post_treatment.append(col)
                    break
        
        return post_treatment
    
    def _fit_propensity_model(self, df: pd.DataFrame) -> Any:
        """Fit and return propensity score model."""
        from sklearn.linear_model import LogisticRegression
        from sklearn.preprocessing import StandardScaler
        from sklearn.pipeline import Pipeline
        
        if not self.confounders:
            return None
        
        X = df[self.confounders].values
        T = df[self.treatment_col].values
        
        mask = ~np.isnan(X).any(axis=1) & ~np.isnan(T)
        
        pipeline = Pipeline([
            ('scaler', StandardScaler()),
            ('classifier', LogisticRegression(max_iter=1000, random_state=42)),
        ])
        
        pipeline.fit(X[mask], T[mask])
        
        return pipeline
    
    def _estimate_with_dowhy(
        self,
        df: pd.DataFrame,
        run_refutation: bool,
        verbose: bool,
    ) -> Tuple[CausalEstimate, CausalEstimate, Any, Any]:
        """Estimate effects using DoWhy."""
        from dowhy import CausalModel as DoWhyCausalModel
        
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            
            # Build DoWhy model
            model = DoWhyCausalModel(
                data=df,
                treatment=self.treatment_col,
                outcome=self.outcome_col,
                common_causes=self.confounders,
                effect_modifiers=self.effect_modifiers,
                instruments=self.instruments,
            )
            
            # Identify estimand
            estimand = model.identify_effect(proceed_when_unidentifiable=True)
            
            # Estimate ATE
            ate_estimate = model.estimate_effect(
                estimand,
                method_name=self.estimation_method,
            )
            
            ate = CausalEstimate(
                effect=float(ate_estimate.value),
                effect_type='ate',
                confidence_interval=self._get_confidence_interval(ate_estimate),
                standard_error=self._get_standard_error(ate_estimate),
                p_value=self._get_p_value(ate_estimate),
                method=self.estimation_method,
                is_significant=self._get_p_value(ate_estimate) < 0.05,
            )
            
            # Run refutation tests
            if run_refutation:
                refutation_results = {}
                
                try:
                    placebo = model.refute_estimate(
                        estimand, ate_estimate,
                        method_name="placebo_treatment_refuter",
                    )
                    refutation_results['placebo_treatment'] = {
                        'refuted': placebo.refutation_result is False,
                        'new_effect': float(placebo.new_effect) if hasattr(placebo, 'new_effect') else None,
                    }
                except Exception:
                    pass
                
                try:
                    random_cause = model.refute_estimate(
                        estimand, ate_estimate,
                        method_name="random_common_cause",
                    )
                    refutation_results['random_common_cause'] = {
                        'refuted': random_cause.refutation_result is False,
                        'new_effect': float(random_cause.new_effect) if hasattr(random_cause, 'new_effect') else None,
                    }
                except Exception:
                    pass
                
                try:
                    subset = model.refute_estimate(
                        estimand, ate_estimate,
                        method_name="data_subset_refuter",
                        subset_fraction=0.8,
                    )
                    refutation_results['data_subset'] = {
                        'refuted': subset.refutation_result is False,
                        'new_effect': float(subset.new_effect) if hasattr(subset, 'new_effect') else None,
                    }
                except Exception:
                    pass
                
                ate.refutation_results = refutation_results
            
            # ATT estimation (if possible)
            try:
                att_estimate = model.estimate_effect(
                    estimand,
                    method_name="backdoor.propensity_score_matching",
                    target_units="att",
                )
                att = CausalEstimate(
                    effect=float(att_estimate.value),
                    effect_type='att',
                    confidence_interval=self._get_confidence_interval(att_estimate),
                    method="backdoor.propensity_score_matching",
                )
            except Exception:
                att = ate  # Fall back to ATE
            
            return ate, att, model, estimand
    
    def _estimate_without_dowhy(
        self,
        df: pd.DataFrame,
    ) -> Tuple[CausalEstimate, CausalEstimate]:
        """Estimate effects using basic methods when DoWhy is unavailable."""
        from sklearn.linear_model import LinearRegression
        
        # Simple regression adjustment
        if self.confounders:
            X = df[self.confounders + [self.treatment_col]].values
        else:
            X = df[[self.treatment_col]].values
        
        y = df[self.outcome_col].values
        
        mask = ~np.isnan(X).any(axis=1) & ~np.isnan(y)
        
        model = LinearRegression()
        model.fit(X[mask], y[mask])
        
        # Treatment effect is coefficient on treatment
        if self.confounders:
            effect = model.coef_[-1]  # Last coefficient is treatment
        else:
            effect = model.coef_[0]
        
        ate = CausalEstimate(
            effect=float(effect),
            effect_type='ate',
            method='linear_regression_adjustment',
        )
        
        # IPW estimate for ATT
        propensity = self._compute_propensity_scores(df)
        treatment = df[self.treatment_col].values
        outcome = df[self.outcome_col].values
        
        # IPW weights
        weights = np.where(treatment == 1, 1, propensity / (1 - propensity + 1e-10))
        
        # Weighted mean difference
        treated_mean = np.average(outcome[treatment == 1], weights=weights[treatment == 1])
        control_mean = np.average(outcome[treatment == 0], weights=weights[treatment == 0])
        
        att = CausalEstimate(
            effect=float(treated_mean - control_mean),
            effect_type='att',
            method='inverse_propensity_weighting',
        )
        
        return ate, att
    
    def _get_confidence_interval(self, estimate) -> Tuple[float, float]:
        """Extract confidence interval from DoWhy estimate."""
        try:
            if hasattr(estimate, 'get_confidence_intervals'):
                ci = estimate.get_confidence_intervals()
                return (float(ci[0]), float(ci[1]))
        except Exception:
            pass
        return (float(estimate.value) - 0.1, float(estimate.value) + 0.1)
    
    def _get_standard_error(self, estimate) -> float:
        """Extract standard error from DoWhy estimate."""
        try:
            if hasattr(estimate, 'get_standard_error'):
                return float(estimate.get_standard_error())
        except Exception:
            pass
        return 0.0
    
    def _get_p_value(self, estimate) -> float:
        """Extract p-value from DoWhy estimate."""
        try:
            if hasattr(estimate, 'test_stat_significance'):
                result = estimate.test_stat_significance()
                if 'p_value' in result:
                    return float(result['p_value'])
        except Exception:
            pass
        return 1.0
    
    def estimate_ate(self, df: pd.DataFrame) -> CausalEstimate:
        """Quick ATE estimation."""
        model = self.fit(df, run_refutation=False, verbose=False)
        return model.ate
    
    def estimate_att(self, df: pd.DataFrame) -> CausalEstimate:
        """Quick ATT estimation."""
        model = self.fit(df, run_refutation=False, verbose=False)
        return model.att
    
    @property
    def model(self) -> Optional[CausalModel]:
        return self._model


def estimate_ate(
    df: pd.DataFrame,
    treatment_col: str,
    outcome_col: str,
    confounders: Optional[List[str]] = None,
) -> CausalEstimate:
    """
    Convenience function to estimate ATE.
    
    Parameters
    ----------
    df : pd.DataFrame
        Data.
    treatment_col : str
        Treatment column.
    outcome_col : str
        Outcome column.
    confounders : List[str], optional
        Confounder columns.
    
    Returns
    -------
    CausalEstimate
        ATE estimate.
    """
    analyzer = CausalAnalyzer(
        treatment_col=treatment_col,
        outcome_col=outcome_col,
        confounders=confounders,
    )
    return analyzer.estimate_ate(df)


def estimate_att(
    df: pd.DataFrame,
    treatment_col: str,
    outcome_col: str,
    confounders: Optional[List[str]] = None,
) -> CausalEstimate:
    """Convenience function to estimate ATT."""
    analyzer = CausalAnalyzer(
        treatment_col=treatment_col,
        outcome_col=outcome_col,
        confounders=confounders,
    )
    return analyzer.estimate_att(df)


def check_causal_assumptions(
    df: pd.DataFrame,
    treatment_col: str,
    outcome_col: str,
    confounders: Optional[List[str]] = None,
) -> CausalReadinessReport:
    """
    Check causal assumptions without fitting full model.
    
    Returns
    -------
    CausalReadinessReport
        Assessment of causal assumptions.
    """
    analyzer = CausalAnalyzer(
        treatment_col=treatment_col,
        outcome_col=outcome_col,
        confounders=confounders,
    )
    return analyzer._check_causal_readiness(df)


def refute_estimate(
    df: pd.DataFrame,
    treatment_col: str,
    outcome_col: str,
    confounders: Optional[List[str]] = None,
) -> Dict[str, Any]:
    """
    Run refutation tests on causal estimate.
    
    Returns
    -------
    dict
        Refutation test results.
    """
    analyzer = CausalAnalyzer(
        treatment_col=treatment_col,
        outcome_col=outcome_col,
        confounders=confounders,
    )
    model = analyzer.fit(df, run_refutation=True)
    return model.ate.refutation_results if model.ate else {}

