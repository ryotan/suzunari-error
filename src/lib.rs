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
//! ```
//! use suzunari_error::*;
//!
//! #[suzunari_error]
//! #[suzu(display("operation failed"))]
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
//!
//! # Feature Flags
//!
//! | Feature | Default | Provides |
//! |---------|---------|----------|
//! | `std`   | Yes     | `alloc` + [`StackReport`]'s [`Termination`](std::process::Termination) impl + [`#[report]`](macro@report) macro |
//! | `alloc` | via `std` | [`BoxedStackError`] + `From<T> for BoxedStackError` generation |
//! | _(none)_ | —      | Core-only: [`Location`], [`StackError`], [`StackReport`] (formatting only), [`DisplayError`] |
//!
//! # `#[suzu(...)]` Attribute
//!
//! Use `#[suzu(...)]` for all attributes under [`#[suzunari_error]`](macro@suzunari_error).
//! It is a superset of `#[snafu(...)]` — standard snafu keywords (`display`, `source`,
//! `visibility`, etc.) pass through as-is, plus suzunari extensions are available.
//! `#[snafu(...)]` also works but `#[suzu(...)]` is preferred for consistency.
//!
//! Suzunari extensions:
//!
//! - **`from`** (field-level) — wraps field type in [`DisplayError<T>`] and generates
//!   `#[snafu(source(from(T, DisplayError::new)))]`
//! - **`location`** (field-level) — marks a field as the location field with a custom name;
//!   converts to `#[stack(location)]` + `#[snafu(implicit)]`
//!
//! # Known Limitations
//!
//! - **Location type detection** uses the last path segment name (`Location`), not the
//!   full path. A user-defined `my_module::Location` type may trigger false auto-detection.
//!   Use `#[suzu(location)]` or `#[stack(location)]` to disambiguate.
//! - **Crate renaming** (`my_error = { package = "suzunari-error" }`) is not supported.
//!   The generated code always references `::suzunari_error`. This matches the approach
//!   used by snafu and thiserror.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

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
pub use boxed_stack_error::BoxedStackError;
pub use display_error::DisplayError;
pub use location::Location;
pub use stack_error::StackError;
pub use stack_report::StackReport;

// Re-export snafu so downstream crates don't need it as a direct dependency.
// The proc-macro generates `#[snafu(crate_root(::suzunari_error::snafu))]`
// to redirect snafu's generated paths here.
// Note: bumping the snafu dependency version is a semver-breaking change for
// downstream crates, because these re-exports are part of our public API.
pub use snafu;
// OptionExt is not used within this crate but re-exported for downstream
// convenience: it enables `.context()` on `Option<T>` to convert None into errors.
pub use snafu::{OptionExt, ResultExt, ensure};

// Proc-macro re-exports (wildcard is the only way to re-export proc macros).
pub use suzunari_error_macro_impl::*;
