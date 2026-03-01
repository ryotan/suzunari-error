use crate::helper::{LocationLookup, ensure_snafu_implicit, get_crate_path, lookup_location_field};
use crate::suzu_attr;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Colon;
use syn::{Data, DeriveInput, Error, Field, FieldMutability, Fields, FieldsNamed, Visibility};

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

    // Reject unions early — before process_suzu_attrs, so the error message
    // refers to #[suzunari_error] (the macro the user actually wrote).
    if matches!(input.data, Data::Union(_)) {
        return Err(Error::new(
            input.span(),
            "#[suzunari_error] cannot be used on unions",
        ));
    }

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
                    "#[suzunari_error] can only be used on structs with named fields",
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
                        variant.fields = Fields::Named(FieldsNamed {
                            brace_token: Default::default(),
                            named: fields,
                        });
                    }
                    _ => {
                        return Err(Error::new(
                            variant.fields.span(),
                            "#[suzunari_error] can only be used on enum variants with named fields",
                        ));
                    }
                }
            }
        }
        Data::Union(_) => unreachable!("unions are rejected before this point"),
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
/// Delegates steps 1–3 (marker check, type heuristic, name conflict) to
/// [`lookup_location_field`], then applies the result:
/// - `Found` → ensure `#[stack(location)]` + `#[snafu(implicit)]` on the field
/// - `NotFound` → auto-inject a synthetic `location: Location` field
fn resolve_and_inject_location(
    fields: &mut FieldsNamed,
    crate_path: &TokenStream,
) -> Result<(), Error> {
    match lookup_location_field(fields, "#[suzu(location)]")? {
        LocationLookup::Found {
            index,
            needs_stack_attr,
        } => {
            let field = &mut fields.named[index];
            if needs_stack_attr {
                field.attrs.push(syn::parse_quote!(#[stack(location)]));
            }
            ensure_snafu_implicit(field);
        }
        LocationLookup::NotFound => {
            fields.named.push(location_field_impl(crate_path));
        }
    }
    Ok(())
}

/// Constructs a synthetic `location: Location` field with
/// `#[snafu(implicit)]` + `#[stack(location)]`.
fn location_field_impl(crate_path: &TokenStream) -> Field {
    Field {
        attrs: vec![
            syn::parse_quote!(#[snafu(implicit)]),
            syn::parse_quote!(#[stack(location)]),
        ],
        vis: Visibility::Inherited,
        ident: Some(format_ident!("location")),
        colon_token: Some(Colon::default()),
        ty: syn::parse_quote!(#crate_path::Location),
        mutability: FieldMutability::None,
    }
}
