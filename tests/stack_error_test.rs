#![cfg(feature = "std")]
// Tests use raw #[derive(Snafu)] + manual impl to test StackError trait
// independently from proc-macro layer. .build() is snafu's standard test pattern.

use core::error::Error;
use snafu::{ResultExt, Snafu};
use suzunari_error::{Location, StackError, StackReport};

#[derive(Debug, Snafu)]
struct NestedError {
    source: std::io::Error,
    #[snafu(implicit)]
    location: Location,
}

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
    fn type_name(&self) -> &'static str {
        match self {
            TestError::External { .. } => "TestError::External",
            TestError::Internal { .. } => "TestError::Internal",
            TestError::Simple { .. } => "TestError::Simple",
        }
    }
    fn stack_source(&self) -> Option<&dyn StackError> {
        match self {
            // Box<dyn Error + Send + Sync> does NOT implement StackError
            TestError::External { .. } => None,
            // NestedError implements StackError
            TestError::Internal { source, .. } => Some(source),
            TestError::Simple { .. } => None,
        }
    }
}

impl StackError for NestedError {
    fn location(&self) -> &Location {
        &self.location
    }
    fn type_name(&self) -> &'static str {
        "NestedError"
    }
    // source is io::Error (not StackError) → default None
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

#[test]
fn test_error_propagation() {
    let result = function_a();

    assert!(result.is_err());
    let error = result.unwrap_err();

    let file = file!();
    let report = format!("{:?}", StackReport::from_error(error));

    // TestError::External's source is Box<dyn Error + Send + Sync>,
    // so stack_source() returns None. The rest of the chain is
    // traversed via Error::source() without location info.
    assert!(report.contains(&format!("Error: TestError::External: Whoops, at {file}:")));
    assert!(report.contains("Caused by"));
    assert!(report.contains("1| Internal"));
    assert!(report.contains("2| NestedError"));
    assert!(report.contains("3| "));
}
