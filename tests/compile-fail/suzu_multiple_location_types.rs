// Multiple Location-typed fields without #[suzu(location)] is ambiguous.
use suzunari_error::*;

#[suzunari_error]
#[snafu(display("error"))]
struct MyError {
    #[snafu(implicit)]
    origin: Location,
    #[snafu(implicit)]
    other_loc: Location,
}

fn main() {}
