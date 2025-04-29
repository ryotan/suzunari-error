//! Procedural macros for suzunari-error
//!
//! This crate provides procedural macros for the suzunari-error crate.

mod derive_stack_error_impl;
mod helper;
mod suzunari_location_impl;

use crate::derive_stack_error_impl::derive_stack_error_impl;
use crate::helper::{get_crate_name, has_location};
use crate::suzunari_location_impl::suzunari_location_impl;
use proc_macro::TokenStream;

#[proc_macro_derive(StackError)]
pub fn derive_stack_error(input: TokenStream) -> TokenStream {
    derive_stack_error_impl(input.into()).into()
}
#[proc_macro_attribute]
pub fn suzunari_location(_attr: TokenStream, item: TokenStream) -> TokenStream {
    suzunari_location_impl(item.into()).into()
}
