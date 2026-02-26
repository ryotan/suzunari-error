use alloc::boxed::Box;

use crate::{Location, StackError};
use core::error::Error;
use core::fmt::{Debug, Display, Formatter, Result};

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

impl Display for BoxedStackError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.inner)
    }
}

impl Debug for BoxedStackError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.fmt_stack(f)
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
        assert!(format!("{:?}", error).contains("Test error: Test message"));
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

    fn handle_stack_error<T: StackError>(_: T) {}
}
