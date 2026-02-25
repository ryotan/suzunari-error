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
- Tracks error propagation through the call stack
- Enables rich debugging information while keeping error messages clean

### Derive Macros

- `#[derive(StackError)]`: Implements the StackError trait for structs and enums
- Used in conjunction with SNAFU's derive macro: `#[derive(Snafu, StackError)]`

### Attribute Macros

- `#[suzunari_location]`: Adds a location field to error types
- Works with SNAFU's implicit context to capture error locations

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
#[derive(Debug, Snafu, StackError)]
#[suzunari_location]
pub enum DatabaseError {
    #[snafu(display("Failed to connect to database at {}: {}", connection_string, source))]
    ConnectionFailed {
        connection_string: String,
        #[snafu(source)]
        source: std::io::Error,
    },
    
    #[snafu(display("Query execution failed: {}", source))]
    QueryFailed {
        query: String,  // Not displayed but available for debugging
        #[snafu(source)]
        source: sqlx::Error,
    },
    
    #[snafu(display("Record not found with ID: {}", id))]
    RecordNotFound {
        id: String,
        table: String,  // Not displayed but available for debugging
    },
}
```

### Error Propagation

```rust
fn get_user(id: &str, conn: &Connection) -> Result<User, DatabaseError> {
    let query = format!("SELECT * FROM users WHERE id = '{}'", id);
    
    let result = conn.execute(&query)
        .context(QueryFailedSnafu { query })?;
        
    result.first()
        .ok_or_else(|| RecordNotFoundSnafu {
            id: id.to_string(),
            table: "users".to_string(),
        }.build())
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
