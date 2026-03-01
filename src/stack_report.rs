use crate::StackError;
use core::fmt::{Debug, Display, Formatter};
use std::io::{Write, stderr};
use std::process::{ExitCode, Termination};

/// Formats a [`StackError`] chain as a stack-trace-like report with type names and locations.
///
/// Wraps `Result<(), E>` and provides formatted output via `Display` (and `Debug`, which
/// delegates to `Display`). Used at error display boundaries such as `main()`.
///
/// Create via [`StackReport::from_error`] or `Result<(), E>::into()`.
///
/// # Output Format
///
/// ```text
/// Error: AppError::IoFailed: io failed, at src/main.rs:42:5
/// Caused by the following errors (recent errors listed first):
///   1| InfraError::Read: read failed, at src/infra.rs:10:9
///   2| No such file or directory (os error 2)
/// ```
///
/// The first line shows the top-level error with type name and location.
/// StackError sources (with location) are numbered in phase 1, then
/// plain `Error::source()` chain entries (without location) follow.
///
/// With the `std` feature, implements [`Termination`] for use as the
/// return type of `main()`. The [`#[suzunari_error::report]`](crate::report) macro
/// can transform `fn() -> Result<(), E>` into `fn() -> StackReport<E>` automatically.
pub struct StackReport<E: StackError>(Result<(), E>);

impl<E: StackError> StackReport<E> {
    /// Creates a report from an error value.
    #[must_use]
    pub fn from_error(error: E) -> Self {
        Self(Err(error))
    }
}

impl<E: StackError> From<Result<(), E>> for StackReport<E> {
    fn from(result: Result<(), E>) -> Self {
        Self(result)
    }
}

impl<E: StackError> From<E> for StackReport<E> {
    fn from(error: E) -> Self {
        Self::from_error(error)
    }
}

impl<E: StackError> Debug for StackReport<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<E: StackError> Display for StackReport<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match &self.0 {
            Ok(()) => Ok(()),
            Err(e) => Display::fmt(&StackReportFormatter(e), f),
        }
    }
}

#[cfg(feature = "std")]
impl<E: StackError> Termination for StackReport<E> {
    fn report(self) -> ExitCode {
        match self.0 {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                // Ignore write errors — stderr may be closed, and
                // panicking here would mask the original error.
                let _ =
                    Write::write_fmt(&mut stderr(), format_args!("{}", StackReportFormatter(&e)));
                ExitCode::FAILURE
            }
        }
    }
}

/// Internal formatter that formats a StackError chain.
pub(crate) struct StackReportFormatter<'a>(pub(crate) &'a dyn StackError);

impl Debug for StackReportFormatter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for StackReportFormatter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let error = self.0;

        // Top-level error with type name and location (no index)
        write!(
            f,
            "Error: {}: {error}, at {}",
            error.type_name(),
            error.location()
        )?;

        // Check if there are any causes
        let has_stack_cause = error.stack_source().is_some();
        let has_error_cause = error.source().is_some();
        if !(has_stack_cause || has_error_cause) {
            return writeln!(f);
        }

        writeln!(
            f,
            "\nCaused by the following errors (recent errors listed first):"
        )?;

        let mut index = 1;

        // Phase 1: StackError chain (with location)
        let mut current_stack: &dyn StackError = error;
        while let Some(next) = current_stack.stack_source() {
            writeln!(
                f,
                "  {index}| {}: {next}, at {}",
                next.type_name(),
                next.location()
            )?;
            index += 1;
            current_stack = next;
        }

        // Phase 2: Error chain (without location)
        let mut current_error = current_stack.source();
        while let Some(e) = current_error {
            writeln!(f, "  {index}| {e}")?;
            index += 1;
            current_error = e.source();
        }

        Ok(())
    }
}
