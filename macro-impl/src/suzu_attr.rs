//! Processes `#[suzu(...)]` attributes on types, variants, and fields.
//!
//! `#[suzu(...)]` is a superset of `#[snafu(...)]`: suzunari-specific keywords
//! (`from`, `location`) are handled here, and everything else is passed
//! through as `#[snafu(...)]`.

use crate::helper::{
    combine_errors, extract_display_error_inner, has_snafu_keyword, looks_like_location_type,
};
use proc_macro2::{Span, TokenStream};
use std::collections::HashSet;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Error, Field, Fields, GenericParam, Ident, Meta, Token};

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
    process_non_field_attrs(&mut input.attrs)?;

    let generic_type_params: HashSet<Ident> = input
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Type(tp) => Some(tp.ident.clone()),
            _ => None,
        })
        .collect();

    match &mut input.data {
        Data::Struct(data_struct) => {
            match &mut data_struct.fields {
                Fields::Named(fields) => {
                    process_fields(&mut fields.named, crate_path, &generic_type_params)?
                }
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
                if let Err(e) = process_non_field_attrs(&mut variant.attrs) {
                    errors.push(e);
                }
                match &mut variant.fields {
                    Fields::Named(fields) => {
                        if let Err(e) =
                            process_fields(&mut fields.named, crate_path, &generic_type_params)
                        {
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
                    "#[suzu(...)] is not supported on unnamed fields; use named fields instead",
                ));
            }
        }
    }
    combine_errors(errors)
}

/// Processes `#[suzu(...)]` on type/variant-level attributes.
/// Only passthrough to `#[snafu(...)]` is allowed; `from`/`location` are errors.
fn process_non_field_attrs(attrs: &mut Vec<Attribute>) -> Result<(), Error> {
    let level = Level::NonField;
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
    generic_type_params: &HashSet<Ident>,
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
        // Per-field span: Some(span) means this field has the keyword.
        // first_from_span/first_location_span track cross-field duplicates.
        let mut current_from_span: Option<Span> = None;
        let mut current_location_span: Option<Span> = None;

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
                        SuzuEffect::From(keyword_span) => {
                            if let Some(first_span) = first_from_span {
                                // Distinguish same-field duplicate from cross-field duplicate:
                                // current_from_span is set when this field already has `from`.
                                let msg = if current_from_span.is_some() {
                                    "duplicate #[suzu(from)] on the same field"
                                } else {
                                    "multiple #[suzu(from)] fields; only one source field is allowed per struct/variant"
                                };
                                let mut err = Error::new(keyword_span, msg);
                                err.combine(Error::new(
                                    first_span,
                                    "first occurrence of #[suzu(from)] is here",
                                ));
                                errors.push(err);
                                // Clear current_from_span so the post-loop match skips
                                // apply_from — applying it after a duplicate error would
                                // produce spurious secondary errors.
                                current_from_span = None;
                            } else {
                                first_from_span = Some(keyword_span);
                                current_from_span = Some(keyword_span);
                            }
                        }
                        SuzuEffect::Location(keyword_span) => {
                            // Detect duplicate #[suzu(location)] early so error points
                            // at the original #[suzu(location)] attr, not the generated
                            // #[stack(location)] (which has a call_site span).
                            if let Some(first_span) = first_location_span {
                                let msg = if current_location_span.is_some() {
                                    "duplicate #[suzu(location)] on the same field"
                                } else {
                                    "multiple #[suzu(location)] fields; only one is allowed per struct/variant"
                                };
                                let mut err = Error::new(keyword_span, msg);
                                err.combine(Error::new(
                                    first_span,
                                    "first occurrence of #[suzu(location)] is here",
                                ));
                                errors.push(err);
                                // Clear current_location_span so the post-loop match
                                // skips apply_location on this field.
                                current_location_span = None;
                            } else {
                                first_location_span = Some(keyword_span);
                                current_location_span = Some(keyword_span);
                            }
                        }
                        SuzuEffect::PassthroughOnly => {}
                    }
                }
                Err(e) => errors.push(e),
            }
        }

        // Apply from/location after the attrs loop so the field is freely borrowable.
        //
        // from+location conflict is checked in three places:
        //   1. Within-attr: #[suzu(location, from)] — caught in process_single_suzu_attr
        //   2. Within-attr: #[suzu(from, location)] — caught in process_single_suzu_attr
        //   3. Cross-attr: #[suzu(from)] #[suzu(location)] — caught here
        // (1) and (2) provide better spans (pointing to the conflicting keyword),
        // while (3) catches the cross-attribute case that within-attr checks cannot see.
        match (current_from_span, current_location_span) {
            (Some(from_span), Some(loc_span)) => {
                let mut err = Error::new(
                    from_span,
                    "`from` and `location` cannot be used on the same field",
                );
                err.combine(Error::new(loc_span, "`location` defined here"));
                errors.push(err);
            }
            (Some(from_span), None) => match apply_from(
                field,
                &new_attrs,
                crate_path,
                from_span,
                generic_type_params,
            ) {
                Ok(snafu_source_attr) => new_attrs.push(snafu_source_attr),
                Err(e) => errors.push(e),
            },
            (None, Some(_)) => {
                if !looks_like_location_type(&field.ty) {
                    errors.push(Error::new(
                        field.ty.span(),
                        "#[suzu(location)] requires the field type to be `suzunari_error::Location`",
                    ));
                } else {
                    apply_location(&mut new_attrs);
                }
            }
            (None, None) => {}
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
/// effects carry no suzunari semantics. Each variant carries the keyword's
/// span for precise error messages in cross-field duplicate detection.
enum SuzuEffect {
    /// No suzunari keyword — all tokens passed through to snafu.
    PassthroughOnly,
    /// `from` keyword found — wraps field type in `DisplayError<T>`.
    From(Span),
    /// `location` keyword found — marks field as the location field.
    Location(Span),
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
            // `from` must be a bare keyword — reject list/name-value forms
            if !matches!(meta, Meta::Path(_)) {
                return Err(Error::new(
                    meta.span(),
                    "`from` does not accept arguments; use `#[suzu(from)]` as a bare keyword",
                ));
            }
            if matches!(level, Level::NonField) {
                return Err(Error::new(meta.span(), "`from` can only be used on fields"));
            }
            if matches!(effect, SuzuEffect::Location(_)) {
                // Within-attr conflict: #[suzu(location, from)] — point to the `from` keyword.
                return Err(Error::new(
                    meta.span(),
                    "`from` and `location` cannot be used on the same field",
                ));
            }
            effect = SuzuEffect::From(meta.span());
        } else if meta.path().is_ident("location") {
            // `location` must be a bare keyword — reject list/name-value forms
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
            if matches!(effect, SuzuEffect::From(_)) {
                // Within-attr conflict: #[suzu(from, location)] — point to the `location` keyword.
                return Err(Error::new(
                    meta.span(),
                    "`from` and `location` cannot be used on the same field",
                ));
            }
            effect = SuzuEffect::Location(meta.span());
        } else {
            if meta.path().is_ident("source") {
                has_source_in_passthrough = true;
            }
            passthrough_tokens.push(meta.clone());
        }
    }

    // Conflict: from + source(...) in the same #[suzu(...)]
    if matches!(effect, SuzuEffect::From(_)) && has_source_in_passthrough {
        return Err(Error::new(
            attr.span(),
            "`from` conflicts with `source(...)`: `from` generates `source(from(...))` automatically",
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
/// `#[snafu(source(from(T, __wrap)))]` where `__wrap` uses autoref specialization
/// to resolve `get_source` delegation at compile time.
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
    from_span: Span,
    generic_type_params: &HashSet<Ident>,
) -> Result<Attribute, Error> {
    // Check for conflict with existing #[snafu(source(...))]
    if has_snafu_keyword(existing_attrs, "source") {
        return Err(Error::new(
            from_span,
            "`from` conflicts with existing `#[snafu(source(...))]`",
        ));
    }

    // Reject #[suzu(from)] on fields whose type involves generic type parameters.
    // The autoref specialization trick requires a concrete type at compile time
    // to resolve whether `source()` should delegate to the inner type. Generic
    // parameters prevent this. Users should implement `From` manually instead.
    if type_uses_generic_params(&field.ty, generic_type_params) {
        return Err(Error::new(
            from_span,
            "`from` cannot be used on fields with generic type parameters; \
             use `#[suzu(source(from(T, DisplayError::new)))]` with an explicit \
             `DisplayError<T>` field type, or implement `From` manually",
        ));
    }

    let original_type = match extract_display_error_inner(&field.ty) {
        // Already DisplayError<T> — just extract the inner type for source(from(...))
        Some(inner) => inner.clone(),
        // Wrap the type: T → DisplayError<T>
        None => {
            let orig = field.ty.clone();
            field.ty = parse_quote!(#crate_path::DisplayError<#orig>);
            orig
        }
    };

    // Generate a block expression that defines a local wrapping function and
    // returns it. Inside the local function, the concrete `original_type` is
    // known, so Deref-based autoref specialization correctly resolves
    // `get_source_fn()`: inherent method wins when `original_type: Error`,
    // otherwise Deref falls back to `DisplayErrorSourceFallback`.
    //
    // Using a local function (instead of an inline closure) is necessary
    // because closures inside snafu attributes inherit proc-macro token hygiene,
    // which can prevent Deref fallback from being discovered during method
    // resolution.
    Ok(parse_quote!(
        #[snafu(source(from(#original_type, {
            fn __wrap(__source: #original_type) -> #crate_path::DisplayError<#original_type> {
                let __get_source: fn(&#original_type) -> Option<&(dyn core::error::Error + 'static)>
                    = #crate_path::__private::DisplayErrorSourceResolver(&__source).get_source_fn();
                #crate_path::DisplayError::with_get_source(__source, __get_source)
            }
            __wrap
        })))]
    ))
}

/// Checks whether `ty` references any of the given generic type parameters.
fn type_uses_generic_params(ty: &syn::Type, params: &HashSet<Ident>) -> bool {
    use syn::{GenericArgument, PathArguments, ReturnType, Type};

    match ty {
        Type::Path(type_path) => type_path.path.segments.iter().any(|seg| {
            params.contains(&seg.ident)
                || match &seg.arguments {
                    PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| match arg {
                        GenericArgument::Type(inner) => type_uses_generic_params(inner, params),
                        _ => false,
                    }),
                    PathArguments::Parenthesized(paren) => {
                        paren
                            .inputs
                            .iter()
                            .any(|t| type_uses_generic_params(t, params))
                            || matches!(&paren.output, ReturnType::Type(_, t) if type_uses_generic_params(t, params))
                    }
                    PathArguments::None => false,
                }
        }),
        Type::Reference(type_ref) => type_uses_generic_params(&type_ref.elem, params),
        Type::Tuple(type_tuple) => type_tuple
            .elems
            .iter()
            .any(|t| type_uses_generic_params(t, params)),
        Type::Array(type_array) => type_uses_generic_params(&type_array.elem, params),
        Type::Slice(type_slice) => type_uses_generic_params(&type_slice.elem, params),
        Type::Paren(type_paren) => type_uses_generic_params(&type_paren.elem, params),
        // Conservatively reject unknown type forms when generics exist.
        _ => !params.is_empty(),
    }
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
    // Guard against duplicate #[stack(location)] — can happen if the user
    // writes both #[stack(location)] and #[suzu(location)] on the same field.
    let already_has_stack_location = attrs.iter().any(|a| a.path().is_ident("stack"));
    if !already_has_stack_location {
        attrs.push(parse_quote!(#[stack(location)]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Calling apply_location twice must not duplicate #[snafu(implicit)] or #[stack(location)].
    #[test]
    fn test_apply_location_is_idempotent() {
        let mut attrs: Vec<Attribute> = Vec::new();
        apply_location(&mut attrs);
        assert_eq!(
            attrs.len(),
            2,
            "first call should add implicit + stack(location)"
        );

        apply_location(&mut attrs);
        assert_eq!(
            attrs.len(),
            2,
            "second call should not duplicate any attributes"
        );
    }

    /// apply_location must not add #[snafu(implicit)] if already present.
    #[test]
    fn test_apply_location_preserves_existing_implicit() {
        let mut attrs: Vec<Attribute> = vec![parse_quote!(#[snafu(implicit)])];
        apply_location(&mut attrs);
        assert_eq!(attrs.len(), 2, "should add only #[stack(location)]");

        let implicit_count = attrs.iter().filter(|a| a.path().is_ident("snafu")).count();
        assert_eq!(implicit_count, 1, "should not duplicate #[snafu(implicit)]");
    }

    /// apply_location must not add #[stack(location)] if already present.
    #[test]
    fn test_apply_location_preserves_existing_stack_location() {
        let mut attrs: Vec<Attribute> = vec![parse_quote!(#[stack(location)])];
        apply_location(&mut attrs);
        assert_eq!(attrs.len(), 2, "should add only #[snafu(implicit)]");

        let stack_count = attrs.iter().filter(|a| a.path().is_ident("stack")).count();
        assert_eq!(stack_count, 1, "should not duplicate #[stack(location)]");
    }
}
