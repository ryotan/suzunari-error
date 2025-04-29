use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, TokenStream};
use quote::format_ident;
use syn::spanned::Spanned;
use syn::{Error, FieldsNamed};

/// Helper function to get the crate name
pub(crate) fn get_crate_name(original_name: &str, stream: &TokenStream) -> Result<Ident, Error> {
    match crate_name(original_name) {
        Ok(FoundCrate::Itself) => Ok(format_ident!("crate")),
        Ok(FoundCrate::Name(name)) => Ok(format_ident!("{name}")),
        Err(_) => Err(Error::new(
            stream.span(),
            format!(
                "Failed to find the crate '{original_name}'. Ensure it is added as a dependency."
            ),
        )),
    }
}

pub(crate) fn has_location(fields: &FieldsNamed) -> bool {
    fields.named.iter().any(|field| {
        field
            .ident
            .as_ref()
            .is_some_and(|ident| ident == "location")
    })
}
