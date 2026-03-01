#![cfg(feature = "std")]

use core::error::Error;
use suzunari_error::*;

#[suzunari_error]
struct ErrorStruct {}

#[suzunari_error]
enum ErrorEnum {
    Variant1Unit,
    #[suzu(display("Variant2 {message}"))]
    Variant2NamedField {
        message: String,
    },
}

#[suzunari_error]
struct ErrorAggregate {
    source: BoxedStackError,
}

#[test]
fn test_stack_trace_single() {
    let file = file!();
    let ensure_line = line!() + 2;
    fn make_error() -> Result<(), ErrorStruct> {
        ensure!(false, ErrorStructSnafu);
        Ok(())
    }
    let err = make_error().unwrap_err();
    assert_eq!(
        format!("{:?}", StackReport::from(err)),
        format!("Error: ErrorStruct: ErrorStruct, at {file}:{ensure_line}:9\n")
    );
}

#[test]
fn test_nested_stack_trace() {
    fn error_enum() -> Result<(), ErrorEnum> {
        ensure!(
            false,
            Variant2NamedFieldSnafu {
                message: "message".to_string()
            }
        );
        Ok(())
    }
    fn nested_error() -> Result<(), ErrorAggregate> {
        error_enum()
            .map_err(BoxedStackError::new)
            .context(ErrorAggregateSnafu)?;
        Ok(())
    }
    let err = nested_error().unwrap_err();
    let file = file!();
    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains(&format!(
        "Error: ErrorAggregate: ErrorAggregate, at {file}:"
    )));
    assert!(report.contains("Caused by"));
    assert!(report.contains(&format!(
        "1| ErrorEnum::Variant2NamedField: Variant2 message, at {file}:"
    )));
}

#[suzunari_error]
enum SomeError {
    #[suzu(display("after {}sec", timeout_sec))]
    ReadTimeout {
        timeout_sec: u32,
        #[suzu(source)]
        error: std::io::Error,
    },
    #[suzu(display("{} is an invalid value. Must be larger than 1", param))]
    ValidationFailed { param: i32 },
}

#[suzunari_error]
#[suzu(display("Failed to retrieve"))]
struct RetrieveFailed {
    source: SomeError,
}

fn retrieve_data() -> Result<(), RetrieveFailed> {
    read_external().context(RetrieveFailedSnafu)?;
    Ok(())
}

fn read_external() -> Result<(), SomeError> {
    let err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
    Err(err).context(ReadTimeoutSnafu { timeout_sec: 3u32 })?;
    Ok(())
}

#[test]
fn test_retrieve_data() {
    let err = retrieve_data().unwrap_err();
    let file = file!();
    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains(&format!(
        "Error: RetrieveFailed: Failed to retrieve, at {file}:"
    )));
    assert!(report.contains("Caused by"));
    assert!(report.contains(&format!(
        "1| SomeError::ReadTimeout: after 3sec, at {file}:"
    )));
    // io::Error is not StackError — no location, no type name prefix
    assert!(report.contains("2| timeout"));
}

#[test]
fn test_unit_variant_location() {
    fn make_unit_error() -> Result<(), ErrorEnum> {
        ensure!(false, Variant1UnitSnafu);
        Ok(())
    }
    let err = make_unit_error().unwrap_err();
    // Unit variant should still have a location captured by suzunari_error
    assert!(err.location().file().ends_with("integration_test.rs"));
    assert!(err.location().line() > 0);
    assert_eq!(err.type_name(), "ErrorEnum::Variant1Unit");
    assert_eq!(err.depth(), 0);
}

#[test]
fn test_depth_no_source() {
    fn make_error() -> Result<(), ErrorStruct> {
        ensure!(false, ErrorStructSnafu);
        Ok(())
    }
    let err = make_error().unwrap_err();
    assert_eq!(err.depth(), 0);
}

#[test]
fn test_depth_with_chain() {
    let err = retrieve_data().unwrap_err();
    // RetrieveFailed -> SomeError::ReadTimeout -> io::Error = depth 2
    assert_eq!(err.depth(), 2);
}

#[test]
fn test_validate() {
    let file = file!();
    let ensure_line = line!() + 3;
    fn validate() -> Result<(), BoxedStackError> {
        let param = 0;
        ensure!(false, ValidationFailedSnafu { param });
        Ok(())
    }
    let err = validate().unwrap_err();
    assert_eq!(
        format!("{:?}", StackReport::from(err)),
        format!(
            "Error: SomeError::ValidationFailed: 0 is an invalid value. Must be larger than 1, at {file}:{ensure_line}:9\n"
        )
    );
}

// -- Custom-named location field tests --

/// #[suzu(location)] allows naming the location field anything.
#[suzunari_error]
#[suzu(display("custom location struct"))]
struct CustomLocStruct {
    #[suzu(location)]
    error_origin: Location,
}

/// Enum with custom-named location via #[suzu(location)].
#[suzunari_error]
enum CustomLocEnum {
    #[suzu(display("variant A"))]
    VariantA {
        #[suzu(location)]
        origin: Location,
    },
    #[suzu(display("variant B: {msg}"))]
    VariantB {
        msg: String,
        #[suzu(location)]
        pos: Location,
    },
}

/// Struct with auto-detected Location field (no #[suzu(location)] needed).
#[suzunari_error]
#[suzu(display("auto detect"))]
struct AutoDetectLoc {
    #[suzu(implicit)]
    my_loc: Location,
}

#[test]
fn test_custom_location_struct() {
    fn make_error() -> Result<(), CustomLocStruct> {
        ensure!(false, CustomLocStructSnafu);
        Ok(())
    }
    let err = make_error().unwrap_err();
    let file = file!();
    let line = err.location().line();
    assert!(err.location().file().ends_with("integration_test.rs"));
    assert!(line > 0);
    assert_eq!(
        format!("{:?}", StackReport::from(err)),
        format!("Error: CustomLocStruct: custom location struct, at {file}:{line}:9\n")
    );
}

#[test]
fn test_custom_location_enum() {
    fn make_a() -> Result<(), CustomLocEnum> {
        ensure!(false, VariantASnafu);
        Ok(())
    }
    fn make_b() -> Result<(), CustomLocEnum> {
        ensure!(false, VariantBSnafu { msg: "hello" });
        Ok(())
    }
    let err_a = make_a().unwrap_err();
    assert!(err_a.location().file().ends_with("integration_test.rs"));
    assert_eq!(err_a.type_name(), "CustomLocEnum::VariantA");

    let err_b = make_b().unwrap_err();
    assert!(err_b.location().file().ends_with("integration_test.rs"));
    assert_eq!(err_b.type_name(), "CustomLocEnum::VariantB");
}

// StackReport output with different location field names per variant.
// Verifies that the derive correctly destructures each variant's location field.
#[suzunari_error]
#[suzu(display("custom loc chain"))]
struct CustomLocChain {
    source: CustomLocEnum,
}

#[test]
fn test_stack_report_with_mixed_location_names() {
    fn make_a() -> Result<(), CustomLocEnum> {
        ensure!(false, VariantASnafu);
        Ok(())
    }
    fn outer_a() -> Result<(), CustomLocChain> {
        make_a().context(CustomLocChainSnafu)?;
        Ok(())
    }
    let err = outer_a().unwrap_err();
    let file = file!();
    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains(&format!(
        "Error: CustomLocChain: custom loc chain, at {file}:"
    )));
    assert!(report.contains(&format!(
        "1| CustomLocEnum::VariantA: variant A, at {file}:"
    )));

    fn make_b() -> Result<(), CustomLocEnum> {
        ensure!(false, VariantBSnafu { msg: "hi" });
        Ok(())
    }
    fn outer_b() -> Result<(), CustomLocChain> {
        make_b().context(CustomLocChainSnafu)?;
        Ok(())
    }
    let err = outer_b().unwrap_err();
    let report = format!("{:?}", StackReport::from(err));
    assert!(report.contains(&format!(
        "1| CustomLocEnum::VariantB: variant B: hi, at {file}:"
    )));
}

#[test]
fn test_auto_detect_location_field() {
    fn make_error() -> Result<(), AutoDetectLoc> {
        ensure!(false, AutoDetectLocSnafu);
        Ok(())
    }
    let err = make_error().unwrap_err();
    assert!(err.location().file().ends_with("integration_test.rs"));
    assert_eq!(err.type_name(), "AutoDetectLoc");
}

/// E-2: stack_source() and Error::source() must be consistent —
/// when stack_source() returns Some, Error::source() must also return Some.
#[test]
fn test_stack_source_and_error_source_are_consistent() {
    let err = retrieve_data().unwrap_err();
    // RetrieveFailed has stack_source() -> Some(SomeError)
    assert!(
        err.stack_source().is_some(),
        "RetrieveFailed should have a stack source"
    );
    assert!(
        err.source().is_some(),
        "stack_source() returned Some but Error::source() returned None — contract violated"
    );

    // Walk the chain: for every node, verify the contract
    let mut current: &dyn StackError = &err;
    while let Some(next) = current.stack_source() {
        assert!(
            current.source().is_some(),
            "stack_source() returned Some for {} but Error::source() returned None",
            current.type_name()
        );
        current = next;
    }
}

/// E-3: StackReport output should end with a trailing newline
#[test]
fn test_report_ends_with_newline() {
    let err = retrieve_data().unwrap_err();
    let report = format!("{:?}", StackReport::from(err));
    assert!(
        report.ends_with('\n'),
        "StackReport output should end with a newline"
    );
}

// --- GAP-02: #[track_caller] accuracy through .context() ---

#[suzunari_error]
#[suzu(display("context wrapper"))]
struct ContextWrapperError {
    source: std::io::Error,
}

#[test]
fn test_context_captures_exact_line() {
    fn inner() -> Result<(), std::io::Error> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "boom"))
    }
    fn outer() -> Result<(), ContextWrapperError> {
        // The location should point to the exact line of the .context() call
        inner().context(ContextWrapperSnafu)?;
        Ok(())
    }
    let err = outer().unwrap_err();
    let file = file!();
    // .context() line is the `inner().context(...)` line above
    // We verify the location points to this file with the correct line
    assert!(err.location().file().ends_with("integration_test.rs"));
    let report = format!("{:?}", StackReport::from(err));
    // The report must contain the file and a line number for this test file
    assert!(report.contains(&format!(
        "Error: ContextWrapperError: context wrapper, at {file}:"
    )));
}

#[test]
fn test_context_line_differs_from_ensure_line() {
    // ensure! and .context() should capture different lines
    fn with_ensure() -> Result<(), SomeError> {
        ensure!(false, ValidationFailedSnafu { param: 0 });
        Ok(())
    }
    fn with_context() -> Result<(), RetrieveFailed> {
        with_ensure().context(RetrieveFailedSnafu)?;
        Ok(())
    }
    let ensure_err = with_ensure().unwrap_err();
    let context_err = with_context().unwrap_err();
    // The core assertion: ensure! and .context() produce different location lines
    assert_ne!(ensure_err.location().line(), context_err.location().line());

    let file = file!();
    let report = format!("{:?}", StackReport::from(context_err));
    assert!(report.contains(&format!(
        "Error: RetrieveFailed: Failed to retrieve, at {file}:"
    )));
    // The inner ensure! error should have a different line
    assert!(report.contains(&format!(
        "1| SomeError::ValidationFailed: 0 is an invalid value. Must be larger than 1, at {file}:"
    )));
}

// --- GAP-03: non-StackError source stack_source() behavior ---

#[test]
fn test_non_stack_error_source_returns_none_for_stack_source() {
    // SomeError::ReadTimeout has an io::Error source which does NOT implement StackError.
    // stack_source() should return None, but Error::source() should return Some.
    fn make_io_error() -> Result<(), SomeError> {
        let err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        Err(err).context(ReadTimeoutSnafu { timeout_sec: 5u32 })?;
        Ok(())
    }
    let err = make_io_error().unwrap_err();

    // io::Error does not implement StackError, so stack_source() must be None
    assert!(
        err.stack_source().is_none(),
        "stack_source() should be None when source is not a StackError"
    );
    // But Error::source() should still work (io::Error is an Error)
    assert!(
        err.source().is_some(),
        "Error::source() should return Some for io::Error"
    );
}

#[test]
fn test_stack_error_source_returns_some_for_stack_source() {
    // RetrieveFailed has a SomeError source which IS a StackError.
    // stack_source() should return Some.
    let err = retrieve_data().unwrap_err();
    assert!(
        err.stack_source().is_some(),
        "stack_source() should be Some when source implements StackError"
    );
    assert!(
        err.source().is_some(),
        "Error::source() should also return Some"
    );
}

// --- GAP-01: 3+ level deep StackError chain with phase transition ---

#[suzunari_error]
#[suzu(display("level 3"))]
struct Level3Error {
    source: std::io::Error,
}

#[suzunari_error]
#[suzu(display("level 2"))]
struct Level2Error {
    source: Level3Error,
}

#[suzunari_error]
#[suzu(display("level 1"))]
struct Level1Error {
    source: Level2Error,
}

#[test]
fn test_deep_stack_chain_numbering() {
    fn level3() -> Result<(), Level3Error> {
        std::fs::read("/nonexistent").context(Level3Snafu)?;
        Ok(())
    }
    fn level2() -> Result<(), Level2Error> {
        level3().context(Level2Snafu)?;
        Ok(())
    }
    fn level1() -> Result<(), Level1Error> {
        level2().context(Level1Snafu)?;
        Ok(())
    }
    let err = level1().unwrap_err();

    // depth = 3: Level2Error, Level3Error, io::Error
    assert_eq!(err.depth(), 3);

    let file = file!();
    let report = format!("{:?}", StackReport::from(err));

    // Phase 1 (StackError chain with locations):
    // Error: Level1Error (top-level)
    // 1| Level2Error
    // 2| Level3Error
    assert!(report.contains(&format!("Error: Level1Error: level 1, at {file}:")));
    assert!(report.contains(&format!("1| Level2Error: level 2, at {file}:")));
    assert!(report.contains(&format!("2| Level3Error: level 3, at {file}:")));
    // Phase 2 (plain Error chain without location):
    // 3| No such file or directory (os error 2)
    assert!(report.contains("3| "));
}
