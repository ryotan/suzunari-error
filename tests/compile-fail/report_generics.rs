// #[report] does not support generic parameters
use suzunari_error::*;

#[suzunari_error]
#[snafu(display("error"))]
struct MyError {}

#[suzunari_error::report]
fn main<T>() -> Result<(), MyError> {
    Ok(())
}
