// #[suzu(location)] requires the field type to be Location.
use suzunari_error::*;

#[suzunari_error]
#[suzu(display("error"))]
struct MyError {
    #[suzu(location)]
    name: String,
}

fn main() {}
