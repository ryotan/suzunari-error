// #[suzu(from)] on a tuple variant field is rejected
use suzunari_error::suzunari_error;

#[suzunari_error]
enum MyError {
    #[suzu(display("error"))]
    Tuple(#[suzu(from)] String),
}

fn main() {}
