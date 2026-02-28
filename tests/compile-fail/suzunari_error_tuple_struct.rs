// #[suzunari_error] requires named fields, not tuple structs
use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("tuple"))]
pub struct TupleError(String);

fn main() {}
