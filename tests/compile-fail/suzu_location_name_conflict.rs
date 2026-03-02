// Auto-inject fails when the "location" field exists but is not Location type.
use suzunari_error::*;

#[suzunari_error]
#[suzu(display("error"))]
struct MyError {
    location: String,
}

fn main() {}
