use suzunari_error::suzunari_error;

#[suzunari_error]
#[snafu(display("error"))]
struct MyError {
    #[suzu(from, source(from(String, suzunari_error::DisplayError::new)))]
    source: String,
}

fn main() {}
