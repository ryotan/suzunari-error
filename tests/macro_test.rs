#![cfg(feature = "std")]
// Tests verify derive macros and attributes individually and in combination.
// Uses #[suzunari_error] for the all-in-one macro, raw #[derive(StackError)] with
// manual location field, and manual enum construction to test each layer independently.
// .build() is snafu's standard test pattern.

use snafu::prelude::*;
use suzunari_error::{Location, StackError, StackReport, suzunari_error};

// Test struct with #[suzunari_error] (auto-injects location)
#[suzunari_error]
#[suzu(display("{}", message))]
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

// --- GAP-05: source(false) interaction ---

#[suzunari_error]
#[suzu(display("source false test"))]
struct SourceFalseError {
    // Field named "source" but explicitly disabled as a snafu source.
    // stack_source() should return None for this field.
    #[suzu(source(false))]
    source: String,
}

#[test]
fn test_source_false_suppresses_stack_source() {
    let err = SourceFalseSnafu {
        source: "not a real source".to_string(),
    }
    .build();
    assert!(
        err.stack_source().is_none(),
        "source(false) should suppress stack_source()"
    );
    // Error::source() should also be None because snafu respects source(false)
    use core::error::Error;
    assert!(err.source().is_none());
}

// --- GAP-07: single-variant enum ---

#[suzunari_error]
enum SingleVariantEnum {
    #[suzu(display("only variant: {msg}"))]
    Only { msg: String },
}

#[test]
fn test_single_variant_enum() {
    let err = OnlySnafu {
        msg: "hello".to_string(),
    }
    .build();
    assert_eq!(err.type_name(), "SingleVariantEnum::Only");
    assert!(err.location().file().ends_with("macro_test.rs"));
    assert_eq!(format!("{err}"), "only variant: hello");
}

// --- GAP-11: generic struct with where clause ---

#[suzunari_error]
#[suzu(display("where clause: {value}"))]
struct WhereClauseError<T>
where
    T: core::fmt::Display + core::fmt::Debug + Send + Sync + 'static,
{
    value: T,
}

#[test]
fn test_generic_with_where_clause() {
    let err: WhereClauseError<i32> = WhereClauseSnafu { value: 123 }.build();
    assert_eq!(format!("{err}"), "where clause: 123");
    assert!(err.location().file().ends_with("macro_test.rs"));

    // Verify From<T> for BoxedStackError works with where clause
    let boxed: suzunari_error::BoxedStackError = err.into();
    assert_eq!(format!("{boxed}"), "where clause: 123");
}

// --- GAP-12: error types in nested modules ---

mod nested {
    use suzunari_error::*;

    #[suzunari_error]
    #[suzu(visibility(pub), display("nested module error"))]
    pub struct NestedModError {}

    #[suzunari_error]
    #[suzu(visibility(pub))]
    pub enum NestedModEnum {
        #[suzu(display("nested variant"))]
        Variant {},
    }
}

#[test]
fn test_nested_module_errors() {
    fn make_struct_error() -> Result<(), nested::NestedModError> {
        ensure!(false, nested::NestedModSnafu);
        Ok(())
    }
    let err = make_struct_error().unwrap_err();
    assert_eq!(err.type_name(), "NestedModError");
    assert!(err.location().file().ends_with("macro_test.rs"));

    fn make_enum_error() -> Result<(), nested::NestedModEnum> {
        ensure!(false, nested::VariantSnafu);
        Ok(())
    }
    let err = make_enum_error().unwrap_err();
    assert_eq!(err.type_name(), "NestedModEnum::Variant");
}
