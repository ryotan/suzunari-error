// #[suzu()] with empty parentheses is rejected
use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu()]
struct MyError {
    source: std::io::Error,
}

fn main() {}
