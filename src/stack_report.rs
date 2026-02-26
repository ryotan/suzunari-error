use crate::StackError;
use core::fmt;

/// Formats a StackError chain as a stack trace with depth and location.
///
/// Wraps `Result<(), E>` and provides formatted error output via
/// Debug and Display. Used at error display boundaries.
pub struct StackReport<E: StackError>(Result<(), E>);

impl<E: StackError> StackReport<E> {
    /// Creates a report from an error value.
    pub fn from_error(error: E) -> Self {
        Self(Err(error))
    }
}

impl<E: StackError> From<Result<(), E>> for StackReport<E> {
    fn from(result: Result<(), E>) -> Self {
        Self(result)
    }
}

impl<E: StackError> fmt::Debug for StackReport<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<E: StackError> fmt::Display for StackReport<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Ok(()) => Ok(()),
            Err(e) => fmt::Display::fmt(&StackReportFormatter(e), f),
        }
    }
}

/// Internal formatter that formats a StackError chain.
pub(crate) struct StackReportFormatter<'a>(pub(crate) &'a dyn StackError);

impl fmt::Debug for StackReportFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for StackReportFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error = self.0;

        // Top-level error with type name and location (no index)
        write!(
            f,
            "Error: {}: {error}, at {:?}",
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
                "  {index}| {}: {next}, at {:?}",
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
