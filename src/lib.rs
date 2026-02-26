//! Error handling utilities
//!
//! This crate provides error handling utilities for Rust applications.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

mod display_error;
mod location;
mod stack_error;

#[cfg(feature = "alloc")]
mod boxed_stack_error;

#[cfg(feature = "alloc")]
pub use boxed_stack_error::*;
pub use display_error::*;
pub use location::*;
pub use stack_error::*;
pub use suzunari_error_macro_impl::*;
