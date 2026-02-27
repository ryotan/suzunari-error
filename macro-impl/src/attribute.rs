use crate::helper::{get_crate_name, has_location_field};
use crate::suzu_attr::{self, SuzuResult};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields};

pub(crate) fn suzunari_error_impl(stream: TokenStream) -> Result<TokenStream, Error> {
    let mut input: DeriveInput = syn::parse2(stream.clone())?;

    let crate_path = get_crate_name("suzunari-error", &stream)?;
    let snafu_path = get_crate_name("snafu", &stream)?;

    // 1. Process #[suzu(...)] attrs (translate, location, snafu passthrough)
    let suzu_result = suzu_attr::process_suzu_attrs(&mut input, &crate_path)?;

    // 2. Inject location fields where not explicitly provided
    inject_location_fields(&mut input, &crate_path, &suzu_result)?;

    // 3. Generate derives (no more #[suzunari_location])
    let derive_attribute = quote! { #[derive(Debug, #snafu_path::Snafu, #crate_path::StackError)] };

    Ok(quote! {
        #derive_attribute
        #input
    })
}

/// Injects `location: Location` fields with `#[snafu(implicit)]` where needed.
///
/// Skips injection for structs/variants that:
/// - Already have a field named `location`
/// - Have a field with `#[suzu(location)]` (indicated by `suzu_result`)
fn inject_location_fields(
    input: &mut DeriveInput,
    crate_path: &Ident,
    suzu_result: &SuzuResult,
) -> Result<(), Error> {
    match &mut input.data {
        Data::Struct(data_struct) => {
            let has_explicit = suzu_result
                .has_explicit_location
                .first()
                .copied()
                .unwrap_or(false);
            match &mut data_struct.fields {
                Fields::Named(fields) => {
                    if !has_explicit && !has_location_field(fields) {
                        fields.named.push(make_location_field(crate_path));
                    }
                }
                Fields::Unit => {
                    // Cannot happen — struct with unit fields can't have suzu(location)
                    // and we need named fields for location injection
                    return Err(Error::new(
                        data_struct.fields.span(),
                        "#[suzunari_error] requires structs with named fields (or unit enum variants)",
                    ));
                }
                _ => {
                    return Err(Error::new(
                        data_struct.fields.span(),
                        "#[suzunari_error] can only be used on structs with named fields",
                    ));
                }
            }
        }
        Data::Enum(data_enum) => {
            for (i, variant) in data_enum.variants.iter_mut().enumerate() {
                let has_explicit = suzu_result
                    .has_explicit_location
                    .get(i)
                    .copied()
                    .unwrap_or(false);
                match &mut variant.fields {
                    Fields::Named(fields) => {
                        if !has_explicit && !has_location_field(fields) {
                            fields.named.push(make_location_field(crate_path));
                        }
                    }
                    Fields::Unit => {
                        // Convert unit variant to named-fields variant with location
                        let location_field = make_location_field(crate_path);
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
                            "#[suzunari_error] can only be used on enum variants with named fields",
                        ));
                    }
                }
            }
        }
        Data::Union(_) => {
            return Err(Error::new(
                input.span(),
                "#[suzunari_error] cannot be used on unions",
            ));
        }
    }
    Ok(())
}

fn make_location_field(crate_path: &Ident) -> syn::Field {
    syn::Field {
        attrs: vec![syn::parse_quote!(#[snafu(implicit)])],
        vis: syn::Visibility::Inherited,
        ident: Some(format_ident!("location")),
        colon_token: Some(syn::token::Colon::default()),
        ty: syn::parse_quote!(#crate_path::Location),
        mutability: syn::FieldMutability::None,
    }
}
