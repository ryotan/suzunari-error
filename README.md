# Suzunari Error

Suzunari Error is a crate that provides a highly traceable and noise-free error system by propagating error locations as
error contexts and minimizing the information output to the log. It also provides utilities to simplify error definition
and handling.

This crate is designed to help implement my philosophy of error design. It would be helpful for you to understand my
policy on error design. If you want to know more about my approach to error design, check out the section titled "Error
Design."

Suzunari Error uses SNAFU as a foundation for easily constructing traceable errors. We were inspired by the ideas in the
articles "About Stack Error" and "About Stack Error" when we came up with the features we provide in this crate.

## Features

- StackError
  - A trait for error location aware contextual chained error
- StackError derive macro
  - Implement StackError for struct and enum variants
  - Implement Debug to log error location, stack depth and contextual message
- suzunari_location attribute macro
  - Add location field which SNAFU implicitly adds from context
- Location
  - Memory-efficient Location structure compatible with SNAFU's implicit context

## Usage

```rust
use snafu::Snafu;
use suzunari::{StackError, suzunari_location};

#[derive(Snafu, StackError)]
#[suzunari_location]
#[sunafu(display("", timeout_sec))]
struct RetrieveFailed {
  source: ReadTimeout,
}

#[derive(Snafu, StackError)]
#[suzunari_location]
enum SomeError {
  #[sunafu(display("after {}sec", timeout_sec))]
  ReadTimeout {
    timeout_sec: u32,
    #[snafu(source)]
    error: std::io::Error,
  },
  #[snafu(display("{} is an invalid value. Must be larger than 1", param))]
  ValidationFailed {
    param: String,
  },
}

#[derive(Snafu, StackError)]
#[suzunari_location]
#[sunafu(display("", timeout_sec))]
struct RetrieveFailed {
  source: ReadTimeout,
}

fn retrieve_data() -> Result<(), Result<(), RetrieveFailed>> {
  read_external().context(RetrieveFailedSnafu)?;
  Ok(())
}

fn read_external() -> Result<(), SomeError> {
  let err = std::io::Error::new(/* ... */);
  Err(err).context(ReadTimeoutSnafu { timeout_sec: 3 })?;
  Ok(())
}

fn validate() -> Result<(), SomeError> {
  let param = 0;
  ensure!(i > 1, ValidationFailedSnafu { param });
  Ok(())
}
```
