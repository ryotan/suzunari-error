// #[report] requires exactly Result<(), E> with 2 type arguments.
// Result<()> has only 1 type arg — should be rejected.
use suzunari_error::suzunari_error;

#[suzunari_error]
#[suzu(display("test error"))]
struct MyError {}

#[suzunari_error::report]
fn main() -> Result<()> {
    Ok(())
}
