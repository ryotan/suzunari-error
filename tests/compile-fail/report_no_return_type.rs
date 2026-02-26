// #[report] requires a Result<(), E> return type
use suzunari_error::*;

#[suzunari_error::report]
fn main() {
    println!("hello");
}
