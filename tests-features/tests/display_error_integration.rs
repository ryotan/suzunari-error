//! Verifies that DisplayError works correctly with both snafu and suzunari-error.

use snafu::prelude::*;
use suzunari_error::*;

// A type that implements Debug + Display but NOT Error (simulates e.g. argon2::Error)
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

// --- Pattern A: automatic conversion via source(from(...)) ---
#[suzunari_error]
#[snafu(display("operation failed"))]
struct AutoConvertError {
    #[snafu(source(from(FakeLibError, DisplayError::new)))]
    source: DisplayError<FakeLibError>,
}

// --- Pattern A2: automatic conversion via #[suzu(translate)] ---
#[suzunari_error]
#[suzu(display("translate op failed"))]
struct TranslateConvertError {
    #[suzu(translate)]
    source: FakeLibError,
}

// --- Pattern A3: #[suzu(translate)] with already-wrapped DisplayError ---
#[suzunari_error]
#[suzu(display("translate already wrapped"))]
struct TranslateAlreadyWrappedError {
    #[suzu(translate)]
    source: DisplayError<FakeLibError>,
}

// --- Pattern B: manual conversion via map_err ---
#[suzunari_error]
#[snafu(display("manual convert failed"))]
struct ManualConvertError {
    source: DisplayError<FakeLibError>,
}

#[test]
fn test_source_from_auto_convert() {
    // FakeLibError → DisplayError conversion is auto-applied via source(from(...))
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "fake lib broke",
        })
    }
    let err = fake_op().context(AutoConvertSnafu).unwrap_err();

    let report = format!("{:?}", StackReport::from_error(err));
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

    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("manual convert failed"));
    assert!(report.contains("manual"));
}

#[test]
fn test_translate_attr_auto_convert() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "translate broke",
        })
    }
    let err = fake_op().context(TranslateConvertSnafu).unwrap_err();

    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("translate op failed"));
    assert!(report.contains("translate broke"));
}

#[test]
fn test_translate_attr_already_wrapped() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "already wrapped",
        })
    }
    let err = fake_op().context(TranslateAlreadyWrappedSnafu).unwrap_err();

    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("translate already wrapped"));
}

#[cfg(feature = "test-alloc")]
#[test]
fn test_display_error_with_boxed_stack_error() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError { message: "boxed" })
    }
    let err = fake_op().context(AutoConvertSnafu).unwrap_err();
    let boxed: BoxedStackError = err.into();
    let report = format!("{:?}", StackReport::from_error(boxed));
    assert!(report.contains("operation failed"));
}
