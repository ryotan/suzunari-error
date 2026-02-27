//! Processes `#[suzu(...)]` attributes on types, variants, and fields.
//!
//! `#[suzu(...)]` is a superset of `#[snafu(...)]`: suzunari-specific keywords
//! (`translate`, `location`) are handled here, and everything else is passed
//! through as `#[snafu(...)]`.

use crate::helper::extract_display_error_inner;
use proc_macro2::Ident;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Error, Field, Fields, Meta, Token};

/// Tracks which fields have an explicit `#[suzu(location)]`.
///
/// The outer `Vec` index corresponds to:
/// - For structs: a single-element vec (the struct itself)
/// - For enums: one entry per variant, in declaration order
///
/// Each inner `bool` is `true` if the variant/struct has a field with
/// `#[suzu(location)]`.
pub(crate) struct SuzuResult {
    pub has_explicit_location: Vec<bool>,
}

/// Processes all `#[suzu(...)]` attributes on `input`, consuming them.
///
/// - `translate` and `location` are handled as suzunari extensions.
/// - All other tokens are forwarded as `#[snafu(...)]`.
/// - Returns [`SuzuResult`] so the caller can decide auto-location injection.
pub(crate) fn process_suzu_attrs(
    input: &mut DeriveInput,
    crate_path: &Ident,
) -> Result<SuzuResult, Error> {
    match &mut input.data {
        Data::Struct(data_struct) => {
            // Type-level attrs
            process_type_level_attrs(&mut input.attrs)?;

            let has_explicit = match &mut data_struct.fields {
                Fields::Named(fields) => process_fields(&mut fields.named, crate_path)?,
                Fields::Unit => false,
                _ => false,
            };
            Ok(SuzuResult {
                has_explicit_location: vec![has_explicit],
            })
        }
        Data::Enum(data_enum) => {
            // Type-level attrs
            process_type_level_attrs(&mut input.attrs)?;

            let mut has_explicit_location = Vec::with_capacity(data_enum.variants.len());
            for variant in &mut data_enum.variants {
                // Variant-level attrs
                process_variant_level_attrs(&mut variant.attrs)?;

                let has_explicit = match &mut variant.fields {
                    Fields::Named(fields) => process_fields(&mut fields.named, crate_path)?,
                    Fields::Unit => false,
                    _ => false,
                };
                has_explicit_location.push(has_explicit);
            }
            Ok(SuzuResult {
                has_explicit_location,
            })
        }
        Data::Union(_) => Err(Error::new(input.span(), "#[suzu] cannot be used on unions")),
    }
}

/// Processes `#[suzu(...)]` on type-level attributes.
/// Only passthrough to `#[snafu(...)]` is allowed; `translate`/`location` are errors.
fn process_type_level_attrs(attrs: &mut Vec<Attribute>) -> Result<(), Error> {
    let mut new_attrs = Vec::new();
    let mut errors = Vec::new();

    for attr in attrs.drain(..) {
        if !attr.path().is_ident("suzu") {
            new_attrs.push(attr);
            continue;
        }
        match process_single_suzu_attr(&attr, Level::Type) {
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

/// Processes `#[suzu(...)]` on variant-level attributes.
/// Only passthrough to `#[snafu(...)]` is allowed; `translate`/`location` are errors.
fn process_variant_level_attrs(attrs: &mut Vec<Attribute>) -> Result<(), Error> {
    let mut new_attrs = Vec::new();
    let mut errors = Vec::new();

    for attr in attrs.drain(..) {
        if !attr.path().is_ident("suzu") {
            new_attrs.push(attr);
            continue;
        }
        match process_single_suzu_attr(&attr, Level::Variant) {
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
/// Returns `true` if any field has `#[suzu(location)]`.
fn process_fields(
    fields: &mut Punctuated<Field, Token![,]>,
    crate_path: &Ident,
) -> Result<bool, Error> {
    let mut has_explicit_location = false;
    let mut errors = Vec::new();

    for field in fields.iter_mut() {
        // Take ownership of attrs to avoid borrow conflicts when mutating field.ty
        let old_attrs = std::mem::take(&mut field.attrs);
        let mut new_attrs = Vec::new();
        let mut needs_translate = false;
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
                    if result.has_translate {
                        needs_translate = true;
                    }
                    if result.has_location {
                        needs_location = true;
                    }
                }
                Err(e) => errors.push(e),
            }
        }

        // Apply translate/location after the attrs loop so field is freely borrowable
        if needs_translate {
            match apply_translate(field, &new_attrs, crate_path) {
                Ok(snafu_source_attr) => new_attrs.push(snafu_source_attr),
                Err(e) => errors.push(e),
            }
        }
        if needs_location {
            apply_location(&mut new_attrs);
            has_explicit_location = true;
        }

        field.attrs = new_attrs;
    }

    combine_errors(errors)?;
    Ok(has_explicit_location)
}

#[derive(Clone, Copy)]
enum Level {
    Type,
    Variant,
    Field,
}

struct SingleAttrResult {
    /// The passthrough `#[snafu(...)]` attribute, if any non-suzunari tokens exist.
    snafu_passthrough: Option<Attribute>,
    /// Whether `translate` was found.
    has_translate: bool,
    /// Whether `location` was found.
    has_location: bool,
}

/// Parses a single `#[suzu(...)]` attribute.
///
/// Separates suzunari keywords from snafu passthrough tokens.
fn process_single_suzu_attr(attr: &Attribute, level: Level) -> Result<SingleAttrResult, Error> {
    let Meta::List(meta_list) = &attr.meta else {
        return Err(Error::new(attr.span(), "#[suzu] requires arguments"));
    };

    let nested: Punctuated<Meta, Token![,]> =
        meta_list.parse_args_with(Punctuated::parse_terminated)?;

    if nested.is_empty() {
        return Err(Error::new(attr.span(), "#[suzu] requires arguments"));
    }

    let mut has_translate = false;
    let mut has_location = false;
    let mut passthrough_tokens: Vec<Meta> = Vec::new();
    let mut has_source_in_passthrough = false;

    for meta in &nested {
        let is_translate = meta_is_ident(meta, "translate");
        let is_location = meta_is_ident(meta, "location");

        if is_translate {
            match level {
                Level::Field => has_translate = true,
                _ => {
                    return Err(Error::new(
                        meta.span(),
                        "`translate` can only be used on fields",
                    ));
                }
            }
        } else if is_location {
            match level {
                Level::Field => has_location = true,
                _ => {
                    return Err(Error::new(
                        meta.span(),
                        "`location` can only be used on fields",
                    ));
                }
            }
        } else {
            // Check if this is a `source(...)` passthrough (for conflict detection)
            if meta_is_ident_prefix(meta, "source") {
                has_source_in_passthrough = true;
            }
            passthrough_tokens.push(meta.clone());
        }
    }

    // Conflict: translate + source(...) in the same #[suzu(...)]
    if has_translate && has_source_in_passthrough {
        return Err(Error::new(
            attr.span(),
            "`translate` conflicts with `source(...)` — `translate` generates `source(from(...))` automatically",
        ));
    }

    let snafu_passthrough = if passthrough_tokens.is_empty() {
        None
    } else {
        Some(parse_quote!(#[snafu(#(#passthrough_tokens),*)]))
    };

    Ok(SingleAttrResult {
        snafu_passthrough,
        has_translate,
        has_location,
    })
}

/// Applies `translate` to a field: wraps type in `DisplayError<T>` and generates
/// `#[snafu(source(from(T, DisplayError::new)))]`.
///
/// `existing_attrs` contains all non-suzu attrs plus passthrough snafu attrs
/// already collected for this field. `field.attrs` is empty at this point
/// (taken via `std::mem::take` in the caller).
fn apply_translate(
    field: &mut Field,
    existing_attrs: &[Attribute],
    crate_path: &Ident,
) -> Result<Attribute, Error> {
    // Check for conflict with existing #[snafu(source(...))]
    if has_snafu_source(existing_attrs) {
        return Err(Error::new(
            field.span(),
            "`translate` conflicts with existing `#[snafu(source(...))]`",
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

/// Applies `location` to a field: adds `#[snafu(implicit)]` if not already present.
///
/// `attrs` contains all non-suzu attrs plus passthrough snafu attrs already
/// collected for this field. `field.attrs` is empty at this point.
fn apply_location(attrs: &mut Vec<Attribute>) {
    if !has_snafu_implicit(attrs) {
        attrs.push(parse_quote!(#[snafu(implicit)]));
    }
}

// --- Helpers ---

fn meta_is_ident(meta: &Meta, name: &str) -> bool {
    matches!(meta, Meta::Path(p) if p.is_ident(name))
}

fn meta_is_ident_prefix(meta: &Meta, name: &str) -> bool {
    match meta {
        Meta::Path(p) => p.is_ident(name),
        Meta::List(l) => l.path.is_ident(name),
        Meta::NameValue(nv) => nv.path.is_ident(name),
    }
}

fn has_snafu_source(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("snafu") {
            return false;
        }
        let Meta::List(meta_list) = &attr.meta else {
            return false;
        };
        let Ok(nested) = meta_list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        else {
            return false;
        };
        nested.iter().any(|m| meta_is_ident_prefix(m, "source"))
    })
}

fn has_snafu_implicit(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("snafu") {
            return false;
        }
        let Meta::List(meta_list) = &attr.meta else {
            return false;
        };
        let Ok(nested) = meta_list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        else {
            return false;
        };
        nested.iter().any(|m| meta_is_ident(m, "implicit"))
    })
}

fn combine_errors(errors: Vec<Error>) -> Result<(), Error> {
    let mut iter = errors.into_iter();
    let Some(mut combined) = iter.next() else {
        return Ok(());
    };
    for e in iter {
        combined.combine(e);
    }
    Err(combined)
}
