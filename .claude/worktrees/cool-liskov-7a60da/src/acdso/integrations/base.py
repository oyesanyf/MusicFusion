"""
ACDSO v2.0 - Base Integration Classes
======================================

Base classes and interfaces for AutoML integration adapters.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional, Tuple, Union

import time
import numpy as np
import pandas as pd

from sklearn.base import BaseEstimator, ClassifierMixin, RegressorMixin


@dataclass
class CandidateSource:
    """
    Represents a candidate model from an external source.

    Attributes
    ----------
    name : str
        Unique identifier for this candidate.
    source : str
        Name of the AutoML tool that generated this candidate.
    estimator : Any
        The model/estimator object (sklearn-compatible or wrapped).
    metadata : Dict[str, Any]
        Additional metadata from the source tool.
    """

    name: str
    source: str
    estimator: Any
    metadata: Dict[str, Any] = field(default_factory=dict)

    def __repr__(self) -> str:
        return f"CandidateSource(name='{self.name}', source='{self.source}')"


class ExternalModelWrapper(BaseEstimator):
    """
    Wrapper to make external models sklearn-compatible.

    This wrapper provides a uniform interface for models from
    non-sklearn tools (H2O, AutoGluon, etc.) so ACDSO can
    evaluate them using the same metrics.

    Parameters
    ----------
    predict_fn : callable
        Function that takes X and returns predictions.
    predict_proba_fn : callable, optional
        Function that takes X and returns probability predictions.
    name : str, default="ExternalModel"
        Name identifier for this model.
    source : str, default="external"
        Name of the source tool.
    metadata : dict, optional
        Additional metadata about the model.

    Examples
    --------
    >>> # Wrap an H2O model
    >>> wrapper = ExternalModelWrapper(
    ...     predict_fn=lambda X: h2o_model.predict(h2o.H2OFrame(X)).as_data_frame()['predict'].values,
    ...     predict_proba_fn=lambda X: h2o_model.predict(h2o.H2OFrame(X)).as_data_frame()[['p0', 'p1']].values,
    ...     name="H2O_GBM",
    ...     source="h2o",
    ... )
    """

    def __init__(
        self,
        predict_fn: Callable[[Any], np.ndarray],
        predict_proba_fn: Optional[Callable[[Any], np.ndarray]] = None,
        name: str = "ExternalModel",
        source: str = "external",
        metadata: Optional[Dict[str, Any]] = None,
    ):
        self.predict_fn = predict_fn
        self.predict_proba_fn = predict_proba_fn
        self.name = name
        self.source = source
        self.metadata = metadata or {}
        self._is_fitted = True  # External models are already fitted

    def fit(self, X: Any, y: Any) -> "ExternalModelWrapper":
        """No-op fit (external models are pre-fitted)."""
        return self

    def predict(self, X: Any) -> np.ndarray:
        """Make predictions using the wrapped predict function."""
        return np.asarray(self.predict_fn(X))

    def predict_proba(self, X: Any) -> np.ndarray:
        """Make probability predictions if available."""
        if self.predict_proba_fn is None:
            raise AttributeError(
                f"Model '{self.name}' does not support predict_proba()"
            )
        return np.asarray(self.predict_proba_fn(X))

    def __repr__(self) -> str:
        return f"ExternalModelWrapper(name='{self.name}', source='{self.source}')"


class BaseAutoMLAdapter(ABC):
    """
    Abstract base class for AutoML tool adapters.

    Subclasses implement the `generate_candidates()` method to run
    their respective AutoML tool and return candidate models.

    Parameters
    ----------
    name : str
        Name identifier for this adapter.
    top_k : int, default=5
        Number of top candidates to return.
    random_state : int, optional
        Random seed for reproducibility.
    verbose : int, default=0
        Verbosity level.

    Examples
    --------
    >>> class MyAutoMLAdapter(BaseAutoMLAdapter):
    ...     def generate_candidates(self, X, y, task):
    ...         # Run your AutoML tool
    ...         models = my_automl.fit(X, y)
    ...         # Return as CandidateSource objects
    ...         return [
    ...             CandidateSource(
    ...                 name=f"model_{i}",
    ...                 source="my_automl",
    ...                 estimator=model,
    ...             )
    ...             for i, model in enumerate(models[:self.top_k])
    ...         ]
    """

    def __init__(
        self,
        name: str,
        top_k: int = 5,
        random_state: Optional[int] = None,
        verbose: int = 0,
    ):
        self.name = name
        self.top_k = top_k
        self.random_state = random_state
        self.verbose = verbose
        self._generation_time: float = 0.0
        self._generation_metadata: Dict[str, Any] = {}

    @abstractmethod
    def generate_candidates(
        self,
        X: Any,
        y: Any,
        task: str,
    ) -> List[CandidateSource]:
        """
        Generate candidate models using the underlying AutoML tool.

        Parameters
        ----------
        X : array-like
            Training features.
        y : array-like
            Target values.
        task : str
            Task type ("classification" or "regression").

        Returns
        -------
        List[CandidateSource]
            List of candidate models wrapped as CandidateSource objects.
        """
        pass

    def _log(self, msg: str) -> None:
        """Log message if verbose mode is enabled."""
        if self.verbose:
            print(f"[{self.name}] {msg}")

    def get_generation_stats(self) -> Dict[str, Any]:
        """Return statistics about the last generation run."""
        return {
            "adapter": self.name,
            "generation_time_s": self._generation_time,
            "top_k": self.top_k,
            **self._generation_metadata,
        }

    @abstractmethod
    def is_available(self) -> bool:
        """Check if the underlying AutoML tool is installed/available."""
        pass

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(name='{self.name}', top_k={self.top_k})"


class SklearnNativeAdapter(BaseAutoMLAdapter):
    """
    Base class for sklearn-native AutoML tool adapters.

    These tools return sklearn-compatible estimators directly,
    so no wrapping is needed.
    """

    def wrap_estimator(
        self,
        estimator: BaseEstimator,
        name: str,
        rank: int,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> CandidateSource:
        """
        Wrap a sklearn estimator as a CandidateSource.

        Parameters
        ----------
        estimator : BaseEstimator
            sklearn-compatible estimator.
        name : str
            Name for this candidate.
        rank : int
            Rank in the source tool's leaderboard.
        metadata : dict, optional
            Additional metadata.

        Returns
        -------
        CandidateSource
            Wrapped candidate.
        """
        return CandidateSource(
            name=name,
            source=self.name,
            estimator=estimator,
            metadata={
                "rank": rank,
                **(metadata or {}),
            },
        )


class ExternalAutoMLAdapter(BaseAutoMLAdapter):
    """
    Base class for external (non-sklearn) AutoML tool adapters.

    These tools require wrapping their models to be sklearn-compatible.
    """

    def wrap_external_model(
        self,
        predict_fn: Callable,
        name: str,
        rank: int,
        predict_proba_fn: Optional[Callable] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> CandidateSource:
        """
        Wrap an external model as a CandidateSource.

        Parameters
        ----------
        predict_fn : callable
            Prediction function.
        name : str
            Name for this candidate.
        rank : int
            Rank in the source tool's leaderboard.
        predict_proba_fn : callable, optional
            Probability prediction function.
        metadata : dict, optional
            Additional metadata.

        Returns
        -------
        CandidateSource
            Wrapped candidate with ExternalModelWrapper.
        """
        wrapper = ExternalModelWrapper(
            predict_fn=predict_fn,
            predict_proba_fn=predict_proba_fn,
            name=name,
            source=self.name,
            metadata=metadata or {},
        )
        return CandidateSource(
            name=name,
            source=self.name,
            estimator=wrapper,
            metadata={
                "rank": rank,
                **(metadata or {}),
            },
        )

