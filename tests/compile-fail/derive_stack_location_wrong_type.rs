// #[stack(location)] on a non-Location field should fail.
use suzunari_error::*;

#[derive(Debug, snafu::Snafu, StackError)]
#[snafu(display("error"))]
struct MyError {
    #[snafu(implicit)]
    #[stack(location)]
    name: String,
}

fn main() {}
