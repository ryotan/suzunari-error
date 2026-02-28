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

/// Finds the location field in a struct/variant's named fields.
///
/// Resolution order (symmetric with `find_source_field`):
/// 1. `#[stack(location)]` marker — highest priority, any field name
/// 2. Single field of type `Location` — automatic fallback
/// 3. Error if no location field found
///
/// Used by `derive(StackError)` to resolve the location field dynamically.
pub(crate) fn find_location_field(fields: &FieldsNamed) -> Result<&Field, Error> {
    // Priority 1: explicit #[stack(location)] marker
    let mut marked = Vec::new();
    for field in &fields.named {
        if has_stack_location_attr(field)? {
            marked.push(field);
        }
    }

    match marked.len() {
        1 => return Ok(marked[0]),
        n if n > 1 => {
            return Err(Error::new(
                marked[1].span(),
                "multiple #[stack(location)] fields; only one is allowed per struct/variant",
            ));
        }
        _ => {}
    }

    // Priority 2: single field with Location type
    let location_typed: Vec<_> = fields
        .named
        .iter()
        .filter(|f| looks_like_location_type(&f.ty))
        .collect();

    match location_typed.len() {
        1 => return Ok(location_typed[0]),
        n if n > 1 => {
            return Err(Error::new(
                location_typed[1].span(),
                "multiple Location fields found; use #[stack(location)] to specify which one",
            ));
        }
        _ => {}
    }

    // Near-miss: field named "location" but wrong type
    if let Some(field) = fields.named.iter().find(|f| {
        f.ident.as_ref().is_some_and(|i| i == "location") && !looks_like_location_type(&f.ty)
    }) {
        return Err(Error::new(
            field.span(),
            "field 'location' exists but is not of type Location; \
             use Location type or mark the correct field with #[stack(location)]",
        ));
    }

    // No location field found
    Err(Error::new(
        fields.span(),
        "StackError requires a Location field. Use #[suzunari_error] to auto-inject, \
         or add a field of type Location manually.",
    ))
}

/// Returns true if the field has `#[stack(location)]` attribute.
///
/// Unlike `is_source_field` (which defers parse errors to snafu), this function
/// propagates parse errors because `#[stack(...)]` is consumed by our own
/// `derive(StackError)` — no other macro will report the error.
pub(crate) fn has_stack_location_attr(field: &Field) -> Result<bool, Error> {
    let mut found = false;
    for attr in field.attrs.iter().filter(|a| a.path().is_ident("stack")) {
        let Meta::List(meta_list) = &attr.meta else {
            // Reject bare #[stack] or #[stack = ...] — this crate owns
            // the stack attribute namespace, so malformed usage is always an error.
            return Err(Error::new(
                attr.span(),
                "#[stack] requires arguments, e.g., #[stack(location)]",
            ));
        };
        let nested = meta_list.parse_args_with(
            syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
        )?;
        if nested.is_empty() {
            return Err(Error::new(
                attr.span(),
                "#[stack()] requires arguments, e.g., #[stack(location)]",
            ));
        }
        // Reject unknown tokens — only `location` is supported.
        if let Some(unknown) = nested
            .iter()
            .find(|meta| !matches!(meta, Meta::Path(p) if p.is_ident("location")))
        {
            return Err(Error::new(
                unknown.span(),
                "unknown #[stack(...)] argument; only `location` is supported",
            ));
        }
        found = true;
    }
    Ok(found)
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

pub(crate) fn looks_like_location_type(ty: &syn::Type) -> bool {
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
            // Try structured parsing first (handles simple cases).
            if let Ok(nested) = meta_list.parse_args_with(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
            ) {
                // Use filter_map + last to be consistent with the outer .last():
                // both within a single #[snafu(...)] and across multiple #[snafu]
                // attributes, the last `source` directive wins.
                return nested
                    .iter()
                    .filter_map(|meta| match meta {
                        Meta::Path(path) if path.is_ident("source") => Some(true),
                        Meta::List(list) if list.path.is_ident("source") => {
                            // `source(false)` explicitly disables source detection.
                            let is_disabled = list
                                .parse_args_with(Ident::parse_any)
                                .is_ok_and(|ident| ident == "false");
                            Some(!is_disabled)
                        }
                        _ => None,
                    })
                    .last();
            }
            // Fallback: token-level scan for closure syntax like
            // source(from(T, |e| transform(e))) which fails Meta parsing.
            // snafu itself will validate the syntax; we only need to detect
            // that `source` is present for StackError's stack_source() generation.
            if snafu_tokens_contain_keyword(&meta_list.tokens, "source") {
                return Some(true);
            }
            None
        })
        .last();

    snafu_source.unwrap_or(is_named_source)
}

/// Scans a token stream for `keyword` as a leading ident in any
/// comma-separated segment. Does not descend into groups (parentheses,
/// brackets, braces), so `display("source")` won't match `source`.
///
/// Used as fallback when `Punctuated<Meta>` parsing fails (e.g., snafu's
/// closure syntax in `source(from(T, |e| transform(e)))`).
pub(crate) fn snafu_tokens_contain_keyword(
    tokens: &proc_macro2::TokenStream,
    keyword: &str,
) -> bool {
    let mut at_start = true;
    for tt in tokens.clone() {
        match &tt {
            proc_macro2::TokenTree::Ident(ident) if at_start && *ident == keyword => return true,
            proc_macro2::TokenTree::Punct(p) if p.as_char() == ',' => at_start = true,
            _ => at_start = false,
        }
    }
    false
}
