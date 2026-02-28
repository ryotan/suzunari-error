// derive(StackError) requires named fields, not tuple structs
use suzunari_error::{Location, StackError};

#[derive(Debug, snafu::Snafu, StackError)]
pub struct TupleError(String, Location);

fn main() {}
