// #[report] requires Result<(), E>, not Result<i32, E>
use suzunari_error::*;

#[suzunari_error]
#[snafu(display("error"))]
struct MyError {}

#[suzunari_error::report]
fn main() -> Result<i32, MyError> {
    Ok(42)
}
