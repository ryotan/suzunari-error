# Development Guide

This document provides guidelines for contributors to this project. It includes important information you should know when participating in the project, such as how to write code, commit rules, how to create PRs, and more.

## Directory Structure

```
<root>/
├── .github/                  # GitHub-related files (workflows, templates, etc.)
├── .junie/                   # Project guidelines and documentation
│   ├── coding-practices.md   # General coding practices
│   ├── guidelines-general.md # High-level development guidelines
│   ├── guidelines-git.md     # Git and commit conventions
│   ├── guidelines-rs.md      # Rust-specific guidelines
│   └── guidelines.md         # Main guidelines document with references
├── macro-impl/               # Macro implementation crate
│   ├── src/                  # Source code for macros
│   │   └── lib.rs            # Main library file for macros
│   └── Cargo.toml            # Cargo configuration for macro-impl
├── src/                      # Main library source code
│   └── lib.rs                # Main library file
├── target/                   # Build artifacts (generated)
├── CONTRIBUTING.md           # Contribution guidelines
├── Cargo.toml                # Cargo configuration for main crate
├── Cargo.lock                # Locked dependencies
├── README.md                 # Project overview and setup instructions
└── rust-toolchain.toml       # Rust toolchain configuration
```

## Development Environment Setup

1. Install Rust (if not already installed):
   - Visit [rustup.rs](https://rustup.rs/) and follow the instructions for your platform
   - This project uses a specific Rust version defined in `rust-toolchain.toml`

2. Clone the repository:
   ```
   git clone https://github.com/ryotan/suzunari-error.git
   cd suzunari-error
   ```

3. Build the project:
   ```
   cargo build
   ```

4. Run tests:
   ```
   cargo test
   ```

5. Optional: Install development tools:
   - Install [mise](https://mise.jdx.dev/) for running predefined project commands
   - Install [rustfmt](https://github.com/rust-lang/rustfmt) for code formatting: `rustup component add rustfmt`
   - Install [Clippy](https://github.com/rust-lang/rust-clippy) for linting: `rustup component add clippy`


## Coding Conventions

See [Rust Style Guide](./.junie/guidelines-rs.md)

## Commit Message Conventions

See [Commit Message Conventions](./.junie/guidelines-git.md)

## Creating Pull Requests (PRs)

1. Create a new branch (include feature name or bug fix name):
   ```
   git checkout -b feat/git-diff-viewer
   ```

2. Implement changes and commit:
   ```
   git add .
   git commit -m "feat: Add Git diff display feature"
   ```

3. Push to remote branch:
   ```
   git push -u origin feat/git-diff-viewer
   ```

4. Create a pull request on GitHub. Include the following in the PR description:

   - Summary of changes
   - Related issue number
   - How to test
   - Screenshots (if there are UI changes)

## Testing

See "Testing" section in [Rust Style Guide](./.junie/guidelines-rs.md)

- Run tests with `cargo test`
- Use `cargo clippy` for code quality checks
- Use `cargo fmt --check` to verify code formatting

Note: While the project includes a `.mise.toml` file for potential command shortcuts, these commands are not currently configured. You can set up your own mise commands or use the standard cargo commands above.

## Release

1. The `main` branch is always kept in a stable state.
2. Releases are made by tagging: `v1.0.0`, `v1.0.1`, etc.
3. Follow semantic versioning:
   - Major: Incompatible changes
   - Minor: Backward-compatible feature additions
   - Patch: Backward-compatible bug fixes

## Help

If you have questions or need assistance, create an Issue or contact the project administrator.

---

Thank you for your cooperation!
