#![no_std]

#[cfg(feature = "test-alloc")]
extern crate alloc;

// --- core-only tier: always-available API ---
use suzunari_error::{DisplayError, StackError};

// Derive macros work in core-only mode
#[suzunari_error::suzunari_error]
#[snafu(display("core only error"))]
pub struct CoreOnlyError {}

// DisplayError is available in core-only mode
fn _use_display_error() {
    let _: DisplayError<&str> = DisplayError::new("test");
}

// depth() and fmt_stack() are available as StackError default methods
fn _use_trait_methods(err: &CoreOnlyError) {
    let _: usize = err.depth();
    let _ =
        StackError::fmt_stack as fn(&CoreOnlyError, &mut core::fmt::Formatter) -> core::fmt::Result;
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
