# Contributing to AtomicSettle

Thank you for your interest in contributing to AtomicSettle! This document provides guidelines for contributing to the project.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please read and follow our Code of Conduct.

## How to Contribute

### Reporting Issues

- Search existing issues before creating a new one
- Use issue templates when available
- Provide as much detail as possible
- Include steps to reproduce for bugs

### Feature Requests

- Check if the feature has already been requested
- Explain the use case clearly
- Consider if it aligns with project goals

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Write/update tests
5. Run the test suite
6. Commit with clear messages
7. Push to your fork
8. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.75+
- Python 3.9+
- PostgreSQL 15+
- Redis 7+

### Building

```bash
# Clone the repository
git clone https://github.com/atomicsettle/atomicsettle.git
cd atomicsettle

# Build Rust components
cd reference
cargo build

# Install Python SDK in development mode
cd ../sdk/python
pip install -e ".[dev]"
```

### Running Tests

```bash
# Rust tests
cd reference
cargo test

# Python tests
cd sdk/python
pytest

# Integration tests
cd tests
./run_integration_tests.sh
```

### Code Style

#### Rust

- Follow standard Rust style (rustfmt)
- Run `cargo clippy` before committing
- Document public APIs

```bash
cargo fmt
cargo clippy --all-targets
```

#### Python

- Follow PEP 8
- Use type hints
- Run ruff and mypy

```bash
ruff check .
mypy .
```

## Commit Messages

Follow conventional commits:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

Examples:
```
feat(coordinator): add netting support
fix(sdk-python): handle connection timeout
docs: update integration guide
```

## Pull Request Process

1. Update documentation if needed
2. Add tests for new functionality
3. Ensure CI passes
4. Request review from maintainers
5. Address review feedback
6. Squash commits if requested

## Architecture Decisions

For significant changes, create an Architecture Decision Record (ADR):

1. Create `docs/adr/NNNN-title.md`
2. Use the ADR template
3. Discuss in issue before implementing

## Security Issues

**Do not** report security vulnerabilities through public issues.

Email security@atomicsettle.network with:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License.

## Questions?

- Open a discussion on GitHub
- Join our Slack channel
- Email contributors@atomicsettle.network

Thank you for contributing!
