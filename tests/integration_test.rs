#![cfg(feature = "std")]

use core::error::Error;
use snafu::{ResultExt, ensure};
use suzunari_error::*;

#[suzunari_error]
struct ErrorStruct {}

#[suzunari_error]
enum ErrorEnum {
    Variant1Unit,
    #[snafu(display("Variant2 {message}"))]
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
        format!("{:?}", StackReport::from_error(err)),
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
    let report = format!("{:?}", StackReport::from_error(err));
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
    #[snafu(display("after {}sec", timeout_sec))]
    ReadTimeout {
        timeout_sec: u32,
        #[snafu(source)]
        error: std::io::Error,
    },
    #[snafu(display("{} is an invalid value. Must be larger than 1", param))]
    ValidationFailed { param: i32 },
}

#[suzunari_error]
#[snafu(display("Failed to retrieve"))]
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
    let report = format!("{:?}", StackReport::from_error(err));
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
        format!("{:?}", StackReport::from_error(err)),
        format!(
            "Error: SomeError::ValidationFailed: 0 is an invalid value. Must be larger than 1, at {file}:{ensure_line}:9\n"
        )
    );
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
    let report = format!("{:?}", StackReport::from_error(err));
    assert!(
        report.ends_with('\n'),
        "StackReport output should end with a newline"
    );
}
