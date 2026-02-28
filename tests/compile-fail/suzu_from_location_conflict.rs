// `from` and `location` cannot be used on the same field.
use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("test"))]
pub struct MyError {
    #[suzu(from, location)]
    source: suzunari_error::Location,
}

fn main() {}
