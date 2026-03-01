#![cfg(feature = "std")]
// Tests verify derive macros and attributes individually and in combination.
// Uses #[suzunari_error] for the all-in-one macro, raw #[derive(StackError)] with
// manual location field, and manual enum construction to test each layer independently.
// .build() is snafu's standard test pattern.

use snafu::prelude::*;
use suzunari_error::{Location, StackError, StackReport, suzunari_error};

// Test struct with #[suzunari_error] (auto-injects location)
#[suzunari_error]
#[snafu(display("{}", message))]
struct TestError {
    message: String,
}

// Test struct with manual location field (raw derive)
#[derive(Debug, Snafu, StackError)]
struct TestErrorWithLocation {
    message: String,
    #[snafu(implicit)]
    location: Location,
}

// Test enum with #[suzunari_error]
#[suzunari_error]
enum TestErrorEnum {
    Variant1 { message: String },
    Variant2 { context: String },
}

// Test enum with manual location field (raw derive)
#[derive(Debug, Snafu, StackError)]
enum TestErrorEnumWithLocation {
    Variant3 { message: String, location: Location },
    Variant4 { context: String, location: Location },
}

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
fn test_manual_location_struct() {
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

// Generic struct with #[suzunari_error]
#[suzunari_error]
#[suzu(display("generic: {value}"))]
struct GenericError<T: core::fmt::Display + core::fmt::Debug> {
    value: T,
}

#[test]
fn test_generic_struct() {
    let error: GenericError<i32> = GenericSnafu { value: 42 }.build();
    assert!(error.location().file().ends_with("macro_test.rs"));
    assert_eq!(format!("{error}"), "generic: 42");
}

// Generic enum with #[suzunari_error]
#[suzunari_error]
enum GenericEnumError<T: core::fmt::Display + core::fmt::Debug> {
    #[suzu(display("a: {value}"))]
    VariantA { value: T },
    #[suzu(display("b: {msg}"))]
    VariantB { msg: String },
}

#[test]
fn test_generic_enum() {
    let error: GenericEnumError<i32> = VariantASnafu { value: 99 }.build();
    assert!(error.location().file().ends_with("macro_test.rs"));
    assert_eq!(format!("{error}"), "a: 99");

    let error: GenericEnumError<i32> = VariantBSnafu {
        msg: "hello".to_string(),
    }
    .build();
    assert_eq!(format!("{error}"), "b: hello");
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
