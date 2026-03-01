#![cfg(feature = "std")]
//! Integration tests for the `#[suzu(...)]` attribute.
//! Tests verify `from`, `location`, and snafu passthrough behavior.
//! .build() is snafu's standard test pattern for constructing errors in tests.

use snafu::prelude::*;
use suzunari_error::*;

// --- from: basic enum usage ---

// Simulates a third-party error without Error impl
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
enum FromEnumError {
    #[suzu(display("hashing failed"))]
    HashFailed {
        #[suzu(from)]
        source: FakeLibError,
    },
    #[suzu(display("io error"))]
    Io { source: std::io::Error },
}

#[test]
fn test_from_enum_basic() {
    fn fake_hash() -> Result<(), FakeLibError> {
        Err(FakeLibError { message: "boom" })
    }
    let err = fake_hash().context(HashFailedSnafu).unwrap_err();
    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("hashing failed"));
    assert!(report.contains("boom"));
}

#[test]
fn test_from_enum_non_from_variant() {
    fn io_op() -> Result<(), std::io::Error> {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not found",
        ))
    }
    let err = io_op().context(IoSnafu).unwrap_err();
    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("io error"));
    assert!(report.contains("not found"));
}

// --- from: struct usage ---

#[suzunari_error]
#[suzu(display("struct from error"))]
struct FromStructError {
    #[suzu(from)]
    source: FakeLibError,
}

#[test]
fn test_from_struct() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "struct boom",
        })
    }
    let err = fake_op().context(FromStructSnafu).unwrap_err();
    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("struct from error"));
    assert!(report.contains("struct boom"));
}

// --- from: already DisplayError<T> (no double-wrapping) ---

#[suzunari_error]
#[suzu(display("already wrapped"))]
struct AlreadyWrappedError {
    #[suzu(from)]
    source: DisplayError<FakeLibError>,
}

#[test]
fn test_from_already_display_error() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "already wrapped",
        })
    }
    let err = fake_op().context(AlreadyWrappedSnafu).unwrap_err();
    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("already wrapped"));
}

// --- location: explicit #[suzu(location)] ---

#[suzunari_error]
#[suzu(display("explicit location"))]
struct ExplicitLocationError {
    #[suzu(location)]
    location: Location,
}

#[test]
fn test_explicit_location() {
    let err = ExplicitLocationSnafu.build();
    assert!(err.location().file().ends_with("suzu_attr_test.rs"));
}

// --- location: mixed explicit and auto-inject in enum ---

#[suzunari_error]
enum MixedLocationEnum {
    #[suzu(display("auto injected"))]
    AutoInjected { message: String },
    #[suzu(display("explicit loc"))]
    ExplicitLoc {
        message: String,
        #[suzu(location)]
        location: Location,
    },
}

#[test]
fn test_mixed_location_enum() {
    let err = AutoInjectedSnafu {
        message: "auto".to_string(),
    }
    .build();
    assert!(err.location().file().ends_with("suzu_attr_test.rs"));

    let err = ExplicitLocSnafu {
        message: "explicit".to_string(),
    }
    .build();
    assert!(err.location().file().ends_with("suzu_attr_test.rs"));
}

// --- snafu passthrough only (no suzunari extensions) ---

#[suzunari_error]
enum PassthroughOnlyError {
    #[suzu(display("pass {msg}"))]
    Passthrough { msg: String },
}

#[test]
fn test_passthrough_only() {
    let err = PassthroughSnafu {
        msg: "through".to_string(),
    }
    .build();
    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("pass through"));
}

// --- mixed: suzu(display(...)) on variant + suzu(from) on field ---

#[suzunari_error]
enum MixedSuzuError {
    #[suzu(display("mixed display: {context}"))]
    Mixed {
        context: String,
        #[suzu(from)]
        source: FakeLibError,
    },
}

#[test]
fn test_mixed_suzu_attrs() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "mixed boom",
        })
    }
    let err = fake_op()
        .context(MixedSnafu {
            context: "ctx".to_string(),
        })
        .unwrap_err();
    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("mixed display: ctx"));
    assert!(report.contains("mixed boom"));
}

// --- StackReport output verification ---

#[suzunari_error]
#[suzu(display("outer error"))]
struct OuterError {
    source: FromEnumError,
}

#[test]
fn test_stack_report_with_from_chain() {
    fn fake_hash() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "hash fail",
        })
    }
    fn inner() -> Result<(), FromEnumError> {
        fake_hash().context(HashFailedSnafu)?;
        Ok(())
    }
    fn outer() -> Result<(), OuterError> {
        inner().context(OuterSnafu)?;
        Ok(())
    }

    let err = outer().unwrap_err();
    // Should have depth 2: OuterError -> FromEnumError::HashFailed -> DisplayError
    assert_eq!(err.depth(), 2);
    let report = format!("{:?}", StackReport::from_error(err));
    assert!(report.contains("outer error"));
    assert!(report.contains("hashing failed"));
    assert!(report.contains("hash fail"));
}
