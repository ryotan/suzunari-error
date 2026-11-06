use suzunari_error::*;

#[suzunari_error]
#[suzu(display("borrowed failure"))]
struct BorrowedError<'a> {
    value: &'a str,
}

fn main() {
    let value = String::from("borrowed");
    let error = None::<()>
        .context(BorrowedSnafu { value: value.as_str() })
        .unwrap_err();
    let _: BoxedStackError = error.into();
}
