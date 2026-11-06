// Unknown #[suzunari_error] options are rejected.
use suzunari_error::suzunari_error;

#[suzunari_error(skip_location)]
#[suzu(display("test"))]
pub struct MyError {}

fn main() {}
