//! Location information for errors
//!
//! This module provides the `Location` struct for representing error occurrence locations.
//! Using this struct, you can obtain the exact file name, line number, and column number
//! where an error occurred. It's primarily designed to be used in conjunction with the
//! `snafu` crate.
//!
//! # Examples
//!
//! ```rust
//! use suzunari_error::Location;
//!
//! // Get information about the current call site
//! let location = Location::current();
//! println!("Error occurred at: {location:?}"); // Outputs e.g., src/example.rs:10:5
//! ```

/// A structure representing a location in source code.
///
/// This struct wraps Rust's standard library `core::panic::Location`, making it easier
/// to track error occurrence locations. Internally, it holds a reference to a
/// `core::panic::Location` with a static lifetime.
///
/// # Examples
///
/// ```rust
/// use suzunari_error::Location;
///
/// let loc = Location::current();
/// println!("Current location: {loc:?}"); // Outputs in file:line:column format
/// ```
pub struct Location(&'static core::panic::Location<'static>);

impl Location {
    /// Creates a Location from the current call site.
    ///
    /// This function uses the `#[track_caller]` attribute, which means it automatically
    /// captures information about the location where this function was called.
    ///
    /// # Returns
    ///
    /// A `Location` instance containing information about the current call site.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use suzunari_error::Location;
    ///
    /// let loc = Location::current();
    /// assert_eq!(loc.file(), file!());
    /// assert_eq!(loc.line(), line!() - 2);
    /// assert!(0 < loc.column());
    /// ```
    #[track_caller]
    pub fn current() -> Self {
        Self(core::panic::Location::caller())
    }
}

/// Enables direct access to the underlying `core::panic::Location` through `Location`.
///
/// This implementation allows you to directly access methods of the original
/// `core::panic::Location` through a `Location` instance (e.g., `file()`, `line()`, `column()`).
impl core::ops::Deref for Location {
    type Target = core::panic::Location<'static>;

    /// Returns a reference to the inner `core::panic::Location`.
    ///
    /// # Returns
    ///
    /// A reference to the inner `core::panic::Location` held by this `Location`.
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Defines the display format for `Location`.
///
/// With this implementation, the `Debug` output of a `Location` will be in the format
/// "filename:line_number:column_number".
///
/// # Examples
///
/// ```rust
/// use suzunari_error::Location;
///
/// let loc = Location::current();
/// println!("{loc:?}"); // Outputs e.g., "src/location.rs:123:45"
/// ```
impl core::fmt::Debug for Location {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}:{}", self.file(), self.line(), self.column())
    }
}

/// Trait implementation for integration with the `snafu` crate.
///
/// This implementation allows you to add `Location` as an implicit field to error
/// structures using `snafu`. This automatically records the location where the error
/// was generated.
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
///     Ok(())
/// }
/// ```
impl snafu::GenerateImplicitData for Location {
    /// Generates a `Location` from the current call site.
    ///
    /// This function is automatically called by the `snafu` crate and sets the value
    /// for the `location` field in the error structure.
    ///
    /// # Returns
    ///
    /// A `Location` instance containing information about the current call site.
    #[track_caller]
    fn generate() -> Self {
        Self::current()
    }
}

/// Tests for the `Location` struct
#[cfg(test)]
mod tests {
    use super::*;
    use snafu::GenerateImplicitData;

    /// Tests for the `current()` and `generate()` methods
    ///
    /// Verifies that both methods correctly capture the file and line number,
    /// and that a valid column number is provided.
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

    /// Test for the `Debug` implementation
    ///
    /// Verifies that the Debug format produces the expected string representation
    /// in the format "file:line:column".
    #[test]
    fn test_debug_format() {
        let loc = Location::current();
        assert_eq!(
            format!("{:?}", loc),
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        );
    }

    /// Test for the `Deref` implementation
    ///
    /// Verifies that methods from `core::panic::Location` can be directly accessed
    /// through a Location instance via deref coercion.
    #[test]
    fn test_deref() {
        let loc = Location::current();

        // Direct access to file() method from the inner core::panic::Location
        let file_str = loc.file();
        assert_eq!(file_str, file!());

        // Direct access to line() method from the inner core::panic::Location
        let line_num = loc.line();
        assert!(line_num > 0, "Line should be a positive number");

        // Direct access to column() method from the inner core::panic::Location
        let col_num = loc.column();
        assert!(col_num > 0, "Column should be a positive number");
    }

    /// Tests the GenerateImplicitData implementation for integration with snafu
    ///
    /// Verifies that the implementation correctly provides location information
    /// when used with snafu's error generation mechanisms.
    #[test]
    fn test_generate_implicit_data() {
        // Define a test function that captures the current location
        fn get_location_via_implicit_data() -> Location {
            Location::generate()
        }

        let loc = get_location_via_implicit_data();

        // Verify this location points to the get_location_via_implicit_data function
        assert_eq!(loc.file(), file!());

        // The location should have a valid line number and column
        assert!(loc.line() > 0, "Line should be a positive number");
        assert!(loc.column() > 0, "Column should be a positive number");
    }

    /// Tests consistency between Location methods
    ///
    /// Verifies that different ways of accessing location information
    /// provide consistent results.
    #[test]
    fn test_method_consistency() {
        let loc = Location::current();

        // Direct access vs Debug format
        let direct_format = format!("{}:{}:{}", loc.file(), loc.line(), loc.column());
        let debug_format = format!("{:?}", loc);

        assert_eq!(
            direct_format, debug_format,
            "Direct format and Debug format should match"
        );

        // Create a new location and verify it differs from the first
        fn get_another_location() -> Location {
            Location::current()
        }

        let another_loc = get_another_location();

        // Locations from different call sites should differ
        assert_ne!(
            format!("{:?}", loc),
            format!("{:?}", another_loc),
            "Locations from different call sites should differ"
        );
    }
}
