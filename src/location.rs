//! Location information for errors

/// A structure representing a location in source code.
pub struct Location(&'static core::panic::Location<'static>);

impl Location {
    /// Creates a Location from the current call site.
    #[track_caller]
    pub fn current() -> Self {
        Self(std::panic::Location::caller())
    }
}

impl std::ops::Deref for Location {
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

/// Creates an implicit Location to add to an error.
///
/// # Examples
///
/// ```rust
/// use suzunari_error::Location;
/// use snafu::prelude::*;
///
/// #[derive(Debug, Snafu)]
/// struct SomeError {
///     #[snafu(implicit)]
///     location: Location,
/// }
///
/// fn some_function() -> Result<(), SomeError> {
///     // capture the current location
///     ensure!(false, SomeSnafu);
/// }
/// ```
impl snafu::GenerateImplicitData for Location {
    #[track_caller]
    fn generate() -> Self {
        Self::current()
    }
}

#[cfg(test)]
mod tests {
    use snafu::GenerateImplicitData;
    use super::*;
    #[test]
    fn test_current() {
        let loc = Location::current();
        assert_eq!(loc.file(), file!());
        assert_eq!(loc.line(), line!() - 2);
        assert_eq!(loc.column(), 19);

        let loc = Location::generate();
        assert_eq!(loc.file(), file!());
        assert_eq!(loc.line(), line!() - 2);
        assert_eq!(loc.column(), 19);
    }

    #[test]
    fn test_debug_format() {
        let loc = Location::current();
        assert_eq!(
            format!("{:?}", loc),
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        );
    }
}
