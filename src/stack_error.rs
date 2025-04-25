//! StackError trait for error location awareness
//!
//! This module defines the StackError trait, which provides methods for error location awareness,
//! tracking error propagation through the call stack, and rich debugging information.

/// A trait for error location aware contextual chained errors.
///
/// This trait extends `core::error::Error` to provide additional functionality for tracking
/// error locations, error chain depth, and contextual messages. It also provides methods for
/// adding context to errors as they propagate through the call stack.
///
/// # Examples
///
/// ```rust
/// use snafu::Snafu;
/// use suzunari_error::{Location, StackError};
///
/// #[derive(Debug, Snafu)]
/// #[snafu(display("{}", message))]
/// struct MyError {
///     message: String,
///     #[snafu(implicit)]
///     location: Location,
/// }
/// impl StackError for MyError {
///     fn location(&self) -> &Location {
///         &self.location
///     }
/// }
/// ```
pub trait StackError: core::error::Error {
    /// Returns the location where this error was created.
    ///
    /// This method provides access to the file, line, and column information
    /// where the error was originally created.
    fn location(&self) -> &crate::Location;
}

impl<T: StackError> StackError for Box<T> {
    fn location(&self) -> &crate::Location {
        self.as_ref().location()
    }
}

impl<T: ?Sized + StackError> StackError for std::sync::Arc<T> {
    fn location(&self) -> &crate::Location {
        self.as_ref().location()
    }
}

impl<T: StackError + 'static> From<T> for Box<dyn StackError> {
    fn from(e: T) -> Self {
        Box::new(e)
    }
}

pub struct BoxedStackError {
    inner: Box<dyn StackError + Send + Sync>,
}

impl BoxedStackError {
    pub fn new<T: StackError + Send + Sync + 'static>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
    pub fn into_inner(self) -> Box<dyn StackError + Send + Sync> {
        self.inner
    }
}

impl core::fmt::Display for BoxedStackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl core::fmt::Debug for BoxedStackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

impl core::error::Error for BoxedStackError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.inner.source()
    }
}

impl From<BoxedStackError> for Box<dyn StackError + Send + Sync> {
    fn from(boxed: BoxedStackError) -> Self {
        boxed.inner
    }
}

impl From<Box<dyn StackError + Send + Sync>> for BoxedStackError {
    fn from(boxed: Box<dyn StackError + Send + Sync>) -> Self {
        Self { inner: boxed }
    }
}

impl<T: StackError + Send + Sync + 'static> From<T> for BoxedStackError {
    fn from(err: T) -> Self {
        Self::new(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Location;
    use snafu::{ErrorCompat, Snafu};
    use std::error::Error;
    use std::fmt::{Debug, Formatter};
    use std::rc::Rc;
    use std::sync::Arc;

    // A simple error type for testing
    #[derive(Snafu)]
    #[snafu(display("{}", message))]
    struct TestError {
        message: String,
        #[snafu(implicit)]
        location: Location,
    }

    // これは、実際はderive macroで実装されるようにする
    impl Debug for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            // ErrorCompatはSnafuでimplされるので、iter_chainが利用できる
            let depth = self.iter_chain().count();
            write!(f, "{}: {}, at {:?}", depth, &self, &self.location)?;
            if let Some(e) = self.source() {
                write!(f, "{e:?}")?;
            }
            Ok(())
        }
    }

    impl StackError for TestError {
        fn location(&self) -> &Location {
            &self.location
        }
    }

    fn error() -> TestError {
        TestSnafu {
            message: "Test error",
        }
        .build()
    }

    #[test]
    fn test_stack_error_basics() {
        // impl TestError fo Box<TestError>は、Snafuが実装してくれている？
        let box_error: BoxedStackError = error().into();
        handle_stack_error(box_error.into_inner().as_ref());
        let arc_error = Arc::new(error());
        handle_stack_error(&arc_error);
        handle_arc_error(arc_error.clone());

        // Test location
        assert_eq!(arc_error.location().file(), file!());

        // Test context message
        assert_eq!(format!("{}", arc_error), "Test error");

        // Test debug message
        assert_eq!(
            format!("{:?}", arc_error),
            "TestError { message: \"Test error\", location: Location { file: \"tests/stack_error_test.rs\", line: 25, column: 13 } }"
        );
    }

    fn handle_stack_error(error: &dyn StackError) {
        println!("{error:?}");
    }
    fn handle_arc_error<T: StackError>(error: Arc<T>) {
        println!("{error:?}");
    }
}
