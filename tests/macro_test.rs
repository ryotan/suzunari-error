#![cfg(feature = "std")]
// Tests verify derive macros and attributes individually and in combination.
// Uses #[suzunari_error] with #[suzu(location)] for explicit location fields,
// raw #[derive(StackError)] with manual location field, and #[suzunari_error]
// with auto-injection to test each layer independently.
// .build() is snafu's standard test pattern.

use snafu::prelude::*;
use suzunari_error::{Location, StackError, StackReport, suzunari_error};

// Test struct with StackError derive macro and explicit location via #[suzu(location)]
#[suzunari_error]
#[snafu(display("{}", message))]
struct TestError {
    message: String,
}

// Test struct with manual location field
#[derive(Debug, Snafu, StackError)]
struct TestErrorWithLocation {
    message: String,
    #[snafu(implicit)]
    location: Location,
}

// No need to implement Error manually, Snafu already does this

// Test enum with StackError derive macro
#[suzunari_error]
enum TestErrorEnum {
    Variant1 { message: String },
    Variant2 { context: String },
}

// Test enum with manual location field
#[derive(Debug, Snafu, StackError)]
enum TestErrorEnumWithLocation {
    Variant3 { message: String, location: Location },
    Variant4 { context: String, location: Location },
}

// No need to implement Error manually, Snafu already does this

#[test]
fn test_stack_error_derive() {
    let error = TestSnafu {
        message: "Test error".to_string(),
    }
    .build();

    let file = file!();
    let line = line!() - 3;
    assert_eq!(error.location().file(), file);
    assert_eq!(
        format!("{:?}", StackReport::from_error(error)),
        format!("Error: TestError: Test error, at {file}:{line}:6\n")
    );
}

#[test]
fn test_manual_location_field() {
    let error = TestErrorWithLocationSnafu {
        message: "Test error".to_string(),
    }
    .build();

    let file = file!();
    let line = line!() - 3;
    assert_eq!(error.location().file(), file);
    assert_eq!(
        format!("{:?}", StackReport::from_error(error)),
        format!("Error: TestErrorWithLocation: TestErrorWithLocation, at {file}:{line}:6\n")
    );
}

#[test]
fn test_stack_error_enum_derive() {
    let error = Variant1Snafu {
        message: "Test error".to_string(),
    }
    .build();

    let file = file!();
    let line = line!() - 3;
    assert_eq!(error.location().file(), file);
    assert_eq!(
        format!("{:?}", StackReport::from_error(error)),
        format!("Error: TestErrorEnum::Variant1: Variant1, at {file}:{line}:6\n")
    );

    let error = Variant2Snafu {
        context: "Test context".to_string(),
    }
    .build();

    let file = file!();
    let line = line!() - 3;
    assert_eq!(error.location().file(), file);
    assert_eq!(
        format!("{:?}", StackReport::from_error(error)),
        format!("Error: TestErrorEnum::Variant2: Variant2, at {file}:{line}:6\n")
    );
}

#[test]
fn test_manual_location_enum() {
    let error = TestErrorEnumWithLocation::Variant3 {
        message: "Test error".to_string(),
        location: Location::current(),
    };

    let file = file!();
    let line = line!() - 4;
    assert_eq!(error.location().file(), file);
    assert_eq!(
        format!("{:?}", StackReport::from_error(error)),
        format!("Error: TestErrorEnumWithLocation::Variant3: Variant3, at {file}:{line}:19\n")
    );

    let error = TestErrorEnumWithLocation::Variant4 {
        context: "Test context".to_string(),
        location: Location::current(),
    };

    let file = file!();
    let line = line!() - 4;
    assert_eq!(error.location().file(), file);
    assert_eq!(
        format!("{:?}", StackReport::from_error(error)),
        format!("Error: TestErrorEnumWithLocation::Variant4: Variant4, at {file}:{line}:19\n")
    );
}

#[test]
fn test_chain_context() {
    let error = TestError {
        message: "Root error".to_string(),
        location: Location::current(),
    };

    let file = file!();
    let line = line!() - 4;
    assert_eq!(error.location().file(), file);
    assert_eq!(
        format!("{:?}", StackReport::from_error(error)),
        format!("Error: TestError: Root error, at {file}:{line}:19\n")
    );
}
