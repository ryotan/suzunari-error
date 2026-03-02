use crate::Location;
use core::error::Error;

/// Error trait extension that adds source code location tracking.
///
/// Types implementing this trait carry a `Location` at each level of the
/// error chain, enabling `StackReport` to produce stack-trace-like output.
///
/// # Design note: no `Send + Sync` supertrait
///
/// This trait requires only `Error`, not `Send + Sync + 'static` (unlike
/// anyhow/eyre). This aligns with snafu, which does not impose `Send + Sync`
/// on error types. `BoxedStackError` adds `Send + Sync` bounds for the
/// thread-safe trait object case.
///
/// # Design note: unsealed trait
///
/// This trait is intentionally unsealed — external crates may implement it
/// for custom wrapper types (e.g., similar to `BoxedStackError`). Future
/// method additions must provide default implementations to avoid breaking
/// downstream impls.
///
/// # Deriving
///
/// Use `#[suzunari_error]` (recommended) or `#[derive(StackError)]` directly.
/// Both resolve the location field via `#[stack(location)]` or by detecting a
/// `Location`-typed field. Manual impl is only needed for wrapper types like
/// `BoxedStackError`.
///
/// # Example
///
/// ```
/// use suzunari_error::*;
///
/// #[suzunari_error]
/// #[suzu(display("fetch failed for {url}"))]
/// struct FetchError {
///     url: String,
///     source: std::io::Error,
/// }
///
/// fn fetch(url: &str) -> Result<(), FetchError> {
///     std::fs::read(url).context(FetchSnafu { url })?;
///     Ok(())
/// }
///
/// let err = fetch("/nonexistent").unwrap_err();
///
/// // StackError methods:
/// assert!(err.location().file().ends_with(".rs"));
/// assert_eq!(err.type_name(), "FetchError");
/// assert!(err.stack_source().is_none()); // io::Error is not StackError
/// assert_eq!(err.depth(), 1);            // 1 cause in the chain
/// ```
pub trait StackError: Error {
    /// Returns the location where this error was constructed.
    fn location(&self) -> &Location;

    /// Returns a human-readable type name for display in stack traces.
    ///
    /// The derive macro generates this as a `&'static str` literal:
    /// - Structs: `"StructName"`
    /// - Enum variants: `"EnumName::VariantName"`
    ///
    /// Generic type parameters are not included. This is intended for display
    /// purposes only — do not parse or match against it programmatically.
    fn type_name(&self) -> &'static str;

    /// Returns the source error as a StackError, if available.
    ///
    /// This enables StackReport to traverse the error chain with
    /// location info. The derive macro generates this automatically
    /// using autoref specialization (see the `__private` module).
    ///
    /// # Contract
    /// If `stack_source()` returns `Some(s)`, then `Error::source()`
    /// must also return `Some(e)` where `e` and `s` refer to the same
    /// underlying error value (i.e., `s` is a `&dyn StackError` view
    /// of the `&dyn Error` returned by `source()`). The derive macro
    /// upholds this automatically; manual impls must ensure consistency.
    ///
    /// Violating this contract causes `StackReport` to produce incomplete
    /// output in release builds (the `debug_assert!` that checks this is
    /// stripped). In debug builds, a panic will occur instead.
    fn stack_source(&self) -> Option<&dyn StackError> {
        None
    }

    /// Returns the number of errors in the `Error::source()` chain (excluding self).
    ///
    /// Traverses the full `Error::source()` chain (not `stack_source()`),
    /// counting both `StackError` and non-`StackError` causes.
    ///
    /// Note: this count may differ from the number of lines in `StackReport`
    /// output, which also shows the top-level error on the first line.
    fn depth(&self) -> usize {
        // successors() can't be used here due to trait object lifetime constraints:
        // source() returns Option<&dyn Error> with a lifetime tied to &self,
        // but `successors` requires the closure output lifetime to match its input.
        let mut count = 0;
        let mut current = self.source();
        while let Some(e) = current {
            count += 1;
            current = e.source();
        }
        count
    }
}

#[cfg(feature = "alloc")]
mod alloc_impls {
    use super::*;
    use alloc::boxed::Box;
    use alloc::sync::Arc;

    // This impl requires T: Sized (implicit bound). Box<dyn StackError> is NOT
    // covered here — it needs separate Error + StackError impls below because
    // although core provides impl<T: Error + ?Sized> Error for Box<T>, we still
    // need to manually impl StackError for the unsized trait object.
    impl<T: StackError> StackError for Box<T> {
        fn location(&self) -> &Location {
            self.as_ref().location()
        }
        fn type_name(&self) -> &'static str {
            self.as_ref().type_name()
        }
        fn stack_source(&self) -> Option<&dyn StackError> {
            self.as_ref().stack_source()
        }
    }
    impl<T: ?Sized + StackError> StackError for Arc<T> {
        fn location(&self) -> &Location {
            self.as_ref().location()
        }
        fn type_name(&self) -> &'static str {
            self.as_ref().type_name()
        }
        fn stack_source(&self) -> Option<&dyn StackError> {
            self.as_ref().stack_source()
        }
    }

    impl Error for Box<dyn StackError> {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Error::source(Box::as_ref(self))
        }
    }
    impl StackError for Box<dyn StackError> {
        fn location(&self) -> &Location {
            self.as_ref().location()
        }
        fn type_name(&self) -> &'static str {
            self.as_ref().type_name()
        }
        fn stack_source(&self) -> Option<&dyn StackError> {
            self.as_ref().stack_source()
        }
    }

    impl Error for Box<dyn StackError + Send + Sync> {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Error::source(Box::as_ref(self))
        }
    }
    impl StackError for Box<dyn StackError + Send + Sync> {
        fn location(&self) -> &Location {
            self.as_ref().location()
        }
        fn type_name(&self) -> &'static str {
            self.as_ref().type_name()
        }
        fn stack_source(&self) -> Option<&dyn StackError> {
            self.as_ref().stack_source()
        }
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    // Tests use raw #[derive(Snafu)] + manual impl to test StackError trait
    // independently of proc-macro layer. .build() is snafu's standard test pattern.
    use super::*;
    use crate::StackReport;
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::string::String;
    use alloc::sync::Arc;
    use snafu::prelude::*;

    #[derive(Debug, Snafu)]
    #[snafu(display("Simple test error: {}", message))]
    struct SimpleError {
        message: String,
        #[snafu(implicit)]
        location: Location,
    }
    impl StackError for SimpleError {
        fn location(&self) -> &Location {
            &self.location
        }
        fn type_name(&self) -> &'static str {
            "SimpleError"
        }
    }

    #[derive(Debug, Snafu)]
    #[snafu(display("Wrapper error: {}", message))]
    struct WrapperError {
        message: String,
        source: Box<dyn StackError + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    }
    impl StackError for WrapperError {
        fn location(&self) -> &Location {
            &self.location
        }
        fn type_name(&self) -> &'static str {
            "WrapperError"
        }
        fn stack_source(&self) -> Option<&dyn StackError> {
            // Box<dyn StackError + Send + Sync> implements StackError
            Some(self.source.as_ref())
        }
    }

    #[test]
    fn test_basic_location() {
        let error = SimpleSnafu {
            message: "Something went wrong",
        }
        .build();
        assert_eq!(error.location().file(), file!());
        assert!(error.location().line() > 0);
        assert!(format!("{}", error).contains("Simple test error"));
        assert!(format!("{}", error).contains("Something went wrong"));

        handle_stack_error(error)
    }

    #[test]
    fn test_error_boxing() {
        let concrete_error = SimpleSnafu {
            message: "Original error",
        }
        .build();
        let boxed_error: Box<dyn StackError> = Box::new(concrete_error);

        assert_eq!(boxed_error.location().file(), file!());
        assert!(boxed_error.location().line() > 0);
        assert!(format!("{}", boxed_error).contains("Simple test error"));
        assert!(format!("{}", boxed_error).contains("Original error"));

        handle_stack_error(boxed_error)
    }

    #[test]
    fn test_error_chaining() {
        fn gen_root_error() -> Result<(), Box<dyn StackError + Send + Sync + 'static>> {
            let root_error = SimpleSnafu {
                message: "Root cause",
            }
            .build();
            Err(Box::new(root_error))
        }
        let root_error = gen_root_error();
        let root_location = root_error.unwrap_err().location().line();

        let wrapper_error = gen_root_error()
            .context(WrapperSnafu {
                message: "Something failed",
            })
            .unwrap_err();

        assert!(wrapper_error.location().file().ends_with("stack_error.rs"));
        assert_ne!(wrapper_error.location().line(), root_location);

        let report = format!("{:?}", StackReport::from(wrapper_error));
        let file = file!();
        assert!(report.contains("Error: WrapperError: Wrapper error: Something failed"));
        assert!(report.contains(&format!(", at {file}:")));
        assert!(report.contains("Caused by"));
        assert!(report.contains("1| SimpleError: Simple test error: Root cause"));
    }

    #[test]
    fn test_arc_errors() {
        let error = SimpleSnafu {
            message: "Arc-wrapped error",
        }
        .build();
        let original_location = error.location().line();
        let arc_error = Arc::new(error);

        assert_eq!(arc_error.location().line(), original_location);

        let cloned_arc = arc_error.clone();
        assert_eq!(cloned_arc.location().line(), original_location);

        handle_stack_error(arc_error);

        let arc_error: Arc<dyn StackError> = Arc::new(SimpleSnafu { message: "Simple" }.build());
        handle_stack_error(arc_error);
    }

    #[test]
    fn test_from_implementation() {
        let concrete_error = SimpleSnafu {
            message: "Converted error",
        }
        .build();
        let original_location = concrete_error.location().line();
        let boxed_error: Box<dyn StackError + Send + Sync + 'static> = Box::new(concrete_error);

        assert_eq!(boxed_error.location().line(), original_location);
        handle_stack_error(boxed_error);
    }

    #[test]
    fn test_depth_one() {
        fn gen_root() -> Result<(), Box<dyn StackError + Send + Sync + 'static>> {
            let root = SimpleSnafu { message: "root" }.build();
            Err(Box::new(root))
        }
        let wrapper = gen_root()
            .context(WrapperSnafu { message: "wrapper" })
            .unwrap_err();
        // WrapperError has one source (SimpleError), so depth == 1
        assert_eq!(wrapper.depth(), 1);
    }

    #[test]
    fn test_box_concrete_stack_error() {
        // Box<T: Sized + StackError> blanket impl
        let concrete = SimpleSnafu {
            message: "boxed concrete",
        }
        .build();
        let original_line = concrete.location().line();
        let boxed: Box<SimpleError> = Box::new(concrete);

        assert_eq!(boxed.location().line(), original_line);
        assert_eq!(boxed.type_name(), "SimpleError");
        assert!(boxed.stack_source().is_none());
        handle_stack_error(boxed);
    }

    fn handle_stack_error<T: StackError>(_: T) {}

    // --- GAP-08: Box<dyn StackError> (non-Send-Sync) Error and StackError impls ---
    #[test]
    fn test_box_dyn_stack_error_non_send_sync() {
        let concrete = SimpleSnafu {
            message: "boxed non-send-sync",
        }
        .build();
        let original_line = concrete.location().line();
        let boxed: Box<dyn StackError> = Box::new(concrete);

        // StackError methods should work
        assert_eq!(boxed.location().line(), original_line);
        assert_eq!(boxed.type_name(), "SimpleError");
        assert!(boxed.stack_source().is_none());

        // Error impl should work
        let err: &dyn Error = &boxed;
        assert!(format!("{err}").contains("boxed non-send-sync"));
    }
}
