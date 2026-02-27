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
- Follow the standard Rust library structure:
  - `lib.rs`: Main library entry point and API
  - Organize related functionality into separate modules

## Error Handling

### Core Principles

- Use `Result<T, E>` for operations that can fail
- Create custom error types using `snafu` for domain-specific errors with context
- Prefer using the `?` operator for error propagation over `match` or `unwrap()`
- Avoid using `unwrap()` or `expect()` in production code
- Use meaningful error messages that help with debugging

### Suzunari Error Approach

- Use `#[suzunari_error]` for defining error types (location injection + `#[derive(Debug, Snafu, StackError)]`)
- Structure error types to capture relevant context:
  - Include fields that provide context about the error situation
  - Use the `source` field to chain errors
  - Use descriptive display messages with context variables
- Add context when propagating errors up the call stack:
  - Use `.context(ErrorContextSnafu { context_var })` to add context
  - Use `ensure!()` for validation checks that can result in errors
- Design error types for optimal debugging:
  - Use `StackReport` to format error chains with location info at display boundaries
  - Keep error types focused and specific to their domain
  - Group related errors in enum variants
- Use the memory-efficient `Location` structure for error context
  - Compatible with SNAFU's implicit context
  - Minimizes memory overhead while providing traceability

### Example Pattern

```rust
#[suzunari_error]
enum ApiError {
    #[snafu(display("data fetch failed"))]
    FetchFailed {
        source: reqwest::Error,
    },

    #[snafu(display("invalid parameter '{param_name}': {reason}"))]
    ValidationFailed {
        param_name: String,
        reason: String,
    },
}

fn fetch_data(url: &str) -> Result<Data, ApiError> {
    let response = reqwest::get(url)
        .await
        .context(FetchFailedSnafu)?;

    // Validation example
    ensure!(
        response.status().is_success(),
        ValidationFailedSnafu {
            param_name: "url",
            reason: format!("received status code {}", response.status())
        }
    );

    Ok(Data::from_response(response))
}
```

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

- Use `async`/`await` for asynchronous operations when appropriate
- Be explicit about thread safety using appropriate types (`Arc`, `Mutex`, etc.)
- Prefer message passing over shared state when possible
- Document thread safety assumptions in multi-threaded code

## Testing

- Write unit tests for all public functions
- Use `#[cfg(test)]` module for test code
- Name test functions descriptively, using the pattern `test_<functionality_being_tested>`
- Use meaningful assertions that clearly indicate what's being tested

## Library-Specific Guidelines

- Design clear and intuitive public APIs
- Use proper error handling for library functions
- Serialize/deserialize data using `serde` with consistent patterns when needed
- Keep public functions focused on a single responsibility
- Use appropriate logging levels for debugging and error reporting
- Consider API stability and backward compatibility

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
