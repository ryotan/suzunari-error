use proc_macro2::Ident;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::format_ident;
use syn::FieldsNamed;

/// Helper function to get the crate name
pub(crate) fn get_crate_name(original_name: &str) -> Result<Ident, proc_macro_crate::Error> {
    match crate_name(original_name) {
        Ok(FoundCrate::Itself) => Ok(format_ident!("crate")),
        Ok(FoundCrate::Name(name)) => Ok(format_ident!("{name}")),
        Err(e) => Err(e),
    }
}

pub(crate) fn has_location(fields: &FieldsNamed) -> bool {
    fields.named.iter().any(|field| {
        field.ident.as_ref().is_some_and(|ident| ident == "location")
    })
}