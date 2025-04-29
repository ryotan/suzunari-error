use suzunari_error::{Location, StackError, suzunari_location};
use snafu::prelude::*;

// Test struct with StackError derive macro
#[suzunari_location]
#[derive(Snafu, StackError)]
#[snafu(display("{}", message))]
struct TestError {
    message: String,
}

// Test struct with manual location field
#[derive(Snafu, StackError)]
struct TestErrorWithLocation {
    message: String,
    #[snafu(implicit)]
    location: Location,
}

// No need to implement Error manually, Snafu already does this

// Test enum with StackError derive macro
#[suzunari_location]
#[derive(Snafu, StackError)]
enum TestErrorEnum {
    Variant1 {
        message: String,
    },
    Variant2 {
        context: String,
    },
}

// Test enum with manual location field
#[derive(Snafu, StackError)]
enum TestErrorEnumWithLocation {
    Variant3 {
        message: String,
        location: Location,
    },
    Variant4 {
        context: String,
        location: Location,
    },
}

// No need to implement Error manually, Snafu already does this

#[test]
fn test_stack_error_derive() {
    let error = TestSnafu {
        message: "Test error".to_string(),
    }.build();

    let file = file!();
    let line = line!() - 3;
    assert_eq!(error.location().file(), file);
    assert_eq!(format!("{error:?}"), format!("0: Test error, at {file}:{line}:7\n"));
}

#[test]
fn test_suzunari_location_attribute() {
    let error = TestErrorWithLocationSnafu {
        message: "Test error".to_string(),
    }.build();

    let file = file!();
    let line = line!() - 3;
    assert_eq!(error.location().file(), file);
    assert_eq!(format!("{error:?}"), format!("0: TestErrorWithLocation, at {file}:{line}:7\n"));
}

#[test]
fn test_stack_error_enum_derive() {
    let error = Variant1Snafu {
        message: "Test error".to_string(),
    }.build();

    let file = file!();
    let line = line!() - 3;
    assert_eq!(error.location().file(), file);
    assert_eq!(format!("{error:?}"), format!("0: Variant1, at {file}:{line}:7\n"));

    let error = Variant2Snafu {
        context: "Test context".to_string(),
    }.build();

    let file = file!();
    let line = line!() - 3;
    assert_eq!(error.location().file(), file);
    assert_eq!(format!("{error:?}"), format!("0: Variant2, at {file}:{line}:7\n"));
}

#[test]
fn test_suzunari_location_enum_attribute() {
    let error = TestErrorEnumWithLocation::Variant3 {
        message: "Test error".to_string(),
        location: Location::current(),
    };

    let file = file!();
    let line = line!() - 4;
    assert_eq!(error.location().file(), file);
    assert_eq!(format!("{error:?}"), format!("0: Variant3, at {file}:{line}:19\n"));

    let error = TestErrorEnumWithLocation::Variant4 {
        context: "Test context".to_string(),
        location: Location::current(),
    };

    let file = file!();
    let line = line!() - 4;
    assert_eq!(error.location().file(), file);
    assert_eq!(format!("{error:?}"), format!("0: Variant4, at {file}:{line}:19\n"));
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
    assert_eq!(format!("{error:?}"), format!("0: Root error, at {file}:{line}:19\n"));
}
