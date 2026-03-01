// #[stack] without arguments is rejected.
use suzunari_error::StackError;

#[derive(Debug, snafu::Snafu, StackError)]
#[snafu(display("test"))]
pub struct MyError {
    #[stack]
    location: suzunari_error::Location,
}

fn main() {}
