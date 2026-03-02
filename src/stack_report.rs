use crate::StackError;
use core::fmt::{Debug, Display, Formatter};

#[cfg(feature = "std")]
use std::io::{Write, stderr};
#[cfg(feature = "std")]
use std::process::{ExitCode, Termination};

/// Formats a [`StackError`] chain as a stack-trace-like report with type names and locations.
///
/// Wraps `Result<(), E>` and provides formatted output via `Display` (and `Debug`, which
/// delegates to `Display`). Used at error display boundaries such as `main()`.
///
/// Create via `StackReport::from(error)`, `Result::<(), E>::into()`, or `error.into()`.
///
/// # Output Format
///
/// ```text
/// Error: AppError::IoFailed: io failed, at src/main.rs:42:5
/// Caused by (recent first):
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
///
/// # Example
///
/// ```
/// use suzunari_error::*;
///
/// #[suzunari_error]
/// #[suzu(display("app error"))]
/// struct AppError {
///     source: std::io::Error,
/// }
///
/// fn run() -> Result<(), AppError> {
///     std::fs::read("/nonexistent").context(AppSnafu)?;
///     Ok(())
/// }
///
/// let err = run().unwrap_err();
/// let report = StackReport::from(err);
///
/// let output = format!("{report}");
/// assert!(output.contains("Error: AppError: app error"));
/// assert!(output.contains("Caused by"));
/// ```
///
/// # Notes
///
/// - Both `Display` and `Debug` produce an empty string for the `Ok` case.
///   This is intentional — in the `Termination` use case, success should be silent.
/// - **`Debug` delegates to `Display`** (same output). This intentionally
///   deviates from the [C-DEBUG](https://rust-lang.github.io/api-guidelines/debuggability.html#c-debug)
///   guideline because `Termination` calls `Debug::fmt` to produce the error
///   output. Making `Debug` structural (e.g., `StackReport(Err(...))`) would
///   render the terminal output useless. Since `StackReport` is a display
///   boundary type (not a general-purpose data carrier), the human-readable
///   format is appropriate for both traits.
/// - `Display` output does **not** include a trailing newline. This matches
///   the convention for `Display` implementations and avoids double newlines
///   with `eprintln!("{report}")`. The `Termination` impl adds a trailing
///   newline when writing to stderr.
pub struct StackReport<E>(Result<(), E>);

impl<E: StackError> From<Result<(), E>> for StackReport<E> {
    fn from(result: Result<(), E>) -> Self {
        Self(result)
    }
}

impl<E: StackError> From<E> for StackReport<E> {
    fn from(error: E) -> Self {
        Self(Err(error))
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
                // Trailing `\n` is added here because Display omits it
                // (Display convention: no trailing newline).
                let _ = Write::write_fmt(
                    &mut stderr(),
                    format_args!("{}\n", StackReportFormatter(&e)),
                );
                ExitCode::FAILURE
            }
        }
    }
}

/// Internal formatter that formats a StackError chain.
struct StackReportFormatter<'a>(&'a dyn StackError);

impl Display for StackReportFormatter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let error = self.0;

        // Top-level error with type name and location (no index).
        // No trailing newline — Display convention.
        write!(
            f,
            "Error: {}: {error}, at {}",
            error.type_name(),
            error.location()
        )?;

        // Check if there are any causes.
        // source() suffices: the StackError contract guarantees that
        // stack_source().is_some() implies source().is_some().
        if error.source().is_none() {
            return Ok(());
        }

        // Prefix each subsequent line with `\n` instead of appending trailing `\n`,
        // so the overall output has no trailing newline.
        write!(f, "\nCaused by (recent first):")?;

        let mut index = 1;

        // Phase 1: StackError chain (with location)
        let mut current_stack: &dyn StackError = error;
        while let Some(next) = current_stack.stack_source() {
            // Invariant: stack_source() implies source() (StackError is a sub-chain of Error).
            // In release builds this assertion is stripped; a broken impl would produce
            // truncated output (missing causes) rather than a panic, which is preferable
            // to crashing inside a Display formatter.
            debug_assert!(
                current_stack.source().is_some(),
                "StackError::stack_source() returned Some but Error::source() returned None \
                 for type {}. This indicates an incorrect StackError implementation.",
                current_stack.type_name()
            );
            write!(
                f,
                "\n  {index}| {}: {next}, at {}",
                next.type_name(),
                next.location()
            )?;
            index += 1;
            current_stack = next;
        }

        // Phase 2: Error chain (without location)
        let mut current_error = current_stack.source();
        while let Some(e) = current_error {
            write!(f, "\n  {index}| {e}")?;
            index += 1;
            current_error = e.source();
        }

        Ok(())
    }
}
