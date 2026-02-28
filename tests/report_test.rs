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
