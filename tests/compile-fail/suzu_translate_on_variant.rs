use suzunari_error::suzunari_error;

#[suzunari_error]
enum MyError {
    #[suzu(translate)]
    Variant {},
}

fn main() {}
