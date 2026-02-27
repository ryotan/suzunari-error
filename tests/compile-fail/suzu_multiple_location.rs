use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("test"))]
pub struct MyError {
    #[suzu(location)]
    loc1: suzunari_error::Location,
    #[suzu(location)]
    loc2: suzunari_error::Location,
}

fn main() {}
