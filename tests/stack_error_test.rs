use core::error::Error;
use snafu::{ResultExt, Snafu};
use suzunari_error::{Location, StackError, write_error_log, write_stack_error_log};

#[derive(Snafu)]
struct NestedError {
    source: std::io::Error,
}

// A simple error type for testing
#[derive(Snafu)]
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
fn function_c() -> Result<Vec<u8>, NestedError> {
    std::fs::read("not exist").context(NestedSnafu)
}

fn function_b() -> Result<(), Box<dyn Error + Send + Sync>> {
    function_c().context(InternalSnafu)?;
    Ok(())
}

fn function_a() -> Result<(), TestError> {
    function_b().context(ExternalSnafu { message: "Whoops" })?;
    Ok(())
}

impl core::fmt::Debug for NestedError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write_error_log(f, self)
    }
}

impl core::fmt::Debug for TestError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write_stack_error_log(f, self)
    }
}

#[test]
fn test_error_propagation() {
    let result = function_a();

    assert!(result.is_err());
    let error = result.unwrap_err();

    // Test final context message
    let file = file!();
    let expected = format!(
        "3: Whoops, at {file}:72:18
2: Internal, at {file}:67:18
1: NestedError
Os {{ code: 2, kind: NotFound, message: \""
    );
    assert!(format!("{error:?}").starts_with(&expected));
}
