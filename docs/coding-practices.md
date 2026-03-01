# Coding Practices

This document outlines the coding practices and principles followed in this project. These practices guide our
development approach and help maintain code quality and consistency.

- Start small and extend incrementally
- Avoid excessive abstraction
- Focus on types rather than code
- Adjust approach according to complexity

### Functional approach (FP)

We prefer a functional programming approach for its clarity, testability, and reduced side effects:

- Function-first (structs and enums for data representation)
- Leverage Rust's strong type system
- Preference for pure functions (functions that always return the same output for the same input and have no side
  effects)
- Use immutable data structures when possible
- Use of functional update patterns (e.g., with struct updates)
- Isolate side-effects (keep I/O operations separate from business logic)
- Flatten conditional branches with early returns

### Test Driven Development (TDD)

We follow Test Driven Development to ensure code quality and maintainability:

- Red-Green-Refactor cycle (write a failing test, make it pass, then refactor)
- Treat tests as specifications (tests document what the code should do)
- Iterate in small units (make incremental changes with frequent testing)
- Continuous refactoring (improve code design without changing behavior)

### Testing strategy

Our testing approach focuses on efficiency and effectiveness:

- Priority for unit testing of pure functions (they're easier to test and provide high value)
- Repository testing with in-memory implementation (allows testing without external dependencies)
- Build testability into design (consider how code will be tested during design phase)
- Assert first: work backwards from expected results (start with the assertion and work backwards to setup)

## Implementation steps

We follow a structured approach to implementation that prioritizes type safety, testability, and separation of concerns.
The recommended sequence is:

1.**Type design**.

- First, define the types
- Represent the language of the domain with types.

2.**Implementation from pure functions**

- Functions without external dependencies first.
- Write tests first.

3.**Separate side-effects**.

- Push IO operations to function boundaries
- Use Result or Option types for operations with side-effects
- Consider using async/await for asynchronous operations

4.**Adapter implementation**

- Use traits for abstraction of external dependencies
- Implement traits for concrete implementations
- Create mock implementations for testing

## Rust Examples

Below are concrete Rust examples that demonstrate the principles outlined above:

### Functional Programming Example

```rust
// Example of pure function with immutable data
fn calculate_total(items: &[Item]) -> f64 {
    items.iter()
        .map(|item| item.price * item.quantity as f64)
        .sum()
}

// Example of using Option for safe operations
fn find_item_by_id(items: &[Item], id: u32) -> Option<&Item> {
    items.iter().find(|item| item.id == id)
}

// Example of functional update pattern with structs
fn apply_discount(item: Item, discount_percent: f64) -> Item {
    Item {
        price: item.price * (1.0 - discount_percent / 100.0),
        ..item // Rest of the fields remain the same
    }
}
```

### Testing Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_total() {
        // Arrange
        let items = vec![
            Item { id: 1, name: "Item 1".to_string(), price: 10.0, quantity: 2 },
            Item { id: 2, name: "Item 2".to_string(), price: 15.0, quantity: 1 },
        ];

        // Act
        let total = calculate_total(&items);

        // Assert
        assert_eq!(total, 35.0);
    }

    #[test]
    fn test_order_cannot_be_submitted_when_empty() {
        // Arrange
        let mut order = Order {
            id: OrderId::new(),
            items: vec![],
            status: OrderStatus::Draft,
        };

        // Act
        let result = order.submit();

        // Assert
        assert!(matches!(result, Err(DomainError::EmptyOrder)));
    }
}
```

### Error Handling Example

```rust
use suzunari_error::*;

#[suzunari_error]
enum ApiError {
    #[suzu(display("data fetch failed"))]
    FetchFailed {
        source: reqwest::Error,
    },

    #[suzu(display("response parse failed"))]
    ParseFailed {
        source: serde_json::Error,
    },
}

async fn fetch_data(url: &str) -> Result<Data, ApiError> {
    let response = reqwest::get(url)
        .await
        .context(FetchFailedSnafu)?;

    let data = response.json::<Data>()
        .await
        .context(ParseFailedSnafu)?;

    Ok(data)
}
```

These examples demonstrate how to apply the coding practices and principles in Rust code, focusing on type safety, immutability, error handling, and testability.
