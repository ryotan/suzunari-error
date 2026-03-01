// derive(StackError) requires named fields in all enum variants
use suzunari_error::{Location, StackError};

#[derive(Debug, snafu::Snafu, StackError)]
pub enum MyError {
    Named {
        #[snafu(implicit)]
        location: Location,
    },
    Tuple(String),
}

fn main() {}
