# Suzunari Error

A highly traceable and noise-free error handling library for Rust. Propagates error locations as error contexts and minimizes information output to logs.

Built on [SNAFU](https://docs.rs/snafu), inspired by [Error Handling for Large Rust Projects - Best Practice in GreptimeDB](https://greptime.com/blogs/2024-05-07-error-rust) and [tamanegi-error](https://crates.io/crates/tamanegi-error).

## Features

- **`#[suzunari_error]`** — The primary macro. Annotate your error type and get `Snafu` + `StackError` derives plus automatic `location` field injection. This is all you need in most cases.
- **`StackError` trait** — Error location-aware contextual chained errors. Provides `location()`, `type_name()`, and `stack_source()` for traversing error chains with location info.
- **`StackReport`** — Formats a `StackError` chain as a stack-trace-like report with type names and locations at each level. Use at error display boundaries.
- **`Location`** — Memory-efficient location structure compatible with SNAFU's implicit context.
- **`DisplayError<E>`** — Adapter to wrap external types that implement `Debug + Display` but not `Error`, making them usable as snafu `source` fields.
- **`BoxedStackError`** — Type-erased `StackError` wrapper for uniform error handling across module boundaries (requires `alloc`).
- **`#![no_std]` compatible** — Works in core-only, `alloc`, and `std` environments via feature flags.

## Usage

```rust
use suzunari_error::*;
use snafu::ResultExt;

#[suzunari_error]
enum SomeError {
    #[snafu(display("after {}sec", timeout_sec))]
    ReadTimeout {
        timeout_sec: u32,
        #[snafu(source)]
        error: std::io::Error,
    },
    #[snafu(display("{} is an invalid value. Must be larger than 1", param))]
    ValidationFailed { param: i32 },
}

#[suzunari_error]
#[snafu(display("Failed to retrieve"))]
struct RetrieveFailed {
    source: SomeError,
}

fn retrieve_data() -> Result<(), RetrieveFailed> {
    read_external().context(RetrieveFailedSnafu)?;
    Ok(())
}

fn read_external() -> Result<(), SomeError> {
    let err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
    Err(err).context(ReadTimeoutSnafu { timeout_sec: 3u32 })?;
    Ok(())
}
```

### `StackReport` — Formatted error chain output

Use `StackReport` at error display boundaries to produce stack-trace-like output:

```rust
use suzunari_error::*;

fn run() -> Result<(), RetrieveFailed> {
    retrieve_data()?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", StackReport::from_error(e));
        // Output:
        // Error: RetrieveFailed: Failed to retrieve, at src/main.rs:4:5
        // Caused by the following errors (recent errors listed first):
        //   1| SomeError::ReadTimeout: after 3sec, at src/lib.rs:12:5
        //   2| timeout
    }
}
```

### `BoxedStackError` — Uniform error handling across module boundaries

```rust
use suzunari_error::*;
use snafu::ResultExt;

#[suzunari_error]
#[snafu(display("database query failed"))]
struct DbError {
    source: BoxedStackError,
}

fn run() -> Result<(), DbError> {
    query_user()
        .map_err(BoxedStackError::new)
        .context(DbSnafu)?;
    Ok(())
}
```

### `DisplayError` — Wrapping non-`Error` types

For third-party types that implement `Debug + Display` but not `Error`:

```rust
use suzunari_error::*;

#[suzunari_error]
#[snafu(display("hashing failed"))]
struct HashError {
    #[snafu(source(from(argon2::Error, DisplayError::new)))]
    source: DisplayError<argon2::Error>,
}
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | Yes     | Enables `alloc` + `snafu/std` |
| `alloc` | No      | Enables `BoxedStackError` and `From<T> for BoxedStackError` macro generation |

For `no_std` usage, disable default features:

```toml
[dependencies]
suzunari-error = { version = "0.1", default-features = false }
```

## Known Issues

- When using `#[suzunari_error]` without a wildcard import, IntelliJ IDEA may report false compile errors. `cargo build` / `cargo test` will succeed. Workaround: `use suzunari_error::*;`

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
