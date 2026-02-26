//! Error handling utilities
//!
//! This crate provides error handling utilities for Rust applications.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

mod display_error;
mod location;
mod stack_error;
mod stack_report;

#[doc(hidden)]
pub mod __private;

#[cfg(feature = "alloc")]
mod boxed_stack_error;

#[cfg(feature = "alloc")]
pub use boxed_stack_error::*;
pub use display_error::*;
pub use location::*;
pub use stack_error::*;
pub use stack_report::*;
pub use suzunari_error_macro_impl::*;
