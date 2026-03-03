//! Verifies that DisplayError works correctly with both snafu and suzunari-error.
#![cfg(feature = "test-std")]

use suzunari_error::*;

// A type that implements Debug + Display but NOT Error (simulates e.g., argon2::Error)
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
// ^ Intentionally does NOT implement Error!

// --- Pattern B: manual conversion via source(from(...)) ---
#[suzunari_error]
#[suzu(display("operation failed"))]
struct ManualSourceFromError {
    #[snafu(source(from(FakeLibError, DisplayError::new)))]
    source: DisplayError<FakeLibError>,
}

// --- Pattern A2: automatic conversion via #[suzu(from)] ---
#[suzunari_error]
#[suzu(display("from convert op failed"))]
struct FromConvertError {
    #[suzu(from)]
    source: FakeLibError,
}

// --- Pattern A3: #[suzu(from)] with already-wrapped DisplayError ---
#[suzunari_error]
#[suzu(display("from already wrapped"))]
struct FromAlreadyWrappedError {
    #[suzu(from)]
    source: DisplayError<FakeLibError>,
}

// --- Pattern B: manual conversion via map_err ---
#[suzunari_error]
#[suzu(display("manual convert failed"))]
struct ManualConvertError {
    source: DisplayError<FakeLibError>,
}

#[test]
fn test_source_from_manual_convert() {
    // FakeLibError → DisplayError conversion is applied via manual source(from(...))
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "fake lib broke",
        })
    }
    let err = fake_op().context(ManualSourceFromSnafu).unwrap_err();

    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains("operation failed"));
    assert!(report.contains("fake lib broke"));
}

#[test]
fn test_map_err_manual_convert() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError { message: "manual" })
    }
    // Wrap with DisplayError::new via map_err
    let err = fake_op()
        .map_err(DisplayError::new)
        .context(ManualConvertSnafu)
        .unwrap_err();

    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains("manual convert failed"));
    assert!(report.contains("manual"));
}

#[test]
fn test_from_attr_auto_convert() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "from broke",
        })
    }
    let err = fake_op().context(FromConvertSnafu).unwrap_err();

    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains("from convert op failed"));
    assert!(report.contains("from broke"));
}

#[test]
fn test_from_attr_already_wrapped() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "already wrapped",
        })
    }
    let err = fake_op().context(FromAlreadyWrappedSnafu).unwrap_err();

    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains("from already wrapped"));
}

// --- Source chain delegation via #[suzu(from)] ---

// A type that implements Error (source chain should be preserved)
#[derive(Debug)]
struct RealInner;
impl core::fmt::Display for RealInner {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("real inner")
    }
}
impl std::error::Error for RealInner {}

#[derive(Debug)]
struct RealOuter(RealInner);
impl core::fmt::Display for RealOuter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("real outer")
    }
}
impl std::error::Error for RealOuter {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[suzunari_error]
#[suzu(display("source chain test"))]
struct SourceChainError {
    #[suzu(from)]
    source: RealOuter,
}

#[test]
fn test_from_preserves_source_chain_for_error_types() {
    fn real_op() -> Result<(), RealOuter> {
        Err(RealOuter(RealInner))
    }
    let err = real_op().context(SourceChainSnafu).unwrap_err();
    // DisplayError should delegate source() to RealOuter::source()
    use std::error::Error;
    let display_err = err.source().expect("should have source (DisplayError)");
    let inner = display_err
        .source()
        .expect("DisplayError should delegate to RealOuter::source()");
    assert_eq!(format!("{inner}"), "real inner");
}

#[test]
fn test_from_returns_none_source_for_non_error_types() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "no Error impl",
        })
    }
    let err = fake_op().context(FromConvertSnafu).unwrap_err();
    // FakeLibError doesn't implement Error, so DisplayError::source() → None.
    use std::error::Error;
    let display_err = err.source().expect("should have source (DisplayError)");
    assert!(
        display_err.source().is_none(),
        "non-Error type should have None source"
    );
}

// BoxedStackError requires alloc. This test lives in the test-std file (not
// alloc_tier_test.rs) because it reuses ManualSourceFromError defined above.
// The test-std feature implies test-alloc, so the gate is always true here;
// the annotation documents the dependency rather than gating execution.
#[cfg(feature = "test-alloc")]
#[test]
fn test_display_error_with_boxed_stack_error() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError { message: "boxed" })
    }
    let err = fake_op().context(ManualSourceFromSnafu).unwrap_err();
    let boxed: BoxedStackError = err.into();
    let report = format!("{:?}", StackReport::from(boxed));
    assert!(report.contains("operation failed"));
}
