// derive(StackError) rejects multiple #[stack(location)] fields
use suzunari_error::{Location, StackError};

#[derive(Debug, snafu::Snafu, StackError)]
#[snafu(display("error"))]
pub struct MyError {
    #[snafu(implicit)]
    #[stack(location)]
    loc1: Location,
    #[snafu(implicit)]
    #[stack(location)]
    loc2: Location,
}

fn main() {}
