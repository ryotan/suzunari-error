// Verifies that errors from multiple variants are accumulated and reported at once.
// Both variants have issues (no location field), so both errors should appear.
use suzunari_error::StackError;

#[derive(Debug, snafu::Snafu, StackError)]
pub enum MyError {
    #[snafu(display("variant a"))]
    VariantA { msg: String },
    #[snafu(display("variant b"))]
    VariantB { ctx: String },
}

fn main() {}
