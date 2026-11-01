"""
ACDSO v2.0 - LLM Advisor Examples
==================================

This script demonstrates configuring and using LLM advisors
for semantic data analysis in ACDSO.

The LLM advisor detects issues that statistical analysis cannot find:
- Semantic leakage (column names suggesting target information)
- Temporal leakage (features looking into the future)
- Data quality concerns based on naming patterns
"""

import numpy as np
import pandas as pd
from sklearn.datasets import make_classification
from sklearn.model_selection import train_test_split


def example_openai_advisor():
    """
    Example: Using OpenAI as the LLM provider.

    Requires:
    - pip install openai
    - OPENAI_API_KEY environment variable set
    """
    print("=" * 60)
    print("OpenAI LLM Advisor Example")
    print("=" * 60)

    from acdso import ACDSOClassifier, create_advisor, OpenAIProvider

    # Check if OpenAI is available
    provider = OpenAIProvider()
    if not provider.is_available():
        print("OpenAI not available. Set OPENAI_API_KEY environment variable.")
        print("Skipping this example.\n")
        return None

    # Create advisor using factory function
    advisor = create_advisor(
        provider="openai",
        model="gpt-4o-mini",  # Cost-effective model
    )

    # Create synthetic data with suspicious column names
    np.random.seed(42)
    n = 200
    X = pd.DataFrame({
        "customer_age": np.random.randint(18, 80, n),
        "income": np.random.randint(20000, 150000, n),
        "credit_score": np.random.randint(300, 850, n),
        "future_default": np.random.randn(n),  # SUSPICIOUS: future info
        "outcome_label": np.random.randn(n),   # SUSPICIOUS: target leak
        "will_churn": np.random.randn(n),      # SUSPICIOUS: future info
    })
    y = np.random.randint(0, 2, n)

    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=42)

    # Create classifier with LLM advisor
    clf = ACDSOClassifier(
        llm_advisor=advisor,
        scorer="f1",
        cv=3,
        verbose=1,
    )

    print("\nFitting with OpenAI LLM advisor...")
    clf.fit(X_train, y_train)

    # Show LLM analysis results
    print("\nRisk Report Notes (including LLM analysis):")
    for note in clf.risk_report_.get("notes", []):
        print(f"  - {note}")

    return clf


def example_anthropic_advisor():
    """
    Example: Using Anthropic Claude as the LLM provider.

    Requires:
    - pip install anthropic
    - ANTHROPIC_API_KEY environment variable set
    """
    print("\n" + "=" * 60)
    print("Anthropic Claude LLM Advisor Example")
    print("=" * 60)

    from acdso import AnthropicProvider, LLMAdvisor, LLMConfig

    # Check if Anthropic is available
    provider = AnthropicProvider()
    if not provider.is_available():
        print("Anthropic not available. Set ANTHROPIC_API_KEY environment variable.")
        print("Skipping this example.\n")
        return None

    # Create advisor with custom configuration
    config = LLMConfig(
        max_tokens=800,
        temperature=0.2,
        cache_enabled=True,
    )

    advisor = LLMAdvisor(
        provider=AnthropicProvider(model="claude-3-haiku-20240307"),
        config=config,
    )

    print("Anthropic advisor configured successfully.")
    print(f"Model: {advisor.provider.name}")

    return advisor


def example_ollama_advisor():
    """
    Example: Using Ollama for free, local LLM inference.

    Requires:
    - Ollama installed: https://ollama.ai
    - A model pulled: ollama pull llama3.2
    """
    print("\n" + "=" * 60)
    print("Ollama (Local) LLM Advisor Example")
    print("=" * 60)

    from acdso import OllamaProvider, LLMAdvisor

    # Check if Ollama is running
    provider = OllamaProvider(model="llama3.2")
    if not provider.is_available():
        print("Ollama not available. Make sure Ollama is running:")
        print("  1. Install from https://ollama.ai")
        print("  2. Run: ollama pull llama3.2")
        print("  3. Ollama should be running on localhost:11434")
        print("Skipping this example.\n")
        return None

    advisor = LLMAdvisor(provider=provider)

    print("Ollama advisor configured successfully.")
    print(f"Model: {advisor.provider.name}")
    print("Note: First request may be slow as model loads.")

    return advisor


def example_custom_prompt():
    """
    Example: Customizing the LLM prompt template.
    """
    print("\n" + "=" * 60)
    print("Custom Prompt Template Example")
    print("=" * 60)

    from acdso import PromptTemplate, LLMAdvisor, OpenAIProvider

    # Define custom prompts
    custom_prompt = PromptTemplate(
        system_prompt="""You are a financial data scientist specializing in
credit risk modeling and regulatory compliance. Analyze datasets for:
1. Features that could lead to unfair lending decisions
2. Data leakage from future information
3. GDPR/PII concerns in column names
Be concise and specific.""",

        user_prompt_template="""Review this credit scoring dataset:

Columns: {columns}
Task: {task}
Target: {target_type}
Size: {n_rows} rows

Statistical notes: {notes}

Identify concerns and respond in JSON format:
{{
    "leakage_risk": {{"severity": "...", "columns": [...]}},
    "fairness_concerns": ["..."],
    "pii_columns": ["..."],
    "summary": "..."
}}""",
    )

    print("Custom prompt template defined:")
    print(f"  System prompt: {custom_prompt.system_prompt[:80]}...")
    print(f"  User template has {len(custom_prompt.user_prompt_template)} chars")

    # Use with advisor
    # advisor = LLMAdvisor(
    #     provider=OpenAIProvider(),
    #     prompt_template=custom_prompt,
    # )

    return custom_prompt


def example_caching_and_fallback():
    """
    Example: Using caching and fallback providers.
    """
    print("\n" + "=" * 60)
    print("Caching and Fallback Example")
    print("=" * 60)

    from acdso import (
        LLMAdvisor,
        LLMConfig,
        OpenAIProvider,
        OllamaProvider,
        StubLLMAdvisor,
    )

    # Configure caching
    config = LLMConfig(
        cache_enabled=True,
        cache_ttl_s=3600,  # 1 hour cache
        max_retries=3,
        retry_delay_s=1.0,
    )

    print("Caching configuration:")
    print(f"  Enabled: {config.cache_enabled}")
    print(f"  TTL: {config.cache_ttl_s}s")
    print(f"  Max retries: {config.max_retries}")

    # Example with fallback (commented to avoid API calls)
    # advisor = LLMAdvisor(
    #     provider=OpenAIProvider(model="gpt-4o"),  # Primary
    #     fallback_provider=OllamaProvider(model="llama3"),  # Fallback
    #     config=config,
    # )

    print("\nFallback pattern:")
    print("  Primary: OpenAI gpt-4o")
    print("  Fallback: Ollama llama3 (local)")

    return config


def example_stub_advisor():
    """
    Example: Using the keyword-based stub advisor (no LLM required).
    """
    print("\n" + "=" * 60)
    print("Stub Advisor (No LLM) Example")
    print("=" * 60)

    from acdso import ACDSOClassifier, llm_advisor_stub, StubLLMAdvisor

    # Create data with suspicious column names
    np.random.seed(42)
    n = 100
    X = pd.DataFrame({
        "age": np.random.randint(18, 80, n),
        "income": np.random.randint(20000, 150000, n),
        "target_value": np.random.randn(n),    # Will be flagged
        "future_outcome": np.random.randn(n),  # Will be flagged
        "prediction_label": np.random.randn(n), # Will be flagged
    })
    y = np.random.randint(0, 2, n)

    # Use stub advisor (keyword matching, no API)
    clf = ACDSOClassifier(
        llm_advisor=llm_advisor_stub,
        scorer="accuracy",
        cv=3,
        verbose=1,
    )

    clf.fit(X, y)

    print("\nRisk notes (keyword-based analysis):")
    for note in clf.risk_report_.get("notes", []):
        print(f"  - {note}")

    # Direct use of stub advisor
    advisor = StubLLMAdvisor()
    result = advisor.advise({
        "columns": list(X.columns),
        "task": "classification",
        "target_type": "binary",
    })

    print(f"\nDirect stub result:")
    print(f"  Leakage severity: {result['leakage_severity']}")
    print(f"  Suspicious columns: {result['suspicious_columns']}")

    return clf


def example_litellm_provider():
    """
    Example: Using LiteLLM for 100+ model providers.

    LiteLLM provides a unified interface to many LLM providers:
    - OpenAI, Anthropic, Cohere, Replicate, HuggingFace, etc.

    Requires:
    - pip install litellm
    """
    print("\n" + "=" * 60)
    print("LiteLLM Multi-Provider Example")
    print("=" * 60)

    from acdso import LiteLLMProvider, LLMAdvisor

    # Check if LiteLLM is available
    provider = LiteLLMProvider(model="gpt-4o-mini")
    if not provider.is_available():
        print("LiteLLM not installed. Run: pip install litellm")
        print("Skipping this example.\n")
        return None

    print("LiteLLM supports many providers with same interface:")
    print("  - OpenAI: model='gpt-4o'")
    print("  - Anthropic: model='claude-3-sonnet-20240229'")
    print("  - Cohere: model='cohere/command-r'")
    print("  - Ollama: model='ollama/llama3'")
    print("  - Replicate: model='replicate/...'")
    print("  - HuggingFace: model='huggingface/...'")

    return provider


if __name__ == "__main__":
    # Run examples that don't require API keys first
    example_stub_advisor()
    example_custom_prompt()
    example_caching_and_fallback()

    # Try examples that require external services
    example_ollama_advisor()
    example_litellm_provider()
    example_anthropic_advisor()
    example_openai_advisor()

    print("\n" + "=" * 60)
    print("All examples completed!")
    print("=" * 60)

