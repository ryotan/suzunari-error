# Rust Style Guide

This style guide outlines the conventions and best practices for Rust code in this project. Following these
guidelines will ensure code consistency, readability, and maintainability across the codebase.

## Code Formatting

- Use `cargo fmt` before committing code to ensure consistent formatting

## Naming Conventions

- Use **snake_case** for variables, functions, methods, and modules
- Use **PascalCase** for types, traits, enums, and struct names
- Use **SCREAMING_SNAKE_CASE** for constants and static variables
- Use descriptive names that clearly convey purpose
- Prefer complete words over abbreviations unless the abbreviation is more widely recognized

## Code Organization

- Organize code into modules based on functionality
- Keep files focused on a single responsibility
- Follow the standard Tauri application structure:
  - `main.rs`: Application entry point
  - `lib.rs`: Core application logic
  - Organize related functionality into separate modules

## Error Handling

- Use `Result<T, E>` for operations that can fail
- Create custom error types using `snafu` for domain-specific errors with context
- Use the StackError pattern for better error tracing and context (see `app/src-tauri/src/error.rs` for implementation)
- Prefer using the `?` operator for error propagation over `match` or `unwrap()`
- Avoid using `unwrap()` or `expect()` in production code
- Use meaningful error messages that help with debugging
- Add context to errors when propagating them up the call stack
- Implement the StackError pattern for complex applications to improve error tracing

## Documentation

- Document all public APIs using rustdoc comments (`///`)
- Include examples in documentation where appropriate
- Document complex or non-obvious implementations with regular comments (`//`)
- Use `//TODO:` or `//FIXME:` comments for temporary solutions or known issues

## Functional Programming Style

- Prefer immutable variables (`let` without `mut`) when possible
- Use iterator methods (`map`, `filter`, `fold`) instead of explicit loops when appropriate
- Use closures for short, one-off functions
- Leverage Rust's pattern matching capabilities for cleaner code

## Performance Considerations

- Be mindful of memory allocations, especially in performance-critical code
- Use references (`&T`) instead of cloning when possible
- Consider using `Cow<T>` for values that may or may not need to be owned
- Profile before optimizing; avoid premature optimization

## Concurrency

- Use `async`/`await` for asynchronous operations, especially when interacting with Tauri APIs
- Be explicit about thread safety using appropriate types (`Arc`, `Mutex`, etc.)
- Prefer message passing over shared state when possible
- Document thread safety assumptions in multi-threaded code

## Testing

- Write unit tests for all public functions
- Use `#[cfg(test)]` module for test code
- Name test functions descriptively, using the pattern `test_<functionality_being_tested>`
- Use meaningful assertions that clearly indicate what's being tested

## Tauri-Specific Guidelines

- Register commands with appropriate permissions
- Use proper error handling in commands that can be called from the frontend
- Serialize/deserialize data using `serde` with consistent patterns
- Keep command handlers small and focused on a single responsibility
- Use Tauri's logging facilities for debugging and error reporting

## Common Pitfalls to Avoid

- Excessive cloning of data
- Deeply nested error handling
- Overuse of macros for simple tasks
- Premature optimization
- Inconsistent error handling patterns
- Mixing synchronous and asynchronous code without clear boundaries

## Dependencies

- Be selective about adding new dependencies
- Prefer well-maintained crates with good documentation
- Consider the licensing implications of dependencies
- Keep dependencies up to date using `cargo update`

By following these guidelines, we can maintain a high-quality, consistent, and maintainable Rust codebase for this project.
