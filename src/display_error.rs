use core::error::Error;
use core::fmt::{Debug, Display, Formatter};
use core::hash::{Hash, Hasher};

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
/// ```
/// use suzunari_error::*;
///
/// // A third-party type that implements Debug + Display but not Error.
/// #[derive(Debug)]
/// struct LibError(String);
/// impl std::fmt::Display for LibError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         f.write_str(&self.0)
///     }
/// }
///
/// #[suzunari_error]
/// #[suzu(display("operation failed"))]
/// struct AppError {
///     #[suzu(from)]
///     source: LibError,  // becomes DisplayError<LibError>
/// }
/// ```
///
/// ## Pattern B: Manual `source(from(...))` — explicit control
///
/// Uses `#[snafu(source(from(...)))]` directly with `DisplayError::new`.
/// Note: `DisplayError::new` always returns `None` from `source()`, so this
/// pattern does not preserve the source chain even if `LibError` implements
/// `Error`. Use Pattern A (`#[suzu(from)]`) for automatic source chain
/// preservation.
///
/// ```
/// use suzunari_error::*;
///
/// #[derive(Debug)]
/// struct LibError(String);
/// impl std::fmt::Display for LibError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         f.write_str(&self.0)
///     }
/// }
///
/// #[suzunari_error]
/// #[suzu(display("operation failed"))]
/// struct AppError {
///     #[snafu(source(from(LibError, DisplayError::new)))]
///     source: DisplayError<LibError>,
/// }
/// ```
///
/// ## Source chain preservation
///
/// When constructed via `#[suzu(from)]`, `DisplayError` automatically detects
/// whether the wrapped type implements `Error` at compile time (using autoref
/// specialization). If it does, `source()` delegates to the inner type's
/// `source()`. If not, `source()` returns `None`.
///
/// When constructed manually via [`DisplayError::new()`], `source()` always
/// returns `None`. Use `#[suzu(from)]` for automatic source chain preservation.
///
/// ## Pattern C: `map_err` — direct wrapping without snafu context
///
/// ```
/// use suzunari_error::DisplayError;
///
/// #[derive(Debug)]
/// struct LibError(String);
/// impl std::fmt::Display for LibError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         f.write_str(&self.0)
///     }
/// }
///
/// fn fallible() -> Result<(), LibError> {
///     Err(LibError("boom".into()))
/// }
///
/// // Wrap non-Error type into Error for use with ? or error combinators
/// fn do_something() -> Result<(), Box<dyn std::error::Error>> {
///     fallible().map_err(DisplayError::new)?;
///     Ok(())
/// }
/// ```
pub struct DisplayError<E> {
    inner: E,
    get_source: fn(&E) -> Option<&(dyn core::error::Error + 'static)>,
}

impl<E: Debug + Display> DisplayError<E> {
    /// Wraps `error` in a `DisplayError`, making it usable as a `source` field.
    ///
    /// `source()` will always return `None`. For automatic source chain
    /// preservation, use `#[suzu(from)]` instead.
    #[must_use]
    pub fn new(error: E) -> Self {
        Self {
            inner: error,
            get_source: |_| None,
        }
    }

    /// Internal constructor with an explicit `get_source` resolver.
    /// Use [`DisplayError::new`] in application code.
    pub(crate) fn with_get_source(
        error: E,
        get_source: fn(&E) -> Option<&(dyn core::error::Error + 'static)>,
    ) -> Self {
        Self {
            inner: error,
            get_source,
        }
    }
}

impl<E> DisplayError<E> {
    /// Returns a reference to the wrapped value.
    #[must_use]
    pub fn inner(&self) -> &E {
        &self.inner
    }

    /// Unwraps and returns the inner value.
    #[must_use]
    pub fn into_inner(self) -> E {
        self.inner
    }
}

/// Clones the inner `E` value and copies the `get_source` function pointer,
/// preserving source chain delegation behavior in the clone.
impl<E: Clone> Clone for DisplayError<E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            get_source: self.get_source,
        }
    }
}

impl<E: Display> Display for DisplayError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl<E: Debug> Debug for DisplayError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

/// Compares only the inner `E` value. Two `DisplayError<E>` instances are
/// considered equal if their inner values are equal, regardless of how they
/// were constructed. The `get_source` function pointer is an implementation
/// detail for source chain delegation and is not part of the value identity.
impl<E: PartialEq> PartialEq for DisplayError<E> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<E: Eq> Eq for DisplayError<E> {}

impl<E: Hash> Hash for DisplayError<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

/// Delegates `source()` to the stored `get_source` function pointer.
///
/// When constructed via `#[suzu(from)]` (macro-generated code), this
/// automatically delegates to the inner type's `source()` if it implements
/// `Error`, or returns `None` otherwise. When constructed via `new()`,
/// always returns `None`.
impl<E: Debug + Display> Error for DisplayError<E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        (self.get_source)(&self.inner)
    }
}

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
    fn test_inner_ref() {
        let wrapped = DisplayError::new(FakeLibError {
            message: "ref access",
        });
        assert_eq!(wrapped.inner().message, "ref access");
    }

    #[test]
    fn test_clone() {
        #[derive(Clone)]
        struct ClonableError {
            message: &'static str,
        }
        impl Display for ClonableError {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", self.message)
            }
        }
        impl Debug for ClonableError {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                write!(f, "ClonableError({})", self.message)
            }
        }

        let original = DisplayError::new(ClonableError { message: "test" });
        let cloned = original.clone();
        assert_eq!(cloned.inner().message, "test");
    }

    #[test]
    fn test_error_source_is_none() {
        let wrapped = DisplayError::new(FakeLibError {
            message: "no source",
        });
        let err: &dyn Error = &wrapped;
        assert!(err.source().is_none());
    }

    #[test]
    fn test_with_get_source_none_for_non_error() {
        let wrapped = DisplayError::with_get_source(
            FakeLibError {
                message: "no error impl",
            },
            |_| None,
        );
        let err: &dyn Error = &wrapped;
        assert!(err.source().is_none());
    }

    #[test]
    fn test_partial_eq() {
        let a = DisplayError::new(42);
        let b = DisplayError::new(42);
        let c = DisplayError::new(99);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_hash() {
        // Simple deterministic hasher that does not require std.
        struct SimpleHasher(u64);
        impl Hasher for SimpleHasher {
            fn finish(&self) -> u64 {
                self.0
            }
            fn write(&mut self, bytes: &[u8]) {
                for &b in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(b as u64);
                }
            }
        }
        fn hash_one<T: Hash>(val: &T) -> u64 {
            let mut h = SimpleHasher(0);
            val.hash(&mut h);
            h.finish()
        }
        let a = DisplayError::new(42);
        let b = DisplayError::new(42);
        assert_eq!(hash_one(&a), hash_one(&b));
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn test_with_get_source_delegates_to_inner() {
            #[derive(Debug)]
            struct InnerError;
            impl Display for InnerError {
                fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                    f.write_str("inner")
                }
            }
            impl Error for InnerError {}

            #[derive(Debug)]
            struct OuterError(InnerError);
            impl Display for OuterError {
                fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                    f.write_str("outer")
                }
            }
            impl Error for OuterError {
                fn source(&self) -> Option<&(dyn Error + 'static)> {
                    Some(&self.0)
                }
            }

            let wrapped = DisplayError::with_get_source(OuterError(InnerError), |e| e.source());
            let err: &dyn Error = &wrapped;
            let source = err.source().expect("source should delegate");
            assert_eq!(alloc::format!("{source}"), "inner");
        }

        #[test]
        fn test_clone_preserves_source_delegation() {
            #[derive(Clone, Debug)]
            struct InnerError;
            impl Display for InnerError {
                fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                    f.write_str("inner")
                }
            }
            impl Error for InnerError {}

            #[derive(Clone, Debug)]
            struct OuterError(InnerError);
            impl Display for OuterError {
                fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                    f.write_str("outer")
                }
            }
            impl Error for OuterError {
                fn source(&self) -> Option<&(dyn Error + 'static)> {
                    Some(&self.0)
                }
            }

            let original = DisplayError::with_get_source(OuterError(InnerError), |e| e.source());
            let cloned = original.clone();
            let err: &dyn Error = &cloned;
            let source = err
                .source()
                .expect("clone should preserve source delegation");
            assert_eq!(alloc::format!("{source}"), "inner");
        }

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
