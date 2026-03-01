//! Verifies alloc-tier features work correctly without std.
//!
//! Tests BoxedStackError and From<T> for BoxedStackError generation,
//! which require alloc but not std.
#![cfg(all(feature = "test-alloc", not(feature = "test-std")))]

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
        ensure!(true == false, InnerAllocSnafu);
        Ok(())
    }

    let err = inner_op()
        .map_err(BoxedStackError::new)
        .context(OuterAllocSnafu)
        .unwrap_err();

    // Verify the chain: OuterAllocError -> BoxedStackError(InnerAllocError)
    assert_eq!(err.type_name(), "OuterAllocError");
    let stack_src = err.stack_source().expect("should have stack source");
    assert_eq!(stack_src.type_name(), "InnerAllocError");
}
