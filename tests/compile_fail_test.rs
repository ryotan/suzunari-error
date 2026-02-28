#![cfg(feature = "std")]

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/report_*.rs");
    t.compile_fail("tests/compile-fail/derive_*.rs");
}
