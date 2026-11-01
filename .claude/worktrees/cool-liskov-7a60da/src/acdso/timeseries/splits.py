"""
Time-Aware Cross-Validation Splitters
======================================

Critical for time series: standard KFold causes data leakage by using
future data to predict the past. These splitters maintain temporal order.

Splitters:
- ExpandingWindowSplit: Train on all data up to t, test on t+1 to t+h
- RollingWindowSplit: Fixed-size training window that slides forward
- WalkForwardSplit: Industry-standard walk-forward validation
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Generator, Optional, Tuple, List
import numpy as np
import pandas as pd


@dataclass
class SplitInfo:
    """Information about a single train/test split."""
    fold: int
    train_start: int
    train_end: int
    test_start: int
    test_end: int
    train_size: int
    test_size: int
    gap: int  # Gap between train end and test start (for embargo)


class TimeSeriesSplitter(ABC):
    """Abstract base class for time series cross-validation splitters."""
    
    @abstractmethod
    def split(self, X: np.ndarray, y: np.ndarray = None) -> Generator[Tuple[np.ndarray, np.ndarray], None, None]:
        """Generate train/test indices for each fold."""
        pass
    
    @abstractmethod
    def get_n_splits(self, X: np.ndarray = None) -> int:
        """Return the number of splits."""
        pass
    
    def get_split_info(self) -> List[SplitInfo]:
        """Return detailed information about each split."""
        return self._split_info if hasattr(self, '_split_info') else []


class ExpandingWindowSplit(TimeSeriesSplitter):
    """
    Expanding window (anchored) time series split.
    
    Train set grows with each fold, always starting from the beginning.
    Test set is a fixed-size window that moves forward.
    
    Example with n_splits=3, test_size=10:
        Fold 1: Train [0:50], Test [50:60]
        Fold 2: Train [0:60], Test [60:70]
        Fold 3: Train [0:70], Test [70:80]
    
    Parameters
    ----------
    n_splits : int
        Number of splits (folds).
    test_size : int, optional
        Size of test set. If None, computed as n_samples / (n_splits + 1).
    gap : int, default=0
        Number of samples to skip between train and test (embargo period).
    min_train_size : int, optional
        Minimum required training samples. If None, uses test_size.
    """
    
    def __init__(
        self,
        n_splits: int = 5,
        test_size: Optional[int] = None,
        gap: int = 0,
        min_train_size: Optional[int] = None,
    ):
        self.n_splits = n_splits
        self.test_size = test_size
        self.gap = gap
        self.min_train_size = min_train_size
        self._split_info: List[SplitInfo] = []
    
    def split(self, X: np.ndarray, y: np.ndarray = None) -> Generator[Tuple[np.ndarray, np.ndarray], None, None]:
        n_samples = len(X)
        
        # Compute test_size if not provided
        test_size = self.test_size or n_samples // (self.n_splits + 1)
        min_train = self.min_train_size or test_size
        
        # Calculate fold boundaries
        # Reserve space for all test sets + gaps
        total_test_space = self.n_splits * (test_size + self.gap)
        if n_samples < min_train + total_test_space:
            raise ValueError(
                f"Not enough samples ({n_samples}) for {self.n_splits} splits "
                f"with test_size={test_size}, gap={gap}, min_train={min_train}. "
                f"Need at least {min_train + total_test_space} samples."
            )
        
        self._split_info = []
        
        for i in range(self.n_splits):
            # Test set position moves forward
            test_end = n_samples - (self.n_splits - i - 1) * (test_size + self.gap)
            test_start = test_end - test_size
            
            # Train set always starts at 0, ends before gap
            train_end = test_start - self.gap
            train_start = 0
            
            if train_end - train_start < min_train:
                continue
            
            train_idx = np.arange(train_start, train_end)
            test_idx = np.arange(test_start, test_end)
            
            self._split_info.append(SplitInfo(
                fold=i,
                train_start=train_start,
                train_end=train_end,
                test_start=test_start,
                test_end=test_end,
                train_size=len(train_idx),
                test_size=len(test_idx),
                gap=self.gap,
            ))
            
            yield train_idx, test_idx
    
    def get_n_splits(self, X: np.ndarray = None) -> int:
        return self.n_splits


class RollingWindowSplit(TimeSeriesSplitter):
    """
    Rolling (sliding) window time series split.
    
    Both train and test sets have fixed sizes and slide forward together.
    
    Example with train_size=50, test_size=10, step=10:
        Fold 1: Train [0:50], Test [50:60]
        Fold 2: Train [10:60], Test [60:70]
        Fold 3: Train [20:70], Test [70:80]
    
    Parameters
    ----------
    train_size : int
        Size of training window.
    test_size : int
        Size of test window.
    step : int, optional
        Step size between folds. If None, uses test_size.
    gap : int, default=0
        Gap between train and test (embargo period).
    """
    
    def __init__(
        self,
        train_size: int,
        test_size: int,
        step: Optional[int] = None,
        gap: int = 0,
    ):
        self.train_size = train_size
        self.test_size = test_size
        self.step = step or test_size
        self.gap = gap
        self._split_info: List[SplitInfo] = []
        self._n_splits: int = 0
    
    def split(self, X: np.ndarray, y: np.ndarray = None) -> Generator[Tuple[np.ndarray, np.ndarray], None, None]:
        n_samples = len(X)
        
        if n_samples < self.train_size + self.gap + self.test_size:
            raise ValueError(
                f"Not enough samples ({n_samples}) for rolling window with "
                f"train_size={self.train_size}, gap={self.gap}, test_size={self.test_size}. "
                f"Need at least {self.train_size + self.gap + self.test_size} samples."
            )
        
        self._split_info = []
        fold = 0
        
        train_start = 0
        while True:
            train_end = train_start + self.train_size
            test_start = train_end + self.gap
            test_end = test_start + self.test_size
            
            if test_end > n_samples:
                break
            
            train_idx = np.arange(train_start, train_end)
            test_idx = np.arange(test_start, test_end)
            
            self._split_info.append(SplitInfo(
                fold=fold,
                train_start=train_start,
                train_end=train_end,
                test_start=test_start,
                test_end=test_end,
                train_size=len(train_idx),
                test_size=len(test_idx),
                gap=self.gap,
            ))
            
            yield train_idx, test_idx
            
            fold += 1
            train_start += self.step
        
        self._n_splits = fold
    
    def get_n_splits(self, X: np.ndarray = None) -> int:
        if X is not None and self._n_splits == 0:
            # Calculate without actually splitting
            n_samples = len(X)
            usable = n_samples - self.train_size - self.gap - self.test_size
            if usable < 0:
                return 0
            return 1 + usable // self.step
        return self._n_splits


class WalkForwardSplit(TimeSeriesSplitter):
    """
    Walk-forward validation (industry standard for time series).
    
    Combines expanding training window with fixed test horizon.
    Optionally supports purging (removing samples near boundary) and
    embargo (gap between train and test) to prevent leakage.
    
    Parameters
    ----------
    n_splits : int, default=5
        Number of forward walks.
    test_size : int or float, optional
        Test set size. If float, interpreted as fraction of data.
        If None, computed automatically.
    min_train_size : int or float, optional
        Minimum training samples. If float, interpreted as fraction.
    expanding : bool, default=True
        If True, training window expands. If False, uses rolling window.
    gap : int, default=0
        Embargo period between train and test.
    purge : int, default=0
        Number of samples to remove from end of train (for overlapping features).
    """
    
    def __init__(
        self,
        n_splits: int = 5,
        test_size: Optional[int] = None,
        min_train_size: Optional[int] = None,
        expanding: bool = True,
        gap: int = 0,
        purge: int = 0,
    ):
        self.n_splits = n_splits
        self.test_size = test_size
        self.min_train_size = min_train_size
        self.expanding = expanding
        self.gap = gap
        self.purge = purge
        self._split_info: List[SplitInfo] = []
    
    def split(self, X: np.ndarray, y: np.ndarray = None) -> Generator[Tuple[np.ndarray, np.ndarray], None, None]:
        n_samples = len(X)
        
        # Compute sizes
        if isinstance(self.test_size, float):
            test_size = int(n_samples * self.test_size)
        else:
            test_size = self.test_size or n_samples // (self.n_splits + 1)
        
        if isinstance(self.min_train_size, float):
            min_train = int(n_samples * self.min_train_size)
        else:
            min_train = self.min_train_size or test_size
        
        # Calculate window positions
        total_reserved = self.n_splits * test_size + (self.n_splits - 1) * self.gap
        initial_train = n_samples - total_reserved
        
        if initial_train < min_train:
            # Adjust to fit
            available = n_samples - min_train
            actual_splits = available // (test_size + self.gap)
            if actual_splits < 1:
                raise ValueError(
                    f"Cannot create any valid splits with {n_samples} samples. "
                    f"Need more data or smaller test_size/min_train_size."
                )
            initial_train = min_train
        
        self._split_info = []
        
        for i in range(self.n_splits):
            # Test window slides forward
            test_start = initial_train + i * (test_size + self.gap) + self.gap
            test_end = test_start + test_size
            
            if test_end > n_samples:
                break
            
            # Train window
            if self.expanding:
                train_start = 0
            else:
                # Rolling window: fixed size ending at test_start - gap - purge
                train_start = max(0, test_start - self.gap - self.purge - initial_train - i * test_size)
            
            train_end = test_start - self.gap - self.purge
            
            if train_end - train_start < min_train:
                continue
            
            train_idx = np.arange(train_start, train_end)
            test_idx = np.arange(test_start, test_end)
            
            self._split_info.append(SplitInfo(
                fold=i,
                train_start=train_start,
                train_end=train_end,
                test_start=test_start,
                test_end=test_end,
                train_size=len(train_idx),
                test_size=len(test_idx),
                gap=self.gap + self.purge,
            ))
            
            yield train_idx, test_idx
    
    def get_n_splits(self, X: np.ndarray = None) -> int:
        return self.n_splits


def create_time_series_cv(
    cv_type: str = "expanding",
    n_splits: int = 5,
    **kwargs
) -> TimeSeriesSplitter:
    """
    Factory function to create time series cross-validators.
    
    Parameters
    ----------
    cv_type : str
        One of: 'expanding', 'rolling', 'walk_forward'
    n_splits : int
        Number of splits
    **kwargs : dict
        Additional arguments for the splitter
    
    Returns
    -------
    TimeSeriesSplitter
        Configured splitter instance
    """
    splitters = {
        "expanding": ExpandingWindowSplit,
        "rolling": RollingWindowSplit,
        "walk_forward": WalkForwardSplit,
    }
    
    if cv_type not in splitters:
        raise ValueError(f"Unknown cv_type '{cv_type}'. Choose from: {list(splitters.keys())}")
    
    return splitters[cv_type](n_splits=n_splits, **kwargs)

