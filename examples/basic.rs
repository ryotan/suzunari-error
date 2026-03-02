//! Basic error chain with location tracking and StackReport output.
//!
//! Run: cargo run --example basic

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

fn read_external() -> Result<(), AppError> {
    let err = std::io::Error::new(std::io::ErrorKind::TimedOut, "connection timed out");
    Err(err).context(ReadTimeoutSnafu { timeout_sec: 3u32 })?;
    Ok(())
}

fn retrieve_data() -> Result<(), RetrieveFailed> {
    read_external().context(RetrieveFailedSnafu)?;
    Ok(())
}

#[suzunari_error::report]
fn main() -> Result<(), RetrieveFailed> {
    retrieve_data()?;
    Ok(())
}
