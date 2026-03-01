//! Internal helpers for derive macro code generation.
//! Not public API — may change without notice.
//!
//! Uses the **autoref specialization** technique to conditionally resolve
//! `stack_source()` at compile time. When the source type implements
//! `StackError`, the inherent `resolve()` method takes priority via autoref.
//! Otherwise, `Deref` coercion kicks in, calling the fallback `resolve()` on
//! `NotStackErrorFallback` which returns `None`. This avoids requiring
//! `StackError` bounds on source types in generated code.
//!
//! See: <https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md>

use crate::StackError;

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
