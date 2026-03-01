// Auto-inject fails when "location" field exists but is not Location type.
use suzunari_error::*;

#[suzunari_error]
#[snafu(display("error"))]
struct MyError {
    location: String,
}

fn main() {}
