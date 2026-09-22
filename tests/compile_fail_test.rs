#![cfg(feature = "std")]

#[test]
fn test_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/report_*.rs");
    t.compile_fail("tests/compile-fail/derive_*.rs");
    t.compile_fail("tests/compile-fail/suzu_*.rs");
    t.compile_fail("tests/compile-fail/suzunari_*.rs");
    t.compile_fail("tests/compile-fail/stack_*.rs");
}

/// The serde cases need the feature: without it they fail on the feature check
/// instead, with a different message.
#[cfg(feature = "serde")]
#[test]
fn test_compile_fail_serde() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/serialize_*.rs");
}
