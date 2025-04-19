# Coding Practices

This document outlines the coding practices and principles followed in this project. These practices guide our
development approach and help maintain code quality and consistency.

- Start small and extend incrementally
- Avoid excessive abstraction
- Focus on types rather than code
- Adjust approach according to complexity

### Functional approach (FP)

We prefer a functional programming approach for its clarity, testability, and reduced side effects:

- Function-first (classes only if necessary)
- Ensure type safety
- Preference for pure functions (functions that always return the same output for the same input and have no side
  effects)
- Use invariant data structures
- Use of invariant update patterns
- Isolate side-effects (keep I/O operations separate from business logic)
- Flatten conditional branches with early returns

### Domain-driven design (DDD)

We apply domain-driven design principles to model complex business domains effectively:

- Distinguish between value objects (immutable, identified by attributes) and entities (mutable, identified by ID)
- Aggregation guarantees consistency (treat related entities as a single unit)
- Abstraction of data access in repositories
- Bounded context awareness (recognize that the same term may have different meanings in different contexts)
- Enumerated definitions of errors and use cases

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

## Notes about stereotypes

In software development, certain patterns and concepts are frequently used but can be interpreted differently by
different developers. We clearly define commonly used stereotypes whose roles are often ambiguous to ensure consistent
implementation across the codebase.

### Repository

- Deals only with domain models
- Hides persistence details
- Provides in-memory implementation for testing

### Adapter pattern

- Abstraction of external dependencies
- Interfaces are defined by the caller
- Easily replaceable for testing

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
- Wrap operations with side-effects in Promise.

4.**Adapter implementation**

- Abstraction of access to external services and DB
- Prepare mock for testing.
