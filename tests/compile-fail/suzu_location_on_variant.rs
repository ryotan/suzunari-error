use suzunari_error::suzunari_error;

#[suzunari_error]
enum MyError {
    #[suzu(location)]
    Variant {},
}

fn main() {}
