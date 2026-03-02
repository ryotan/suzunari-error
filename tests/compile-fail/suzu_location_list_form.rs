// #[suzu(location(...))] list form is rejected — location does not accept arguments
use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("error"))]
struct MyError {
    #[suzu(location(true))]
    loc: suzunari_error::Location,
}

fn main() {}
