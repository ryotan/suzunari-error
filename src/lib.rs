//! Error handling utilities for Suzunari
//!
//! This crate provides error handling utilities for Rust applications.

mod boxed_stack_error;
mod location;
mod stack_error;

pub use boxed_stack_error::*;
pub use location::*;
pub use stack_error::*;
pub use suzunari_error_macro_impl::*;

use snafu::Snafu;
/// Example error type
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("IO error: {source}"))]
    Io { source: std::io::Error },

    #[snafu(display("Custom error: {message}"))]
    Custom { message: String },
}

/// Result type alias for this crate
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Example function
pub fn example() -> Result<()> {
    // Example implementation
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = example();
        assert!(result.is_ok());
    }
}
