//! Verifies alloc-tier features work correctly without std.
//!
//! Tests BoxedStackError and From<T> for BoxedStackError generation,
//! which require alloc but not std.
#![cfg(all(feature = "test-alloc", not(feature = "test-std")))]

extern crate alloc;
use suzunari_error::*;

#[suzunari_error]
#[suzu(display("inner alloc error"))]
struct InnerAllocError {}

#[suzunari_error]
#[suzu(display("outer alloc error"))]
struct OuterAllocError {
    source: BoxedStackError,
}

#[test]
fn test_boxed_stack_error_alloc_only() {
    fn fail() -> Result<(), InnerAllocError> {
        ensure!(false, InnerAllocSnafu);
        Ok(())
    }
    let inner = fail().unwrap_err();
    let boxed = BoxedStackError::new(inner);

    assert!(boxed.location().line() > 0);
    assert_eq!(boxed.type_name(), "InnerAllocError");
}

#[test]
fn test_from_into_boxed_stack_error() {
    fn fail() -> Result<(), InnerAllocError> {
        ensure!(false, InnerAllocSnafu);
        Ok(())
    }
    let inner = fail().unwrap_err();
    let boxed: BoxedStackError = inner.into();

    assert_eq!(boxed.type_name(), "InnerAllocError");
}

#[test]
fn test_boxed_stack_error_as_source() {
    fn inner_op() -> Result<(), InnerAllocError> {
        ensure!(false, InnerAllocSnafu);
        Ok(())
    }

    let err = inner_op()
        .map_err(BoxedStackError::new)
        .context(OuterAllocSnafu)
        .unwrap_err();

    // Verify the chain: OuterAllocError -> BoxedStackError(InnerAllocError)
    assert_eq!(err.type_name(), "OuterAllocError");
    assert!(err.stack_source().is_some(), "should have stack source");
    assert_eq!(err.stack_source().unwrap().type_name(), "InnerAllocError");
}

#[test]
fn test_stack_report_format_alloc_only() {
    // StackReport formatting works in alloc tier (uses core::fmt, not std).
    fn fail() -> Result<(), InnerAllocError> {
        ensure!(false, InnerAllocSnafu);
        Ok(())
    }
    let inner = fail().unwrap_err();
    let report = alloc::format!("{}", StackReport::from(inner));
    assert!(report.contains("Error: InnerAllocError: inner alloc error"));
    assert!(report.contains("at"));
}
