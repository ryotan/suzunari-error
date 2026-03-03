#![cfg(feature = "test-core-only")]

use suzunari_error::*;

#[test]
fn test_location_core_only() {
    let loc = Location::current();
    assert!(loc.line() > 0);
    assert!(loc.column() > 0);
    assert!(!loc.file().is_empty());
}

// StackReport is available in the core-only tier (without Termination impl).
// Uses raw #[derive(Snafu)] + manual StackError impl to test the trait
// behavior independently from the proc-macro layer.
#[derive(Debug, snafu::Snafu)]
#[snafu(display("core error"))]
struct CoreTestError {
    #[snafu(implicit)]
    location: Location,
}

impl StackError for CoreTestError {
    fn location(&self) -> &Location {
        &self.location
    }
    fn type_name(&self) -> &'static str {
        "CoreTestError"
    }
}

// Simple stack buffer for core::fmt::Write (no alloc needed).
struct StackBuf {
    buf: [u8; 512],
    len: usize,
}

impl StackBuf {
    fn new() -> Self {
        Self {
            buf: [0; 512],
            len: 0,
        }
    }
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap()
    }
}

impl core::fmt::Write for StackBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buf.len() - self.len;
        if bytes.len() > remaining {
            return Err(core::fmt::Error);
        }
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

#[test]
fn test_stack_report_core_only() {
    use core::fmt::Write;

    let error = CoreTestError {
        location: Location::current(),
    };
    let report = StackReport::from(error);

    // StackReport::Display works in core-only (uses core::fmt)
    let mut buf = StackBuf::new();
    write!(buf, "{report}").unwrap();
    assert!(buf.as_str().contains("Error: CoreTestError: core error"));
    assert!(buf.as_str().contains("at"));
}

#[test]
fn test_display_error_new_and_into_inner() {
    let wrapped = DisplayError::new("test");
    let inner = wrapped.into_inner();
    assert_eq!(inner, "test");
}

// --- Source chain delegation via #[suzu(from)] in core-only tier ---

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

#[suzunari_error]
#[suzu(display("source chain core"))]
struct SourceChainCoreError {
    #[suzu(from)]
    source: RealOuter,
}

#[test]
fn test_from_preserves_source_chain_core_only() {
    fn real_op() -> Result<(), RealOuter> {
        Err(RealOuter(RealInner))
    }
    let err = real_op().context(SourceChainCoreSnafu).unwrap_err();
    use core::error::Error;
    let display_err = err.source().expect("should have source (DisplayError)");
    let inner = display_err
        .source()
        .expect("DisplayError should delegate to RealOuter::source()");
    // No alloc::format! in core-only, use StackBuf for verification
    use core::fmt::Write;
    let mut buf = StackBuf::new();
    write!(buf, "{inner}").unwrap();
    assert_eq!(buf.as_str(), "real inner");
}

#[test]
fn test_from_returns_none_source_core_only() {
    // DisplayError::new() always returns None from source(), even in core-only tier.
    let wrapped = DisplayError::new("not an error");
    let err: &dyn core::error::Error = &wrapped;
    assert!(err.source().is_none());
}
