#![cfg(feature = "std")]

use snafu::ensure;
use suzunari_error::*;

#[suzunari_error]
#[snafu(display("test error: {message}"))]
struct TestReportError {
    message: String,
}

// #[suzunari_error::report] transforms the return type to StackReport<E>
// and wraps the body in a closure
#[suzunari_error::report]
fn success_case() -> Result<(), TestReportError> {
    Ok(())
}

#[suzunari_error::report]
fn failure_case() -> Result<(), TestReportError> {
    ensure!(false, TestReportSnafu { message: "boom" });
    Ok(())
}

#[test]
fn test_report_success() {
    let report: StackReport<TestReportError> = success_case();
    // Display of success case is empty
    assert_eq!(format!("{report}"), "");
}

#[test]
fn test_report_failure() {
    let report: StackReport<TestReportError> = failure_case();
    let output = format!("{report}");
    assert!(output.contains("Error: TestReportError: test error: boom"));
    assert!(output.contains("at"));
}

#[test]
fn test_report_debug_equals_display() {
    let report: StackReport<TestReportError> = failure_case();
    assert_eq!(format!("{report}"), format!("{report:?}"));
}

#[test]
fn test_report_from_result_ok() {
    let result: Result<(), TestReportError> = Ok(());
    let report = StackReport::from(result);
    assert_eq!(format!("{report}"), "");
}

#[test]
fn test_report_from_result_err() {
    let result: Result<(), TestReportError> = Err(TestReportError {
        message: "direct".to_string(),
        location: Location::current(),
    });
    let report = StackReport::from(result);
    let output = format!("{report}");
    assert!(output.contains("test error: direct"));
}

// #[report] with ? operator — verifies error propagation works through the closure wrapper
#[suzunari_error]
#[suzu(display("io wrapper"))]
struct IoWrapperError {
    source: std::io::Error,
}

#[suzunari_error::report]
fn report_with_question_mark() -> Result<(), IoWrapperError> {
    use snafu::ResultExt;
    // This will fail because the file doesn't exist, testing ? propagation
    std::fs::read("this_file_does_not_exist_for_test").context(IoWrapperSnafu)?;
    Ok(())
}

#[test]
fn test_report_with_question_mark_propagation() {
    let report: StackReport<IoWrapperError> = report_with_question_mark();
    let output = format!("{report}");
    assert!(output.contains("Error: IoWrapperError: io wrapper"));
    assert!(output.contains("Caused by"));
}

#[test]
fn test_report_termination_success() {
    use std::process::Termination;
    let report: StackReport<TestReportError> = success_case();
    // Termination::report() should not panic on success
    let _ = report.report();
}

#[test]
fn test_report_termination_failure() {
    use std::process::Termination;
    let report: StackReport<TestReportError> = failure_case();
    // Termination::report() should not panic on failure
    // (it writes to stderr and returns FAILURE)
    let _ = report.report();
}
