// #[suzunari_error] does not accept arguments.
use suzunari_error::suzunari_error;

#[suzunari_error(skip_location)]
#[suzu(display("test"))]
pub struct MyError {}

fn main() {}
