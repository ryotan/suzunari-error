//! Procedural macros for `suzunari-error`.
//!
//! Provides 3 proc-macros:
//!
//! - [`#[suzunari_error]`](suzunari_error) — The main entry point. Processes
//!   `#[suzu(...)]` attributes, injects `location: Location` fields, and appends
//!   `#[derive(Debug, Snafu, StackError)]`.
//! - [`#[derive(StackError)]`](derive_stack_error) — Generates `StackError` impl
//!   and `From<T> for BoxedStackError` (when `alloc` enabled).
//! - [`#[report]`](report) — Transforms `fn() -> Result<(), E>` into
//!   `fn() -> StackReport<E>` for formatted error output on failure (`std` only).

mod attribute;
mod derive;
mod helper;
mod report;
mod suzu_attr;

use crate::attribute::suzunari_error_impl;
use crate::derive::stack_error_impl;
use crate::report::report_impl;
use proc_macro::TokenStream;

/// Derives the [`StackError`] trait for a struct or enum.
///
/// Requires a `location: Location` field in every struct/variant (added
/// automatically by `#[suzunari_error]`).
/// Also generates `From<T> for BoxedStackError` when the `alloc` feature is enabled.
#[proc_macro_derive(StackError)]
pub fn derive_stack_error(input: TokenStream) -> TokenStream {
    stack_error_impl(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// The main entry point for defining error types.
///
/// Processes `#[suzu(...)]` attributes (suzunari extensions + snafu passthrough),
/// injects `location: Location` fields, and appends
/// `#[derive(Debug, Snafu, StackError)]`.
///
/// # `#[suzu(...)]` attributes
///
/// `#[suzu(...)]` is a superset of `#[snafu(...)]`. All snafu keywords are
/// passed through as-is. Additionally:
///
/// - **`translate`** (field-level): Wraps the field type in `DisplayError<T>` and
///   generates `#[snafu(source(from(T, DisplayError::new)))]`.
/// - **`location`** (field-level): Marks a field as the location field and adds
///   `#[snafu(implicit)]`. Suppresses automatic location injection for that
///   struct/variant.
#[proc_macro_attribute]
pub fn suzunari_error(_attr: TokenStream, item: TokenStream) -> TokenStream {
    suzunari_error_impl(item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Transforms `fn() -> Result<(), E>` into `fn() -> StackReport<E>`.
///
/// Primarily designed for `fn main()` where `StackReport`'s `Termination` impl
/// formats error chains on failure. Can also be applied to other functions to
/// convert `Result<(), E>` to `StackReport<E>` (e.g., for testing).
///
/// Does not support generics, `where` clauses, `async fn`, or type aliases.
#[proc_macro_attribute]
pub fn report(attr: TokenStream, item: TokenStream) -> TokenStream {
    report_impl(attr.into(), item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
