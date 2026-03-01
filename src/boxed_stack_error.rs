use alloc::boxed::Box;

use crate::{Location, StackError};
use core::error::Error;
use core::fmt::{Debug, Display, Formatter, Result};

/// Type-erased wrapper around `Box<dyn StackError + Send + Sync>`.
///
/// Provides uniform handling of heterogeneous `StackError` types while
/// preserving location tracking through the error chain. Use this instead
/// of `Box<dyn StackError + Send + Sync>` for shorter type signatures
/// and automatic `From` generation by the derive macro.
///
/// Note: downcasting to the concrete type is not supported through this
/// wrapper. Use `into_inner()` if you need the raw trait object.
///
/// `Clone` is not implemented because the inner trait object
/// (`Box<dyn StackError + Send + Sync>`) cannot be cloned.
///
/// # Example
///
/// ```
/// use suzunari_error::*;
///
/// #[suzunari_error]
/// #[suzu(display("inner error"))]
/// struct InnerError {}
///
/// #[suzunari_error]
/// #[suzu(display("outer error"))]
/// struct OuterError {
///     source: BoxedStackError,
/// }
///
/// fn inner() -> Result<(), InnerError> {
///     ensure!(false, InnerSnafu);
///     Ok(())
/// }
///
/// fn outer() -> Result<(), OuterError> {
///     inner()
///         .map_err(BoxedStackError::new)
///         .context(OuterSnafu)?;
///     Ok(())
/// }
///
/// let err = outer().unwrap_err();
/// assert_eq!(err.type_name(), "OuterError");
/// assert!(err.stack_source().is_some());
/// ```
pub struct BoxedStackError {
    inner: Box<dyn StackError + Send + Sync>,
}

impl BoxedStackError {
    /// Wraps a concrete `StackError` in a type-erased box.
    #[must_use]
    pub fn new<T: StackError + Send + Sync + 'static>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Returns a reference to the inner trait object.
    #[must_use]
    pub fn inner(&self) -> &(dyn StackError + Send + Sync) {
        &*self.inner
    }

    /// Unwraps into the inner trait object.
    #[must_use]
    pub fn into_inner(self) -> Box<dyn StackError + Send + Sync> {
        self.inner
    }
}

impl Display for BoxedStackError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.inner)
    }
}

impl Debug for BoxedStackError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:?}", self.inner)
    }
}

impl Error for BoxedStackError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}

impl StackError for BoxedStackError {
    fn location(&self) -> &Location {
        self.inner.location()
    }
    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }
    fn stack_source(&self) -> Option<&dyn StackError> {
        self.inner.stack_source()
    }
}

impl From<Box<dyn StackError + Send + Sync>> for BoxedStackError {
    fn from(inner: Box<dyn StackError + Send + Sync>) -> Self {
        Self { inner }
    }
}

impl From<BoxedStackError> for Box<dyn StackError + Send + Sync> {
    fn from(inner: BoxedStackError) -> Self {
        inner.into_inner()
    }
}

#[cfg(test)]
mod tests {
    // Tests use raw #[derive(Snafu)] + manual impl to test StackError trait
    // independently from proc-macro layer. .build() is snafu's standard test pattern.
    use super::*;
    use crate::Location;
    use alloc::format;
    use snafu::IntoError;
    use snafu::prelude::*;

    #[derive(Debug, Snafu)]
    #[snafu(display("Test error: {}", message))]
    struct TestError {
        message: alloc::string::String,
        #[snafu(implicit)]
        location: Location,
    }

    impl StackError for TestError {
        fn location(&self) -> &Location {
            &self.location
        }
        fn type_name(&self) -> &'static str {
            "TestError"
        }
    }

    #[test]
    fn test_basic_error() {
        let test_error = TestSnafu {
            message: "Test message",
        }
        .build();
        let error = BoxedStackError::new(test_error);

        assert!(format!("{}", error).contains("Test error"));
        assert!(format!("{}", error).contains("Test message"));
        // Debug delegates to inner's derive(Debug), not stack trace
        assert!(format!("{:?}", error).contains("Test message"));
        assert!(error.source().is_none());

        handle_stack_error(error);
    }

    #[test]
    fn test_error_location() {
        let test_error = TestSnafu {
            message: "Location test",
        }
        .build();
        let original_line = test_error.location().line();
        let error = BoxedStackError::new(test_error);

        assert_eq!(error.location().file(), file!());
        assert_eq!(error.location().line(), original_line);

        handle_stack_error(error);
    }

    #[test]
    fn test_error_conversion() {
        let test_error = TestSnafu {
            message: "Convert test",
        }
        .build();
        let boxed: Box<dyn StackError + Send + Sync> = Box::new(test_error);
        let generic: BoxedStackError = boxed.into();
        let back_to_box: Box<dyn StackError + Send + Sync> = generic.into();

        assert!(format!("{:?}", back_to_box).contains("Convert test"));
    }

    // --- source chain delegation and depth ---

    #[derive(Debug, Snafu)]
    #[snafu(display("Wrapper: {}", message))]
    struct WrapperTestError {
        message: alloc::string::String,
        source: BoxedStackError,
        #[snafu(implicit)]
        location: Location,
    }
    impl StackError for WrapperTestError {
        fn location(&self) -> &Location {
            &self.location
        }
        fn type_name(&self) -> &'static str {
            "WrapperTestError"
        }
        fn stack_source(&self) -> Option<&dyn StackError> {
            Some(&self.source)
        }
    }

    #[test]
    fn test_source_chain_delegation() {
        // BoxedStackError wrapping an error with a source should delegate source()
        let inner = TestSnafu { message: "root" }.build();
        let boxed = BoxedStackError::new(inner);
        let wrapper = WrapperTestSnafu { message: "wrap" }.into_error(boxed);
        let outer = BoxedStackError::new(wrapper);

        // Error::source() should return Some (delegates to WrapperTestError's source)
        assert!(outer.source().is_some());
        // stack_source() should return Some (WrapperTestError has a stack_source)
        assert!(outer.stack_source().is_some());
    }

    #[test]
    fn test_depth() {
        // Leaf error: depth == 0
        let leaf = BoxedStackError::new(TestSnafu { message: "leaf" }.build());
        assert_eq!(leaf.depth(), 0);

        // Wrapped error: depth == 1 (the BoxedStackError source counts as 1)
        let inner = BoxedStackError::new(TestSnafu { message: "inner" }.build());
        let wrapper = WrapperTestSnafu { message: "outer" }.into_error(inner);
        let outer = BoxedStackError::new(wrapper);
        assert_eq!(outer.depth(), 1);
    }

    fn handle_stack_error<T: StackError>(_: T) {}

    #[test]
    fn test_inner_ref() {
        let test_error = TestSnafu {
            message: "inner ref test",
        }
        .build();
        let original_line = test_error.location().line();
        let error = BoxedStackError::new(test_error);

        let inner = error.inner();
        assert_eq!(inner.location().line(), original_line);
        assert_eq!(inner.type_name(), "TestError");
    }

    #[test]
    fn test_into_inner_round_trip() {
        let test_error = TestSnafu {
            message: "round trip",
        }
        .build();
        let original_line = test_error.location().line();

        // BoxedStackError → into_inner → Box<dyn StackError + Send + Sync>
        let boxed = BoxedStackError::new(test_error);
        let inner: Box<dyn StackError + Send + Sync> = boxed.into_inner();

        assert_eq!(inner.location().line(), original_line);
        assert_eq!(inner.type_name(), "TestError");
        assert!(format!("{inner}").contains("round trip"));

        // Box<dyn StackError + Send + Sync> → BoxedStackError (via From)
        let boxed_again: BoxedStackError = inner.into();
        assert_eq!(boxed_again.location().line(), original_line);
    }
}
