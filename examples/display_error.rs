//! Wrapping non-Error types with #[suzu(from)] and DisplayError.
//!
//! Run: cargo run --example display_error

use suzunari_error::*;

// Simulates a third-party type that implements Debug + Display but not Error.
#[derive(Debug)]
struct ExternalLibError(String);

impl std::fmt::Display for ExternalLibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "external lib error: {}", self.0)
    }
}

// #[suzu(from)] automatically wraps ExternalLibError in DisplayError<ExternalLibError>
// and generates the source(from(...)) annotation.
#[suzunari_error]
#[suzu(display("processing failed"))]
struct ProcessError {
    #[suzu(from)]
    source: ExternalLibError,
}

fn call_external_lib() -> Result<(), ExternalLibError> {
    Err(ExternalLibError("invalid input format".into()))
}

fn process() -> Result<(), ProcessError> {
    call_external_lib().context(ProcessSnafu)?;
    Ok(())
}

#[suzunari_error::report]
fn main() -> Result<(), ProcessError> {
    process()?;
    Ok(())
}
