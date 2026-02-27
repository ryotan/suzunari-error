use suzunari_error::suzunari_error;

#[suzunari_error]
enum MyError {
    #[suzu(from)]
    Variant {},
}

fn main() {}
