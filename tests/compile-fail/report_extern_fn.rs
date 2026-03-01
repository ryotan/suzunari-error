// #[report] does not support extern functions
use suzunari_error::Location;

#[derive(Debug, snafu::Snafu, suzunari_error::StackError)]
#[snafu(display("error"))]
struct MyError {
    #[snafu(implicit)]
    location: Location,
}

#[suzunari_error::report]
extern "C" fn main() -> Result<(), MyError> {
    Ok(())
}
