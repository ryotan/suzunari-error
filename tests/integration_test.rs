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

fn error_struct() -> Result<(), ErrorStruct> {
    ensure!(false, ErrorStructSnafu);
    Ok(())
}

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

#[test]
fn test_stack_trace_single() {
    let err = error_struct().unwrap_err();
    let file = file!();
    assert_eq!(
        format!("{err:?}"),
        format!("0: ErrorStruct, at {file}:22:5\n")
    );
}

#[test]
fn test_nested_stack_trace() {
    let err = nested_error().unwrap_err();
    let file = file!();
    assert_eq!(
        format!("{err:?}"),
        format!("1: ErrorAggregate, at {file}:37:18\n0: Variant2 message, at {file}:27:5\n")
    );
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

fn validate() -> Result<(), BoxedStackError> {
    let param = 0;
    ensure!(false, ValidationFailedSnafu { param });
    Ok(())
}
