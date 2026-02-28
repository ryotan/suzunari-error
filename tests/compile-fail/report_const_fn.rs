// #[report] does not support const functions.
use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("boom"))]
pub struct MyError {}

#[suzunari_error::report]
const fn main() -> Result<(), MyError> {
    Ok(())
}
