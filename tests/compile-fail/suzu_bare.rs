use suzunari_error::suzunari_error;

#[suzunari_error]
#[snafu(display("error"))]
struct MyError {
    #[suzu]
    source: String,
}

fn main() {}
