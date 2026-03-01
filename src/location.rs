use core::fmt::{Debug, Display, Formatter, Result};
use core::ops::Deref;

/// Source code location captured via `#[track_caller]`.
///
/// A newtype wrapper around `core::panic::Location` that integrates with
/// snafu's `GenerateImplicitData` for automatic capture at error construction sites.
/// Implements `Copy`, `Eq`, and `Hash`. Derefs to `core::panic::Location` for
/// access to `file()`, `line()`, and `column()`.
///
/// # Example
///
/// ```
/// use suzunari_error::Location;
///
/// let loc = Location::current();
/// assert!(loc.file().ends_with(".rs"));
/// assert!(loc.line() > 0);
/// ```
///
/// When using `#[suzunari_error]`, Location fields are automatically injected
/// and populated — you rarely need to call `current()` directly.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Location(&'static core::panic::Location<'static>);

impl Location {
    /// Returns the location of the immediate caller.
    ///
    /// Captures file, line, and column of the call site via `#[track_caller]`.
    #[must_use]
    #[track_caller]
    pub fn current() -> Self {
        Self(core::panic::Location::caller())
    }
}

// NOTE: Deref on a non-smart-pointer type deviates from C-DEREF guidelines.
// Pragmatic for v0.1: gives ergonomic access to file()/line()/column().
// Risk: methods added to core::panic::Location auto-appear on Location.
// Revisit before v1.0 — consider replacing with explicit delegation methods.
impl Deref for Location {
    type Target = core::panic::Location<'static>;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}:{}:{}", self.file(), self.line(), self.column())
    }
}

impl Debug for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Display::fmt(self, f)
    }
}

impl snafu::GenerateImplicitData for Location {
    #[track_caller]
    fn generate() -> Self {
        Self::current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snafu::GenerateImplicitData;

    #[test]
    fn test_current() {
        let loc = Location::current();
        assert_eq!(loc.file(), file!());
        assert_eq!(loc.line(), line!() - 2);
        assert!(loc.column() > 0, "Column should be a positive number");

        let loc = Location::generate();
        assert_eq!(loc.file(), file!());
        assert_eq!(loc.line(), line!() - 2);
        assert!(loc.column() > 0, "Column should be a positive number");
    }

    #[test]
    fn test_deref() {
        let loc = Location::current();

        let file_str = loc.file();
        assert_eq!(file_str, file!());

        let line_num = loc.line();
        assert!(line_num > 0, "Line should be a positive number");
        let col_num = loc.column();
        assert!(col_num > 0, "Column should be a positive number");
    }

    #[test]
    fn test_generate_implicit_data() {
        fn get_location_via_implicit_data() -> Location {
            Location::generate()
        }

        let loc = get_location_via_implicit_data();

        assert_eq!(loc.file(), file!());
        assert!(loc.line() > 0, "Line should be a positive number");
        assert!(loc.column() > 0, "Column should be a positive number");
    }

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;
        use alloc::format;

        #[test]
        fn test_debug_format() {
            let loc = Location::current();
            assert_eq!(
                format!("{:?}", loc),
                format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
            );
        }

        #[test]
        fn test_method_consistency() {
            let loc = Location::current();

            let direct_format = format!("{}:{}:{}", loc.file(), loc.line(), loc.column());
            let debug_format = format!("{:?}", loc);

            assert_eq!(
                direct_format, debug_format,
                "Direct format and Debug format should match"
            );

            fn get_another_location() -> Location {
                Location::current()
            }
            let another_loc = get_another_location();
            assert_ne!(
                format!("{:?}", loc),
                format!("{:?}", another_loc),
                "Locations from different call sites should differ"
            );
        }
    }
}
