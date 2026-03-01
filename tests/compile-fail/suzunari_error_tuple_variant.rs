// #[suzunari_error] requires named fields on enum variants
use suzunari_error::suzunari_error;

#[suzunari_error]
pub enum MyError {
    #[suzu(display("tuple"))]
    Tuple(String),
}

fn main() {}
