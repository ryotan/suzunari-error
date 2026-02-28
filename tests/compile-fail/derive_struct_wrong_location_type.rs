// Uses raw #[derive(StackError)] to test the derive macro's own validation diagnostic;
// #[suzunari_error] would inject the correctly-typed location field and suppress this error.
use suzunari_error::*;

#[derive(Debug, snafu::Snafu, StackError)]
#[snafu(display("error"))]
struct MyError {
    location: String,
}

fn main() {}
