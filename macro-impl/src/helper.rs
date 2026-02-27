use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, TokenStream};
use quote::format_ident;
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{Error, Field, FieldsNamed, Meta, PathArguments, Type};

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

/// Returns true if a field named "location" exists (regardless of type).
///
/// Used by `suzunari_location` to decide whether to inject the field.
pub(crate) fn has_location_field(fields: &FieldsNamed) -> bool {
    fields.named.iter().any(|field| {
        field
            .ident
            .as_ref()
            .is_some_and(|ident| ident == "location")
    })
}

/// Validates that a `location: Location` field exists with the correct type.
///
/// Used by `derive(StackError)` for clear error messages when the field
/// is missing or has an unexpected type.
pub(crate) fn validate_location(fields: &FieldsNamed) -> Result<(), Error> {
    let location_field = fields
        .named
        .iter()
        .find(|f| f.ident.as_ref().is_some_and(|i| i == "location"));
    match location_field {
        None => Err(Error::new(
            fields.span(),
            "StackError requires a 'location' field of type Location",
        )),
        Some(field) => {
            if looks_like_location_type(&field.ty) {
                Ok(())
            } else {
                Err(Error::new(
                    field.ty.span(),
                    "field 'location' must be of type Location (from suzunari_error). \
                     If you intended a custom field, rename it to avoid conflict.",
                ))
            }
        }
    }
}

/// Extracts the inner type `T` from `DisplayError<T>`.
///
/// Returns `Some(&T)` if the type's last path segment is `DisplayError` with
/// a single angle-bracket argument. Returns `None` otherwise.
pub(crate) fn extract_display_error_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "DisplayError" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    if args.args.len() != 1 {
        return None;
    }
    match &args.args[0] {
        syn::GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

fn looks_like_location_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(p) => p
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "Location"),
        _ => false,
    }
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
            // Intentionally ignore parse errors: if the snafu attribute
            // has syntax we don't understand, fall back to name-based
            // detection. snafu itself will report the syntax error.
            let nested = meta_list
                .parse_args_with(
                    syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                )
                .ok()?;
            // Use filter_map + last to be consistent with the outer .last():
            // both within a single #[snafu(...)] and across multiple #[snafu]
            // attributes, the last `source` directive wins.
            nested
                .iter()
                .filter_map(|meta| match meta {
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
                .last()
        })
        .last();

    snafu_source.unwrap_or(is_named_source)
}
