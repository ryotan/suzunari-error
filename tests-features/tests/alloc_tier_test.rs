//! Verifies alloc-tier features work correctly without std.
//!
//! Tests BoxedStackError, From<T> for BoxedStackError generation, and
//! DisplayError source chain delegation, which require alloc but not std.
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

// --- Source chain delegation via #[suzu(from)] ---

// RealInner/RealOuter intentionally use manual impl Error (not #[suzunari_error])
// to simulate external library errors — exactly the scenario #[suzu(from)] is
// designed to handle. Using #[suzunari_error] would make them StackErrors rather
// than plain Error types, defeating the purpose of the source chain test.
#[derive(Debug)]
struct RealInner;
impl core::fmt::Display for RealInner {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("real inner")
    }
}
impl core::error::Error for RealInner {}

#[derive(Debug)]
struct RealOuter(RealInner);
impl core::fmt::Display for RealOuter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("real outer")
    }
}
impl core::error::Error for RealOuter {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.0)
    }
}

// A non-Error type (source chain should NOT delegate)
struct FakeLibError {
    message: &'static str,
}
impl core::fmt::Display for FakeLibError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl core::fmt::Debug for FakeLibError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "FakeLibError({})", self.message)
    }
}

#[suzunari_error]
#[suzu(display("source chain test"))]
struct SourceChainAllocError {
    #[suzu(from)]
    source: RealOuter,
}

#[suzunari_error]
#[suzu(display("no source chain test"))]
struct NoSourceChainAllocError {
    #[suzu(from)]
    source: FakeLibError,
}

#[test]
fn test_from_preserves_source_chain_alloc_only() {
    fn real_op() -> Result<(), RealOuter> {
        Err(RealOuter(RealInner))
    }
    let err = real_op().context(SourceChainAllocSnafu).unwrap_err();
    use core::error::Error;
    let display_err = err.source().expect("should have source (DisplayError)");
    let inner = display_err
        .source()
        .expect("DisplayError should delegate to RealOuter::source()");
    assert_eq!(alloc::format!("{inner}"), "real inner");
}

#[test]
fn test_from_returns_none_source_alloc_only() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "no Error impl",
        })
    }
    let err = fake_op().context(NoSourceChainAllocSnafu).unwrap_err();
    use core::error::Error;
    let display_err = err.source().expect("should have source (DisplayError)");
    assert!(
        display_err.source().is_none(),
        "non-Error type should have None source"
    );
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
