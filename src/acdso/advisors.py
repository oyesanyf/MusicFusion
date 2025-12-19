"""
ACDSO v2.0 - LLM Advisor Module
================================

Production-ready, configurable LLM-based semantic analysis for detecting
data issues that cannot be found through statistical analysis alone.

Features:
- Multiple provider support (OpenAI, Anthropic, Azure, local models)
- Configurable prompts and parameters
- Caching for cost efficiency
- Retry logic with exponential backoff
- Structured output parsing
- Custom provider extensibility

Example Usage:
    >>> from acdso import ACDSOClassifier
    >>> from acdso.advisors import LLMAdvisor, OpenAIProvider
    >>>
    >>> # Configure advisor with OpenAI
    >>> advisor = LLMAdvisor(
    ...     provider=OpenAIProvider(model="gpt-4o-mini"),
    ...     cache_enabled=True,
    ... )
    >>>
    >>> clf = ACDSOClassifier(llm_advisor=advisor)
    >>> clf.fit(X, y)
"""

from __future__ import annotations

import hashlib
import json
import os
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional, Union
from functools import lru_cache


# =============================================================================
# Configuration
# =============================================================================

@dataclass
class LLMConfig:
    """
    Configuration for LLM advisor.

    Parameters
    ----------
    max_tokens : int, default=500
        Maximum tokens in response.
    temperature : float, default=0.3
        Sampling temperature (0.0-1.0).
    timeout_s : float, default=30.0
        Request timeout in seconds.
    max_retries : int, default=3
        Maximum retry attempts on failure.
    retry_delay_s : float, default=1.0
        Initial retry delay (exponential backoff).
    cache_enabled : bool, default=True
        Whether to cache responses.
    cache_ttl_s : int, default=3600
        Cache time-to-live in seconds.
    max_columns_in_prompt : int, default=50
        Maximum columns to include in prompt.
    """

    max_tokens: int = 500
    temperature: float = 0.3
    timeout_s: float = 30.0
    max_retries: int = 3
    retry_delay_s: float = 1.0
    cache_enabled: bool = True
    cache_ttl_s: int = 3600
    max_columns_in_prompt: int = 50


@dataclass
class PromptTemplate:
    """
    Customizable prompt template for LLM analysis.

    Parameters
    ----------
    system_prompt : str
        System message defining the LLM's role.
    user_prompt_template : str
        User message template with placeholders.
    response_format : str
        Expected response format description.
    """

    system_prompt: str = """You are a senior data scientist expert in machine learning data quality.
Your task is to analyze dataset characteristics and identify potential issues that could affect model performance.
Focus on:
1. Data leakage - columns that might contain target information
2. Temporal leakage - features that look into the future
3. Semantic issues - column names suggesting problems
4. Data quality concerns - missing values, outliers, encoding issues

Be concise and specific. Only flag genuine concerns."""

    user_prompt_template: str = """Analyze this machine learning dataset for potential issues:

**Task**: {task}
**Target Type**: {target_type}
**Dataset Shape**: {n_rows} rows × {n_cols} columns
**Column Names**: {columns}
**Numeric Columns**: {n_numeric}
**Categorical Columns**: {n_categorical}
**Missing Value Ratio**: {missing_ratio:.2%}

**Existing Statistical Findings**:
{notes}

Please identify:
1. Columns that might cause data leakage (contain target information directly or indirectly)
2. Columns with temporal/future information that shouldn't be available at prediction time
3. Any other semantic concerns based on column names

Respond in JSON format:
{{
    "leakage_risk": {{
        "severity": "none|low|medium|high",
        "suspicious_columns": ["col1", "col2"],
        "explanation": "brief explanation"
    }},
    "temporal_risk": {{
        "severity": "none|low|medium|high",
        "suspicious_columns": ["col1", "col2"],
        "explanation": "brief explanation"
    }},
    "other_concerns": ["concern 1", "concern 2"],
    "recommendations": ["recommendation 1", "recommendation 2"],
    "summary": "one sentence summary"
}}"""

    response_format: str = "json"


# =============================================================================
# Base Provider Interface
# =============================================================================

class BaseLLMProvider(ABC):
    """
    Abstract base class for LLM providers.

    Implement this to add support for new LLM services.
    """

    @abstractmethod
    def complete(
        self,
        system_prompt: str,
        user_prompt: str,
        config: LLMConfig,
    ) -> str:
        """
        Send a completion request to the LLM.

        Parameters
        ----------
        system_prompt : str
            System message.
        user_prompt : str
            User message.
        config : LLMConfig
            Configuration options.

        Returns
        -------
        str
            LLM response text.
        """
        pass

    @abstractmethod
    def is_available(self) -> bool:
        """Check if the provider is properly configured."""
        pass

    @property
    @abstractmethod
    def name(self) -> str:
        """Provider name for logging."""
        pass


# =============================================================================
# Provider Implementations
# =============================================================================

class OpenAIProvider(BaseLLMProvider):
    """
    OpenAI API provider (GPT-4, GPT-3.5, etc.).

    Parameters
    ----------
    api_key : str, optional
        OpenAI API key. If None, reads from OPENAI_API_KEY env var.
    model : str, default="gpt-4o-mini"
        Model to use.
    base_url : str, optional
        Custom API base URL (for proxies or Azure).
    organization : str, optional
        OpenAI organization ID.

    Examples
    --------
    >>> provider = OpenAIProvider(model="gpt-4o")
    >>> advisor = LLMAdvisor(provider=provider)
    """

    def __init__(
        self,
        api_key: Optional[str] = None,
        model: str = "gpt-4o-mini",
        base_url: Optional[str] = None,
        organization: Optional[str] = None,
    ):
        self.api_key = api_key or os.environ.get("OPENAI_API_KEY")
        self.model = model
        self.base_url = base_url
        self.organization = organization
        self._client = None

    @property
    def name(self) -> str:
        return f"OpenAI/{self.model}"

    def is_available(self) -> bool:
        if not self.api_key:
            return False
        try:
            import openai
            return True
        except ImportError:
            return False

    def _get_client(self):
        if self._client is None:
            from openai import OpenAI
            self._client = OpenAI(
                api_key=self.api_key,
                base_url=self.base_url,
                organization=self.organization,
            )
        return self._client

    def complete(
        self,
        system_prompt: str,
        user_prompt: str,
        config: LLMConfig,
    ) -> str:
        client = self._get_client()

        response = client.chat.completions.create(
            model=self.model,
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            max_tokens=config.max_tokens,
            temperature=config.temperature,
            timeout=config.timeout_s,
        )

        return response.choices[0].message.content or ""


class AnthropicProvider(BaseLLMProvider):
    """
    Anthropic API provider (Claude models).

    Parameters
    ----------
    api_key : str, optional
        Anthropic API key. If None, reads from ANTHROPIC_API_KEY env var.
    model : str, default="claude-3-haiku-20240307"
        Model to use.

    Examples
    --------
    >>> provider = AnthropicProvider(model="claude-3-sonnet-20240229")
    >>> advisor = LLMAdvisor(provider=provider)
    """

    def __init__(
        self,
        api_key: Optional[str] = None,
        model: str = "claude-3-haiku-20240307",
    ):
        self.api_key = api_key or os.environ.get("ANTHROPIC_API_KEY")
        self.model = model
        self._client = None

    @property
    def name(self) -> str:
        return f"Anthropic/{self.model}"

    def is_available(self) -> bool:
        if not self.api_key:
            return False
        try:
            import anthropic
            return True
        except ImportError:
            return False

    def _get_client(self):
        if self._client is None:
            from anthropic import Anthropic
            self._client = Anthropic(api_key=self.api_key)
        return self._client

    def complete(
        self,
        system_prompt: str,
        user_prompt: str,
        config: LLMConfig,
    ) -> str:
        client = self._get_client()

        response = client.messages.create(
            model=self.model,
            max_tokens=config.max_tokens,
            system=system_prompt,
            messages=[
                {"role": "user", "content": user_prompt},
            ],
        )

        return response.content[0].text if response.content else ""


class AzureOpenAIProvider(BaseLLMProvider):
    """
    Azure OpenAI API provider.

    Parameters
    ----------
    api_key : str, optional
        Azure API key. If None, reads from AZURE_OPENAI_API_KEY env var.
    endpoint : str, optional
        Azure endpoint URL. If None, reads from AZURE_OPENAI_ENDPOINT env var.
    deployment_name : str
        Name of the deployed model.
    api_version : str, default="2024-02-15-preview"
        API version.

    Examples
    --------
    >>> provider = AzureOpenAIProvider(
    ...     endpoint="https://myresource.openai.azure.com",
    ...     deployment_name="gpt-4",
    ... )
    """

    def __init__(
        self,
        api_key: Optional[str] = None,
        endpoint: Optional[str] = None,
        deployment_name: str = "gpt-4",
        api_version: str = "2024-02-15-preview",
    ):
        self.api_key = api_key or os.environ.get("AZURE_OPENAI_API_KEY")
        self.endpoint = endpoint or os.environ.get("AZURE_OPENAI_ENDPOINT")
        self.deployment_name = deployment_name
        self.api_version = api_version
        self._client = None

    @property
    def name(self) -> str:
        return f"Azure/{self.deployment_name}"

    def is_available(self) -> bool:
        if not self.api_key or not self.endpoint:
            return False
        try:
            from openai import AzureOpenAI
            return True
        except ImportError:
            return False

    def _get_client(self):
        if self._client is None:
            from openai import AzureOpenAI
            self._client = AzureOpenAI(
                api_key=self.api_key,
                azure_endpoint=self.endpoint,
                api_version=self.api_version,
            )
        return self._client

    def complete(
        self,
        system_prompt: str,
        user_prompt: str,
        config: LLMConfig,
    ) -> str:
        client = self._get_client()

        response = client.chat.completions.create(
            model=self.deployment_name,
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            max_tokens=config.max_tokens,
            temperature=config.temperature,
            timeout=config.timeout_s,
        )

        return response.choices[0].message.content or ""


class OllamaProvider(BaseLLMProvider):
    """
    Ollama local model provider.

    Parameters
    ----------
    model : str, default="llama3.2"
        Model name.
    base_url : str, default="http://localhost:11434"
        Ollama server URL.

    Examples
    --------
    >>> provider = OllamaProvider(model="mistral")
    >>> advisor = LLMAdvisor(provider=provider)
    """

    def __init__(
        self,
        model: str = "llama3.2",
        base_url: str = "http://localhost:11434",
    ):
        self.model = model
        self.base_url = base_url

    @property
    def name(self) -> str:
        return f"Ollama/{self.model}"

    def is_available(self) -> bool:
        try:
            import requests
            response = requests.get(f"{self.base_url}/api/tags", timeout=5)
            return response.status_code == 200
        except Exception:
            return False

    def complete(
        self,
        system_prompt: str,
        user_prompt: str,
        config: LLMConfig,
    ) -> str:
        import requests

        response = requests.post(
            f"{self.base_url}/api/generate",
            json={
                "model": self.model,
                "prompt": f"{system_prompt}\n\n{user_prompt}",
                "stream": False,
                "options": {
                    "temperature": config.temperature,
                    "num_predict": config.max_tokens,
                },
            },
            timeout=config.timeout_s,
        )
        response.raise_for_status()
        return response.json().get("response", "")


class LiteLLMProvider(BaseLLMProvider):
    """
    LiteLLM provider for unified access to 100+ LLM providers.

    Parameters
    ----------
    model : str
        Model identifier (e.g., "gpt-4", "claude-3-opus", "ollama/llama3").
    api_key : str, optional
        API key (provider-specific).
    api_base : str, optional
        Custom API base URL.

    Examples
    --------
    >>> # Use any model supported by LiteLLM
    >>> provider = LiteLLMProvider(model="gpt-4")
    >>> provider = LiteLLMProvider(model="claude-3-opus-20240229")
    >>> provider = LiteLLMProvider(model="ollama/llama3")
    """

    def __init__(
        self,
        model: str,
        api_key: Optional[str] = None,
        api_base: Optional[str] = None,
    ):
        self.model = model
        self.api_key = api_key
        self.api_base = api_base

    @property
    def name(self) -> str:
        return f"LiteLLM/{self.model}"

    def is_available(self) -> bool:
        try:
            import litellm
            return True
        except ImportError:
            return False

    def complete(
        self,
        system_prompt: str,
        user_prompt: str,
        config: LLMConfig,
    ) -> str:
        import litellm

        response = litellm.completion(
            model=self.model,
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            max_tokens=config.max_tokens,
            temperature=config.temperature,
            api_key=self.api_key,
            api_base=self.api_base,
            timeout=config.timeout_s,
        )

        return response.choices[0].message.content or ""


# =============================================================================
# Response Parsing
# =============================================================================

@dataclass
class LLMAnalysisResult:
    """
    Structured result from LLM analysis.

    Attributes
    ----------
    leakage_severity : str
        Leakage risk level (none, low, medium, high).
    leakage_columns : List[str]
        Columns suspected of leakage.
    temporal_severity : str
        Temporal leakage risk level.
    temporal_columns : List[str]
        Columns with temporal concerns.
    other_concerns : List[str]
        Other identified concerns.
    recommendations : List[str]
        Suggested actions.
    summary : str
        One-sentence summary.
    raw_response : str
        Original LLM response.
    """

    leakage_severity: str = "none"
    leakage_columns: List[str] = field(default_factory=list)
    temporal_severity: str = "none"
    temporal_columns: List[str] = field(default_factory=list)
    other_concerns: List[str] = field(default_factory=list)
    recommendations: List[str] = field(default_factory=list)
    summary: str = ""
    raw_response: str = ""

    def to_note(self) -> str:
        """Convert to a human-readable note for risk report."""
        parts = []

        if self.leakage_severity != "none":
            cols = ", ".join(self.leakage_columns[:3])
            parts.append(f"Leakage risk ({self.leakage_severity}): {cols}")

        if self.temporal_severity != "none":
            cols = ", ".join(self.temporal_columns[:3])
            parts.append(f"Temporal risk ({self.temporal_severity}): {cols}")

        if self.other_concerns:
            parts.append(f"Other: {self.other_concerns[0]}")

        if not parts:
            return "LLM analysis: No significant concerns identified."

        return "LLM analysis: " + "; ".join(parts)


def parse_llm_response(response: str) -> LLMAnalysisResult:
    """
    Parse LLM response into structured result.

    Parameters
    ----------
    response : str
        Raw LLM response.

    Returns
    -------
    LLMAnalysisResult
        Parsed and structured result.
    """
    result = LLMAnalysisResult(raw_response=response)

    # Try to extract JSON from response
    try:
        # Find JSON block in response
        json_start = response.find("{")
        json_end = response.rfind("}") + 1

        if json_start >= 0 and json_end > json_start:
            json_str = response[json_start:json_end]
            data = json.loads(json_str)

            # Extract leakage info
            if "leakage_risk" in data:
                leak = data["leakage_risk"]
                result.leakage_severity = leak.get("severity", "none")
                result.leakage_columns = leak.get("suspicious_columns", [])

            # Extract temporal info
            if "temporal_risk" in data:
                temp = data["temporal_risk"]
                result.temporal_severity = temp.get("severity", "none")
                result.temporal_columns = temp.get("suspicious_columns", [])

            # Extract other fields
            result.other_concerns = data.get("other_concerns", [])
            result.recommendations = data.get("recommendations", [])
            result.summary = data.get("summary", "")

    except (json.JSONDecodeError, KeyError, TypeError):
        # Fall back to basic text extraction
        response_lower = response.lower()

        if "leakage" in response_lower or "leak" in response_lower:
            result.leakage_severity = "medium"
            result.summary = response[:200]

        if "temporal" in response_lower or "future" in response_lower:
            result.temporal_severity = "medium"

    return result


# =============================================================================
# Main LLM Advisor
# =============================================================================

class LLMAdvisor:
    """
    Production-ready LLM advisor for ACDSO.

    This is the main class for integrating LLM-based analysis into ACDSO.
    It supports multiple providers, caching, retries, and structured output.

    Parameters
    ----------
    provider : BaseLLMProvider
        LLM provider instance (OpenAI, Anthropic, etc.).
    config : LLMConfig, optional
        Configuration options.
    prompt_template : PromptTemplate, optional
        Custom prompt template.
    fallback_provider : BaseLLMProvider, optional
        Fallback provider if primary fails.

    Examples
    --------
    >>> from acdso.advisors import LLMAdvisor, OpenAIProvider
    >>>
    >>> # Basic usage with OpenAI
    >>> advisor = LLMAdvisor(
    ...     provider=OpenAIProvider(model="gpt-4o-mini"),
    ... )
    >>>
    >>> # With custom configuration
    >>> advisor = LLMAdvisor(
    ...     provider=OpenAIProvider(model="gpt-4o"),
    ...     config=LLMConfig(
    ...         max_tokens=1000,
    ...         temperature=0.2,
    ...         cache_enabled=True,
    ...     ),
    ... )
    >>>
    >>> # Use with ACDSOClassifier
    >>> from acdso import ACDSOClassifier
    >>> clf = ACDSOClassifier(llm_advisor=advisor)
    >>> clf.fit(X, y)
    """

    def __init__(
        self,
        provider: BaseLLMProvider,
        config: Optional[LLMConfig] = None,
        prompt_template: Optional[PromptTemplate] = None,
        fallback_provider: Optional[BaseLLMProvider] = None,
    ):
        self.provider = provider
        self.config = config or LLMConfig()
        self.prompt_template = prompt_template or PromptTemplate()
        self.fallback_provider = fallback_provider
        self._cache: Dict[str, tuple] = {}  # hash -> (result, timestamp)

    def _cache_key(self, payload: Dict[str, Any]) -> str:
        """Generate cache key from payload."""
        # Create stable hash of relevant fields
        key_data = {
            "columns": sorted(payload.get("columns", [])),
            "task": payload.get("task"),
            "target_type": payload.get("target_type"),
        }
        key_str = json.dumps(key_data, sort_keys=True)
        return hashlib.sha256(key_str.encode()).hexdigest()[:16]

    def _get_from_cache(self, key: str) -> Optional[Dict[str, Any]]:
        """Get result from cache if valid."""
        if not self.config.cache_enabled:
            return None

        if key in self._cache:
            result, timestamp = self._cache[key]
            if time.time() - timestamp < self.config.cache_ttl_s:
                return result

        return None

    def _save_to_cache(self, key: str, result: Dict[str, Any]) -> None:
        """Save result to cache."""
        if self.config.cache_enabled:
            self._cache[key] = (result, time.time())

    def _format_prompt(self, payload: Dict[str, Any]) -> str:
        """Format the user prompt with payload data."""
        columns = payload.get("columns", [])
        if len(columns) > self.config.max_columns_in_prompt:
            columns = columns[:self.config.max_columns_in_prompt]
            columns.append(f"... and {len(payload.get('columns', [])) - self.config.max_columns_in_prompt} more")

        notes = payload.get("notes", [])
        notes_str = "\n".join(f"- {n}" for n in notes) if notes else "None"

        return self.prompt_template.user_prompt_template.format(
            task=payload.get("task", "unknown"),
            target_type=payload.get("target_type", "unknown"),
            n_rows=payload.get("n_rows", 0),
            n_cols=payload.get("n_cols", len(payload.get("columns", []))),
            columns=columns,
            n_numeric=payload.get("n_numeric", 0),
            n_categorical=payload.get("n_categorical", 0),
            missing_ratio=payload.get("missing_ratio", 0.0),
            notes=notes_str,
        )

    def _call_with_retry(
        self,
        provider: BaseLLMProvider,
        system_prompt: str,
        user_prompt: str,
    ) -> str:
        """Call provider with retry logic."""
        last_error = None

        for attempt in range(self.config.max_retries):
            try:
                return provider.complete(
                    system_prompt=system_prompt,
                    user_prompt=user_prompt,
                    config=self.config,
                )
            except Exception as e:
                last_error = e
                if attempt < self.config.max_retries - 1:
                    delay = self.config.retry_delay_s * (2 ** attempt)
                    time.sleep(delay)

        raise last_error or RuntimeError("LLM call failed")

    def advise(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """
        Analyze dataset payload and return advisory.

        This method is called by ACDSO during risk analysis.

        Parameters
        ----------
        payload : Dict[str, Any]
            Dataset information including:
            - columns: List[str] - Column names
            - task: str - "classification" or "regression"
            - target_type: str - "binary", "multiclass", or "regression"
            - notes: List[str] - Existing risk notes
            - n_rows, n_cols: int - Dataset dimensions
            - missing_ratio: float - Missing value ratio

        Returns
        -------
        Dict[str, Any]
            Advisory containing:
            - note: str - Human-readable summary
            - suspicious_columns: List[str] - Potentially problematic columns
            - recommendations: List[str] - Suggested actions
            - analysis: LLMAnalysisResult - Full structured analysis
        """
        # Check cache
        cache_key = self._cache_key(payload)
        cached = self._get_from_cache(cache_key)
        if cached is not None:
            return cached

        # Check provider availability
        if not self.provider.is_available():
            if self.fallback_provider and self.fallback_provider.is_available():
                provider = self.fallback_provider
            else:
                return {
                    "note": "LLM advisor not available (provider not configured)",
                    "suspicious_columns": [],
                    "recommendations": [],
                }
        else:
            provider = self.provider

        # Format prompts
        system_prompt = self.prompt_template.system_prompt
        user_prompt = self._format_prompt(payload)

        try:
            # Call LLM
            response = self._call_with_retry(provider, system_prompt, user_prompt)

            # Parse response
            analysis = parse_llm_response(response)

            # Build result
            all_suspicious = list(set(
                analysis.leakage_columns + analysis.temporal_columns
            ))

            result = {
                "note": analysis.to_note(),
                "suspicious_columns": all_suspicious[:10],
                "recommendations": analysis.recommendations,
                "leakage_severity": analysis.leakage_severity,
                "temporal_severity": analysis.temporal_severity,
                "summary": analysis.summary,
                "analysis": analysis,
            }

            # Cache result
            self._save_to_cache(cache_key, result)

            return result

        except Exception as e:
            return {
                "note": f"LLM advisor error: {type(e).__name__}",
                "suspicious_columns": [],
                "recommendations": [],
                "error": str(e),
            }

    def __call__(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Allow advisor to be called as a function."""
        return self.advise(payload)


# =============================================================================
# Backward Compatibility & Convenience Functions
# =============================================================================

class BaseLLMAdvisor(ABC):
    """
    Abstract base class for custom LLM advisors.

    Implement this if you need full control over the advisory logic.
    For most use cases, use LLMAdvisor with a provider instead.
    """

    @abstractmethod
    def advise(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Provide semantic analysis for the given payload."""
        pass

    def __call__(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        return self.advise(payload)


class StubLLMAdvisor(BaseLLMAdvisor):
    """
    Stub LLM advisor using keyword matching (no LLM required).

    Useful for testing and development without API costs.
    """

    LEAKAGE_KEYWORDS = [
        "target", "outcome", "future", "result", "label",
        "prediction", "predicted", "forecast", "forecasted",
        "answer", "solution", "ground_truth", "groundtruth",
        "y_true", "y_pred", "actual", "expected",
    ]

    TEMPORAL_KEYWORDS = [
        "future", "next", "tomorrow", "will", "upcoming",
        "forecast", "predicted", "next_day", "next_week",
    ]

    def advise(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        columns = payload.get("columns", [])
        cols_lower = [str(c).lower() for c in columns]

        suspicious_leakage = [
            c for c, cl in zip(columns, cols_lower)
            if any(k in cl for k in self.LEAKAGE_KEYWORDS)
        ]

        suspicious_temporal = [
            c for c, cl in zip(columns, cols_lower)
            if any(k in cl for k in self.TEMPORAL_KEYWORDS)
        ]

        all_suspicious = list(set(suspicious_leakage + suspicious_temporal))

        severity = "none"
        if len(all_suspicious) > 3:
            severity = "high"
        elif len(all_suspicious) > 0:
            severity = "medium"

        note_parts = []
        if suspicious_leakage:
            note_parts.append(f"Leakage-related names: {suspicious_leakage[:3]}")
        if suspicious_temporal:
            note_parts.append(f"Temporal-related names: {suspicious_temporal[:3]}")

        return {
            "note": f"Keyword analysis: {'; '.join(note_parts)}" if note_parts else "No suspicious column names detected.",
            "suspicious_columns": all_suspicious[:10],
            "recommendations": [],
            "leakage_severity": severity if suspicious_leakage else "none",
            "temporal_severity": "medium" if suspicious_temporal else "none",
        }


def llm_advisor_stub(payload: Dict[str, Any]) -> Dict[str, Any]:
    """
    Functional stub for LLM advisor (backward compatibility).

    Uses keyword matching - no LLM API required.
    """
    advisor = StubLLMAdvisor()
    return advisor.advise(payload)


def create_advisor(
    provider: str = "openai",
    model: Optional[str] = None,
    api_key: Optional[str] = None,
    **kwargs,
) -> LLMAdvisor:
    """
    Factory function to create an LLM advisor.

    Parameters
    ----------
    provider : str, default="openai"
        Provider name: "openai", "anthropic", "azure", "ollama", "litellm".
    model : str, optional
        Model name (provider-specific).
    api_key : str, optional
        API key (uses environment variable if not provided).
    **kwargs
        Additional provider-specific arguments.

    Returns
    -------
    LLMAdvisor
        Configured LLM advisor.

    Examples
    --------
    >>> advisor = create_advisor("openai", model="gpt-4o")
    >>> advisor = create_advisor("anthropic", model="claude-3-sonnet-20240229")
    >>> advisor = create_advisor("ollama", model="llama3")
    """
    provider_lower = provider.lower()

    if provider_lower == "openai":
        llm_provider = OpenAIProvider(
            api_key=api_key,
            model=model or "gpt-4o-mini",
            **kwargs,
        )
    elif provider_lower == "anthropic":
        llm_provider = AnthropicProvider(
            api_key=api_key,
            model=model or "claude-3-haiku-20240307",
        )
    elif provider_lower == "azure":
        llm_provider = AzureOpenAIProvider(
            api_key=api_key,
            deployment_name=model or "gpt-4",
            **kwargs,
        )
    elif provider_lower == "ollama":
        llm_provider = OllamaProvider(
            model=model or "llama3.2",
            **kwargs,
        )
    elif provider_lower == "litellm":
        llm_provider = LiteLLMProvider(
            model=model or "gpt-4o-mini",
            api_key=api_key,
            **kwargs,
        )
    else:
        raise ValueError(f"Unknown provider: {provider}")

    return LLMAdvisor(provider=llm_provider)


# =============================================================================
# LLM-Powered Label Analysis
# =============================================================================

@dataclass
class LabelAnalysisResult:
    """
    Result of LLM-based label analysis.
    
    Attributes
    ----------
    recommended_scorer : str
        Recommended scorer for the task (e.g., 'f1_weighted', 'accuracy').
    label_type : str
        Type of labels ('binary', 'multiclass', 'ordinal', 'continuous').
    should_encode : bool
        Whether labels should be encoded to 0, 1, 2, ...
    encoding_map : Optional[Dict]
        Suggested encoding mapping if should_encode is True.
    interpretation : str
        Human-readable interpretation of the labels.
    warnings : List[str]
        Any warnings about the labels.
    """
    recommended_scorer: str
    label_type: str
    should_encode: bool
    encoding_map: Optional[Dict[Any, int]] = None
    interpretation: str = ""
    warnings: List[str] = field(default_factory=list)


class LLMLabelAnalyzer:
    """
    LLM-powered label analyzer for intelligent scorer selection.
    
    Uses an LLM to understand the meaning of labels and recommend
    the appropriate scorer and preprocessing.
    
    Parameters
    ----------
    provider : Optional[BaseLLMProvider]
        LLM provider to use. If None, falls back to rule-based analysis.
    
    Examples
    --------
    >>> from acdso.advisors import LLMLabelAnalyzer, OpenAIProvider
    >>> analyzer = LLMLabelAnalyzer(provider=OpenAIProvider())
    >>> result = analyzer.analyze(y, column_name="disease_status")
    >>> print(result.recommended_scorer)  # e.g., 'f1_weighted'
    """
    
    LABEL_ANALYSIS_PROMPT = """Analyze these target labels from a machine learning dataset.

Column name: {column_name}
Unique labels: {unique_labels}
Label counts: {label_counts}
Total samples: {total_samples}

Respond in JSON format:
{{
    "label_type": "binary|multiclass|ordinal|continuous",
    "interpretation": "Brief interpretation of what these labels represent",
    "recommended_scorer": "One of: accuracy, f1_weighted, f1_macro, roc_auc_ovr_weighted, r2, neg_mean_squared_error",
    "should_encode": true/false (whether to remap labels to 0,1,2...),
    "encoding_suggestion": {{"original_label": new_int_label, ...}} or null,
    "warnings": ["any concerns about these labels"]
}}

Consider:
- If labels are like 0/1 or Yes/No → binary classification
- If labels are integers but represent categories (e.g., 0, 59) → multiclass, needs encoding
- If labels have many unique values and are numeric → regression
- If labels seem ordinal (e.g., low/medium/high) → ordinal classification
"""
    
    def __init__(self, provider: Optional["BaseLLMProvider"] = None):
        self.provider = provider
        self._cache: Dict[str, LabelAnalysisResult] = {}
    
    def analyze(
        self,
        y: Any,
        column_name: str = "target",
        use_cache: bool = True,
    ) -> LabelAnalysisResult:
        """
        Analyze target labels and recommend scorer.
        
        Parameters
        ----------
        y : array-like
            Target values.
        column_name : str
            Name of the target column (for context).
        use_cache : bool
            Whether to cache results.
            
        Returns
        -------
        LabelAnalysisResult
            Analysis result with recommendations.
        """
        import numpy as np
        import pandas as pd
        
        y_arr = np.asarray(y)
        unique_labels = np.unique(y_arr)
        n_unique = len(unique_labels)
        
        # Create cache key
        cache_key = f"{column_name}:{n_unique}:{hash(tuple(unique_labels[:20]))}"
        if use_cache and cache_key in self._cache:
            return self._cache[cache_key]
        
        # Calculate label counts
        if hasattr(y, 'value_counts'):
            label_counts = y.value_counts().to_dict()
        else:
            unique, counts = np.unique(y_arr, return_counts=True)
            label_counts = dict(zip([str(u) for u in unique], [int(c) for c in counts]))
        
        # Try LLM analysis if provider is available
        if self.provider is not None:
            try:
                result = self._llm_analyze(
                    unique_labels, label_counts, len(y_arr), column_name
                )
                if use_cache:
                    self._cache[cache_key] = result
                return result
            except Exception as e:
                # Fall back to rule-based
                pass
        
        # Rule-based fallback
        result = self._rule_based_analyze(
            unique_labels, label_counts, len(y_arr), column_name, y_arr
        )
        if use_cache:
            self._cache[cache_key] = result
        return result
    
    def _llm_analyze(
        self,
        unique_labels: Any,
        label_counts: Dict,
        total_samples: int,
        column_name: str,
    ) -> LabelAnalysisResult:
        """Use LLM to analyze labels."""
        prompt = self.LABEL_ANALYSIS_PROMPT.format(
            column_name=column_name,
            unique_labels=list(unique_labels)[:50],  # Limit for prompt
            label_counts=dict(list(label_counts.items())[:50]),
            total_samples=total_samples,
        )
        
        response = self.provider.generate(
            system_prompt="You are a machine learning expert analyzing dataset labels.",
            user_prompt=prompt,
        )
        
        # Parse JSON response
        import json
        try:
            # Extract JSON from response
            text = response.get("content", response.get("text", str(response)))
            # Find JSON in response
            start = text.find('{')
            end = text.rfind('}') + 1
            if start >= 0 and end > start:
                data = json.loads(text[start:end])
            else:
                raise ValueError("No JSON found in response")
            
            encoding_map = None
            if data.get("should_encode") and data.get("encoding_suggestion"):
                encoding_map = data["encoding_suggestion"]
            
            return LabelAnalysisResult(
                recommended_scorer=data.get("recommended_scorer", "accuracy"),
                label_type=data.get("label_type", "multiclass"),
                should_encode=data.get("should_encode", False),
                encoding_map=encoding_map,
                interpretation=data.get("interpretation", ""),
                warnings=data.get("warnings", []),
            )
        except json.JSONDecodeError:
            # If JSON parsing fails, fall back to rule-based
            raise ValueError("Failed to parse LLM response as JSON")
    
    def _rule_based_analyze(
        self,
        unique_labels: Any,
        label_counts: Dict,
        total_samples: int,
        column_name: str,
        y_arr: Any,
    ) -> LabelAnalysisResult:
        """Rule-based fallback for label analysis."""
        import numpy as np
        
        n_unique = len(unique_labels)
        labels_set = set(unique_labels)
        
        # Check if regression
        if np.issubdtype(y_arr.dtype, np.floating) and n_unique > 20:
            return LabelAnalysisResult(
                recommended_scorer="r2",
                label_type="continuous",
                should_encode=False,
                interpretation=f"Continuous target with {n_unique} unique values",
                warnings=[],
            )
        
        # Check if standard binary (0/1)
        if n_unique == 2 and labels_set == {0, 1}:
            return LabelAnalysisResult(
                recommended_scorer="f1",
                label_type="binary",
                should_encode=False,
                interpretation="Standard binary classification (0/1)",
                warnings=[],
            )
        
        # Binary but non-standard labels (e.g., 0/59, Yes/No)
        if n_unique == 2:
            # Create encoding map
            sorted_labels = sorted(unique_labels)
            encoding_map = {sorted_labels[0]: 0, sorted_labels[1]: 1}
            
            return LabelAnalysisResult(
                recommended_scorer="f1_weighted",
                label_type="binary",
                should_encode=True,
                encoding_map=encoding_map,
                interpretation=f"Binary classification with non-standard labels: {list(unique_labels)}",
                warnings=[
                    f"Labels {list(unique_labels)} are not standard 0/1. Using f1_weighted scorer.",
                    "Consider encoding labels to 0/1 for better compatibility."
                ],
            )
        
        # Multiclass
        if n_unique <= 20:
            # Create encoding map for multiclass
            sorted_labels = sorted(unique_labels)
            encoding_map = {label: i for i, label in enumerate(sorted_labels)}
            
            return LabelAnalysisResult(
                recommended_scorer="f1_weighted",
                label_type="multiclass",
                should_encode=labels_set != set(range(n_unique)),
                encoding_map=encoding_map if labels_set != set(range(n_unique)) else None,
                interpretation=f"Multiclass classification with {n_unique} classes",
                warnings=[
                    f"Using f1_weighted for {n_unique}-class classification."
                ] if n_unique > 10 else [],
            )
        
        # Many classes - likely regression or high-cardinality classification
        return LabelAnalysisResult(
            recommended_scorer="accuracy",
            label_type="multiclass",
            should_encode=False,
            interpretation=f"High-cardinality classification ({n_unique} classes)",
            warnings=[
                f"Very high number of classes ({n_unique}). Consider if this should be regression."
            ],
        )
    
    def get_recommended_scorer(self, y: Any, column_name: str = "target") -> str:
        """
        Quick method to just get the recommended scorer.
        
        Parameters
        ----------
        y : array-like
            Target values.
        column_name : str
            Name of the target column.
            
        Returns
        -------
        str
            Recommended scorer name.
        """
        result = self.analyze(y, column_name)
        return result.recommended_scorer


def analyze_labels_with_llm(
    y: Any,
    column_name: str = "target",
    provider: Optional["BaseLLMProvider"] = None,
) -> LabelAnalysisResult:
    """
    Convenience function to analyze labels with optional LLM support.
    
    Parameters
    ----------
    y : array-like
        Target values.
    column_name : str
        Name of the target column.
    provider : Optional[BaseLLMProvider]
        LLM provider. If None, uses rule-based analysis.
        
    Returns
    -------
    LabelAnalysisResult
        Analysis result with recommendations.
        
    Examples
    --------
    >>> from acdso.advisors import analyze_labels_with_llm
    >>> result = analyze_labels_with_llm(y, column_name="Sec")
    >>> print(f"Use scorer: {result.recommended_scorer}")
    >>> print(f"Encode labels: {result.should_encode}")
    """
    analyzer = LLMLabelAnalyzer(provider=provider)
    return analyzer.analyze(y, column_name)


# =============================================================================
# LLM-Powered Column Analysis & Auto-Exclusion
# =============================================================================

@dataclass
class ColumnAnalysisResult:
    """
    Result of LLM-based column analysis for automatic exclusion.
    
    Attributes
    ----------
    columns_to_exclude : List[str]
        Columns that should be excluded from features.
    exclusion_reasons : Dict[str, str]
        Reason for each excluded column.
    leakage_columns : List[str]
        Columns detected as potential data leakage.
    correlated_columns : List[str]
        Columns highly correlated with target.
    id_columns : List[str]
        ID-like columns (unique identifiers).
    constant_columns : List[str]
        Columns with constant or near-constant values.
    recommendations : List[str]
        Additional recommendations.
    confidence : float
        Confidence score (0.0 to 1.0).
    """
    columns_to_exclude: List[str] = field(default_factory=list)
    exclusion_reasons: Dict[str, str] = field(default_factory=dict)
    leakage_columns: List[str] = field(default_factory=list)
    correlated_columns: List[str] = field(default_factory=list)
    id_columns: List[str] = field(default_factory=list)
    constant_columns: List[str] = field(default_factory=list)
    recommendations: List[str] = field(default_factory=list)
    confidence: float = 0.0


class LLMColumnAnalyzer:
    """
    LLM-powered column analyzer for automatic feature exclusion.
    
    Uses an LLM to understand column semantics and identify problematic
    columns that should be excluded (leakage, IDs, constants, etc.).
    
    Parameters
    ----------
    provider : Optional[BaseLLMProvider]
        LLM provider to use. If None, falls back to rule-based analysis.
    
    Examples
    --------
    >>> from acdso.advisors import LLMColumnAnalyzer, OpenAIProvider
    >>> import pandas as pd
    >>> 
    >>> df = pd.read_csv("data.csv")
    >>> analyzer = LLMColumnAnalyzer(provider=OpenAIProvider())
    >>> result = analyzer.analyze(df, target_column="Sec")
    >>> print(f"Exclude: {result.columns_to_exclude}")
    >>> 
    >>> # Auto-clean the dataframe
    >>> X_clean = df.drop(columns=result.columns_to_exclude + ["Sec"])
    """
    
    COLUMN_ANALYSIS_PROMPT = """Analyze this dataset to identify columns that should be EXCLUDED from machine learning features.

TARGET COLUMN: {target_column}

COLUMN INFORMATION:
{column_info}

SAMPLE DATA (first 5 rows):
{sample_data}

DESCRIBE STATISTICS:
{describe_stats}

Identify columns to EXCLUDE based on:
1. **Data Leakage**: Columns derived from or highly correlated with the target
   - Example: If target is "Sec" (seconds), then "Hour", "Minute", "Day" are leakage
   - Example: If target is "price", then "final_price", "total_cost" might be leakage
   
2. **ID Columns**: Unique identifiers that have no predictive value
   - Example: "user_id", "transaction_id", "row_number"
   
3. **Constant/Near-Constant**: Columns with same value for all/most rows
   
4. **Target Derivatives**: Columns obviously computed from the target

Respond in JSON format:
{{
    "columns_to_exclude": ["col1", "col2", ...],
    "exclusion_reasons": {{
        "col1": "reason for exclusion",
        "col2": "reason for exclusion"
    }},
    "leakage_columns": ["cols that leak target info"],
    "correlated_columns": ["cols highly correlated with target"],
    "id_columns": ["unique identifier columns"],
    "constant_columns": ["constant or near-constant columns"],
    "recommendations": ["additional recommendations"],
    "confidence": 0.0-1.0
}}

Be thorough but not overly aggressive. Only exclude columns with clear problems.
"""
    
    def __init__(self, provider: Optional["BaseLLMProvider"] = None):
        self.provider = provider
        self._cache: Dict[str, ColumnAnalysisResult] = {}
    
    def analyze(
        self,
        df: Any,
        target_column: str,
        use_cache: bool = True,
    ) -> ColumnAnalysisResult:
        """
        Analyze dataset columns and identify those to exclude.
        
        Parameters
        ----------
        df : pd.DataFrame
            Input dataframe.
        target_column : str
            Name of the target column.
        use_cache : bool
            Whether to cache results.
            
        Returns
        -------
        ColumnAnalysisResult
            Analysis result with columns to exclude.
        """
        import pandas as pd
        import numpy as np
        
        if not isinstance(df, pd.DataFrame):
            df = pd.DataFrame(df)
        
        # Create cache key
        cache_key = f"{target_column}:{len(df.columns)}:{hash(tuple(df.columns[:20]))}"
        if use_cache and cache_key in self._cache:
            return self._cache[cache_key]
        
        # Try LLM analysis if provider is available
        if self.provider is not None:
            try:
                result = self._llm_analyze(df, target_column)
                if use_cache:
                    self._cache[cache_key] = result
                return result
            except Exception:
                # Fall back to rule-based
                pass
        
        # Rule-based fallback
        result = self._rule_based_analyze(df, target_column)
        if use_cache:
            self._cache[cache_key] = result
        return result
    
    def _llm_analyze(
        self,
        df: Any,
        target_column: str,
    ) -> ColumnAnalysisResult:
        """Use LLM to analyze columns."""
        import pandas as pd
        
        # Prepare column info
        column_info = []
        for col in df.columns:
            dtype = str(df[col].dtype)
            nunique = df[col].nunique()
            null_pct = df[col].isnull().mean() * 100
            column_info.append(f"- {col}: dtype={dtype}, unique={nunique}, null%={null_pct:.1f}")
        
        # Get sample data (limit columns for prompt size)
        sample_cols = list(df.columns)[:30]
        sample_data = df[sample_cols].head().to_string()
        
        # Get describe stats
        try:
            describe_stats = df.describe(include='all').to_string()
        except Exception:
            describe_stats = "Unable to compute describe()"
        
        prompt = self.COLUMN_ANALYSIS_PROMPT.format(
            target_column=target_column,
            column_info="\n".join(column_info[:50]),  # Limit for prompt
            sample_data=sample_data,
            describe_stats=describe_stats[:2000],  # Limit size
        )
        
        response = self.provider.generate(
            system_prompt="You are a machine learning expert analyzing dataset columns for potential issues.",
            user_prompt=prompt,
        )
        
        # Parse JSON response
        import json
        try:
            text = response.get("content", response.get("text", str(response)))
            start = text.find('{')
            end = text.rfind('}') + 1
            if start >= 0 and end > start:
                data = json.loads(text[start:end])
            else:
                raise ValueError("No JSON found in response")
            
            return ColumnAnalysisResult(
                columns_to_exclude=data.get("columns_to_exclude", []),
                exclusion_reasons=data.get("exclusion_reasons", {}),
                leakage_columns=data.get("leakage_columns", []),
                correlated_columns=data.get("correlated_columns", []),
                id_columns=data.get("id_columns", []),
                constant_columns=data.get("constant_columns", []),
                recommendations=data.get("recommendations", []),
                confidence=data.get("confidence", 0.8),
            )
        except json.JSONDecodeError:
            raise ValueError("Failed to parse LLM response as JSON")
    
    def _rule_based_analyze(
        self,
        df: Any,
        target_column: str,
    ) -> ColumnAnalysisResult:
        """Rule-based fallback for column analysis."""
        import pandas as pd
        import numpy as np
        
        columns_to_exclude = []
        exclusion_reasons = {}
        leakage_columns = []
        id_columns = []
        constant_columns = []
        
        target_lower = target_column.lower()
        
        # Common time component names that might cause leakage
        time_components = {
            'second', 'sec', 'minute', 'min', 'hour', 'hr',
            'day', 'month', 'year', 'date', 'time', 'timestamp',
            'week', 'weekday', 'dayofweek', 'quarter'
        }
        
        # If target looks like a time component, flag related columns
        target_is_time = any(t in target_lower for t in time_components)
        
        for col in df.columns:
            if col == target_column:
                continue
                
            col_lower = col.lower()
            
            # Check for ID columns
            if any(id_word in col_lower for id_word in ['_id', 'id_', 'userid', 'rowid', 'index']):
                if df[col].nunique() > 0.9 * len(df):
                    id_columns.append(col)
                    columns_to_exclude.append(col)
                    exclusion_reasons[col] = "ID-like column (mostly unique values)"
                    continue
            
            # Check for constant columns
            if df[col].nunique() <= 1:
                constant_columns.append(col)
                columns_to_exclude.append(col)
                exclusion_reasons[col] = "Constant column (only one unique value)"
                continue
            
            # Check for time component leakage
            if target_is_time:
                if any(t in col_lower for t in time_components):
                    leakage_columns.append(col)
                    columns_to_exclude.append(col)
                    exclusion_reasons[col] = f"Potential time-based leakage (target '{target_column}' is time-related)"
                    continue
            
            # Check for obvious target derivatives
            if target_lower in col_lower or col_lower in target_lower:
                if col != target_column:
                    leakage_columns.append(col)
                    columns_to_exclude.append(col)
                    exclusion_reasons[col] = f"Column name contains target name - potential leakage"
        
        recommendations = []
        if leakage_columns:
            recommendations.append(f"Found {len(leakage_columns)} potential leakage columns - review carefully")
        if id_columns:
            recommendations.append(f"Found {len(id_columns)} ID columns that won't help prediction")
        
        return ColumnAnalysisResult(
            columns_to_exclude=columns_to_exclude,
            exclusion_reasons=exclusion_reasons,
            leakage_columns=leakage_columns,
            correlated_columns=[],
            id_columns=id_columns,
            constant_columns=constant_columns,
            recommendations=recommendations,
            confidence=0.7,  # Rule-based has lower confidence
        )
    
    def get_clean_features(
        self,
        df: Any,
        target_column: str,
    ) -> Any:
        """
        Analyze and return a cleaned dataframe with problematic columns removed.
        
        Parameters
        ----------
        df : pd.DataFrame
            Input dataframe.
        target_column : str
            Name of the target column.
            
        Returns
        -------
        pd.DataFrame
            Cleaned dataframe with target and problematic columns removed.
        """
        import pandas as pd
        
        result = self.analyze(df, target_column)
        
        cols_to_drop = set(result.columns_to_exclude)
        cols_to_drop.add(target_column)
        cols_to_drop = [c for c in cols_to_drop if c in df.columns]
        
        return df.drop(columns=cols_to_drop)


def auto_exclude_columns(
    df: Any,
    target_column: str,
    provider: Optional["BaseLLMProvider"] = None,
    verbose: bool = True,
) -> ColumnAnalysisResult:
    """
    Automatically analyze dataset and identify columns to exclude.
    
    This is the main entry point for LLM-powered column exclusion.
    
    Parameters
    ----------
    df : pd.DataFrame
        Input dataframe.
    target_column : str
        Name of the target column.
    provider : Optional[BaseLLMProvider]
        LLM provider. If None, uses rule-based analysis.
    verbose : bool
        Whether to print analysis results.
        
    Returns
    -------
    ColumnAnalysisResult
        Analysis result with columns to exclude.
        
    Examples
    --------
    >>> from acdso.advisors import auto_exclude_columns, OpenAIProvider
    >>> import pandas as pd
    >>> 
    >>> df = pd.read_csv("brain.csv")
    >>> result = auto_exclude_columns(df, target_column="Sec", provider=OpenAIProvider())
    >>> 
    >>> # Print what will be excluded
    >>> for col, reason in result.exclusion_reasons.items():
    ...     print(f"  - {col}: {reason}")
    >>> 
    >>> # Get clean features
    >>> X = df.drop(columns=result.columns_to_exclude + ["Sec"])
    >>> y = df["Sec"]
    """
    analyzer = LLMColumnAnalyzer(provider=provider)
    result = analyzer.analyze(df, target_column)
    
    if verbose and result.columns_to_exclude:
        print(f"\n  🔍 Auto-Exclusion Analysis (confidence: {result.confidence:.0%})")
        print(f"  ─" * 30)
        for col in result.columns_to_exclude:
            reason = result.exclusion_reasons.get(col, "Unknown reason")
            print(f"    ✗ {col}: {reason}")
        if result.recommendations:
            print(f"\n  Recommendations:")
            for rec in result.recommendations:
                print(f"    ℹ {rec}")
    
    return result


# =============================================================================
# Backward Compatibility Alias
# =============================================================================

# Keep old name for backward compatibility
OpenAIAdvisorTemplate = OpenAIProvider
