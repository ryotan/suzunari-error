use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("error"))]
struct MyError {
    #[suzu(from)]
    #[suzu(from)]
    source: String,
}

fn main() {}
