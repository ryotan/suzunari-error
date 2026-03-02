# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Language Rules

- Code comments: English
- Commit messages, PR titles, PR bodies: English
- Variable and function names: English

## Git Conventions

GitMoji + Conventional Commits format: `<emoji>: <summary>`

Emojis: ✨ feature, 🐛 bugfix, ♻️ refactor, ✅ test, 📝 docs, 🎨 structure, ⚡️ perf, 🔥 remove, 🚧 WIP

- Commit messages should describe the **purpose/intent** of the change, not what was changed
- PRs are squash-merged — the title becomes the commit subject and the body becomes the commit body
- PR title: follow the `<emoji>: <summary>` format
- PR body: explain the purpose and context of the change
- Do NOT add Co-Authored-By trailers
- Do NOT add "Generated with Claude Code" footers to PR bodies

### Branch Naming

Conventional Branch format: `<type>/<description>` (lowercase, hyphens only)

Types: `feature/`, `bugfix/`, `hotfix/`, `release/`, `chore/`

## Build & Test Commands

```bash
# Build (all features)
cargo build --all-features

# Test (all features — same as CI)
cargo test --all-features

# Run a single test
cargo test <test_name>
cargo test --package suzunari-error <test_name>

# Lint (same as CI)
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings

# Feature-tier tests (tests-features crate)
cargo test -p suzunari-error-feature-tests --features test-std
cargo test -p suzunari-error-feature-tests --features test-alloc
cargo test -p suzunari-error-feature-tests --no-default-features --features test-core-only  # core-only
```

## Architecture

A snafu-based error handling library with automatic location tracking. `#![no_std]` compatible with 3 feature tiers: core-only / alloc / std.

### Core Concepts

- **`Location`** — Wrapper around `core::panic::Location`. Uses `#[track_caller]` + snafu's `GenerateImplicitData` to automatically capture error origin
- **`StackError` trait** — Extends `Error` with `location()`, `type_name()`, `stack_source()`, and `depth()`. Use `StackReport` to format error chains with location info
- **`BoxedStackError`** — Wrapper around `Box<dyn StackError + Send + Sync>` for uniform handling of heterogeneous errors (requires alloc)
- **`DisplayError<E>`** — Adapter that wraps `Debug + Display` types (without `Error` impl) into `core::error::Error`

### Macro Crate (`macro-impl/`)

Provides 3 proc-macros:

- **`#[suzunari_error]`** — The main entry point. Processes `#[suzu(...)]` attributes, resolves/injects location fields, and appends `#[derive(Debug, Snafu, StackError)]`. Use this by default
- **`#[derive(StackError)]`** — Generates `StackError` impl and `From<T> for BoxedStackError` (when alloc enabled). Does NOT generate `Debug` — use `#[derive(Debug)]` or `#[suzunari_error]`
- **`#[suzunari_error::report]`** — Transforms `fn main() -> Result<(), E>` into `fn main() -> StackReport<E>` for formatted error output on failure (std only)

Key source files in `macro-impl/src/`:
- `attribute.rs` — `#[suzunari_error]` entry point (location resolution, field injection, derive appending)
- `suzu_attr.rs` — `#[suzu(...)]` processing (keyword separation, `from`/`location` effects, snafu passthrough)
- `derive.rs` — `derive(StackError)` implementation
- `report.rs` — `#[report]` implementation
- `helper.rs` — Shared utilities (`lookup_location_field`, `find_location_field`, `find_source_field`, `combine_errors`, etc.)

The `macro-impl` crate has its own `alloc` feature flag. `cfg!(feature = "alloc")` controls whether `From<T> for BoxedStackError` impl is generated.

### `#[suzu(...)]` Attribute

`#[suzu(...)]` is a superset of `#[snafu(...)]` — all snafu keywords pass through as-is. Suzunari extensions:

- **`from`** (field-level) — Wraps field type in `DisplayError<T>` and generates `#[snafu(source(from(T, DisplayError::new)))]`
- **`location`** (field-level) — Marks a field as the location field. Converts to `#[stack(location)]` + `#[snafu(implicit)]`. Allows custom field names. Requires `Location` type

### Field-Level Attributes

Attribute ownership: each attribute is consumed by a specific macro.

- **`#[suzu(location)]`** → consumed by `#[suzunari_error]`. Marks a field as the location field. Converted to `#[stack(location)]` + `#[snafu(implicit)]`. Requires `Location` type
- **`#[stack(location)]`** → consumed by `derive(StackError)`. Tells the derive which field provides the location. Supports any field name
- **`#[snafu(...)]`** → consumed by `derive(Snafu)`. Standard snafu attributes (`source`, `implicit`, `display`, etc.)

### Location Resolution (by `#[suzunari_error]`)

1. `#[suzu(location)]` → convert to `#[stack(location)]` + `#[snafu(implicit)]`; error if not `Location` type
2. Count `#[stack(location)]` fields: 1 = OK, 2+ = error
3. Count `Location`-typed fields: 1 = auto-mark with `#[stack(location)]`, 2+ = error
4. Check for `location` name conflict (non-Location type) = error
5. Otherwise: auto-inject `location: Location` with `#[stack(location)]` + `#[snafu(implicit)]`

### Feature Flags

- `std` (default) → `alloc` + `snafu/std` + `StackReport` `Termination` impl + `#[report]` macro. Note: `StackReport` itself uses only `core::fmt` and is available in all tiers; only `Termination` impl and `#[report]` require `std`
- `alloc` → `snafu/alloc` + `BoxedStackError` + macro generates `From<T> for BoxedStackError`

### Test Structure

- `tests/` — Integration tests (assumes std feature)
- `tests-features/` — Feature-tier compile checks and integration tests. Uses `test-std` / `test-alloc` / `test-core-only` features to test each tier independently

## Coding Philosophy

- **FP-first**: Prefer pure functions, immutable data, and iterator chains. Use structs/enums for data, functions for behavior. Isolate side effects at boundaries.
- **TDD**: Red-Green-Refactor cycle. Tests serve as specifications.
- **Implementation order**: Type design → Pure functions → Side-effect separation → Adapter implementation
- **Start small**: Avoid excessive abstraction. Focus on types rather than code. Extend incrementally.

## Error Handling Rules

- **Always use `#[suzunari_error]`** for defining error types. Do NOT define errors with raw `#[derive(Snafu)]` alone or hand-written `impl Error`.
- **Always propagate errors with `?` and `.context()`** (`snafu::ResultExt`). Use `ensure!()` for validation checks. Do NOT use `unwrap()`, `expect()`, `.build()`, or `.fail()` in production code.
- **Test code**: `unwrap()` is acceptable. But prefer `.context()` propagation even in tests when it improves clarity.
- **Exception for trait-level unit tests**: Tests that verify `StackError` trait behavior independently from the proc-macro layer may use raw `#[derive(Snafu)]` + manual `impl StackError` + `.build()`. These tests must include a comment explaining the reason (e.g., testing the trait without macro coupling).

## Error Design Principles

1. **Traceable** — Errors carry location context through the call stack
2. **Context-rich** — Include relevant parameters and state at each level
3. **Noise-free** — No duplicated information; focus on essentials for debugging
4. **Hierarchical** — Enum variants for categories, `source` chaining for the error trail
5. **Performance-conscious** — Defer formatting until display; minimize allocations in error paths

## Publication Scope

This is a **personal library**, not targeting broad crates.io discoverability. Basic crates.io metadata (`name`, `description`, `version`, `license`, `repository`, `readme`) is present for publication. Advanced metadata such as `keywords`, `categories`, `documentation`, CI badges, and CHANGELOG are intentionally omitted. Focus code quality and API design reviews on the library itself, not on packaging/infrastructure.

## Toolchain

Rust 1.85.1 (pinned in `rust-toolchain.toml`). Edition 2024.
