#![cfg(feature = "std")]
//! Integration tests for the Location struct and its integration with snafu.
//!
//! These tests verify that the Location struct properly captures source code locations
//! and integrates well with the snafu error handling system.

use snafu::{Snafu, ensure};
use std::collections::HashSet;
use std::path::Path;
use suzunari_error::Location;

/// Tests the automatic generation of Location when used as an implicit field in a snafu error.
///
/// This test verifies that:
/// 1. The Location is correctly captured when an error is generated using snafu
/// 2. The file name and line number in the Location match where the error was created
/// 3. The error's Display and Debug formats work as expected
// Tests Location's GenerateImplicitData integration with raw snafu,
// without #[suzunari_error], to verify the trait impl works independently.
#[test]
fn test_snafu_implicit_generation() {
    #[derive(Debug, Snafu)]
    struct SomeError {
        #[snafu(implicit)]
        location: Location,
    }

    fn some_function() -> Result<(), SomeError> {
        // capture the current location
        ensure!(false, SomeSnafu);
        Ok(())
    }

    let error = some_function().unwrap_err();

    let file = file!();
    let line = line!() - 7; // 7 lines above is where SomeSnafu is used
    assert_eq!(format!("{error}"), "SomeError");
    // Location is a type alias for &'static core::panic::Location<'static>, so Debug
    // produces struct-style output via core::panic::Location's own derive(Debug).
    let debug = format!("{error:?}");
    // file!() is escaped via {:?} to match Debug's backslash escaping on Windows.
    assert!(debug.contains(&format!("file: {:?}", file)));
    assert!(debug.contains(&format!("line: {line}")));
    assert!(debug.contains("col: 9"));
}

/// Tests using Location with a custom error type manually.
///
/// This test demonstrates:
/// 1. How to manually include a Location in a custom error type
/// 2. How to access the location information from the error
#[test]
fn test_manual_location_in_error() {
    #[derive(Debug)]
    struct CustomError {
        message: String,
        location: Location,
    }

    impl core::fmt::Display for CustomError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{} (at {:?})", self.message, self.location)
        }
    }

    fn create_error() -> CustomError {
        CustomError {
            message: "Something went wrong".to_string(),
            location: core::panic::Location::caller(),
        }
    }

    let error = create_error();

    // Verify the location in the error
    let expected_file = file!();
    let expected_line = line!() - 8; // 8 lines above is where core::panic::Location::caller() was called

    assert_eq!(error.location.file(), expected_file);
    assert_eq!(error.location.line(), expected_line);

    // Verify the location appears in the display format.
    // Display uses {:?} for Location, so backslashes are escaped on Windows.
    let display_str = format!("{}", error);
    assert!(display_str.contains(&format!("{:?}", expected_file)));
    assert!(display_str.contains(&expected_line.to_string()));
}

/// Tests the compatibility of Location with filesystem paths.
///
/// This test verifies that:
/// 1. The file path in Location can be converted to a Path
/// 2. The file path is valid and exists
#[test]
fn test_location_file_path() {
    let loc = core::panic::Location::caller();
    let file_path = loc.file();

    // Test that the file path can be converted to a Path
    let path = Path::new(file_path);

    // Verify this path is a valid file path
    assert!(path.exists(), "The file path from Location should exist");
    assert!(path.is_file(), "The file path should point to a file");

    // Verify the path contains the expected file name
    let file_name = path.file_name().unwrap().to_str().unwrap();
    assert_eq!(
        file_name, "location_test.rs",
        "The file name should match this test file"
    );
}

// --- GAP-03: Location PartialEq, Eq, Hash, Clone, Copy ---

#[test]
fn test_location_eq_same_site() {
    // Copy of the same Location should be equal
    let a = core::panic::Location::caller();
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn test_location_ne_different_site() {
    let a = core::panic::Location::caller();
    let b = core::panic::Location::caller();
    // Different lines → not equal
    assert_ne!(a, b);
}

#[test]
fn test_location_hash() {
    let loc = core::panic::Location::caller();
    let mut set = HashSet::new();
    set.insert(loc);
    // The same location inserted again — set size should not change
    set.insert(loc);
    assert_eq!(set.len(), 1);

    // Different location should increase set size
    let loc2 = core::panic::Location::caller();
    set.insert(loc2);
    assert_eq!(set.len(), 2);
}

#[test]
fn test_location_copy() {
    let loc = core::panic::Location::caller();
    let copied = loc; // Copy
    // Both should be usable (not moved)
    assert_eq!(loc.line(), copied.line());
    assert_eq!(loc.file(), copied.file());
}

#[test]
fn test_location_clone() {
    let loc = core::panic::Location::caller();
    // Intentionally using clone() on a Copy type to verify the Location alias supports Clone.
    #[allow(clippy::clone_on_copy)]
    let cloned: Location = Clone::clone(&loc);
    assert_eq!(loc, cloned);
}
