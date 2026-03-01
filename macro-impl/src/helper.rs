use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, Field, FieldsNamed, GenericArgument, Meta, PathArguments, Type};

/// Returns a token stream for the absolute crate path (e.g., `::suzunari_error`).
///
/// Hardcoded like snafu/thiserror instead of using proc-macro-crate, because
/// proc-macro-crate cannot distinguish doc tests from crate-internal code
/// (both return `FoundCrate::Itself`), causing doc tests to fail.
pub(crate) fn get_crate_path(original_name: &str) -> TokenStream {
    let ident = format_ident!("{}", original_name.replace('-', "_"));
    quote! { ::#ident }
}

/// Result of the location field lookup: marker check → type heuristic → name conflict.
pub(crate) enum LocationLookup {
    /// Found a location field at the given index.
    /// `needs_stack_attr`: true if found via type heuristic (caller should add
    /// `#[stack(location)]`), false if already has `#[stack(location)]` marker.
    Found {
        index: usize,
        needs_stack_attr: bool,
    },
    /// No location field found (no markers, no Location types, no name conflicts).
    NotFound,
}

/// Resolves the location field in a struct/variant's named fields.
///
/// Resolution order:
/// 1. `#[stack(location)]` marker — highest priority, any field name
/// 2. Single field of type `Location` — automatic fallback
/// 3. Name conflict check (field named "location" with wrong type)
///
/// `location_attr_hint` is used in ambiguity/conflict error messages to suggest
/// the context-appropriate attribute (e.g., `"#[suzu(location)]"` for
/// `#[suzunari_error]`, `"#[stack(location)]"` for `derive(StackError)`).
pub(crate) fn lookup_location_field(
    fields: &FieldsNamed,
    location_attr_hint: &str,
) -> Result<LocationLookup, Error> {
    // 1. Check #[stack(location)] markers
    let mut marked = Vec::new();
    for (i, field) in fields.named.iter().enumerate() {
        if has_stack_location_attr(field)? {
            marked.push(i);
        }
    }
    match marked.len() {
        1 => {
            return Ok(LocationLookup::Found {
                index: marked[0],
                needs_stack_attr: false,
            });
        }
        2.. => {
            return Err(Error::new(
                fields.named[marked[1]].span(),
                "multiple #[stack(location)] fields; only one is allowed per struct/variant",
            ));
        }
        0 => {}
    }

    // 2. Check Location-typed fields
    let location_typed: Vec<usize> = fields
        .named
        .iter()
        .enumerate()
        .filter(|(_, f)| looks_like_location_type(&f.ty))
        .map(|(i, _)| i)
        .collect();
    match location_typed.len() {
        1 => {
            return Ok(LocationLookup::Found {
                index: location_typed[0],
                needs_stack_attr: true,
            });
        }
        2.. => {
            let mut err = Error::new(
                fields.named[location_typed[1]].span(),
                format!(
                    "multiple fields with type name ending in `Location` found; \
                     use {location_attr_hint} to specify the correct one. \
                     Note: detection uses the last path segment, so types like \
                     `geo::Location` also match."
                ),
            );
            err.combine(Error::new(
                fields.named[location_typed[0]].span(),
                "first Location-typed field found here",
            ));
            return Err(err);
        }
        0 => {}
    }

    // 3. Name conflict: field named "location" with wrong type
    if let Some(field) = fields
        .named
        .iter()
        .find(|f| f.ident.as_ref().is_some_and(|i| i == "location"))
    {
        return Err(Error::new(
            field.span(),
            format!(
                "field 'location' exists but is not of type Location; \
                 rename it or use {location_attr_hint} on the correct field"
            ),
        ));
    }

    Ok(LocationLookup::NotFound)
}

/// Finds the location field in a struct/variant's named fields.
///
/// Delegates to [`lookup_location_field`] for resolution, then returns the
/// field reference or an error if not found.
///
/// Used by `derive(StackError)` to resolve the location field dynamically.
pub(crate) fn find_location_field(fields: &FieldsNamed) -> Result<&Field, Error> {
    match lookup_location_field(fields, "#[stack(location)]")? {
        LocationLookup::Found { index, .. } => {
            let field = &fields.named[index];
            if !looks_like_location_type(&field.ty) {
                return Err(Error::new(
                    field.ty.span(),
                    "#[stack(location)] field must be of type `suzunari_error::Location`",
                ));
            }
            Ok(field)
        }
        LocationLookup::NotFound => Err(Error::new(
            fields.span(),
            "StackError requires a Location field. Use #[suzunari_error] to auto-inject, \
             or add a field of type Location manually.",
        )),
    }
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
        let nested =
            meta_list.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)?;
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
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

/// Returns true if the type's last path segment is `Location`.
///
/// Uses segment name only (not the full path), so `my_module::Location`
/// would also match. See Known Limitations in lib.rs.
///
/// When this heuristic causes a false positive (e.g., `geo::Location`),
/// error messages from [`lookup_location_field`] and [`find_location_field`]
/// explain the heuristic so the user can disambiguate with `#[stack(location)]`.
pub(crate) fn looks_like_location_type(ty: &Type) -> bool {
    match ty {
        Type::Path(p) => p
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

/// Determines whether `field` is a snafu source field.
///
/// Parse errors in `#[snafu(...)]` attributes are silently ignored here:
/// snafu owns its attribute namespace and will separately report syntax
/// errors during its own derive expansion. We only need a best-effort
/// answer for `derive(StackError)`'s `stack_source()` generation.
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
            // Structured Meta parsing handles all current snafu syntax including
            // closure forms like source(from(T, |e| ...)): the closure lives inside
            // a parenthesized group which Meta::List stores as a raw TokenStream.
            let nested = meta_list
                .parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
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

/// Checks if any `#[snafu(...)]` attribute contains `keyword` as a top-level
/// keyword (e.g., `source`, `implicit`).
///
/// Uses token-level scanning via [`snafu_tokens_contain_keyword`].
///
/// Non-list forms (`#[snafu]`, `#[snafu = "..."]`) are silently skipped:
/// snafu doesn't use these forms and will report its own errors during
/// derive expansion. We only need to detect keywords in valid list syntax.
pub(crate) fn has_snafu_keyword(attrs: &[syn::Attribute], keyword: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("snafu") {
            return false;
        }
        let Meta::List(meta_list) = &attr.meta else {
            return false;
        };
        snafu_tokens_contain_keyword(&meta_list.tokens, keyword)
    })
}

/// Ensures the field has `#[snafu(implicit)]`. Adds it if missing.
pub(crate) fn ensure_snafu_implicit(field: &mut Field) {
    if !has_snafu_keyword(&field.attrs, "implicit") {
        field.attrs.push(syn::parse_quote!(#[snafu(implicit)]));
    }
}

/// Scans a token stream for `keyword` as a leading ident in any
/// comma-separated segment. Does not descend into groups (parentheses,
/// brackets, braces), so `display("source")` won't match `source`.
///
/// Because groups are opaque, a hypothetical snafu keyword like
/// `wrapper(source)` where `source` appears only inside parentheses
/// would NOT match. This is the desired behavior for current snafu
/// syntax where keywords are always top-level.
///
/// Used by [`has_snafu_keyword`] for best-effort keyword detection without
/// full Meta parsing.
fn snafu_tokens_contain_keyword(tokens: &TokenStream, keyword: &str) -> bool {
    let mut at_start = true;
    for tt in tokens.clone() {
        match &tt {
            TokenTree::Ident(ident) if at_start && *ident == keyword => return true,
            TokenTree::Punct(p) if p.as_char() == ',' => at_start = true,
            _ => at_start = false,
        }
    }
    false
}

/// Combines multiple `syn::Error`s into a single error, or returns `Ok(())` if empty.
///
/// Used by macro implementations to accumulate and report all errors at once,
/// instead of stopping at the first error.
pub(crate) fn combine_errors(errors: Vec<Error>) -> Result<(), Error> {
    let mut iter = errors.into_iter();
    let Some(mut combined) = iter.next() else {
        return Ok(());
    };
    for e in iter {
        combined.combine(e);
    }
    Err(combined)
}
