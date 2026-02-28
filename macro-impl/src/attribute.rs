use crate::helper::{
    get_crate_path, has_stack_location_attr, looks_like_location_type, snafu_tokens_contain_keyword,
};
use crate::suzu_attr;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields, FieldsNamed, Meta};

/// Implementation of `#[suzunari_error]`.
///
/// Three-step pipeline:
/// 1. `process_suzu_attrs` — rewrites `#[suzu(...)]` to `#[snafu(...)]` + `#[stack(...)]`
/// 2. `resolve_and_inject_location` — ensures every struct/variant has exactly one location field
/// 3. Emit `#[derive(Debug, Snafu, StackError)]` wrapping the rewritten input
pub(crate) fn suzunari_error_impl(stream: TokenStream) -> Result<TokenStream, Error> {
    let mut input: DeriveInput = syn::parse2(stream)?;
    let crate_path = get_crate_path("suzunari-error");
    let snafu_path = get_crate_path("snafu");

    // Step 1: Process #[suzu(...)] attrs (from, location, snafu passthrough)
    // - #[suzu(location)] → #[stack(location)] + #[snafu(implicit)]
    // - #[suzu(from)] → DisplayError wrapping + #[snafu(source(from(...)))]
    // - other #[suzu(...)] tokens → #[snafu(...)] passthrough
    suzu_attr::process_suzu_attrs(&mut input, &crate_path)?;

    // Step 2: Resolve and inject location fields
    match &mut input.data {
        Data::Struct(data_struct) => match &mut data_struct.fields {
            Fields::Named(fields) => {
                resolve_and_inject_location(fields, &crate_path)?;
            }
            _ => {
                return Err(Error::new(
                    data_struct.fields.span(),
                    "suzunari_error can only be used on structs with named fields",
                ));
            }
        },
        Data::Enum(data_enum) => {
            for variant in &mut data_enum.variants {
                match &mut variant.fields {
                    Fields::Named(fields) => {
                        resolve_and_inject_location(fields, &crate_path)?;
                    }
                    Fields::Unit => {
                        let location_field = location_field_impl(&crate_path);
                        let mut fields = Punctuated::new();
                        fields.push(location_field);
                        variant.fields = Fields::Named(syn::FieldsNamed {
                            brace_token: Default::default(),
                            named: fields,
                        });
                    }
                    _ => {
                        return Err(Error::new(
                            variant.span(),
                            "suzunari_error can only be used on enum variants with named fields",
                        ));
                    }
                }
            }
        }
        Data::Union(_) => {
            return Err(Error::new(
                input.span(),
                "suzunari_error cannot be used on unions",
            ));
        }
    }

    // Step 3: Emit derives (location injection is done above)
    let derive_attribute = quote! { #[derive(Debug, #snafu_path::Snafu, #crate_path::StackError)] };

    Ok(quote! {
        #derive_attribute
        #input
    })
}

/// Location resolution flow for a single struct/variant.
///
/// 1. `#[stack(location)]` count → 1: OK, 2+: error
/// 2. Location-typed field count → 1: add `#[stack(location)]`, 2+: error
/// 3. "location" name conflict check → error if non-Location type
/// 4. Auto-inject `location: Location` with `#[stack(location)]` + `#[snafu(implicit)]`
fn resolve_and_inject_location(
    fields: &mut FieldsNamed,
    crate_path: &TokenStream,
) -> Result<(), Error> {
    // 1. Check #[stack(location)] markers
    let mut stack_marked = Vec::new();
    for (i, field) in fields.named.iter().enumerate() {
        if has_stack_location_attr(field)? {
            stack_marked.push(i);
        }
    }

    match stack_marked.len() {
        1 => {
            ensure_snafu_implicit(&mut fields.named[stack_marked[0]]);
            return Ok(());
        }
        n if n > 1 => {
            return Err(Error::new(
                fields.named[stack_marked[1]].span(),
                "multiple #[stack(location)] fields; only one is allowed per struct/variant",
            ));
        }
        _ => {}
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
            let field = &mut fields.named[location_typed[0]];
            field.attrs.push(syn::parse_quote!(#[stack(location)]));
            ensure_snafu_implicit(field);
            return Ok(());
        }
        n if n > 1 => {
            return Err(Error::new(
                fields.named[location_typed[1]].span(),
                "multiple Location fields found; use #[suzu(location)] to specify which one",
            ));
        }
        _ => {}
    }

    // 3. Check for "location" name conflict
    let name_conflict = fields
        .named
        .iter()
        .find(|f| f.ident.as_ref().is_some_and(|i| i == "location"));
    if let Some(field) = name_conflict {
        return Err(Error::new(
            field.span(),
            "field 'location' exists but is not of type Location; \
             rename it or change its type to Location",
        ));
    }

    // 4. Auto-inject location field
    fields.named.push(location_field_impl(crate_path));
    Ok(())
}

/// Ensures the field has `#[snafu(implicit)]`. Adds it if missing.
fn ensure_snafu_implicit(field: &mut syn::Field) {
    let has_implicit = field.attrs.iter().any(|attr| {
        if !attr.path().is_ident("snafu") {
            return false;
        }
        let Meta::List(meta_list) = &attr.meta else {
            return false;
        };
        snafu_tokens_contain_keyword(&meta_list.tokens, "implicit")
    });

    if !has_implicit {
        field.attrs.push(syn::parse_quote!(#[snafu(implicit)]));
    }
}

/// Constructs a synthetic `location: Location` field with
/// `#[snafu(implicit)]` + `#[stack(location)]`.
fn location_field_impl(crate_path: &TokenStream) -> syn::Field {
    syn::Field {
        attrs: vec![
            syn::parse_quote!(#[snafu(implicit)]),
            syn::parse_quote!(#[stack(location)]),
        ],
        vis: syn::Visibility::Inherited,
        ident: Some(format_ident!("location")),
        colon_token: Some(syn::token::Colon::default()),
        ty: syn::parse_quote!(#crate_path::Location),
        mutability: syn::FieldMutability::None,
    }
}
