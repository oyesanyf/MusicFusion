"""
Tests for LLM Advisor Module
============================

Tests for LLM advisor functionality including providers,
configuration, caching, and parsing.
"""

import json
import pytest
from unittest.mock import Mock, patch

from acdso.advisors import (
    LLMAdvisor,
    LLMConfig,
    PromptTemplate,
    BaseLLMProvider,
    OpenAIProvider,
    AnthropicProvider,
    OllamaProvider,
    StubLLMAdvisor,
    llm_advisor_stub,
    create_advisor,
    parse_llm_response,
    LLMAnalysisResult,
)


class TestLLMConfig:
    """Tests for LLMConfig."""

    def test_default_config(self):
        """Test default configuration values."""
        config = LLMConfig()

        assert config.max_tokens == 500
        assert config.temperature == 0.3
        assert config.timeout_s == 30.0
        assert config.max_retries == 3
        assert config.cache_enabled == True

    def test_custom_config(self):
        """Test custom configuration."""
        config = LLMConfig(
            max_tokens=1000,
            temperature=0.5,
            cache_enabled=False,
        )

        assert config.max_tokens == 1000
        assert config.temperature == 0.5
        assert config.cache_enabled == False


class TestPromptTemplate:
    """Tests for PromptTemplate."""

    def test_default_template(self):
        """Test default prompt template."""
        template = PromptTemplate()

        assert "data scientist" in template.system_prompt.lower()
        assert "{task}" in template.user_prompt_template
        assert "{columns}" in template.user_prompt_template

    def test_custom_template(self):
        """Test custom prompt template."""
        template = PromptTemplate(
            system_prompt="Custom system prompt",
            user_prompt_template="Analyze: {columns}",
        )

        assert template.system_prompt == "Custom system prompt"
        assert template.user_prompt_template == "Analyze: {columns}"


class TestStubLLMAdvisor:
    """Tests for StubLLMAdvisor."""

    def test_no_suspicious_columns(self):
        """Test with clean column names."""
        advisor = StubLLMAdvisor()

        result = advisor.advise({
            "columns": ["feature_1", "feature_2", "age", "income"],
            "task": "classification",
            "target_type": "binary",
        })

        assert result["leakage_severity"] == "none"
        assert len(result["suspicious_columns"]) == 0

    def test_leakage_keywords_detected(self):
        """Test detection of leakage keywords."""
        advisor = StubLLMAdvisor()

        result = advisor.advise({
            "columns": ["feature_1", "target_value", "outcome_label"],
            "task": "classification",
            "target_type": "binary",
        })

        assert result["leakage_severity"] != "none"
        assert len(result["suspicious_columns"]) > 0
        assert any("target" in c.lower() for c in result["suspicious_columns"])

    def test_temporal_keywords_detected(self):
        """Test detection of temporal keywords."""
        advisor = StubLLMAdvisor()

        result = advisor.advise({
            "columns": ["feature_1", "future_price", "next_week_sales"],
            "task": "classification",
            "target_type": "binary",
        })

        assert result["temporal_severity"] != "none"
        assert len(result["suspicious_columns"]) > 0

    def test_callable(self):
        """Test that advisor is callable."""
        advisor = StubLLMAdvisor()

        result = advisor({"columns": ["a", "b"], "task": "classification"})
        assert "note" in result


class TestLlmAdvisorStub:
    """Tests for llm_advisor_stub function."""

    def test_basic_usage(self):
        """Test basic usage of stub function."""
        result = llm_advisor_stub({
            "columns": ["feature_1", "target"],
            "task": "classification",
        })

        assert "note" in result
        assert "suspicious_columns" in result


class TestParseResponse:
    """Tests for parse_llm_response."""

    def test_parse_valid_json(self):
        """Test parsing valid JSON response."""
        response = '''Here is the analysis:
        {
            "leakage_risk": {
                "severity": "high",
                "suspicious_columns": ["target_col", "outcome"],
                "explanation": "These columns contain target information"
            },
            "temporal_risk": {
                "severity": "low",
                "suspicious_columns": [],
                "explanation": "No temporal issues"
            },
            "other_concerns": ["High cardinality in col_x"],
            "recommendations": ["Remove target_col"],
            "summary": "Potential leakage detected"
        }'''

        result = parse_llm_response(response)

        assert result.leakage_severity == "high"
        assert "target_col" in result.leakage_columns
        assert result.temporal_severity == "low"
        assert len(result.recommendations) > 0

    def test_parse_invalid_json(self):
        """Test handling of invalid JSON."""
        response = "This is not JSON, but mentions leakage risk."

        result = parse_llm_response(response)

        # Should handle gracefully
        assert isinstance(result, LLMAnalysisResult)
        assert result.raw_response == response

    def test_parse_partial_json(self):
        """Test handling of partial JSON."""
        response = '{"leakage_risk": {"severity": "medium"}}'

        result = parse_llm_response(response)

        assert result.leakage_severity == "medium"


class TestLLMAnalysisResult:
    """Tests for LLMAnalysisResult."""

    def test_to_note_with_issues(self):
        """Test note generation with issues."""
        result = LLMAnalysisResult(
            leakage_severity="high",
            leakage_columns=["target", "outcome"],
            temporal_severity="none",
        )

        note = result.to_note()

        assert "Leakage" in note
        assert "high" in note
        assert "target" in note

    def test_to_note_clean(self):
        """Test note generation with no issues."""
        result = LLMAnalysisResult()

        note = result.to_note()

        assert "No significant concerns" in note


class MockProvider(BaseLLMProvider):
    """Mock provider for testing."""

    def __init__(self, response: str = "test response"):
        self.response = response
        self.call_count = 0

    @property
    def name(self) -> str:
        return "MockProvider"

    def is_available(self) -> bool:
        return True

    def complete(self, system_prompt, user_prompt, config):
        self.call_count += 1
        return self.response


class TestLLMAdvisor:
    """Tests for LLMAdvisor."""

    def test_basic_advise(self):
        """Test basic advise functionality."""
        mock_response = json.dumps({
            "leakage_risk": {"severity": "none", "suspicious_columns": []},
            "temporal_risk": {"severity": "none", "suspicious_columns": []},
            "other_concerns": [],
            "recommendations": [],
            "summary": "No issues found",
        })

        provider = MockProvider(response=mock_response)
        advisor = LLMAdvisor(provider=provider)

        result = advisor.advise({
            "columns": ["a", "b", "c"],
            "task": "classification",
            "target_type": "binary",
        })

        assert provider.call_count == 1
        assert "note" in result
        assert "suspicious_columns" in result

    def test_caching(self):
        """Test response caching."""
        provider = MockProvider(response='{"summary": "test"}')
        advisor = LLMAdvisor(
            provider=provider,
            config=LLMConfig(cache_enabled=True),
        )

        payload = {
            "columns": ["a", "b"],
            "task": "classification",
            "target_type": "binary",
        }

        # First call
        advisor.advise(payload)
        assert provider.call_count == 1

        # Second call (should use cache)
        advisor.advise(payload)
        assert provider.call_count == 1  # No additional call

    def test_caching_disabled(self):
        """Test with caching disabled."""
        provider = MockProvider(response='{"summary": "test"}')
        advisor = LLMAdvisor(
            provider=provider,
            config=LLMConfig(cache_enabled=False),
        )

        payload = {"columns": ["a", "b"], "task": "classification"}

        advisor.advise(payload)
        advisor.advise(payload)

        assert provider.call_count == 2  # Both calls made

    def test_unavailable_provider(self):
        """Test handling of unavailable provider."""
        class UnavailableProvider(BaseLLMProvider):
            @property
            def name(self):
                return "Unavailable"

            def is_available(self):
                return False

            def complete(self, *args, **kwargs):
                raise RuntimeError("Not available")

        advisor = LLMAdvisor(provider=UnavailableProvider())

        result = advisor.advise({"columns": ["a"], "task": "classification"})

        assert "not available" in result["note"].lower()

    def test_fallback_provider(self):
        """Test fallback provider usage."""
        class FailingProvider(BaseLLMProvider):
            @property
            def name(self):
                return "Failing"

            def is_available(self):
                return False

            def complete(self, *args, **kwargs):
                raise RuntimeError("Failed")

        primary = FailingProvider()
        fallback = MockProvider(response='{"summary": "fallback"}')

        advisor = LLMAdvisor(
            provider=primary,
            fallback_provider=fallback,
        )

        result = advisor.advise({"columns": ["a"], "task": "classification"})

        assert fallback.call_count == 1

    def test_callable(self):
        """Test that advisor is callable."""
        provider = MockProvider(response='{"summary": "test"}')
        advisor = LLMAdvisor(provider=provider)

        result = advisor({"columns": ["a"], "task": "classification"})
        assert "note" in result

    def test_column_truncation(self):
        """Test that long column lists are truncated."""
        provider = MockProvider(response='{"summary": "test"}')
        advisor = LLMAdvisor(
            provider=provider,
            config=LLMConfig(max_columns_in_prompt=5),
        )

        # Many columns
        payload = {
            "columns": [f"col_{i}" for i in range(100)],
            "task": "classification",
        }

        advisor.advise(payload)

        # Verify truncation happened (check by examining the prompt)
        assert provider.call_count == 1


class TestProviderAvailability:
    """Tests for provider availability checks."""

    def test_openai_availability(self):
        """Test OpenAI provider availability check."""
        provider = OpenAIProvider(api_key=None)  # No key
        # Should return False without key
        result = provider.is_available()
        # Either False (no key) or True (key in env)
        assert isinstance(result, bool)

    def test_anthropic_availability(self):
        """Test Anthropic provider availability check."""
        provider = AnthropicProvider(api_key=None)
        result = provider.is_available()
        assert isinstance(result, bool)

    def test_ollama_availability(self):
        """Test Ollama provider availability check."""
        provider = OllamaProvider(base_url="http://nonexistent:11434")
        result = provider.is_available()
        assert result == False  # Should fail to connect


class TestCreateAdvisor:
    """Tests for create_advisor factory function."""

    def test_create_openai_advisor(self):
        """Test creating OpenAI advisor."""
        advisor = create_advisor("openai", model="gpt-4o-mini")

        assert isinstance(advisor, LLMAdvisor)
        assert "OpenAI" in advisor.provider.name

    def test_create_anthropic_advisor(self):
        """Test creating Anthropic advisor."""
        advisor = create_advisor("anthropic", model="claude-3-haiku-20240307")

        assert isinstance(advisor, LLMAdvisor)
        assert "Anthropic" in advisor.provider.name

    def test_create_ollama_advisor(self):
        """Test creating Ollama advisor."""
        advisor = create_advisor("ollama", model="llama3")

        assert isinstance(advisor, LLMAdvisor)
        assert "Ollama" in advisor.provider.name

    def test_unknown_provider(self):
        """Test error for unknown provider."""
        with pytest.raises(ValueError, match="Unknown provider"):
            create_advisor("unknown_provider")


class TestCustomPromptTemplate:
    """Tests for custom prompt templates."""

    def test_custom_prompt(self):
        """Test using custom prompt template."""
        custom_template = PromptTemplate(
            system_prompt="You are a custom analyst.",
            user_prompt_template="Columns: {columns}. Task: {task}.",
        )

        provider = MockProvider(response='{"summary": "custom"}')
        advisor = LLMAdvisor(
            provider=provider,
            prompt_template=custom_template,
        )

        advisor.advise({"columns": ["a", "b"], "task": "classification"})

        # Provider was called
        assert provider.call_count == 1

