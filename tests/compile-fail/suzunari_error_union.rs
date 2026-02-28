// #[suzunari_error] cannot be used on unions
use suzunari_error::suzunari_error;

#[suzunari_error]
pub union MyError {
    a: u32,
    b: f32,
}

fn main() {}
