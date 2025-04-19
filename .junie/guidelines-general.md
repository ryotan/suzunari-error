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

## 3. Security Policy

- Request only the minimum necessary system permissions
- Store sensitive data securely
- Operate offline by default

## 4. Testing Policy

- Unit tests are mandatory
- E2E tests are mandatory only for important features
- Target test coverage of 80% or higher

## 5. Documentation Conventions)

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
- Benchmark error handling performance
  - Compare against standard Rust error handling approaches
  - Measure impact on both happy and error paths

## 8. Cross-Platform Support

- Ensure library works consistently across Windows/macOS/Linux
- Use conditional compilation for platform-specific code
- Test on all supported platforms before release
