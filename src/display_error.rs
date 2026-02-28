use core::error::Error;
use core::fmt::{Debug, Display, Formatter};

/// Wrapper that converts a `Debug + Display` type (without `Error` impl) into
/// a `core::error::Error`.
///
/// Useful for wrapping third-party error types that don't implement `Error`,
/// making them usable as snafu `source` fields.
///
/// # Usage
///
/// ## Pattern A: `#[suzu(from)]` — auto-wraps type and generates `source(from(...))` (recommended)
///
/// ```rust,ignore
/// // ignore: uses external crate type for illustration
/// #[suzunari_error]
/// #[suzu(display("hashing failed"))]
/// struct HashError {
///     #[suzu(from)]
///     source: argon2::Error,  // becomes DisplayError<argon2::Error>
/// }
/// ```
///
/// ## Pattern B: Manual `source(from(...))` — explicit control
///
/// ```rust,ignore
/// // ignore: uses external crate type for illustration
/// #[suzunari_error]
/// #[suzu(display("hashing failed"))]
/// struct HashError {
///     #[suzu(source(from(argon2::Error, DisplayError::new)))]
///     source: DisplayError<argon2::Error>,
/// }
/// ```
///
/// ## Pattern C: `map_err` — for ad-hoc conversions
///
/// ```rust,ignore
/// // ignore: uses external crate type for illustration
/// fn hash(input: &[u8]) -> Result<Vec<u8>, HashError> {
///     do_hash(input)
///         .map_err(DisplayError::new)
///         .context(HashSnafu)?;
///     // ...
/// }
/// ```
pub struct DisplayError<E>(E);

impl<E: Debug + Display> DisplayError<E> {
    /// Wraps `error` in a `DisplayError`, making it usable as a `source` field.
    pub fn new(error: E) -> Self {
        Self(error)
    }
}

impl<E> DisplayError<E> {
    /// Unwraps and returns the inner value.
    pub fn into_inner(self) -> E {
        self.0
    }
}

impl<E: Display> Display for DisplayError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<E: Debug> Debug for DisplayError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

/// `source()` always returns `None` because the wrapped type does not
/// implement `Error`, so there is no underlying source to delegate to.
///
/// **Warning:** Wrapping a type that already implements `Error` will lose its
/// original source chain. This type is intended only for non-`Error` types.
impl<E: Debug + Display> Error for DisplayError<E> {}

// No From impl — intentionally omitted to prevent implicit .into() conversions.

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeLibError {
        message: &'static str,
    }
    impl Display for FakeLibError {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(f, "{}", self.message)
        }
    }
    impl Debug for FakeLibError {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(f, "FakeLibError({})", self.message)
        }
    }

    #[test]
    fn test_new_and_into_inner() {
        let original = FakeLibError { message: "oops" };
        let wrapped = DisplayError::new(original);
        let inner = wrapped.into_inner();
        assert_eq!(inner.message, "oops");
    }

    #[test]
    fn test_error_source_is_none() {
        let wrapped = DisplayError::new(FakeLibError {
            message: "no source",
        });
        let err: &dyn Error = &wrapped;
        assert!(err.source().is_none());
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn test_display_delegates() {
            let wrapped = DisplayError::new(FakeLibError {
                message: "display me",
            });
            let s = alloc::format!("{wrapped}");
            assert_eq!(s, "display me");
        }

        #[test]
        fn test_debug_delegates() {
            let wrapped = DisplayError::new(FakeLibError {
                message: "debug me",
            });
            let s = alloc::format!("{wrapped:?}");
            assert_eq!(s, "FakeLibError(debug me)");
        }
    }
}
