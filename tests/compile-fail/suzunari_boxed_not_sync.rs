use std::cell::Cell;
use suzunari_error::*;

#[suzunari_error]
#[suzu(display("non-sync failure"))]
struct NonSyncError {
    state: Cell<u8>,
}

fn main() {
    let error = None::<()>
        .context(NonSyncSnafu { state: Cell::new(1) })
        .unwrap_err();
    let _: BoxedStackError = error.into();
}
