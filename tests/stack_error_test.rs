#![cfg(feature = "std")]
// Tests use raw #[derive(Snafu)] + manual impl to test StackError trait
// independently from proc-macro layer. .build() is snafu's standard test pattern.

use core::error::Error;
use snafu::{ResultExt, Snafu};
use suzunari_error::{Location, StackError};

#[derive(Snafu)]
struct NestedError {
    source: std::io::Error,
    #[snafu(implicit)]
    location: Location,
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

impl StackError for NestedError {
    fn location(&self) -> &Location {
        &self.location
    }
}

impl core::fmt::Debug for NestedError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.fmt_stack(f)
    }
}

impl core::fmt::Debug for TestError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.fmt_stack(f)
    }
}

#[test]
fn test_error_propagation() {
    let result = function_a();

    assert!(result.is_err());
    let error = result.unwrap_err();

    // Test final context message
    let file = file!();
    let debug = format!("{error:?}");
    assert!(debug.contains(&format!("3: Whoops, at {file}:")));
    assert!(debug.contains(&format!("2: Internal, at {file}:")));
    assert!(debug.contains(&format!("1: NestedError, at {file}:")));
}
