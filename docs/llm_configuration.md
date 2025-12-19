# LLM Advisor Configuration Guide

ACDSO supports LLM-based semantic analysis to detect data issues that statistical methods cannot find. This guide covers all configuration options.

## Quick Start

```python
from acdso import ACDSOClassifier, create_advisor

# Simplest usage (OpenAI)
advisor = create_advisor("openai")
clf = ACDSOClassifier(llm_advisor=advisor)
```

## Provider Setup

### OpenAI

**Requirements:**
- `pip install acdso[openai]`
- Set `OPENAI_API_KEY` environment variable

```python
from acdso import create_advisor, OpenAIProvider, LLMAdvisor

# Using factory
advisor = create_advisor("openai", model="gpt-4o-mini")

# Using provider directly
provider = OpenAIProvider(
    api_key="sk-...",  # Or use env var
    model="gpt-4o",
)
advisor = LLMAdvisor(provider=provider)
```

**Available Models:**
- `gpt-4o` - Most capable
- `gpt-4o-mini` - Cost-effective (recommended)
- `gpt-4-turbo` - Fast and capable
- `gpt-3.5-turbo` - Budget option

### Anthropic Claude

**Requirements:**
- `pip install acdso[anthropic]`
- Set `ANTHROPIC_API_KEY` environment variable

```python
from acdso import create_advisor, AnthropicProvider

advisor = create_advisor("anthropic", model="claude-3-haiku-20240307")

# Or directly
provider = AnthropicProvider(
    api_key="sk-ant-...",
    model="claude-3-sonnet-20240229",
)
```

**Available Models:**
- `claude-3-opus-20240229` - Most capable
- `claude-3-sonnet-20240229` - Balanced
- `claude-3-haiku-20240307` - Fast and cheap (recommended)

### Azure OpenAI

**Requirements:**
- `pip install acdso[openai]`
- Set `AZURE_OPENAI_API_KEY` and `AZURE_OPENAI_ENDPOINT`

```python
from acdso import AzureOpenAIProvider, LLMAdvisor

provider = AzureOpenAIProvider(
    api_key="your-key",
    endpoint="https://your-resource.openai.azure.com",
    deployment_name="gpt-4",
    api_version="2024-02-15-preview",
)
advisor = LLMAdvisor(provider=provider)
```

### Ollama (Local, Free)

**Requirements:**
- Install Ollama: https://ollama.ai
- Pull a model: `ollama pull llama3.2`

```python
from acdso import create_advisor, OllamaProvider

# Default (localhost:11434)
advisor = create_advisor("ollama", model="llama3.2")

# Custom URL
provider = OllamaProvider(
    model="mistral",
    base_url="http://192.168.1.100:11434",
)
```

**Recommended Models:**
- `llama3.2` - Good balance
- `mistral` - Fast
- `codellama` - Code-focused

### LiteLLM (100+ Providers)

**Requirements:**
- `pip install acdso[litellm]`

```python
from acdso import LiteLLMProvider, LLMAdvisor

# Use any LiteLLM-supported model
provider = LiteLLMProvider(model="gpt-4o")
provider = LiteLLMProvider(model="claude-3-sonnet-20240229")
provider = LiteLLMProvider(model="ollama/llama3")
provider = LiteLLMProvider(model="cohere/command-r")

advisor = LLMAdvisor(provider=provider)
```

## Configuration Options

### LLMConfig

```python
from acdso import LLMConfig, LLMAdvisor, OpenAIProvider

config = LLMConfig(
    max_tokens=1000,          # Max response length
    temperature=0.2,          # Lower = more deterministic
    timeout_s=30.0,           # Request timeout
    max_retries=3,            # Retry on failure
    retry_delay_s=1.0,        # Initial retry delay (exponential backoff)
    cache_enabled=True,       # Cache responses
    cache_ttl_s=3600,         # Cache for 1 hour
    max_columns_in_prompt=50, # Truncate long column lists
)

advisor = LLMAdvisor(
    provider=OpenAIProvider(),
    config=config,
)
```

### Custom Prompts

```python
from acdso import PromptTemplate, LLMAdvisor, OpenAIProvider

template = PromptTemplate(
    system_prompt="""You are a financial data scientist specializing in
credit risk modeling. Analyze datasets for regulatory compliance and
data leakage issues.""",

    user_prompt_template="""Analyze this dataset:

Columns: {columns}
Task: {task}
Target: {target_type}
Size: {n_rows} rows × {n_cols} columns

Known issues: {notes}

Respond in JSON:
{{
    "leakage_risk": {{"severity": "...", "suspicious_columns": [...]}},
    "recommendations": [...]
}}""",
)

advisor = LLMAdvisor(
    provider=OpenAIProvider(),
    prompt_template=template,
)
```

### Fallback Providers

```python
from acdso import LLMAdvisor, OpenAIProvider, OllamaProvider

# Primary: OpenAI, Fallback: Local Ollama
advisor = LLMAdvisor(
    provider=OpenAIProvider(model="gpt-4o"),
    fallback_provider=OllamaProvider(model="llama3.2"),
)
```

## Caching

Caching reduces API costs by storing responses for similar datasets.

```python
from acdso import LLMConfig, LLMAdvisor, OpenAIProvider

# Enable caching with 1-hour TTL
config = LLMConfig(
    cache_enabled=True,
    cache_ttl_s=3600,
)

advisor = LLMAdvisor(
    provider=OpenAIProvider(),
    config=config,
)

# Same columns + task = cached response
advisor.advise({"columns": ["a", "b"], "task": "classification"})  # API call
advisor.advise({"columns": ["a", "b"], "task": "classification"})  # Cached
```

## Without LLM

For testing or when API costs are a concern:

```python
from acdso import ACDSOClassifier, llm_advisor_stub, StubLLMAdvisor

# Function-based stub
clf = ACDSOClassifier(llm_advisor=llm_advisor_stub)

# Class-based stub (for customization)
advisor = StubLLMAdvisor()
clf = ACDSOClassifier(llm_advisor=advisor)
```

## Environment Variables

| Provider | Variable | Example |
|----------|----------|---------|
| OpenAI | `OPENAI_API_KEY` | `sk-...` |
| Anthropic | `ANTHROPIC_API_KEY` | `sk-ant-...` |
| Azure | `AZURE_OPENAI_API_KEY` | `...` |
| Azure | `AZURE_OPENAI_ENDPOINT` | `https://....openai.azure.com` |

## Cost Optimization

1. **Use caching**: Avoids duplicate API calls
2. **Use cheaper models**: `gpt-4o-mini` or `claude-3-haiku`
3. **Use Ollama**: Free, local inference
4. **Truncate columns**: Set `max_columns_in_prompt`
5. **Use fallback**: Primary cloud, fallback local

```python
config = LLMConfig(
    cache_enabled=True,
    max_columns_in_prompt=30,  # Reduce token usage
)

advisor = LLMAdvisor(
    provider=OpenAIProvider(model="gpt-4o-mini"),  # Cheap
    fallback_provider=OllamaProvider(model="llama3.2"),  # Free
    config=config,
)
```

## Error Handling

The advisor handles errors gracefully:

```python
advisor = LLMAdvisor(
    provider=OpenAIProvider(),
    config=LLMConfig(max_retries=3),
)

# If all retries fail, returns:
# {"note": "LLM advisor error: ...", "suspicious_columns": []}
```

## Testing Configuration

```python
# Check if provider is available
from acdso import OpenAIProvider

provider = OpenAIProvider()
print(f"Available: {provider.is_available()}")

# Test advisor
from acdso import create_advisor

advisor = create_advisor("openai")
result = advisor.advise({
    "columns": ["target_value", "future_outcome"],
    "task": "classification",
    "target_type": "binary",
})
print(result["note"])
```

