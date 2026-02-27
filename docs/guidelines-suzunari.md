# Suzunari Error Library Guidelines

This document provides specific guidelines for working with the Suzunari Error library. These guidelines complement the general Rust guidelines and are focused on the specific patterns and practices recommended for this project.

## Library Purpose

Suzunari Error is designed to provide:

- A highly traceable error system that propagates error locations as error contexts
- A noise-free approach to error handling that minimizes unnecessary information in logs
- Utilities to simplify error definition and handling
- Integration with SNAFU for easily constructing contextual errors

## Core Components

### StackError Trait

The `StackError` trait is the foundation of the Suzunari Error approach:

- Provides error location awareness for contextual chained errors
- `location()` — returns the `Location` where this error was constructed
- `type_name()` — returns the error type name for display in stack traces
- `stack_source()` — returns the source error as a `StackError` if it implements the trait (uses Deref-based specialization in generated code)
- `depth()` — counts the full `Error::source()` chain length

### StackReport

`StackReport<E>` formats a `StackError` chain as a stack-trace-like report. Use at error display boundaries (e.g., `main()`, HTTP handlers, log output).

- Wraps `Result<(), E>` and provides `Debug`/`Display` output
- Phase 1: traverses `stack_source()` chain (with type name + location)
- Phase 2: traverses remaining `Error::source()` chain (without location)
- Create via `StackReport::from_error(e)` or `Result::into()`
- Implements `Termination` (std only) for use as `main()` return type

### Macros

- **`#[suzunari_error]`**: The main entry point for defining error types. Processes `#[suzu(...)]` attributes, injects `location: Location` fields, and appends `#[derive(Debug, Snafu, StackError)]`
- `#[derive(StackError)]`: Implements the StackError trait for structs and enums. Does NOT generate `Debug` — use `#[derive(Debug)]` or `#[suzunari_error]`
- `#[suzunari_error::report]`: Transforms `fn main() -> Result<(), E>` to return `StackReport<E>`. Prints formatted error chain to stderr on failure (std only)

### `#[suzu(...)]` Attribute

`#[suzu(...)]` is a superset of `#[snafu(...)]`. All snafu keywords are passed through as-is. Additionally:

- **`translate`** (field-level): Wraps the field type in `DisplayError<T>` and generates `#[snafu(source(from(T, DisplayError::new)))]`
- **`location`** (field-level): Marks a field as the location field and adds `#[snafu(implicit)]`. Suppresses automatic location injection for that struct/variant

### Location Structure

- Memory-efficient structure for storing error location information
- Compatible with SNAFU's implicit context system
- Provides file, line, and column information for error tracing

## Error Design Principles

1. **Traceability**: Errors should be traceable through the call stack
   - Use the StackError pattern to track error propagation
   - Include contextual information at each level of the stack

2. **Context-Rich**: Errors should provide sufficient context for debugging
   - Include relevant parameters and state information in error types
   - Use descriptive error messages that explain what went wrong

3. **Noise-Free**: Error handling should minimize unnecessary noise
   - Avoid duplicating information in error messages
   - Focus on essential information for debugging

4. **Hierarchical**: Error types should form a logical hierarchy
   - Use enum variants for different error categories
   - Chain errors using the `source` field to maintain the error trail

5. **Performance-Conscious**: Error handling should be efficient
   - Use the memory-efficient Location structure
   - Avoid expensive operations in error paths
   - Defer formatting until errors are actually displayed

## Recommended Patterns

### Error Type Definition

```rust
#[suzunari_error]
pub enum DatabaseError {
    #[suzu(display("connection to {connection_string} failed"))]
    ConnectionFailed {
        connection_string: String,
        source: std::io::Error,
    },

    #[suzu(display("query execution failed"))]
    QueryFailed {
        query: String,
        source: sqlx::Error,
    },

    #[suzu(display("record {id} not found in {table}"))]
    RecordNotFound {
        id: String,
        table: String,
    },

    // #[suzu(translate)] wraps non-Error types in DisplayError automatically
    #[suzu(display("hashing failed"))]
    HashFailed {
        #[suzu(translate)]
        source: argon2::Error,
    },
}
```

### Error Propagation

```rust
use snafu::OptionExt;

fn get_user(id: &str, conn: &Connection) -> Result<User, DatabaseError> {
    let query = format!("SELECT * FROM users WHERE id = '{}'", id);

    let result = conn.execute(&query)
        .context(QueryFailedSnafu { query })?;

    result.first().context(RecordNotFoundSnafu {
        id: id.to_string(),
        table: "users".to_string(),
    })
}
```

### Error Handling

```rust
fn process_user_data(id: &str) -> Result<ProcessedData, ApplicationError> {
    let conn = establish_connection()
        .context(DatabaseConnectionSnafu)?;
        
    let user = get_user(id, &conn)
        .context(UserRetrievalSnafu { user_id: id.to_string() })?;
        
    // Process user data
    Ok(ProcessedData::from_user(user))
}
```

## Testing Error Handling

- Test both success and error paths
- Verify that errors contain the expected context
- Check that error messages are helpful and descriptive
- Ensure errors are properly propagated through the call stack

```rust
#[test]
fn test_error_propagation() {
    let result = get_user("non_existent", &mock_connection());
    
    match result {
        Err(DatabaseError::RecordNotFound { id, table }) => {
            assert_eq!(id, "non_existent");
            assert_eq!(table, "users");
        },
        _ => panic!("Expected RecordNotFound error"),
    }
}
```

## Documentation

- Document error types thoroughly
- Explain what each error variant represents
- Provide examples of how to handle specific errors
- Include information about error context fields

By following these guidelines, you'll ensure that error handling in the Suzunari Error project is consistent, traceable, and maintainable.
