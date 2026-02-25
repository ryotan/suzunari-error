#![cfg(not(feature = "test-alloc"))]

use suzunari_error::*;

#[test]
fn test_location_core_only() {
    let loc = Location::current();
    assert!(loc.line() > 0);
    assert!(loc.column() > 0);
    assert!(!loc.file().is_empty());
}

#[test]
fn test_display_error_new_and_into_inner() {
    let wrapped = DisplayError::new("test");
    let inner = wrapped.into_inner();
    assert_eq!(inner, "test");
}
