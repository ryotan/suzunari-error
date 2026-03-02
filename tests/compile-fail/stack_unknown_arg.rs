// #[stack(unknown)] is rejected — only `location` is supported.
use suzunari_error::StackError;

#[derive(Debug, snafu::Snafu, StackError)]
#[snafu(display("test"))]
pub struct MyError {
    #[stack(loc)]
    location: suzunari_error::Location,
}

fn main() {}
