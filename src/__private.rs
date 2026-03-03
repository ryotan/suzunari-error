//! Internal helpers for derive macro code generation.
//!
//! **Not public API. Do not use.** This module is `#[doc(hidden)]` and not
//! covered by semver guarantees. It exists solely for generated code emitted
//! by `#[derive(StackError)]` and `#[suzunari_error]`.
//!
//! Uses the **autoref specialization** technique to conditionally resolve
//! trait-dependent behavior at compile time. When a source type implements
//! the target trait, the inherent method takes priority via autoref.
//! Otherwise, `Deref` coercion kicks in, calling the fallback method.
//! This avoids requiring trait bounds on source types in generated code.
//!
//! See: <https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md>

use crate::StackError;
use core::error::Error;

// ---------------------------------------------------------------------------
// StackSourceResolver — resolves StackError::stack_source()
// ---------------------------------------------------------------------------

/// Wraps a reference and resolves to the inherent `resolve()` method
/// when `T: StackError`, or falls back via `Deref` → `NotStackErrorFallback`
/// when `T` does not implement `StackError`.
pub struct StackSourceResolver<'a, T: ?Sized>(pub &'a T);

impl<'a, T: StackError> StackSourceResolver<'a, T> {
    pub fn resolve(&self) -> Option<&'a dyn StackError> {
        Some(self.0)
    }
}

/// Fallback target via Deref. Always returns `None`.
pub struct NotStackErrorFallback;

impl NotStackErrorFallback {
    // 'static is required even though this always returns None: with elided
    // lifetime, the return type would be tied to the temporary NotStackErrorFallback
    // created via Deref in generated code, causing a borrow-checker error.
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
// DisplayErrorSourceResolver — resolves get_source fn for DisplayError
// ---------------------------------------------------------------------------

/// Resolves the `get_source` function pointer for [`DisplayError`](crate::DisplayError).
///
/// Uses the same Deref-based autoref specialization as `StackSourceResolver`.
/// When `T: Error + 'static`, the inherent `get_source_fn()` takes priority.
/// Otherwise, Deref falls back to `DisplayErrorSourceFallback`.
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

/// Fallback target via Deref. Returns a `get_source` fn that always yields `None`.
pub struct DisplayErrorSourceFallback;

impl DisplayErrorSourceFallback {
    // The generic `<T>` here requires callers to provide a type annotation
    // so the compiler can infer which `T` to use.
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
