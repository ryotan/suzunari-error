//! Procedural macros for suzunari-error
//!
//! This crate provides procedural macros for the suzunari-error crate.

mod attribute;
mod derive;
mod helper;

use crate::attribute::{suzunari_error_impl, suzunari_location_impl};
use crate::derive::stack_error_impl;
use crate::helper::{get_crate_name, has_location};
use proc_macro::TokenStream;

#[proc_macro_derive(StackError)]
pub fn derive_stack_error(input: TokenStream) -> TokenStream {
    stack_error_impl(input.into()).into()
}

#[proc_macro_attribute]
pub fn suzunari_location(_attr: TokenStream, item: TokenStream) -> TokenStream {
    suzunari_location_impl(item.into()).into()
}
