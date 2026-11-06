//! Local errors remain usable independently of the selected allocation tier.
#![cfg(any(
    feature = "test-core-only",
    feature = "test-alloc",
    feature = "test-std"
))]

use core::cell::Cell;
use suzunari_error::*;

#[suzunari_error]
#[suzu(display("tier-local failure"))]
struct TierLocalError<'a> {
    state: Cell<u8>,
    context: &'a str,
}

#[suzunari_error]
#[suzu(display("sendable tier failure"))]
struct TierSendableError {}

#[test]
fn local_errors_do_not_require_an_allocator_or_sync() {
    let context = String::from("borrowed");
    let error = None::<()>
        .context(TierLocalSnafu {
            state: Cell::new(1),
            context: context.as_str(),
        })
        .unwrap_err();
    error.state.set(2);
    assert_eq!(error.state.get(), 2);
    assert_eq!(error.context, "borrowed");
    assert_eq!(error.type_name(), "TierLocalError");
}

#[test]
fn automatic_conversion_respects_the_selected_allocation_tier() {
    let error = None::<()>.context(TierSendableSnafu).unwrap_err();
    #[cfg(feature = "test-alloc")]
    {
        let boxed: BoxedStackError = error.into();
        assert_eq!(boxed.type_name(), "TierSendableError");
    }
    #[cfg(not(feature = "test-alloc"))]
    assert_eq!(error.type_name(), "TierSendableError");
}
