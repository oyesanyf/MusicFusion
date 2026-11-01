# ACDSO Demo.py Coverage Analysis

## Executive Summary

`demo.py` currently has **23 demos** covering most core functionality, but there are **several gaps** in comprehensive module evaluation. This document identifies what's covered and what's missing.

---

## ✅ Currently Covered Features

### Core ACDSO (9 demos)
- ✅ Basic Classification
- ✅ Production Constraints
- ✅ Risk Analysis
- ✅ Pareto Front Exploration
- ✅ Regression
- ✅ Audit Trail
- ✅ LLM Advisor
- ✅ Real Dataset
- ✅ Complete ML Workflow

### Auto EDA (5 demos)
- ✅ Basic EDA
- ✅ Quality-focused EDA
- ✅ Pareto-optimized EDA
- ✅ LLM-powered EDA
- ✅ EDA Export

### AutoML Integration (2 demos)
- ✅ AutoML Integration (general)
- ✅ FLAML Integration

### EDA Utilities (2 demos)
- ✅ EDA Tool Integrations
- ✅ EDA Utility Functions

### Advanced Features (3 demos)
- ✅ Safe Feature Importance
- ✅ AutoEDA Preprocessing
- ✅ Benchmark Comparison

### New Modules (3 demos)
- ✅ Auto Time Series (basic)
- ✅ Decision Intelligence (full pipeline)
- ✅ Policy Learning (RLlib)

---

## ❌ Missing Coverage

### Time Series Module Gaps

#### 1. **CV Split Types** (Partially Covered)
- ✅ ExpandingWindowSplit (used in AutoTS)
- ❌ RollingWindowSplit (not demonstrated)
- ❌ WalkForwardSplit (not demonstrated)
- ❌ `create_time_series_cv()` helper (not shown)

**Impact**: Users may not know about alternative CV strategies for different scenarios.

#### 2. **Transformers** (Partially Covered)
- ✅ LagFeatureTransformer (basic demo)
- ✅ RollingWindowTransformer (basic demo)
- ✅ DateTimeFeatureTransformer (imported but not used)
- ❌ SeasonalDecomposer (not demonstrated)
- ❌ DifferencingTransformer (not demonstrated)
- ❌ TimeSeriesFeatureUnion (not demonstrated)
- ❌ `create_ts_feature_pipeline()` helper (not shown)

**Impact**: Advanced feature engineering capabilities are hidden.

#### 3. **Metrics** (Partially Covered)
- ✅ MAE, RMSE, MAPE, MASE (used)
- ❌ sMAPE (symmetric MAPE) - not demonstrated
- ❌ WAPE (Weighted APE) - not demonstrated
- ❌ `evaluate_forecast()` function - not shown
- ❌ `compare_forecasts()` function - not shown
- ❌ `get_ts_scorer()` helper - not shown

**Impact**: Users may not know about additional evaluation metrics.

#### 4. **Forecasters** (Partially Covered)
- ✅ NaiveForecaster
- ✅ SeasonalNaiveForecaster
- ❌ MovingAverageForecaster (not demonstrated)
- ❌ ExponentialSmoothingForecaster (not demonstrated)

**Impact**: Baseline model variety not fully showcased.

#### 5. **Time Series Benchmarking** (Missing)
- ❌ Benchmarking against Prophet
- ❌ Benchmarking against AutoTS
- ❌ Benchmarking against StatsForecast
- ❌ Benchmarking against Darts
- ❌ Benchmarking against MLForecast
- ❌ `TimeSeriesBenchmark` class usage (mentioned but not fully demonstrated)
- ❌ `run_ts_benchmark()` function (not shown)

**Impact**: Users can't see how ACDSO compares to other time series tools.

#### 6. **Risk Analysis Details** (Basic Coverage)
- ✅ Basic risk report (frequency, seasonality, trend)
- ❌ `detect_frequency()` standalone function
- ❌ `detect_seasonality()` standalone function
- ❌ `check_stationarity()` standalone function
- ❌ `detect_outliers()` standalone function
- ❌ `detect_trend()` standalone function

**Impact**: Standalone diagnostic functions not demonstrated.

#### 7. **Advanced AutoTS Features** (Missing)
- ❌ Different `cv_type` options (only 'expanding' shown)
- ❌ Exogenous variables usage
- ❌ Multi-step forecasting
- ❌ Forecast intervals/uncertainty
- ❌ `auto_forecast()` convenience function

---

### Decision Intelligence Module Gaps

#### 1. **Causal Analysis Components** (Partially Covered)
- ✅ CausalAnalyzer (full pipeline)
- ✅ CausalReadinessReport (shown)
- ❌ `estimate_ate()` standalone function
- ❌ `estimate_att()` standalone function
- ❌ `check_causal_assumptions()` standalone function
- ❌ `refute_estimate()` function (refutation tests not run)
- ❌ Causal refutation tests (placebo, random common cause, etc.)

**Impact**: Advanced causal validation not demonstrated.

#### 2. **Uplift Modeling Components** (Partially Covered)
- ✅ UpliftModeler (full pipeline)
- ✅ TLearner, SLearner, XLearner (used internally)
- ❌ DRLearner (not demonstrated separately)
- ❌ CausalForestWrapper (not demonstrated)
- ❌ `qini_score()` standalone function
- ❌ `auuc_score()` standalone function
- ❌ `uplift_curve()` function (visualization)
- ❌ `get_uplift_candidates()` helper

**Impact**: Individual uplift model usage and metrics not shown.

#### 3. **Optimization Constraints** (Partially Covered)
- ✅ BudgetConstraint (used)
- ✅ CapacityConstraint (used)
- ❌ FairnessConstraint (not demonstrated)
- ❌ `knapsack_optimize()` standalone function
- ❌ `ilp_optimize()` standalone function
- ❌ `optimize_actions()` convenience function

**Impact**: Fairness constraints and optimization alternatives not shown.

#### 4. **Policy Learning** (Basic Coverage)
- ✅ ContextualBanditEnv (used)
- ❌ SequentialDecisionEnv (not demonstrated)
- ❌ `train_policy()` standalone function
- ❌ `evaluate_policy()` standalone function
- ❌ `get_policy_action()` function
- ❌ Different RL algorithms (only PPO shown)

**Impact**: Sequential decision-making and policy utilities not shown.

#### 5. **Decision Intelligence Utilities** (Missing)
- ❌ `auto_decision()` convenience function
- ❌ DecisionAuditTrail detailed usage
- ❌ Integration with ACDSO AutoML results

---

### Integration & Cross-Module Gaps

#### 1. **Module Integration** (Missing)
- ❌ Using AutoTS results in Decision Intelligence
- ❌ Using AutoML predictions for uplift modeling
- ❌ Time series forecasting for decision optimization
- ❌ EDA insights informing time series preprocessing

#### 2. **Preference Modes** (Partially Covered)
- ✅ "safe" preference (mentioned)
- ❌ "fast" preference demo
- ❌ "balanced" preference demo
- ❌ Comparison of different preferences

#### 3. **Export/Import** (Partially Covered)
- ✅ EDA export (HTML, JSON)
- ❌ Model serialization/loading
- ❌ Result persistence
- ❌ Audit trail export formats

#### 4. **Error Handling & Edge Cases** (Missing)
- ❌ Missing data handling
- ❌ Invalid input validation
- ❌ Constraint violation scenarios
- ❌ Timeout/budget exhaustion
- ❌ Memory limit scenarios

#### 5. **Performance Testing** (Missing)
- ❌ Large dataset handling
- ❌ Stress testing
- ❌ Memory profiling
- ❌ Latency benchmarking

---

## 📊 Coverage Statistics

| Module | Total Features | Covered | Missing | Coverage % |
|--------|---------------|---------|---------|------------|
| **Core ACDSO** | ~15 | 9 | 6 | 60% |
| **Auto EDA** | ~12 | 5 | 7 | 42% |
| **Time Series** | ~25 | 8 | 17 | 32% |
| **Decision Intelligence** | ~30 | 10 | 20 | 33% |
| **Integrations** | ~8 | 2 | 6 | 25% |
| **Utilities** | ~10 | 3 | 7 | 30% |
| **Overall** | ~100 | 37 | 63 | **37%** |

---

## 🎯 Recommended Additional Demos

### High Priority (Core Functionality)

1. **`demo_timeseries_cv_splits()`**
   - Demonstrate ExpandingWindowSplit, RollingWindowSplit, WalkForwardSplit
   - Show when to use each

2. **`demo_timeseries_transformers()`**
   - Show all transformers: SeasonalDecomposer, DifferencingTransformer, DateTimeFeatureTransformer
   - Demonstrate feature engineering pipeline

3. **`demo_timeseries_benchmark()`**
   - Benchmark ACDSO vs Prophet, AutoTS, StatsForecast, Darts
   - Show comparison report

4. **`demo_causal_refutation()`**
   - Run refutation tests (placebo, random common cause)
   - Show robustness checks

5. **`demo_uplift_individual_models()`**
   - Show TLearner, SLearner, XLearner, DRLearner separately
   - Compare performance

6. **`demo_optimization_constraints()`**
   - Show FairnessConstraint
   - Demonstrate multiple constraint types together

### Medium Priority (Advanced Features)

7. **`demo_timeseries_metrics()`**
   - Show all metrics: sMAPE, WAPE, evaluate_forecast, compare_forecasts

8. **`demo_decision_standalone_functions()`**
   - estimate_ate(), estimate_att(), qini_score(), etc.

9. **`demo_policy_sequential()`**
   - SequentialDecisionEnv
   - Different RL algorithms

10. **`demo_integration_workflows()`**
    - AutoTS → Decision Intelligence
    - AutoML → Uplift Modeling
    - EDA → Time Series preprocessing

### Low Priority (Edge Cases & Utilities)

11. **`demo_error_handling()`**
    - Missing data, invalid inputs, constraint violations

12. **`demo_preference_modes()`**
    - Compare "safe", "fast", "balanced"

13. **`demo_export_import()`**
    - Model serialization, result persistence

14. **`demo_performance_stress()`**
    - Large datasets, memory limits, timeouts

---

## 🔧 Implementation Recommendations

### Quick Wins (Add to existing demos)

1. **Enhance `demo_auto_timeseries()`:**
   - Add DateTimeFeatureTransformer usage
   - Show different CV split types
   - Add sMAPE and WAPE metrics
   - Include MovingAverageForecaster and ExponentialSmoothingForecaster

2. **Enhance `demo_decision_intelligence()`:**
   - Run refutation tests
   - Show FairnessConstraint
   - Demonstrate standalone functions (estimate_ate, qini_score)

3. **Enhance `demo_policy_learning()`:**
   - Show SequentialDecisionEnv
   - Compare different RL algorithms

### New Dedicated Demos

Create focused demos for:
- Time Series benchmarking
- Causal refutation tests
- Individual uplift models
- Optimization constraints
- Module integration workflows

---

## ✅ Conclusion

**Current State**: `demo.py` provides good coverage of **core workflows** but lacks **comprehensive feature coverage** (~37% overall).

**Recommendation**: Add **10-15 additional demos** to reach ~70% coverage, focusing on:
1. Time Series advanced features (benchmarking, transformers, metrics)
2. Decision Intelligence components (refutation, individual models, constraints)
3. Module integration workflows
4. Edge cases and error handling

This will make `demo.py` a comprehensive evaluation suite for the entire ACDSO module.

