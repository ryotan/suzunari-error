// #[derive(StackError)] requires a 'location' field
use suzunari_error::*;

#[derive(Debug, snafu::Snafu, StackError)]
#[snafu(display("error"))]
struct MyError {
    message: String,
}

fn main() {}
