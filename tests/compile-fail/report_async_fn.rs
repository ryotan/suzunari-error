// #[report] does not support async fn
use suzunari_error::*;

#[suzunari_error]
#[snafu(display("error"))]
struct MyError {}

#[suzunari_error::report]
async fn main() -> Result<(), MyError> {
    Ok(())
}
