//! Using BoxedStackError for heterogeneous error handling across module boundaries.
//!
//! Run: cargo run --example boxed_error

use suzunari_error::*;

// Module A defines its own error type.
mod auth {
    use suzunari_error::*;

    #[suzunari_error]
    #[suzu(visibility(pub), display("authentication failed: {reason}"))]
    pub struct AuthError {
        pub reason: String,
    }

    pub fn authenticate(token: &str) -> Result<(), AuthError> {
        ensure!(
            !token.is_empty(),
            AuthSnafu {
                reason: "empty token"
            }
        );
        Ok(())
    }
}

// Module B defines a different error type.
mod db {
    use suzunari_error::*;

    #[suzunari_error]
    #[suzu(visibility(pub), display("database query failed"))]
    pub struct DbError {
        pub source: std::io::Error,
    }

    pub fn query() -> Result<String, DbError> {
        std::fs::read_to_string("/nonexistent/db").context(DbSnafu)?;
        unreachable!()
    }
}

// The top-level function uses BoxedStackError to unify different error types.
#[suzunari_error]
#[suzu(display("request failed"))]
struct RequestError {
    source: BoxedStackError,
}

fn handle_request() -> Result<(), RequestError> {
    auth::authenticate("")
        .map_err(BoxedStackError::new)
        .context(RequestSnafu)?;

    db::query()
        .map_err(BoxedStackError::new)
        .context(RequestSnafu)?;

    Ok(())
}

#[suzunari_error::report]
fn main() -> Result<(), RequestError> {
    handle_request()?;
    Ok(())
}
