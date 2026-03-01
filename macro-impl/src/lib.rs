//! Procedural macros for `suzunari-error`.
//!
//! Provides 3 proc-macros:
//!
//! - [`#[suzunari_error]`](suzunari_error) — The main entry point. Processes
//!   `#[suzu(...)]` attributes, resolves/injects location fields, and appends
//!   `#[derive(Debug, Snafu, StackError)]`.
//! - [`#[derive(StackError)]`](derive_stack_error) — Generates `StackError` impl
//!   and `From<T> for BoxedStackError` (when `alloc` enabled). Finds location field
//!   via `#[stack(location)]` marker or `Location` type.
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
/// Requires a `Location` field in every struct/variant (added automatically
/// by `#[suzunari_error]`). The location field is resolved by:
/// 1. `#[stack(location)]` marker — highest priority, supports any field name
/// 2. Single field of type `Location` — automatic fallback
/// 3. Error if neither is found
///
/// When using `#[suzunari_error]`, `#[suzu(location)]` on a field becomes
/// `#[stack(location)]` + `#[snafu(implicit)]`.
///
/// Also generates `From<T> for BoxedStackError` when the `alloc` feature is enabled.
#[proc_macro_derive(StackError, attributes(stack))]
pub fn derive_stack_error(input: TokenStream) -> TokenStream {
    stack_error_impl(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// The main entry point for defining error types.
///
/// Processes `#[suzu(...)]` attributes (suzunari extensions + snafu passthrough),
/// resolves/injects location fields, and appends
/// `#[derive(Debug, Snafu, StackError)]`.
///
/// # `#[suzu(...)]` attributes
///
/// `#[suzu(...)]` is a superset of `#[snafu(...)]`. All snafu keywords are
/// passed through as-is. Additionally:
///
/// - **`from`** (field-level): Wraps the field type in `DisplayError<T>` and
///   generates `#[snafu(source(from(T, DisplayError::new)))]`.
/// - **`location`** (field-level): Marks a field as the location field. Converts
///   to `#[stack(location)]` + `#[snafu(implicit)]`. Allows custom field names
///   instead of the default `location`. Requires `Location` type.
#[proc_macro_attribute]
pub fn suzunari_error(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr2: proc_macro2::TokenStream = attr.into();
    if !attr2.is_empty() {
        use syn::spanned::Spanned;
        return syn::Error::new(
            attr2.span(),
            "#[suzunari_error] does not accept arguments; use #[suzu(...)] on fields instead",
        )
        .to_compile_error()
        .into();
    }
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
