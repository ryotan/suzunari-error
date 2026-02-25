# Contributing

Thank you for your interest in contributing to Suzunari Error!

## Prerequisites

- Rust toolchain: The required version and components (rustfmt, clippy, rust-analyzer) are defined in `rust-toolchain.toml`. Simply install [rustup](https://rustup.rs/) and it will handle the rest.

## Development

```bash
# Build
cargo build --all-features

# Test (all features — same as CI)
cargo test --all-features

# Lint
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings

# Feature-tier tests
cargo test -p suzunari-error-feature-tests --features test-std
cargo test -p suzunari-error-feature-tests --features test-alloc
cargo test -p suzunari-error-feature-tests --no-default-features  # core-only
```

## Project Structure

```
├── src/                  # Main library
│   ├── lib.rs            # Crate root
│   ├── location.rs       # Location type
│   ├── stack_error.rs    # StackError trait + log helpers
│   ├── boxed_stack_error.rs  # BoxedStackError (alloc)
│   └── display_error.rs  # DisplayError adapter
├── macro-impl/           # Proc-macro crate
│   └── src/
│       ├── lib.rs        # Macro entry points
│       ├── attribute.rs  # #[suzunari_error], #[suzunari_location]
│       ├── derive.rs     # #[derive(StackError)]
│       └── helper.rs     # Shared utilities
├── tests/                # Integration tests (std)
├── tests-features/       # Feature-tier compile checks and integration tests
├── docs/                 # Development guidelines
└── .github/              # CI workflows
```

## Coding Conventions

See [docs/guidelines-rs.md](./docs/guidelines-rs.md) for Rust style guidelines and [docs/coding-practices.md](./docs/coding-practices.md) for general coding practices.

## Git Conventions

See [docs/guidelines-git.md](./docs/guidelines-git.md) for details.

### Branches

[Conventional Branch](https://conventional-branch.github.io/) format:

```
feature/add-display-error
bugfix/fix-location-tracking
chore/update-dependencies
```

### Commits

GitMoji + Conventional Commits:

```
✨(macro): Add suzunari_error attribute macro
🐛(location): Fix column tracking in nested calls
♻️: Simplify StackError trait bounds
```

## Pull Requests

1. Create a branch following the naming convention above
2. Implement changes and commit
3. Push and open a pull request on GitHub
4. Include in the PR description:
   - Summary of changes
   - Related issue number (if any)
   - How to test

## Release

- The `main` branch is always kept in a stable state
- Releases are tagged: `v1.0.0`, `v1.0.1`, etc.
- Follow [Semantic Versioning](https://semver.org/)

## Questions?

If you have questions or need help, feel free to [create an issue](https://github.com/ryotan/suzunari-error/issues).
