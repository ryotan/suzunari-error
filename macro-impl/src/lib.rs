//! Procedural macros for `suzunari-error`.
//!
//! Provides 4 proc-macros:
//!
//! - [`#[suzunari_error]`](suzunari_error) — The main entry point. Combines
//!   `#[suzunari_location]` + `#[derive(Debug, Snafu, StackError)]`.
//! - [`#[suzunari_location]`](suzunari_location) — Auto-adds `location: Location`
//!   field with `#[snafu(implicit)]` to structs and each enum variant.
//! - [`#[derive(StackError)]`](derive_stack_error) — Generates `StackError` impl
//!   and `From<T> for BoxedStackError` (when `alloc` enabled).
//! - [`#[report]`](report) — Transforms `fn() -> Result<(), E>` into
//!   `fn() -> StackReport<E>` for formatted error output on failure (`std` only).

mod attribute;
mod derive;
mod helper;
mod report;

use crate::attribute::{suzunari_error_impl, suzunari_location_impl};
use crate::derive::stack_error_impl;
use crate::report::report_impl;
use proc_macro::TokenStream;

/// Derives the [`StackError`] trait for a struct or enum.
///
/// Requires a `location: Location` field in every struct/variant (added
/// automatically by `#[suzunari_location]` or `#[suzunari_error]`).
/// Also generates `From<T> for BoxedStackError` when the `alloc` feature is enabled.
#[proc_macro_derive(StackError)]
pub fn derive_stack_error(input: TokenStream) -> TokenStream {
    stack_error_impl(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Auto-adds a `location: Location` field with `#[snafu(implicit)]` to structs
/// and each enum variant. Skips if a `location` field already exists.
#[proc_macro_attribute]
pub fn suzunari_location(_attr: TokenStream, item: TokenStream) -> TokenStream {
    suzunari_location_impl(item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// The main entry point for defining error types.
///
/// Combines `#[suzunari_location]` + `#[derive(Debug, Snafu, StackError)]` in
/// a single attribute. Use this by default for all error type definitions.
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
