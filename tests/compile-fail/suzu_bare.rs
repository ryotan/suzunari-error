use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("error"))]
struct MyError {
    #[suzu]
    source: String,
}

fn main() {}
