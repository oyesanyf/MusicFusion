"""
ACDSO v2.0 - External AutoML Adapters
======================================

Adapters for non-sklearn AutoML tools that require model wrapping:
- H2O AutoML
- AutoGluon
- Generic wrapper for any model
"""

from __future__ import annotations

import time
import warnings
from typing import Any, Callable, Dict, List, Optional

import numpy as np
import pandas as pd

from acdso.integrations.base import (
    ExternalAutoMLAdapter,
    ExternalModelWrapper,
    CandidateSource,
)


class GenericModelWrapper(ExternalModelWrapper):
    """
    Generic wrapper for any pre-trained model.

    Use this to wrap models from any source that aren't covered
    by the specific adapters.

    Parameters
    ----------
    model : Any
        The model object.
    predict_method : str, default="predict"
        Name of the predict method.
    predict_proba_method : str, optional
        Name of the predict_proba method.
    preprocess_fn : callable, optional
        Function to preprocess X before prediction.
    postprocess_fn : callable, optional
        Function to postprocess predictions.
    name : str, default="GenericModel"
        Name identifier.
    source : str, default="generic"
        Source identifier.

    Examples
    --------
    >>> # Wrap a custom model
    >>> wrapper = GenericModelWrapper(
    ...     model=my_custom_model,
    ...     predict_method="make_prediction",
    ...     name="CustomModel",
    ... )
    >>> wrapper.predict(X)
    """

    def __init__(
        self,
        model: Any,
        predict_method: str = "predict",
        predict_proba_method: Optional[str] = None,
        preprocess_fn: Optional[Callable] = None,
        postprocess_fn: Optional[Callable] = None,
        name: str = "GenericModel",
        source: str = "generic",
        metadata: Optional[Dict[str, Any]] = None,
    ):
        self._model = model
        self._predict_method = predict_method
        self._predict_proba_method = predict_proba_method
        self._preprocess_fn = preprocess_fn
        self._postprocess_fn = postprocess_fn

        def predict_fn(X):
            if self._preprocess_fn:
                X = self._preprocess_fn(X)
            result = getattr(self._model, self._predict_method)(X)
            if self._postprocess_fn:
                result = self._postprocess_fn(result)
            return result

        predict_proba_fn = None
        if predict_proba_method:
            def predict_proba_fn(X):
                if self._preprocess_fn:
                    X = self._preprocess_fn(X)
                return getattr(self._model, self._predict_proba_method)(X)

        super().__init__(
            predict_fn=predict_fn,
            predict_proba_fn=predict_proba_fn,
            name=name,
            source=source,
            metadata=metadata,
        )


class H2OAdapter(ExternalAutoMLAdapter):
    """
    Adapter for H2O AutoML.

    H2O is a distributed machine learning platform with a powerful
    AutoML capability. This adapter runs H2O AutoML and wraps the
    top models for ACDSO evaluation.

    Parameters
    ----------
    max_runtime_secs : int, default=120
        Maximum runtime in seconds.
    max_models : int, default=20
        Maximum number of models to train.
    nfolds : int, default=5
        Number of cross-validation folds.
    seed : int, default=42
        Random seed.
    include_algos : list, optional
        Algorithms to include (e.g., ["GBM", "RF", "XGBoost"]).
    exclude_algos : list, optional
        Algorithms to exclude.
    top_k : int, default=5
        Number of top candidates to extract.
    verbose : int, default=0
        Verbosity level.

    Notes
    -----
    Requires H2O to be installed and a cluster to be running.
    The adapter will attempt to initialize a local cluster if none exists.

    Examples
    --------
    >>> from acdso.integrations import H2OAdapter, ACDSOMetaOptimizer
    >>>
    >>> adapter = H2OAdapter(max_runtime_secs=300, max_models=30)
    >>> meta = ACDSOMetaOptimizer(adapters=[adapter])
    >>> meta.fit(X_train, y_train)
    """

    def __init__(
        self,
        max_runtime_secs: int = 120,
        max_models: int = 20,
        nfolds: int = 5,
        seed: int = 42,
        include_algos: Optional[List[str]] = None,
        exclude_algos: Optional[List[str]] = None,
        top_k: int = 5,
        verbose: int = 0,
    ):
        super().__init__(
            name="H2O",
            top_k=top_k,
            random_state=seed,
            verbose=verbose,
        )
        self.max_runtime_secs = max_runtime_secs
        self.max_models = max_models
        self.nfolds = nfolds
        self.seed = seed
        self.include_algos = include_algos
        self.exclude_algos = exclude_algos

    def is_available(self) -> bool:
        """Check if H2O is installed."""
        try:
            import h2o
            return True
        except ImportError:
            return False

    def _ensure_cluster(self):
        """Ensure H2O cluster is running."""
        import h2o
        try:
            h2o.cluster()
        except Exception:
            self._log("Initializing H2O cluster...")
            h2o.init(nthreads=-1, max_mem_size="4G")

    def generate_candidates(
        self,
        X: Any,
        y: Any,
        task: str,
    ) -> List[CandidateSource]:
        """
        Run H2O AutoML and extract top candidates.

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
            Top K candidates from H2O, wrapped for sklearn compatibility.
        """
        if not self.is_available():
            self._log("H2O not installed. Skipping.")
            return []

        self._log(f"Starting H2O AutoML with {self.max_runtime_secs}s budget...")
        start_time = time.time()

        try:
            import h2o
            from h2o.automl import H2OAutoML

            self._ensure_cluster()

            # Convert to H2O frame
            if isinstance(X, pd.DataFrame):
                df = X.copy()
            else:
                df = pd.DataFrame(X)

            df['__target__'] = y
            h2o_frame = h2o.H2OFrame(df)

            # Set target column type
            if task == "classification":
                h2o_frame['__target__'] = h2o_frame['__target__'].asfactor()

            # Run AutoML
            aml = H2OAutoML(
                max_runtime_secs=self.max_runtime_secs,
                max_models=self.max_models,
                nfolds=self.nfolds,
                seed=self.seed,
                include_algos=self.include_algos,
                exclude_algos=self.exclude_algos,
                verbosity="warn" if self.verbose else "error",
            )

            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                aml.train(
                    y='__target__',
                    training_frame=h2o_frame,
                )

            self._generation_time = time.time() - start_time

            # Get leaderboard
            leaderboard = aml.leaderboard.as_data_frame()
            self._generation_metadata = {
                "total_models": len(leaderboard),
            }

            candidates = []

            for rank in range(min(self.top_k, len(leaderboard))):
                model_id = leaderboard.iloc[rank]['model_id']
                model = h2o.get_model(model_id)

                # Create prediction functions
                def make_predict_fn(m, is_clf):
                    def predict_fn(X_input):
                        if isinstance(X_input, pd.DataFrame):
                            h2o_input = h2o.H2OFrame(X_input)
                        else:
                            h2o_input = h2o.H2OFrame(pd.DataFrame(X_input))
                        preds = m.predict(h2o_input).as_data_frame()
                        if is_clf:
                            return preds['predict'].values
                        else:
                            return preds['predict'].values
                    return predict_fn

                def make_predict_proba_fn(m):
                    def predict_proba_fn(X_input):
                        if isinstance(X_input, pd.DataFrame):
                            h2o_input = h2o.H2OFrame(X_input)
                        else:
                            h2o_input = h2o.H2OFrame(pd.DataFrame(X_input))
                        preds = m.predict(h2o_input).as_data_frame()
                        # Get probability columns (p0, p1, etc.)
                        prob_cols = [c for c in preds.columns if c.startswith('p')]
                        if prob_cols:
                            return preds[prob_cols].values
                        return None
                    return predict_proba_fn

                candidates.append(self.wrap_external_model(
                    predict_fn=make_predict_fn(model, task == "classification"),
                    name=f"h2o_{model_id}",
                    rank=rank,
                    predict_proba_fn=make_predict_proba_fn(model) if task == "classification" else None,
                    metadata={
                        "model_id": model_id,
                        "algo": model.algo,
                    },
                ))

            self._log(f"Generated {len(candidates)} candidates in {self._generation_time:.1f}s")
            return candidates

        except Exception as e:
            self._log(f"H2O failed: {type(e).__name__}: {e}")
            self._generation_time = time.time() - start_time
            return []


class AutoGluonAdapter(ExternalAutoMLAdapter):
    """
    Adapter for AutoGluon AutoML.

    AutoGluon is an AutoML toolkit that automates machine learning tasks
    with state-of-the-art techniques including multi-layer stacking.

    Parameters
    ----------
    time_limit : int, default=120
        Time limit in seconds.
    presets : str, default="medium_quality"
        Quality preset ("best_quality", "high_quality", "good_quality",
        "medium_quality", "optimize_for_deployment").
    eval_metric : str, optional
        Metric to optimize (auto-detected if None).
    hyperparameters : dict, optional
        Custom hyperparameters for models.
    top_k : int, default=5
        Number of top candidates to extract.
    verbose : int, default=0
        Verbosity level.

    Examples
    --------
    >>> from acdso.integrations import AutoGluonAdapter, ACDSOMetaOptimizer
    >>>
    >>> adapter = AutoGluonAdapter(time_limit=300, presets="best_quality")
    >>> meta = ACDSOMetaOptimizer(adapters=[adapter])
    >>> meta.fit(X_train, y_train)
    """

    def __init__(
        self,
        time_limit: int = 120,
        presets: str = "medium_quality",
        eval_metric: Optional[str] = None,
        hyperparameters: Optional[Dict] = None,
        top_k: int = 5,
        random_state: Optional[int] = None,
        verbose: int = 0,
    ):
        super().__init__(
            name="AutoGluon",
            top_k=top_k,
            random_state=random_state,
            verbose=verbose,
        )
        self.time_limit = time_limit
        self.presets = presets
        self.eval_metric = eval_metric
        self.hyperparameters = hyperparameters

    def is_available(self) -> bool:
        """Check if AutoGluon is installed."""
        try:
            from autogluon.tabular import TabularPredictor
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
        Run AutoGluon and extract top candidates.

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
            Top K candidates from AutoGluon, wrapped for sklearn compatibility.
        """
        if not self.is_available():
            self._log("AutoGluon not installed. Skipping.")
            return []

        self._log(f"Starting AutoGluon with {self.time_limit}s budget...")
        start_time = time.time()

        try:
            from autogluon.tabular import TabularPredictor
            import tempfile
            import shutil

            # Prepare data
            if isinstance(X, pd.DataFrame):
                train_data = X.copy()
            else:
                train_data = pd.DataFrame(X)
            train_data['__target__'] = y

            # Create temporary directory for AutoGluon output
            temp_dir = tempfile.mkdtemp(prefix="autogluon_")

            try:
                # Determine problem type
                problem_type = "multiclass" if task == "classification" else "regression"
                if task == "classification":
                    unique_classes = len(np.unique(y))
                    if unique_classes == 2:
                        problem_type = "binary"

                # Set eval metric
                eval_metric = self.eval_metric
                if eval_metric is None:
                    eval_metric = "accuracy" if task == "classification" else "r2"

                # Run AutoGluon
                predictor = TabularPredictor(
                    label='__target__',
                    problem_type=problem_type,
                    eval_metric=eval_metric,
                    path=temp_dir,
                    verbosity=self.verbose,
                )

                with warnings.catch_warnings():
                    warnings.simplefilter("ignore")
                    predictor.fit(
                        train_data,
                        time_limit=self.time_limit,
                        presets=self.presets,
                        hyperparameters=self.hyperparameters,
                    )

                self._generation_time = time.time() - start_time

                # Get leaderboard
                leaderboard = predictor.leaderboard(silent=True)
                self._generation_metadata = {
                    "total_models": len(leaderboard),
                    "presets": self.presets,
                }

                candidates = []

                for rank in range(min(self.top_k, len(leaderboard))):
                    model_name = leaderboard.iloc[rank]['model']

                    # Create prediction functions
                    def make_predict_fn(pred, model):
                        def predict_fn(X_input):
                            if isinstance(X_input, pd.DataFrame):
                                return pred.predict(X_input, model=model).values
                            else:
                                return pred.predict(pd.DataFrame(X_input), model=model).values
                        return predict_fn

                    def make_predict_proba_fn(pred, model):
                        def predict_proba_fn(X_input):
                            if isinstance(X_input, pd.DataFrame):
                                return pred.predict_proba(X_input, model=model).values
                            else:
                                return pred.predict_proba(pd.DataFrame(X_input), model=model).values
                        return predict_proba_fn

                    candidates.append(self.wrap_external_model(
                        predict_fn=make_predict_fn(predictor, model_name),
                        name=f"autogluon_{model_name}",
                        rank=rank,
                        predict_proba_fn=(
                            make_predict_proba_fn(predictor, model_name)
                            if task == "classification" else None
                        ),
                        metadata={
                            "model_name": model_name,
                            "score": float(leaderboard.iloc[rank]['score_val']),
                        },
                    ))

                self._log(f"Generated {len(candidates)} candidates in {self._generation_time:.1f}s")
                return candidates

            finally:
                # Cleanup temporary directory
                try:
                    shutil.rmtree(temp_dir)
                except Exception:
                    pass

        except Exception as e:
            self._log(f"AutoGluon failed: {type(e).__name__}: {e}")
            self._generation_time = time.time() - start_time
            return []

