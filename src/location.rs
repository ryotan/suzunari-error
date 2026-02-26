/// Source code location captured via `#[track_caller]`.
///
/// A newtype wrapper around `core::panic::Location` that integrates with
/// snafu's `GenerateImplicitData` for automatic capture at error construction sites.
pub struct Location(&'static core::panic::Location<'static>);

impl Location {
    #[track_caller]
    pub fn current() -> Self {
        Self(core::panic::Location::caller())
    }
}

impl core::ops::Deref for Location {
    type Target = core::panic::Location<'static>;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl core::fmt::Debug for Location {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}:{}", self.file(), self.line(), self.column())
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
