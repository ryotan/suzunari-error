#![no_std]

#[cfg(feature = "test-alloc")]
extern crate alloc;

// --- core-only tier: always-available API ---
use suzunari_error::{DisplayError, Location, StackError};
use suzunari_error::{write_error_log, write_stack_error_log};

// Derive macros work in core-only mode
#[suzunari_error::suzunari_error]
#[snafu(display("core only error"))]
pub struct CoreOnlyError {}

// DisplayError is available in core-only mode
fn _use_display_error() {
    let _: DisplayError<&str> = DisplayError::new("test");
}

// write_stack_error_log / write_error_log are available in core-only mode
fn _use_write_functions(_loc: &Location, _err: &dyn StackError) {
    // Type-level availability check (no need to call)
    let _ =
        write_stack_error_log as fn(&mut core::fmt::Formatter, &CoreOnlyError) -> core::fmt::Result;
    let _ = write_error_log as fn(&mut core::fmt::Formatter, &CoreOnlyError) -> core::fmt::Result;
}

// --- alloc tier ---
#[cfg(feature = "test-alloc")]
mod alloc_tests {
    use suzunari_error::BoxedStackError;

    // BoxedStackError is available
    fn _use_boxed(e: super::CoreOnlyError) {
        let _: BoxedStackError = BoxedStackError::new(e);
    }

    // Derive macro generates From<T> for BoxedStackError
    fn _use_from(e: super::CoreOnlyError) {
        let _: BoxedStackError = e.into();
    }
}

// std tier is a superset of alloc.
// std = ["alloc", ...] ensures all alloc features are available by definition,
// so no separate compile-check module is needed.
