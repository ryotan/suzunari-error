// derive(StackError) cannot be used on unions
use suzunari_error::StackError;

#[derive(StackError)]
pub union MyUnion {
    a: u32,
    b: f32,
}

fn main() {}
