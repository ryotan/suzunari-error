# Suzunari Error

A highly traceable and noise-free error handling library for Rust. Propagates error locations as error contexts and minimizes information output to logs.

Built on [SNAFU](https://docs.rs/snafu), inspired by [Error Handling for Large Rust Projects - Best Practice in GreptimeDB](https://greptime.com/blogs/2024-05-07-error-rust) and [tamanegi-error](https://crates.io/crates/tamanegi-error).

## Features

- **`#[suzunari_error]`** — The primary macro. Annotate your error type and get `Snafu` + `StackError` derives plus automatic `location` field injection. Supports `#[suzu(...)]` attributes for snafu passthrough and suzunari extensions (`from`, `location`).
- **`StackError` trait** — Error location-aware contextual chained errors. Provides `location()`, `type_name()`, `stack_source()`, and `depth()` for traversing error chains with location info.
- **`StackReport`** — Formats a `StackError` chain as a stack-trace-like report with type names and locations at each level. Use at error display boundaries.
- **`Location`** — Memory-efficient location structure compatible with SNAFU's implicit context.
- **`DisplayError<E>`** — Adapter to wrap external types that implement `Debug + Display` but not `Error`, making them usable as snafu `source` fields.
- **`BoxedStackError`** — Type-erased `StackError` wrapper for uniform error handling across module boundaries (requires `alloc`).
- **`#![no_std]` compatible** — Works in core-only, `alloc`, and `std` environments via feature flags.

## Usage

> **Note:** The examples below use `std::io::Error` and require the default `std` feature. For `no_std` usage, see [Feature Flags](#feature-flags).

`use suzunari_error::*` brings in everything you need — macros, traits (`ResultExt`, `OptionExt`), and the `ensure!` macro. No need to add `snafu` as a direct dependency.

```rust
use suzunari_error::*;

#[suzunari_error]
enum AppError {
    #[suzu(display("read timed out after {timeout_sec}sec"))]
    ReadTimeout {
        timeout_sec: u32,
        #[suzu(source)]
        error: std::io::Error,
    },
    #[suzu(display("{param} is invalid, must be > 0"))]
    ValidationFailed { param: i32 },
}

#[suzunari_error]
#[suzu(display("failed to retrieve data"))]
struct RetrieveFailed {
    source: AppError,
}

fn retrieve_data() -> Result<(), RetrieveFailed> {
    read_external().context(RetrieveFailedSnafu)?;
    Ok(())
}

fn read_external() -> Result<(), AppError> {
    let err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
    Err(err).context(ReadTimeoutSnafu { timeout_sec: 3u32 })?;
    Ok(())
}
```

### `StackReport` — Formatted error chain output

Use `StackReport` at error display boundaries to produce stack-trace-like output:

```rust
use suzunari_error::*;
# #[suzunari_error]
# enum AppError {
#     #[suzu(display("read timed out after {timeout_sec}sec"))]
#     ReadTimeout { timeout_sec: u32, #[suzu(source)] error: std::io::Error },
# }
# #[suzunari_error]
# #[suzu(display("failed to retrieve data"))]
# struct RetrieveFailed { source: AppError }
# fn retrieve_data() -> Result<(), RetrieveFailed> {
#     let err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
#     Err(err).context(ReadTimeoutSnafu { timeout_sec: 3u32 }).context(RetrieveFailedSnafu)?;
#     Ok(())
# }

fn run() -> Result<(), RetrieveFailed> {
    retrieve_data()?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", StackReport::from(e));
        // Output (line numbers are illustrative):
        // Error: RetrieveFailed: failed to retrieve data, at src/main.rs:12:5
        // Caused by the following errors (recent errors listed first):
        //   1| AppError::ReadTimeout: read timed out after 3sec, at src/main.rs:18:5
        //   2| timeout
    }
}
```

### `#[suzunari_error::report]` — Simplified main with error reporting

Use `#[suzunari_error::report]` on `main()` to automatically convert the return type to `StackReport<E>`, which prints a formatted error chain to stderr and exits with a non-zero code on failure:

```rust
use suzunari_error::*;
# #[suzunari_error]
# #[suzu(display("error"))]
# struct RetrieveFailed {}
# fn retrieve_data() -> Result<(), RetrieveFailed> { Ok(()) }

#[suzunari_error::report]
fn main() -> Result<(), RetrieveFailed> {
    retrieve_data()?;
    Ok(())
}
```

This is equivalent to `snafu::report` but uses `StackReport` for location-aware output.

### `BoxedStackError` — Uniform error handling across module boundaries

```rust
use suzunari_error::*;

#[suzunari_error]
#[suzu(display("inner error"))]
struct InnerError {}

#[suzunari_error]
#[suzu(display("database query failed"))]
struct DbError {
    source: BoxedStackError,
}

fn query_user() -> Result<(), InnerError> {
    ensure!(false, InnerSnafu);
    Ok(())
}

fn run() -> Result<(), DbError> {
    query_user()
        .map_err(BoxedStackError::new)
        .context(DbSnafu)?;
    Ok(())
}
```

### `DisplayError` — Wrapping non-`Error` types

For third-party types that implement `Debug + Display` but not `Error`, use `#[suzu(from)]` to automatically wrap the type in `DisplayError` and generate the `source(from(...))` annotation:

```rust
use suzunari_error::*;

// A third-party type: Debug + Display but no Error impl
#[derive(Debug)]
struct LibError(String);
impl std::fmt::Display for LibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[suzunari_error]
#[suzu(display("hashing failed"))]
struct HashError {
    #[suzu(from)]
    source: LibError,
}
```

This expands to the equivalent manual form:

```rust
# use suzunari_error::*;
# #[derive(Debug)]
# struct LibError(String);
# impl std::fmt::Display for LibError {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#         f.write_str(&self.0)
#     }
# }
#[suzunari_error]
#[suzu(display("hashing failed"))]
struct HashError {
    #[suzu(source(from(LibError, DisplayError::new)))]
    source: DisplayError<LibError>,
}
```

## `#[suzu(...)]` vs `#[snafu(...)]`

`#[suzu(...)]` is a superset of `#[snafu(...)]`. All snafu keywords (`display`, `source`, `implicit`, etc.) work inside `#[suzu(...)]` and are passed through to snafu. Additionally, `#[suzu(...)]` supports `from` and `location` extensions.

When using `#[suzunari_error]`, prefer `#[suzu(...)]` over `#[snafu(...)]` for consistency. `#[snafu(...)]` also works but mixing the two styles is discouraged.

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | Yes     | Enables `alloc` + `snafu/std` + `StackReport`'s `Termination` impl + `#[report]` macro |
| `alloc` | No      | Enables `BoxedStackError` and `From<T> for BoxedStackError` macro generation |
| _(none)_ | —      | Core-only: `Location`, `StackError`, `StackReport` (formatting only), `DisplayError` |

> **Note:** `StackReport` itself uses only `core::fmt` and is available in all tiers. Only the `Termination` impl (for use as `main()` return type) and `#[report]` require `std`.

For `no_std` usage, disable default features:

```toml
[dependencies]
suzunari-error = { version = "0.1", default-features = false }
```

## Why suzunari-error?

Standard Rust error approaches have a tradeoff between traceability and ergonomics:

| Approach | Per-level location | Auto-capture | Type-safe chain | `no_std` |
|----------|:-:|:-:|:-:|:-:|
| `thiserror` | - | - | Yes | Limited |
| `anyhow`/`eyre` | Single backtrace | Yes | - | - |
| `snafu` alone | Manual | Manual | Yes | Yes |
| **suzunari-error** | **Automatic** | **Yes** | **Yes** | **Yes (3 tiers)** |

suzunari-error builds on snafu to add what's missing: **automatic per-error-level location tracking** via `#[track_caller]`, a structured `StackReport` formatter that shows type names and locations at each level, and ergonomic macros (`#[suzunari_error]`, `#[suzu(from)]`) that reduce boilerplate.

See `examples/` for runnable demonstrations.

## Known Issues

- When using `#[suzunari_error]` without a wildcard import, IntelliJ IDEA may report false compile errors. `cargo build` / `cargo test` will succeed. Workaround: `use suzunari_error::*;`

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
