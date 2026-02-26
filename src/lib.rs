//! A highly traceable and noise-free error handling library for Rust.
//!
//! Built on [SNAFU](https://docs.rs/snafu), this crate propagates error locations
//! as error contexts and formats error chains as stack-trace-like reports.
//! `#![no_std]` compatible with 3 feature tiers: core-only / `alloc` / `std`.
//!
//! # Quick Start
//!
//! Use [`#[suzunari_error]`](macro@suzunari_error) to define error types — it combines
//! location injection, `Snafu` derive, and `StackError` derive in one attribute:
//!
//! ```rust,ignore
//! use suzunari_error::*;
//! use snafu::ResultExt;
//!
//! #[suzunari_error]
//! #[snafu(display("operation failed"))]
//! struct AppError {
//!     source: std::io::Error,
//! }
//! ```
//!
//! # Key Types
//!
//! - [`Location`] — Captures call-site file/line/column via `#[track_caller]`
//! - [`StackError`] — Extends `Error` with `location()`, `type_name()`, and `stack_source()`
//! - [`StackReport`] — Formats a `StackError` chain for display with location info
//! - [`BoxedStackError`] — Type-erased `StackError` wrapper (requires `alloc`)
//! - [`DisplayError`] — Adapter for `Debug + Display` types that don't implement `Error`

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod display_error;
mod location;
mod stack_error;
mod stack_report;

#[doc(hidden)]
pub mod __private;

#[cfg(feature = "alloc")]
mod boxed_stack_error;

#[cfg(feature = "alloc")]
pub use boxed_stack_error::*;
pub use display_error::*;
pub use location::*;
pub use stack_error::*;
pub use stack_report::*;
pub use suzunari_error_macro_impl::*;
