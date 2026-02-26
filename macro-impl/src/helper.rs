use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, TokenStream};
use quote::format_ident;
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{Error, Field, FieldsNamed, Meta};

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

/// Finds the source field in a struct/variant's named fields.
///
/// A field is considered a source if:
/// - Named "source" (unless `#[snafu(source(false))]`)
/// - Annotated with `#[snafu(source)]` or `#[snafu(source(from(...)))]`
pub(crate) fn find_source_field(fields: &FieldsNamed) -> Option<&Field> {
    fields.named.iter().find(|field| is_source_field(field))
}

fn is_source_field(field: &Field) -> bool {
    let is_named_source = field.ident.as_ref().is_some_and(|ident| ident == "source");

    let snafu_source = field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("snafu"))
        .filter_map(|attr| {
            let Meta::List(meta_list) = &attr.meta else {
                return None;
            };
            let nested = meta_list
                .parse_args_with(
                    syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                )
                .ok()?;
            nested.iter().find_map(|meta| match meta {
                Meta::Path(path) if path.is_ident("source") => Some(true),
                Meta::List(list) if list.path.is_ident("source") => {
                    // `source(false)` explicitly disables source detection.
                    // Use token-based parsing instead of string comparison.
                    let is_disabled = list
                        .parse_args_with(Ident::parse_any)
                        .is_ok_and(|ident| ident == "false");
                    Some(!is_disabled)
                }
                _ => None,
            })
        })
        .last();

    snafu_source.unwrap_or(is_named_source)
}
