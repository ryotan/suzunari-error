//! Internal helpers for derive macro code generation.
//!
//! **Not public API. Do not use.** This module is `#[doc(hidden)]` and not
//! covered by semver guarantees. It exists solely for generated code emitted
//! by `#[derive(StackError)]` and `#[suzunari_error]`.
//!
//! Two forms of autoref specialization are used here. `stack_source()` uses the
//! `Deref`-based form: the inherent method wins when `T: StackError`, otherwise
//! `Deref` reaches the fallback. The `source` field in `payload` uses a
//! trait-based form instead, because its fallback has to borrow the value and a
//! `Deref` target cannot. Both avoid requiring trait bounds on source types in
//! generated code.
//!
//! See: <https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md>

use crate::StackError;
use crate::display_error::DisplayError;
use core::error::Error;
use core::fmt::{Debug, Display};

#[cfg(feature = "serde")]
pub mod payload;

// Generated code refers to serde through this re-export, never a bare `::serde`,
// so downstream crates don't need serde as a direct dependency.
#[cfg(feature = "serde")]
pub use serde;

// ---------------------------------------------------------------------------
// StackSourceResolver — resolves StackError::stack_source()
// ---------------------------------------------------------------------------

/// Wraps a source field so `resolve()` reaches either the inherent method below
/// or [`NotStackErrorFallback`].
pub struct StackSourceResolver<'a, T: ?Sized>(pub &'a T);

impl<'a, T: StackError> StackSourceResolver<'a, T> {
    #[must_use]
    pub fn resolve(&self) -> Option<&'a dyn StackError> {
        Some(self.0)
    }
}

/// Reached by `Deref` when `T` does not implement `StackError`.
pub struct NotStackErrorFallback;

impl NotStackErrorFallback {
    // 'static is required even though this always returns None: with elided
    // lifetime, the return type would be tied to the temporary NotStackErrorFallback
    // created via Deref in generated code, causing a borrow-checker error.
    #[must_use]
    pub fn resolve(&self) -> Option<&'static dyn StackError> {
        None
    }
}

impl<T: ?Sized> core::ops::Deref for StackSourceResolver<'_, T> {
    type Target = NotStackErrorFallback;
    fn deref(&self) -> &NotStackErrorFallback {
        &NotStackErrorFallback
    }
}

// ---------------------------------------------------------------------------
// DisplayError construction helper for macro-generated code
// ---------------------------------------------------------------------------

/// Creates a [`DisplayError`] with an explicit `get_source` resolver.
///
/// [`DisplayError::new`] is the one to use outside generated code.
#[must_use]
pub fn display_error_with_get_source<E: Debug + Display>(
    error: E,
    get_source: fn(&E) -> Option<&(dyn Error + 'static)>,
) -> DisplayError<E> {
    DisplayError::with_get_source(error, get_source)
}

// ---------------------------------------------------------------------------
// DisplayErrorSourceResolver — resolves get_source fn for DisplayError
// ---------------------------------------------------------------------------

/// Resolves the `get_source` function pointer for [`DisplayError`](crate::DisplayError),
/// the same way [`StackSourceResolver`] resolves `stack_source()`.
///
/// The fallback's `get_source_fn` has a method-level generic `<T>`, so callers
/// must provide an explicit type annotation for inference to succeed:
/// ```ignore
/// let __get_source: fn(&OriginalType) -> Option<&(dyn Error + 'static)>
///     = DisplayErrorSourceResolver(&val).get_source_fn();
/// ```
pub struct DisplayErrorSourceResolver<'a, T>(pub &'a T);

impl<T: Error + 'static> DisplayErrorSourceResolver<'_, T> {
    #[must_use]
    pub fn get_source_fn(&self) -> fn(&T) -> Option<&(dyn Error + 'static)> {
        |e| e.source()
    }
}

/// Reached by `Deref` when `T` does not implement `Error`.
pub struct DisplayErrorSourceFallback;

impl DisplayErrorSourceFallback {
    #[must_use]
    pub fn get_source_fn<T>(&self) -> fn(&T) -> Option<&(dyn Error + 'static)> {
        |_| None
    }
}

impl<T> core::ops::Deref for DisplayErrorSourceResolver<'_, T> {
    type Target = DisplayErrorSourceFallback;
    fn deref(&self) -> &DisplayErrorSourceFallback {
        &DisplayErrorSourceFallback
    }
}
