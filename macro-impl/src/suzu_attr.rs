//! Processes `#[suzu(...)]` attributes on types, variants, and fields.
//!
//! `#[suzu(...)]` is a superset of `#[snafu(...)]`: suzunari-specific keywords
//! (`from`, `location`) are handled here, and everything else is passed
//! through as `#[snafu(...)]`.

use crate::helper::{
    combine_errors, extract_display_error_inner, has_snafu_keyword, looks_like_location_type,
};
use proc_macro2::{Span, TokenStream};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Error, Field, Fields, Meta, Token};

/// Processes all `#[suzu(...)]` attributes on `input`, consuming them.
///
/// - `from` and `location` are handled as suzunari extensions.
/// - All other tokens are forwarded as `#[snafu(...)]`.
///
/// After this call, `#[suzu(location)]` fields have `#[stack(location)]` +
/// `#[snafu(implicit)]`, and `#[suzu(from)]` fields have their type wrapped in
/// `DisplayError<T>` with `#[snafu(source(from(...)))]`.
///
pub(crate) fn process_suzu_attrs(
    input: &mut DeriveInput,
    crate_path: &TokenStream,
) -> Result<(), Error> {
    // Type-level attrs are always passthrough-only, regardless of struct/enum.
    process_non_field_attrs(&mut input.attrs, Level::NonField)?;

    match &mut input.data {
        Data::Struct(data_struct) => {
            match &mut data_struct.fields {
                Fields::Named(fields) => process_fields(&mut fields.named, crate_path)?,
                // Tuple structs / unit structs have no named fields to process.
                // Reject any stray #[suzu(...)] on their fields.
                fields => reject_suzu_on_non_named_fields(fields)?,
            }
            Ok(())
        }
        Data::Enum(data_enum) => {
            // Accumulate errors across all variants so the user sees every
            // problem at once, matching the pattern in derive.rs's generate_enum_impl.
            let mut errors = Vec::new();
            for variant in &mut data_enum.variants {
                if let Err(e) = process_non_field_attrs(&mut variant.attrs, Level::NonField) {
                    errors.push(e);
                }
                match &mut variant.fields {
                    Fields::Named(fields) => {
                        if let Err(e) = process_fields(&mut fields.named, crate_path) {
                            errors.push(e);
                        }
                    }
                    fields => {
                        if let Err(e) = reject_suzu_on_non_named_fields(fields) {
                            errors.push(e);
                        }
                    }
                }
            }
            combine_errors(errors)
        }
        // Currently unreachable: suzunari_error_impl rejects unions before calling
        // process_suzu_attrs. Kept as a defensive guard for direct callers.
        Data::Union(_) => Err(Error::new(input.span(), "#[suzu] cannot be used on unions")),
    }
}

/// Rejects `#[suzu(...)]` on fields of tuple/unit structs or variants.
fn reject_suzu_on_non_named_fields(fields: &Fields) -> Result<(), Error> {
    let mut errors = Vec::new();
    for field in fields {
        for attr in &field.attrs {
            if attr.path().is_ident("suzu") {
                errors.push(Error::new(
                    attr.span(),
                    "#[suzu(...)] is not supported on tuple or unit fields",
                ));
            }
        }
    }
    combine_errors(errors)
}

/// Processes `#[suzu(...)]` on type/variant-level attributes.
/// Only passthrough to `#[snafu(...)]` is allowed; `from`/`location` are errors.
fn process_non_field_attrs(attrs: &mut Vec<Attribute>, level: Level) -> Result<(), Error> {
    let mut new_attrs = Vec::new();
    let mut errors = Vec::new();

    for attr in attrs.drain(..) {
        if !attr.path().is_ident("suzu") {
            new_attrs.push(attr);
            continue;
        }
        match process_single_suzu_attr(&attr, level) {
            Ok(result) => {
                if let Some(snafu_attr) = result.snafu_passthrough {
                    new_attrs.push(snafu_attr);
                }
            }
            Err(e) => errors.push(e),
        }
    }

    *attrs = new_attrs;
    combine_errors(errors)
}

/// Processes `#[suzu(...)]` attributes on fields within a single struct/variant.
fn process_fields(
    fields: &mut Punctuated<Field, Token![,]>,
    crate_path: &TokenStream,
) -> Result<(), Error> {
    let mut errors = Vec::new();
    // Track first occurrence spans to detect cross-field duplicates.
    // Both from and location allow at most one per struct/variant.
    let mut first_from_span: Option<Span> = None;
    let mut first_location_span: Option<Span> = None;

    for field in fields.iter_mut() {
        // Take ownership of attrs to avoid borrow conflicts when mutating field.ty
        let old_attrs = std::mem::take(&mut field.attrs);
        let mut new_attrs = Vec::new();
        let mut needs_from = false;
        let mut needs_location = false;

        for attr in old_attrs {
            if !attr.path().is_ident("suzu") {
                new_attrs.push(attr);
                continue;
            }
            match process_single_suzu_attr(&attr, Level::Field) {
                Ok(result) => {
                    if let Some(snafu_attr) = result.snafu_passthrough {
                        new_attrs.push(snafu_attr);
                    }
                    match result.effect {
                        SuzuEffect::From => {
                            if let Some(first_span) = first_from_span {
                                let mut err = Error::new(
                                    attr.span(),
                                    "multiple #[suzu(from)] fields; only one source field is allowed per struct/variant",
                                );
                                err.combine(Error::new(
                                    first_span,
                                    "first #[suzu(from)] defined here",
                                ));
                                errors.push(err);
                            } else {
                                first_from_span = Some(attr.span());
                                needs_from = true;
                            }
                        }
                        SuzuEffect::Location => {
                            // Detect duplicate #[suzu(location)] early so error points
                            // at the original #[suzu(location)] attr, not the generated
                            // #[stack(location)] (which has a call_site span).
                            if let Some(first_span) = first_location_span {
                                let mut err = Error::new(
                                    attr.span(),
                                    "multiple #[suzu(location)] fields; only one is allowed per struct/variant",
                                );
                                err.combine(Error::new(
                                    first_span,
                                    "first #[suzu(location)] defined here",
                                ));
                                errors.push(err);
                            } else {
                                first_location_span = Some(attr.span());
                                needs_location = true;
                            }
                        }
                        SuzuEffect::PassthroughOnly => {}
                    }
                }
                Err(e) => errors.push(e),
            }
        }

        // Apply from/location after the attrs loop so field is freely borrowable.
        // Check cross-attribute conflict first — from and location on the same field
        // is always invalid regardless of how they were specified.
        if needs_from && needs_location {
            errors.push(Error::new(
                field.span(),
                "`from` and `location` cannot be used on the same field",
            ));
        } else {
            if needs_from {
                match apply_from(field, &new_attrs, crate_path) {
                    Ok(snafu_source_attr) => new_attrs.push(snafu_source_attr),
                    Err(e) => errors.push(e),
                }
            }
            if needs_location {
                if !looks_like_location_type(&field.ty) {
                    errors.push(Error::new(
                        field.ty.span(),
                        "#[suzu(location)] requires the field type to be `suzunari_error::Location`",
                    ));
                } else {
                    apply_location(&mut new_attrs);
                }
            }
        }

        field.attrs = new_attrs;
    }

    combine_errors(errors)
}

#[derive(Clone, Copy)]
enum Level {
    /// Type-level or variant-level — only passthrough allowed.
    NonField,
    /// Field-level — `from` and `location` are valid.
    Field,
}

/// What suzunari-specific effect a single `#[suzu(...)]` attribute requests.
///
/// `from` and `location` are mutually exclusive; passthrough-only or empty
/// effects carry no suzunari semantics.
enum SuzuEffect {
    /// No suzunari keyword — all tokens passed through to snafu.
    PassthroughOnly,
    /// `from` keyword found — wraps field type in `DisplayError<T>`.
    From,
    /// `location` keyword found — marks field as the location field.
    Location,
}

struct SingleAttrResult {
    /// The passthrough `#[snafu(...)]` attribute, if any non-suzunari tokens exist.
    snafu_passthrough: Option<Attribute>,
    /// Which suzunari extension (if any) was requested.
    effect: SuzuEffect,
}

/// Parses a single `#[suzu(...)]` attribute.
///
/// Separates suzunari keywords from snafu passthrough tokens.
fn process_single_suzu_attr(attr: &Attribute, level: Level) -> Result<SingleAttrResult, Error> {
    let Meta::List(meta_list) = &attr.meta else {
        return Err(Error::new(
            attr.span(),
            "#[suzu] requires arguments, e.g., #[suzu(location)] or #[suzu(display(\"...\"))]",
        ));
    };

    let nested: Punctuated<Meta, Token![,]> =
        meta_list.parse_args_with(Punctuated::parse_terminated)?;

    if nested.is_empty() {
        return Err(Error::new(
            attr.span(),
            "#[suzu()] requires arguments, e.g., #[suzu(location)] or #[suzu(display(\"...\"))]",
        ));
    }

    let mut effect = SuzuEffect::PassthroughOnly;
    let mut passthrough_tokens: Vec<Meta> = Vec::new();
    let mut has_source_in_passthrough = false;

    for meta in &nested {
        if meta.path().is_ident("from") {
            // `from` must be a bare keyword — reject list/namevalue forms
            if !matches!(meta, Meta::Path(_)) {
                return Err(Error::new(
                    meta.span(),
                    "`from` does not accept arguments; use `#[suzu(from)]` as a bare keyword",
                ));
            }
            if matches!(level, Level::NonField) {
                return Err(Error::new(meta.span(), "`from` can only be used on fields"));
            }
            if matches!(effect, SuzuEffect::Location) {
                return Err(Error::new(
                    attr.span(),
                    "`from` and `location` cannot be used on the same field",
                ));
            }
            effect = SuzuEffect::From;
        } else if meta.path().is_ident("location") {
            // `location` must be a bare keyword — reject list/namevalue forms
            if !matches!(meta, Meta::Path(_)) {
                return Err(Error::new(
                    meta.span(),
                    "`location` does not accept arguments; use `#[suzu(location)]` as a bare keyword",
                ));
            }
            if matches!(level, Level::NonField) {
                return Err(Error::new(
                    meta.span(),
                    "`location` can only be used on fields",
                ));
            }
            if matches!(effect, SuzuEffect::From) {
                return Err(Error::new(
                    attr.span(),
                    "`from` and `location` cannot be used on the same field",
                ));
            }
            effect = SuzuEffect::Location;
        } else {
            if meta.path().is_ident("source") {
                has_source_in_passthrough = true;
            }
            passthrough_tokens.push(meta.clone());
        }
    }

    // Conflict: from + source(...) in the same #[suzu(...)]
    if matches!(effect, SuzuEffect::From) && has_source_in_passthrough {
        return Err(Error::new(
            attr.span(),
            "`from` conflicts with `source(...)` — `from` generates `source(from(...))` automatically",
        ));
    }

    let snafu_passthrough = if passthrough_tokens.is_empty() {
        None
    } else {
        Some(parse_quote!(#[snafu(#(#passthrough_tokens),*)]))
    };

    Ok(SingleAttrResult {
        snafu_passthrough,
        effect,
    })
}

/// Applies `from` to a field: wraps type in `DisplayError<T>` and generates
/// `#[snafu(source(from(T, DisplayError::new)))]`.
///
/// # Preconditions
///
/// - `existing_attrs` must contain all attributes that will be set on this field
///   (i.e., `field.attrs` is not yet populated).
/// - No `#[snafu(source(...))]` should be present in `existing_attrs` (caller
///   should check for conflicts beforehand or let this function report it).
fn apply_from(
    field: &mut Field,
    existing_attrs: &[Attribute],
    crate_path: &TokenStream,
) -> Result<Attribute, Error> {
    // Check for conflict with existing #[snafu(source(...))]
    if has_snafu_keyword(existing_attrs, "source") {
        return Err(Error::new(
            field.span(),
            "`from` conflicts with existing `#[snafu(source(...))]`",
        ));
    }

    let original_type = match extract_display_error_inner(&field.ty) {
        // Already DisplayError<T> — just extract inner type for source(from(...))
        Some(inner) => inner.clone(),
        // Wrap the type: T → DisplayError<T>
        None => {
            let orig = field.ty.clone();
            field.ty = parse_quote!(#crate_path::DisplayError<#orig>);
            orig
        }
    };

    Ok(parse_quote!(
        #[snafu(source(from(#original_type, #crate_path::DisplayError::new)))]
    ))
}

/// Applies `location` to a field: adds `#[snafu(implicit)]` + `#[stack(location)]`.
///
/// `#[stack(location)]` is consumed by `derive(StackError)` to identify the
/// location field. `#[snafu(implicit)]` is consumed by `derive(Snafu)` for
/// auto-filling via `GenerateImplicitData`.
///
/// # Preconditions
///
/// - `attrs` must contain all attributes that will be set on this field
///   (i.e., the field's own `attrs` vec is not yet populated).
fn apply_location(attrs: &mut Vec<Attribute>) {
    if !has_snafu_keyword(attrs, "implicit") {
        attrs.push(parse_quote!(#[snafu(implicit)]));
    }
    attrs.push(parse_quote!(#[stack(location)]));
}
