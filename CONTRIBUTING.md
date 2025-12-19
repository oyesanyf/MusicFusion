# Contributing to ACDSO

Thank you for your interest in contributing to ACDSO! This document provides guidelines and instructions for contributing.

## Getting Started

### Development Setup

1. **Clone the repository**
```bash
git clone https://github.com/ds-tui/acdso.git
cd acdso
```

2. **Create a virtual environment**
```bash
python -m venv venv
source venv/bin/activate  # Linux/macOS
venv\Scripts\activate     # Windows
```

3. **Install in development mode**
```bash
pip install -e ".[dev,all]"
```

4. **Verify installation**
```bash
pytest tests/ -v
```

## Development Workflow

### Code Style

We use the following tools for code quality:

- **Black** for code formatting
- **isort** for import sorting
- **flake8** for linting
- **mypy** for type checking

Run all checks:
```bash
# Format code
black src/ tests/
isort src/ tests/

# Lint
flake8 src/ tests/

# Type check
mypy src/acdso/
```

### Running Tests

```bash
# Run all tests
pytest tests/ -v

# Run with coverage
pytest tests/ -v --cov=acdso --cov-report=html

# Run specific test file
pytest tests/test_classifier.py -v

# Run specific test
pytest tests/test_classifier.py::TestACDSOClassifierBasic::test_fit_predict_binary -v
```

### Pre-commit Checks

Before committing, ensure:
1. All tests pass
2. Code is formatted with Black
3. Imports are sorted with isort
4. No linting errors from flake8

## Project Structure

```
acdso/
├── src/
│   └── acdso/
│       ├── __init__.py          # Package exports
│       ├── core.py              # ACDSOClassifier, ACDSORegressor
│       ├── risk.py              # Risk analysis
│       ├── pareto.py            # Pareto optimization
│       ├── utils.py             # Utility functions
│       ├── advisors.py          # LLM advisors
│       └── integrations/        # AutoML integrations
│           ├── __init__.py
│           ├── base.py
│           ├── sklearn_native.py
│           ├── external.py
│           └── meta_optimizer.py
├── tests/                       # Test files
├── examples/                    # Usage examples
├── pyproject.toml              # Package configuration
└── README.md                   # Documentation
```

## Contribution Types

### Bug Reports

When reporting bugs, please include:
- Python version
- ACDSO version (`import acdso; print(acdso.__version__)`)
- Operating system
- Minimal reproducible example
- Full error traceback

### Feature Requests

For feature requests, please describe:
- The problem you're trying to solve
- Your proposed solution
- Alternative solutions you've considered

### Pull Requests

1. **Fork the repository**
2. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes**
4. **Add tests** for new functionality
5. **Update documentation** if needed
6. **Run tests and linting**
7. **Commit with a clear message**
   ```bash
   git commit -m "feat: add support for XYZ"
   ```
8. **Push and create a PR**

### Commit Message Format

We follow conventional commits:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Test changes
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Maintenance tasks

Examples:
```
feat: add support for custom scoring functions
fix: handle missing values in VIF calculation
docs: add examples for LLM advisor configuration
test: add tests for Pareto knee-point selection
```

## Adding New Features

### Adding a New LLM Provider

1. Create a new class in `advisors.py`:
```python
class NewProvider(BaseLLMProvider):
    def __init__(self, api_key=None, model="default"):
        self.api_key = api_key or os.environ.get("NEW_PROVIDER_API_KEY")
        self.model = model

    @property
    def name(self):
        return f"NewProvider/{self.model}"

    def is_available(self):
        # Check if provider is configured
        return self.api_key is not None

    def complete(self, system_prompt, user_prompt, config):
        # Implement API call
        pass
```

2. Add to `__init__.py` exports
3. Add tests in `test_advisors.py`
4. Update documentation

### Adding a New AutoML Integration

1. Create adapter in `integrations/`:
```python
class NewAutoMLAdapter(SklearnNativeAdapter):
    def __init__(self, **kwargs):
        super().__init__(name="NewAutoML", **kwargs)

    def is_available(self):
        try:
            import new_automl
            return True
        except ImportError:
            return False

    def generate_candidates(self, X, y, task):
        # Run AutoML and return candidates
        pass
```

2. Add to `integrations/__init__.py`
3. Add tests
4. Update documentation

### Adding a New Risk Metric

1. Add detection function in `risk.py`:
```python
def _detect_new_risk(df, y, **kwargs):
    # Implement detection logic
    return severity, details
```

2. Update `analyze_risk()` to call the new function
3. Add field to `RiskReport` dataclass
4. Update `compute_risk_penalty()` if needed
5. Add tests

## Testing Guidelines

### Test Organization

- One test file per module
- Test classes for logical grouping
- Use pytest fixtures for common setup
- Use parametrize for multiple test cases

### Test Naming

```python
class TestACDSOClassifier:
    def test_fit_predict_basic(self):
        """Test basic fit/predict functionality."""
        pass

    def test_constraint_filtering_rejects_slow_models(self):
        """Test that slow models are rejected by constraints."""
        pass
```

### Test Coverage

Aim for:
- 100% coverage on core functionality
- Edge cases and error handling
- Integration tests for key workflows

## Documentation

### Docstring Format

We use NumPy-style docstrings:

```python
def function(param1, param2):
    """
    Short description.

    Longer description if needed.

    Parameters
    ----------
    param1 : type
        Description of param1.
    param2 : type, optional
        Description of param2.

    Returns
    -------
    type
        Description of return value.

    Examples
    --------
    >>> function(1, 2)
    3
    """
    pass
```

### Updating README

When adding features:
1. Update the feature list
2. Add usage examples
3. Update API reference if needed

## Release Process

1. Update version in `pyproject.toml` and `__init__.py`
2. Update CHANGELOG.md
3. Create a git tag
4. Build and upload to PyPI

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Help others learn and grow

## Questions?

- Open a GitHub issue for bugs or features
- Start a GitHub discussion for questions
- Check existing issues/discussions first

Thank you for contributing to ACDSO! 🎉

