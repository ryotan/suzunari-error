#![cfg(feature = "std")]
//! Integration tests for the `#[suzu(...)]` attribute.
//! Tests verify `from`, `location`, and snafu passthrough behavior.
//!
//! `.build()` usage: These tests use `.build()` to construct errors at a known
//! line number for location assertions. `.context()` would capture the wrong line.

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
    let report = format!("{:?}", StackReport::from(err));
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
    let report = format!("{:?}", StackReport::from(err));
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
    let report = format!("{:?}", StackReport::from(err));
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
    let report = format!("{:?}", StackReport::from(err));
    let lines: Vec<&str> = report.lines().collect();
    // AlreadyWrappedError -> DisplayError<FakeLibError> (no source chain)
    assert!(lines[0].starts_with("Error: AlreadyWrappedError: already wrapped, at "));
    assert_eq!(lines[1], "Caused by (recent first):");
    assert_eq!(lines[2], "  1| already wrapped");
    assert_eq!(lines.len(), 3);
}

// --- from: already DisplayError<T> where T: Error (source chain preserved) ---
// When the field is already DisplayError<T> and T implements Error, #[suzu(from)]
// should still set up source chain delegation through autoref specialization.

#[suzunari_error]
#[suzu(display("already wrapped with chain"))]
struct AlreadyWrappedWithChainError {
    #[suzu(from)]
    source: DisplayError<RealOuter>,
}

#[test]
fn test_from_already_display_error_with_error_inner_preserves_chain() {
    fn real_op() -> Result<(), RealOuter> {
        Err(RealOuter(RealInner))
    }
    let err = real_op().context(AlreadyWrappedWithChainSnafu).unwrap_err();
    use std::error::Error;
    let display_err = err.source().expect("should have source (DisplayError)");
    let inner = display_err
        .source()
        .expect("DisplayError should delegate to RealOuter::source()");
    assert_eq!(format!("{inner}"), "real inner");
}

// --- from: non-source-named field ---
// #[suzu(from)] generates #[snafu(source(from(...)))] which implicitly marks
// the field as a source, so it works regardless of field name.

#[suzunari_error]
#[suzu(display("renamed source"))]
struct RenamedSourceError {
    #[suzu(from)]
    cause: FakeLibError,
}

#[test]
fn test_from_non_source_named_field() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError { message: "renamed" })
    }
    let err = fake_op().context(RenamedSourceSnafu).unwrap_err();
    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains("renamed source"));
    assert!(report.contains("renamed"));
}

// --- from: generic type parameter ---
// #[suzu(from)] on generic type params is now rejected at compile time.
// See tests/compile-fail/suzu_from_generic_type_param.rs

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
    let report = format!("{:?}", StackReport::from(err));
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
    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains("mixed display: ctx"));
    assert!(report.contains("mixed boom"));
}

// --- from + location on different fields in the same struct ---

#[suzunari_error]
#[suzu(display("combined from and location"))]
struct CombinedFromLocationError {
    #[suzu(from)]
    source: FakeLibError,
    #[suzu(location)]
    origin: Location,
}

#[test]
fn test_from_and_location_on_different_fields() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "combined test",
        })
    }
    let err = fake_op().context(CombinedFromLocationSnafu).unwrap_err();
    // Verify both effects are applied: `from` wraps source, `location` is tracked
    assert!(err.location().file().ends_with("suzu_attr_test.rs"));
    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains("combined from and location"));
    assert!(report.contains("combined test"));
}

// --- from + location on different fields in an enum variant ---

#[suzunari_error]
enum CombinedFromLocationEnum {
    #[suzu(display("enum combined"))]
    Combined {
        #[suzu(from)]
        source: FakeLibError,
        #[suzu(location)]
        origin: Location,
    },
}

#[test]
fn test_from_and_location_on_different_fields_enum() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "enum combined",
        })
    }
    let err = fake_op().context(CombinedSnafu).unwrap_err();
    assert!(err.location().file().ends_with("suzu_attr_test.rs"));
    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains("enum combined"));
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
    // OuterError -> FromEnumError::HashFailed -> DisplayError<FakeLibError>
    assert_eq!(err.depth(), 2);
    let report = format!("{:?}", StackReport::from(err));
    let lines: Vec<&str> = report.lines().collect();
    // Line 0: top-level error with type name and location
    assert!(lines[0].starts_with("Error: OuterError: outer error, at "));
    // Line 1: "Caused by" header
    assert_eq!(lines[1], "Caused by (recent first):");
    // Line 2: StackError cause with index, type name, message, and location
    assert!(lines[2].starts_with("  1| FromEnumError::HashFailed: hashing failed, at "));
    // Line 3: plain Error cause (DisplayError wrapping FakeLibError, no location)
    assert_eq!(lines[3], "  2| hash fail");
    assert_eq!(lines.len(), 4);
}

// --- from: source chain preservation for Error-implementing types ---
// When the inner type implements Error, #[suzu(from)] should preserve the
// source chain via autoref specialization. DisplayError::source() delegates
// to the inner Error::source().
//
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
#[suzu(display("wrapped with source chain"))]
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
    // The DisplayError should delegate source() to RealOuter::source(),
    // which returns RealInner.
    use std::error::Error;
    let display_err = err.source().expect("should have source (DisplayError)");
    let inner = display_err
        .source()
        .expect("DisplayError should delegate to RealOuter::source()");
    assert_eq!(format!("{inner}"), "real inner");
    // Verify StackReport output hierarchy:
    // SourceChainError -> DisplayError<RealOuter> -> RealInner
    let report = format!("{:?}", StackReport::from(err));
    let lines: Vec<&str> = report.lines().collect();
    assert!(lines[0].starts_with("Error: SourceChainError: wrapped with source chain, at "));
    assert_eq!(lines[1], "Caused by (recent first):");
    // DisplayError delegates source() to RealOuter, which has RealInner as source
    assert_eq!(lines[2], "  1| real outer");
    assert_eq!(lines[3], "  2| real inner");
    assert_eq!(lines.len(), 4);
}

#[test]
fn test_from_returns_none_source_for_non_error_types() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "no Error impl",
        })
    }
    let err = fake_op().context(HashFailedSnafu).unwrap_err();
    // FakeLibError doesn't implement Error, so DisplayError::source() → None.
    use std::error::Error;
    let display_err = err.source().expect("should have source (DisplayError)");
    assert!(
        display_err.source().is_none(),
        "non-Error type should have None source"
    );
}

// --- GAP-10: closure syntax in source(from(...)) ---
// Verifies the token-level fallback scanner correctly detects `source`
// when Meta parsing fails due to closure syntax.

#[suzunari_error]
#[suzu(display("closure source error"))]
struct ClosureSourceError {
    // Closure syntax triggers the token-level fallback in is_source_field
    // because |e| fails Meta parsing. The extra block makes it non-trivial
    // for clippy's redundant_closure lint.
    #[snafu(source(from(FakeLibError, |e| { DisplayError::new(e) })))]
    cause: DisplayError<FakeLibError>,
}

#[test]
fn test_closure_syntax_source() {
    fn fake_op() -> Result<(), FakeLibError> {
        Err(FakeLibError {
            message: "closure test",
        })
    }
    let err = fake_op().context(ClosureSourceSnafu).unwrap_err();
    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains("closure source error"));
    assert!(report.contains("closure test"));
}
