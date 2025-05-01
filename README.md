# Suzunari Error

Suzunari Error is a crate that provides a highly traceable and noise-free error system by propagating error locations as
error contexts and minimizing the information output to the log. It also provides utilities to simplify error definition
and handling.

This crate is designed to help implement my philosophy of error design. It would be helpful for you to understand my
policy on error design. If you want to know more about my approach to error design, check out the section titled "Error
Design."

Suzunari Error uses SNAFU as a foundation for easily constructing traceable errors. We were inspired by the ideas in the
articles [Error Handling for Large Rust Projects - Best Practice in GreptimeDB](https://greptime.com/blogs/2024-05-07-error-rust),
and [tamanegi-error - crates.io](https://crates.io/crates/tamanegi-error) when we came up with the features we provide
in this crate.

## Features

- `StackError`
  - A trait for error location aware contextual chained error
- `Location`
  - Memory-efficient Location structure compatible with SNAFU's implicit context
- `StackError` derive macro
  - Implement StackError for struct and enum variants
  - Implement Debug to log error location, stack depth and contextual message
- `suzunari_location` attribute macro
  - Add location field which SNAFU implicitly adds from context
- `suzunari_error` attribute macro
  - Add `suzunari_location` attribute macro and Snafu and StackError derive macros

## Usage

```rust
use suzunari_error::*;

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

fn validate() -> Result<(), BoxedStackError> {
  let param = 0;
  ensure!(false, ValidationFailedSnafu { param });
  Ok(())
}
```

### Known issues

* If `suzunari_location` is not used and `suzunari_error` is used, IntelliJ IDEA will report a compile error. However,
  `cargo test` or `cargo build` will succeed. To avoid this, it is recommended to use `suzunari_error::*`.
