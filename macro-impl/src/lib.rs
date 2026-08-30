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
mod serialize;
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
///   generates a `source(from(...))` conversion that automatically preserves the
///   `Error::source()` chain when the wrapped type implements `Error`.
/// - **`location`** (field-level): Marks a field as the location field. Converts
///   to `#[stack(location)]` + `#[snafu(implicit)]`. Allows custom field names
///   instead of the default `location`. Requires a `Location` type.
///
/// # Options
///
/// - **`serialize`**: also generates `impl serde::Serialize`, emitting the error
///   chain as a nested payload. Requires the crate's `serde` feature. Opt-in per
///   type, because error types with non-`Serialize` fields are normal and
///   because cargo feature unification would otherwise infect the whole graph.
///   - **`serialize(rename_all = "...")`** renames the declared fields, taking
///     the same cases serde does. It is the only container-level setting
///     accepted: `#[serde(...)]` on the type or on a variant is refused, because
///     the definition generated from it is a different container. One spelling
///     covers both shapes — serde needs `rename_all` on a struct and
///     `rename_all_fields` on an enum, where plain `rename_all` would rename
///     variants that never reach the payload.
///
/// A declared field's own `#[serde(...)]` is carried into the generated
/// definition and applies as it would on a struct derived directly. Two are
/// refused: anything on the `source` or `location` field, which the definition
/// skips, and `getter`, which serde accepts only inside a remote definition.
#[proc_macro_attribute]
pub fn suzunari_error(attr: TokenStream, item: TokenStream) -> TokenStream {
    suzunari_error_impl(attr.into(), item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Transforms `fn() -> Result<(), E>` into `fn() -> StackReport<E>`.
///
/// Primarily designed for `fn main()` where `StackReport`'s `Termination` impl
/// formats error chains on failure. Can also be applied to other functions to
/// convert `Result<(), E>` to `StackReport<E>` (e.g., for testing).
///
/// # Usage
///
/// Use the qualified path `#[suzunari_error::report]` (not `#[report]` alone):
///
/// ```rust,ignore
/// use suzunari_error::*;
///
/// #[suzunari_error::report]
/// fn main() -> Result<(), AppError> {
///     run()?;
///     Ok(())
/// }
/// ```
///
/// # Limitations
///
/// - Does not support generics, `where` clauses, `async fn`, `const fn`,
///   `unsafe fn`, or `extern fn`.
/// - Return type must be written literally as `Result<(), E>` (type aliases
///   are not resolved).
/// - Function parameters with complex patterns (e.g., `(a, b): (u32, u32)`)
///   are forwarded as-is to the generated closure, which may not compile
///   depending on the pattern form.
#[proc_macro_attribute]
pub fn report(attr: TokenStream, item: TokenStream) -> TokenStream {
    report_impl(attr.into(), item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
