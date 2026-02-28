// Uses raw #[derive(StackError)] to test the derive macro's own validation diagnostic;
// #[suzunari_error] would inject the correctly-typed location field and suppress this error.
use suzunari_error::*;

#[derive(Debug, snafu::Snafu, StackError)]
enum MyError {
    Variant1 { location: String },
}

fn main() {}
