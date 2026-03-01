use suzunari_error::suzunari_error;

#[suzunari_error]
#[snafu(display("error"))]
struct MyError {
    #[snafu(source(from(String, suzunari_error::DisplayError::new)))]
    #[suzu(from)]
    source: String,
}

fn main() {}
