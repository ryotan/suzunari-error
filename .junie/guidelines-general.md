# General Development Guidelines

This document outlines the high-level development guidelines for this project. These guidelines apply to all
aspects of the project, regardless of the programming language or technology used.

## 1. Application Basic Policy

- Designed as a collection of personal utility tools
- Keep each feature capable of operating independently
- Maintain a simple and intuitive UI

## 2. UI/UX Guidelines

- Unified design system based on Radix UI
- Full support for keyboard operation
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

- Target memory usage below 100MB
- Response time for each operation within 500ms

## 8. Cross-Platform Support

- Provide equivalent functionality on Windows/macOS/Linux
- Establish an abstraction layer for OS-specific features
- Separate platform-specific settings
