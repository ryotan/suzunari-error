use crate::Location;
use core::error::Error;

pub trait StackError: Error {
    fn location(&self) -> &Location;

    fn depth(&self) -> usize {
        let mut count = 0;
        let mut current = self.source();
        while let Some(e) = current {
            count += 1;
            current = e.source();
        }
        count
    }

    fn fmt_stack(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        writeln!(f, "{}: {self}, at {:?}", self.depth(), self.location())?;
        self.source().map_or(Ok(()), |s| write!(f, "{s:?}"))
    }
}

#[cfg(feature = "alloc")]
mod alloc_impls {
    use super::*;
    use alloc::boxed::Box;
    use alloc::sync::Arc;

    // Box<T> requires T: Sized here because Box<dyn StackError> needs explicit
    // Error + StackError impls (std's blanket impl<T: Error> Error for Box<T>
    // requires T: Sized). Arc doesn't need this because std provides
    // impl<T: Error + ?Sized> Error for Arc<T>.
    impl<T: StackError> StackError for Box<T> {
        fn location(&self) -> &Location {
            self.as_ref().location()
        }
    }
    impl<T: ?Sized + StackError> StackError for Arc<T> {
        fn location(&self) -> &Location {
            self.as_ref().location()
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
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    // Tests use raw #[derive(Snafu)] + manual impl to test StackError trait
    // independently from proc-macro layer. .build() is snafu's standard test pattern.
    use super::*;
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::string::String;
    use alloc::sync::Arc;
    use snafu::prelude::*;

    #[derive(Snafu)]
    #[snafu(display("Simple test error: {}", message))]
    struct SimpleError {
        message: String,
        #[snafu(implicit)]
        location: Location,
    }
    impl core::fmt::Debug for SimpleError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            self.fmt_stack(f)
        }
    }
    impl StackError for SimpleError {
        fn location(&self) -> &Location {
            &self.location
        }
    }

    #[derive(Snafu)]
    #[snafu(display("Wrapper error: {}", message))]
    struct WrapperError {
        message: String,
        source: Box<dyn StackError + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    }
    impl core::fmt::Debug for WrapperError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            self.fmt_stack(f)
        }
    }
    impl StackError for WrapperError {
        fn location(&self) -> &Location {
            &self.location
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

        assert!(format!("{wrapper_error:?}").contains(file!()));
        assert!(format!("{wrapper_error:?}").contains("Wrapper error: "));
        assert!(format!("{wrapper_error:?}").contains("Something failed"));
        assert!(format!("{wrapper_error:?}").contains("Simple test error: "));
        assert!(format!("{wrapper_error:?}").contains("Root cause"));

        let file = file!();
        let debug = format!("{wrapper_error:?}");
        assert!(debug.contains(&format!("1: Wrapper error: Something failed, at {file}:")));
        assert!(debug.contains(&format!("0: Simple test error: Root cause, at {file}:")));

        handle_stack_error(wrapper_error);
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
    fn test_practical_error_handling() {
        fn may_fail(input: i32) -> Result<i32, Box<dyn StackError + Send + Sync + 'static>> {
            if input < 0 {
                return Err(Box::new(
                    SimpleSnafu {
                        message: "Input must be non-negative",
                    }
                    .build(),
                ));
            }
            Ok(input * 2)
        }

        fn process(input: i32) -> Result<i32, Box<WrapperError>> {
            let result = may_fail(input).context(WrapperSnafu {
                message: "Processing failed",
            })?;

            Ok(result + 10)
        }

        assert_eq!(process(5).unwrap(), 20);

        let err: Box<WrapperError> = process(-1).unwrap_err();
        assert!(format!("{}", err).contains("Processing failed"));

        let source = err.source().unwrap();
        assert!(format!("{source:?}").contains("Simple test error: "));
        assert!(format!("{source:?}").contains("Input must be non-negative"));

        handle_stack_error(err);
    }

    fn handle_stack_error<T: StackError>(_: T) {}
}
