use snafu::{ensure, ResultExt, Snafu};
use std::error::Error;
use suzunari_error::{Location, StackError};

#[derive(Debug, Snafu)]
struct NestedError {}

// A simple error type for testing
#[derive(Debug, Snafu)]
enum TestError {
    Simple {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("{}", message))]
    External {
        message: String,
        source: Box<dyn Error + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    Internal {
        source: NestedError,
        #[snafu(implicit)]
        location: Location,
    },
}

impl StackError for TestError {
    fn location(&self) -> &Location {
        match self {
            TestError::External { location, .. } => location,
            TestError::Internal { location, .. } => location,
            TestError::Simple { location, .. } => location,
        }
    }
}

#[test]
fn test_stack_error_basics() {
    let error = SimpleSnafu {}.build();

    // Test location
    assert_eq!(error.location().file(), file!());
}

#[test]
fn test_chain_context() {
    let error = SimpleSnafu {}.build();

    // Test location is updated to the current call site
    // Handle both Windows and Unix-like path separators
    let normalized_path = error.location().file().replace('\\', "/");
    assert!(normalized_path.ends_with("stack_error_test.rs"));
}

// Test error propagation through multiple functions
fn function_c() -> Result<(), NestedError> {
    NestedSnafu.fail()
}

fn function_b() -> Result<(), Box<dyn Error + Send + Sync>> {
    function_c().context(InternalSnafu)?;
    Ok(())
}

fn function_a() -> Result<(), TestError> {
    function_b().context(ExternalSnafu {message: "Whoops"})?;
    Ok(())
}

#[test]
fn test_error_propagation() {
    let result = function_a();

    assert!(result.is_err());
    let error = result.unwrap_err();

    // Test final context message
    assert_eq!(format!("{error:?}"), "Whoops");
}
