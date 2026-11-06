use std::rc::Rc;
use suzunari_error::*;

#[suzunari_error]
#[suzu(display("local failure"))]
struct LocalError<T: std::fmt::Debug> {
    state: T,
}

fn main() {
    let error: LocalError<Rc<()>> = None::<()>
        .context(LocalSnafu { state: Rc::new(()) })
        .unwrap_err();
    let _: BoxedStackError = error.into();
}
