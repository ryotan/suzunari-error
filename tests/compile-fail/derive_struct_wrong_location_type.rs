// #[derive(StackError)] requires location field to be of type Location
use suzunari_error::*;

#[derive(Debug, snafu::Snafu, StackError)]
#[snafu(display("error"))]
struct MyError {
    location: String,
}

fn main() {}
