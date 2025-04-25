use snafu::{ensure, Snafu};
use suzunari_error::Location;

#[test]
fn test_snafu_implicit_generation() {
    #[derive(Debug, Snafu)]
    struct SomeError {
        #[snafu(implicit)]
        location: Location,
    }

    fn some_function() -> Result<(), SomeError> {
        // capture the current location
        ensure!(false, SomeSnafu);
        Ok(())
    }

    let error = some_function().unwrap_err();

    let file = file!();
    let line = line!() - 7; // 7行前でSomeSnafuを利用している。
    assert_eq!(format!("{error}"), "SomeError");
    assert_eq!(format!("{error:?}"), format!("SomeError {{ location: {file}:{line}:9 }}"));
    assert_eq!(format!("{error:#?}"), format!("SomeError {{\n    location: {file}:{line}:9,\n}}"));
}
