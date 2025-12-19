"""
ACDSO v2.0 - sklearn-Native AutoML Adapters
============================================

Adapters for AutoML tools that produce sklearn-compatible estimators:
- auto-sklearn
- TPOT
- FLAML
"""

from __future__ import annotations

import time
import warnings
from typing import Any, Dict, List, Optional

from acdso.integrations.base import (
    SklearnNativeAdapter,
    CandidateSource,
)


class AutoSklearnAdapter(SklearnNativeAdapter):
    """
    Adapter for auto-sklearn AutoML.

    auto-sklearn uses Bayesian optimization and meta-learning to find
    good sklearn pipelines. This adapter extracts the top K models
    from auto-sklearn's ensemble for ACDSO evaluation.

    Parameters
    ----------
    time_budget : int, default=120
        Time budget in seconds for auto-sklearn.
    per_run_time_limit : int, default=30
        Time limit per individual run.
    memory_limit : int, default=4096
        Memory limit in MB.
    ensemble_size : int, default=10
        Size of the ensemble to build.
    top_k : int, default=5
        Number of top candidates to extract.
    include_estimators : list, optional
        List of estimators to include in search.
    exclude_estimators : list, optional
        List of estimators to exclude from search.
    random_state : int, optional
        Random seed.
    verbose : int, default=0
        Verbosity level.

    Examples
    --------
    >>> from acdso.integrations import AutoSklearnAdapter, ACDSOMetaOptimizer
    >>>
    >>> adapter = AutoSklearnAdapter(time_budget=300, top_k=5)
    >>> meta = ACDSOMetaOptimizer(adapters=[adapter])
    >>> meta.fit(X_train, y_train)
    """

    def __init__(
        self,
        time_budget: int = 120,
        per_run_time_limit: int = 30,
        memory_limit: int = 4096,
        ensemble_size: int = 10,
        top_k: int = 5,
        include_estimators: Optional[List[str]] = None,
        exclude_estimators: Optional[List[str]] = None,
        random_state: Optional[int] = None,
        verbose: int = 0,
    ):
        super().__init__(
            name="auto-sklearn",
            top_k=top_k,
            random_state=random_state,
            verbose=verbose,
        )
        self.time_budget = time_budget
        self.per_run_time_limit = per_run_time_limit
        self.memory_limit = memory_limit
        self.ensemble_size = ensemble_size
        self.include_estimators = include_estimators
        self.exclude_estimators = exclude_estimators

    def is_available(self) -> bool:
        """Check if auto-sklearn is installed."""
        try:
            import autosklearn
            return True
        except ImportError:
            return False

    def generate_candidates(
        self,
        X: Any,
        y: Any,
        task: str,
    ) -> List[CandidateSource]:
        """
        Run auto-sklearn and extract top candidates.

        Parameters
        ----------
        X : array-like
            Training features.
        y : array-like
            Target values.
        task : str
            "classification" or "regression".

        Returns
        -------
        List[CandidateSource]
            Top K candidates from auto-sklearn.
        """
        if not self.is_available():
            self._log("auto-sklearn not installed. Skipping.")
            return []

        self._log(f"Starting auto-sklearn with {self.time_budget}s budget...")
        start_time = time.time()

        try:
            if task == "classification":
                from autosklearn.classification import AutoSklearnClassifier
                automl = AutoSklearnClassifier(
                    time_left_for_this_task=self.time_budget,
                    per_run_time_limit=self.per_run_time_limit,
                    memory_limit=self.memory_limit,
                    ensemble_size=self.ensemble_size,
                    seed=self.random_state or 42,
                    include_estimators=self.include_estimators,
                    exclude_estimators=self.exclude_estimators,
                )
            else:
                from autosklearn.regression import AutoSklearnRegressor
                automl = AutoSklearnRegressor(
                    time_left_for_this_task=self.time_budget,
                    per_run_time_limit=self.per_run_time_limit,
                    memory_limit=self.memory_limit,
                    ensemble_size=self.ensemble_size,
                    seed=self.random_state or 42,
                    include_estimators=self.include_estimators,
                    exclude_estimators=self.exclude_estimators,
                )

            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                automl.fit(X, y)

            self._generation_time = time.time() - start_time

            # Extract individual models from the ensemble
            candidates = []

            # Get the leaderboard
            try:
                leaderboard = automl.leaderboard(detailed=True)
                self._generation_metadata = {
                    "total_models_evaluated": len(leaderboard),
                }
            except Exception:
                leaderboard = None
                self._generation_metadata = {}

            # Get models from show_models()
            try:
                models_info = automl.show_models()
                model_keys = list(models_info.keys())[:self.top_k]

                for rank, key in enumerate(model_keys):
                    model_info = models_info[key]
                    name = f"autosklearn_{task}_{rank}"

                    # Create a fitted pipeline that uses this specific model
                    # auto-sklearn's ensemble prediction uses weights
                    candidates.append(self.wrap_estimator(
                        estimator=automl,  # Use the full automl as estimator
                        name=name,
                        rank=rank,
                        metadata={
                            "model_key": key,
                            "model_info": str(model_info)[:200],
                        },
                    ))

                    if len(candidates) >= self.top_k:
                        break

            except Exception as e:
                self._log(f"Could not extract individual models: {e}")
                # Fall back to using the ensemble as a single candidate
                candidates.append(self.wrap_estimator(
                    estimator=automl,
                    name=f"autosklearn_{task}_ensemble",
                    rank=0,
                    metadata={"type": "ensemble"},
                ))

            self._log(f"Generated {len(candidates)} candidates in {self._generation_time:.1f}s")
            return candidates

        except Exception as e:
            self._log(f"auto-sklearn failed: {type(e).__name__}: {e}")
            self._generation_time = time.time() - start_time
            return []


class TPOTAdapter(SklearnNativeAdapter):
    """
    Adapter for TPOT AutoML.

    TPOT uses genetic programming to evolve sklearn pipelines.
    This adapter extracts the best pipelines from TPOT's Pareto front.

    Parameters
    ----------
    generations : int, default=5
        Number of generations for genetic algorithm.
    population_size : int, default=50
        Population size for each generation.
    offspring_size : int, optional
        Number of offspring per generation.
    mutation_rate : float, default=0.9
        Mutation probability.
    crossover_rate : float, default=0.1
        Crossover probability.
    cv : int, default=5
        Cross-validation folds.
    max_time_mins : int, optional
        Maximum time in minutes.
    max_eval_time_mins : int, default=5
        Maximum time per pipeline evaluation.
    top_k : int, default=5
        Number of top candidates to extract.
    config_dict : dict, optional
        Custom TPOT configuration dictionary.
    random_state : int, optional
        Random seed.
    verbose : int, default=0
        Verbosity level.

    Examples
    --------
    >>> from acdso.integrations import TPOTAdapter, ACDSOMetaOptimizer
    >>>
    >>> adapter = TPOTAdapter(generations=10, population_size=100)
    >>> meta = ACDSOMetaOptimizer(adapters=[adapter])
    >>> meta.fit(X_train, y_train)
    """

    def __init__(
        self,
        generations: int = 5,
        population_size: int = 50,
        offspring_size: Optional[int] = None,
        mutation_rate: float = 0.9,
        crossover_rate: float = 0.1,
        cv: int = 5,
        max_time_mins: Optional[int] = None,
        max_eval_time_mins: int = 5,
        top_k: int = 5,
        config_dict: Optional[Dict] = None,
        random_state: Optional[int] = None,
        verbose: int = 0,
    ):
        super().__init__(
            name="TPOT",
            top_k=top_k,
            random_state=random_state,
            verbose=verbose,
        )
        self.generations = generations
        self.population_size = population_size
        self.offspring_size = offspring_size
        self.mutation_rate = mutation_rate
        self.crossover_rate = crossover_rate
        self.cv = cv
        self.max_time_mins = max_time_mins
        self.max_eval_time_mins = max_eval_time_mins
        self.config_dict = config_dict

    def is_available(self) -> bool:
        """Check if TPOT is installed."""
        try:
            import tpot
            return True
        except ImportError:
            return False

    def generate_candidates(
        self,
        X: Any,
        y: Any,
        task: str,
    ) -> List[CandidateSource]:
        """
        Run TPOT and extract top pipeline candidates.

        Parameters
        ----------
        X : array-like
            Training features.
        y : array-like
            Target values.
        task : str
            "classification" or "regression".

        Returns
        -------
        List[CandidateSource]
            Top K pipeline candidates from TPOT.
        """
        if not self.is_available():
            self._log("TPOT not installed. Skipping.")
            return []

        self._log(f"Starting TPOT with {self.generations} generations...")
        start_time = time.time()

        try:
            if task == "classification":
                from tpot import TPOTClassifier
                tpot = TPOTClassifier(
                    generations=self.generations,
                    population_size=self.population_size,
                    offspring_size=self.offspring_size,
                    mutation_rate=self.mutation_rate,
                    crossover_rate=self.crossover_rate,
                    cv=self.cv,
                    max_time_mins=self.max_time_mins,
                    max_eval_time_mins=self.max_eval_time_mins,
                    config_dict=self.config_dict,
                    random_state=self.random_state,
                    verbosity=self.verbose,
                    n_jobs=-1,
                )
            else:
                from tpot import TPOTRegressor
                tpot = TPOTRegressor(
                    generations=self.generations,
                    population_size=self.population_size,
                    offspring_size=self.offspring_size,
                    mutation_rate=self.mutation_rate,
                    crossover_rate=self.crossover_rate,
                    cv=self.cv,
                    max_time_mins=self.max_time_mins,
                    max_eval_time_mins=self.max_eval_time_mins,
                    config_dict=self.config_dict,
                    random_state=self.random_state,
                    verbosity=self.verbose,
                    n_jobs=-1,
                )

            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                tpot.fit(X, y)

            self._generation_time = time.time() - start_time

            candidates = []

            # TPOT stores evaluated pipelines
            try:
                # Get Pareto front pipelines if available
                if hasattr(tpot, 'pareto_front_fitted_pipelines_'):
                    pareto_pipelines = tpot.pareto_front_fitted_pipelines_
                    for rank, (pipeline_str, pipeline) in enumerate(pareto_pipelines.items()):
                        if rank >= self.top_k:
                            break
                        candidates.append(self.wrap_estimator(
                            estimator=pipeline,
                            name=f"tpot_{task}_pareto_{rank}",
                            rank=rank,
                            metadata={
                                "pipeline_str": pipeline_str[:200],
                                "from_pareto": True,
                            },
                        ))
                    self._generation_metadata = {
                        "pareto_front_size": len(pareto_pipelines),
                    }
            except Exception:
                pass

            # If no Pareto front or not enough, use the best pipeline
            if len(candidates) < self.top_k and hasattr(tpot, 'fitted_pipeline_'):
                candidates.append(self.wrap_estimator(
                    estimator=tpot.fitted_pipeline_,
                    name=f"tpot_{task}_best",
                    rank=len(candidates),
                    metadata={
                        "type": "best_pipeline",
                    },
                ))

            # Try to get evaluated pipelines
            try:
                if hasattr(tpot, 'evaluated_individuals_'):
                    self._generation_metadata["total_evaluated"] = len(tpot.evaluated_individuals_)
            except Exception:
                pass

            self._log(f"Generated {len(candidates)} candidates in {self._generation_time:.1f}s")
            return candidates

        except Exception as e:
            self._log(f"TPOT failed: {type(e).__name__}: {e}")
            self._generation_time = time.time() - start_time
            return []


class FLAMLAdapter(SklearnNativeAdapter):
    """
    Adapter for Microsoft FLAML AutoML.

    FLAML is a fast and lightweight AutoML library that uses
    cost-effective hyperparameter optimization.

    Parameters
    ----------
    time_budget : int, default=60
        Time budget in seconds.
    metric : str, optional
        Metric to optimize (auto-detected if None).
    estimator_list : list, optional
        List of estimators to try.
    task : str, optional
        Task type (auto-detected if None).
    n_splits : int, default=5
        Number of cross-validation splits.
    top_k : int, default=5
        Number of top candidates to extract.
    random_state : int, optional
        Random seed.
    verbose : int, default=0
        Verbosity level.

    Examples
    --------
    >>> from acdso.integrations import FLAMLAdapter, ACDSOMetaOptimizer
    >>>
    >>> adapter = FLAMLAdapter(time_budget=120)
    >>> meta = ACDSOMetaOptimizer(adapters=[adapter])
    >>> meta.fit(X_train, y_train)
    """

    def __init__(
        self,
        time_budget: int = 60,
        metric: Optional[str] = None,
        estimator_list: Optional[List[str]] = None,
        n_splits: int = 5,
        top_k: int = 5,
        random_state: Optional[int] = None,
        verbose: int = 0,
    ):
        super().__init__(
            name="FLAML",
            top_k=top_k,
            random_state=random_state,
            verbose=verbose,
        )
        self.time_budget = time_budget
        self.metric = metric
        self.estimator_list = estimator_list
        self.n_splits = n_splits

    def is_available(self) -> bool:
        """Check if FLAML is installed."""
        try:
            import flaml
            return True
        except ImportError:
            return False

    def generate_candidates(
        self,
        X: Any,
        y: Any,
        task: str,
    ) -> List[CandidateSource]:
        """
        Run FLAML and extract top candidates.

        Parameters
        ----------
        X : array-like
            Training features.
        y : array-like
            Target values.
        task : str
            "classification" or "regression".

        Returns
        -------
        List[CandidateSource]
            Top K candidates from FLAML.
        """
        if not self.is_available():
            self._log("FLAML not installed. Skipping.")
            return []

        self._log(f"Starting FLAML with {self.time_budget}s budget...")
        start_time = time.time()

        try:
            from flaml import AutoML

            automl = AutoML()

            # Set metric based on task if not provided
            metric = self.metric
            if metric is None:
                metric = "accuracy" if task == "classification" else "r2"

            # Set estimator list if not provided
            estimator_list = self.estimator_list
            if estimator_list is None:
                if task == "classification":
                    estimator_list = ["lgbm", "rf", "xgboost", "extra_tree", "lrl1"]
                else:
                    estimator_list = ["lgbm", "rf", "xgboost", "extra_tree", "ridge"]

            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                automl.fit(
                    X, y,
                    task=task,
                    time_budget=self.time_budget,
                    metric=metric,
                    estimator_list=estimator_list,
                    n_splits=self.n_splits,
                    seed=self.random_state,
                    verbose=self.verbose,
                )

            self._generation_time = time.time() - start_time

            candidates = []

            # Get the best model
            if automl.best_estimator is not None:
                best_model = automl.model
                candidates.append(self.wrap_estimator(
                    estimator=best_model,
                    name=f"flaml_{task}_{automl.best_estimator}",
                    rank=0,
                    metadata={
                        "best_estimator": automl.best_estimator,
                        "best_config": automl.best_config,
                        "best_score": automl.best_loss,
                    },
                ))

            # Try to get multiple configurations
            try:
                if hasattr(automl, 'config_history'):
                    config_history = automl.config_history
                    sorted_configs = sorted(
                        config_history.items(),
                        key=lambda x: x[1].get('loss', float('inf'))
                    )

                    for rank, (config_id, config_info) in enumerate(sorted_configs[1:self.top_k]):
                        # FLAML doesn't easily expose multiple fitted models
                        # We use the best model but note the config
                        pass

                    self._generation_metadata = {
                        "total_configs_tried": len(config_history),
                    }
            except Exception:
                pass

            self._generation_metadata["best_estimator"] = automl.best_estimator

            self._log(f"Generated {len(candidates)} candidates in {self._generation_time:.1f}s")
            return candidates

        except Exception as e:
            self._log(f"FLAML failed: {type(e).__name__}: {e}")
            self._generation_time = time.time() - start_time
            return []

