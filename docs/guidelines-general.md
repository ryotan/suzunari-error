# General Development Guidelines

This document outlines the high-level development guidelines for this project. These guidelines apply to all
aspects of the project, regardless of the programming language or technology used.

## 1. Library Basic Policy

- Designed as a reusable error handling library
- Keep each feature well-documented and easy to use
- Maintain a simple and intuitive API

## 2. API Design Guidelines

- Consistent naming conventions across the API
- Clear and predictable function behavior
- Error messages should be specific and practical

## 3. Security Considerations

- Error messages should not leak sensitive information (e.g., credentials, connection strings)
- Callers should be mindful of what context they include in error fields

## 4. Testing Policy

- Unit tests are mandatory
- E2E tests are mandatory only for important features

## 5. Documentation Conventions

- Prepare README.md for each feature
- Generate API documentation automatically
- Include concrete examples for error cases

## 6. Versioning

- Follow semantic versioning

## 7. Performance Requirements

- Minimize memory overhead for error types
  - Use compact representations for error context data
  - Avoid unnecessary cloning of error information
  - Consider using references where appropriate
- Avoid expensive operations in error handling paths
  - Keep error construction and propagation lightweight
  - Defer expensive formatting until errors are actually displayed
  - Minimize allocations during error creation and handling
- Consider performance implications of macro expansions
  - Ensure generated code is efficient and minimal
  - Avoid complex recursive macro expansions
  - Test compile times with and without macro usage
- Balance between error context richness and performance
  - Provide sufficient context for debugging without excessive overhead
  - Consider optional detailed context that can be enabled in debug builds
- Consider benchmarking error handling performance if performance regressions are suspected

## 8. `no_std` Compatibility

- The library supports three feature tiers: core-only, `alloc`, and `std`
- Use `#[cfg(feature = "...")]` to gate tier-specific functionality
- Test all three tiers in CI to prevent accidental `std` dependency leaks
- `Location` relies on `core::panic::Location` — available in all tiers
