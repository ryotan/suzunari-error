// #[suzu(from(...))] list form is rejected — from does not accept arguments
use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("error"))]
struct MyError {
    #[suzu(from(String, suzunari_error::DisplayError::new))]
    source: String,
}

fn main() {}
