// #[report] does not support where clauses
use suzunari_error::Location;

#[derive(Debug, snafu::Snafu, suzunari_error::StackError)]
#[snafu(display("error"))]
struct MyError {
    #[snafu(implicit)]
    location: Location,
}

#[suzunari_error::report]
fn main() -> Result<(), MyError>
where
    (): Sized,
{
    Ok(())
}
