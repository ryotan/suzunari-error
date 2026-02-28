// #[report] does not support unsafe functions.
use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("boom"))]
pub struct MyError {}

#[suzunari_error::report]
unsafe fn main() -> Result<(), MyError> {
    Ok(())
}
