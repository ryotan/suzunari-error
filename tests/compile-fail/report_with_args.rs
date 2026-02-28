// #[report] does not accept arguments
use suzunari_error::*;

#[suzunari_error]
#[snafu(display("error"))]
struct MyError {}

#[suzunari_error::report(something)]
fn main() -> Result<(), MyError> {
    Ok(())
}
