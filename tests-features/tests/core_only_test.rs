#![cfg(feature = "test-core-only")]

use suzunari_error::*;

#[test]
fn test_location_core_only() {
    let loc = Location::current();
    assert!(loc.line() > 0);
    assert!(loc.column() > 0);
    assert!(!loc.file().is_empty());
}

// StackReport is available in core-only tier (without Termination impl).
// Uses raw #[derive(Snafu)] + manual StackError impl because #[suzunari_error]
// relies on proc-macro-crate resolution which only works in the main crate.
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
    let report = StackReport::from_error(error);

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
