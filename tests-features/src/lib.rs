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

// depth() and stack_source() are available as StackError default methods
fn _use_trait_methods(err: &CoreOnlyError) {
    let _: usize = err.depth();
    let _: Option<&dyn StackError> = err.stack_source();
    let _: &str = err.type_name();
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

// --- std tier ---
#[cfg(feature = "test-std")]
mod std_tests {
    extern crate std;

    use suzunari_error::StackReport;

    // StackReport implements Termination (std-only)
    fn _report_is_termination()
    where
        StackReport<super::CoreOnlyError>: std::process::Termination,
    {
    }

    // #[suzunari_error::report] macro works
    #[suzunari_error::report]
    fn _report_macro_works() -> Result<(), super::CoreOnlyError> {
        Ok(())
    }
}
