use crate::helper::{get_crate_name, has_location};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields};

pub(crate) fn suzunari_location_impl(stream: TokenStream) -> Result<TokenStream, Error> {
    let mut input: DeriveInput = syn::parse2(stream.clone())?;

    // Try to find the suzunari_error crate
    let crate_path = get_crate_name("suzunari-error", &stream)?;

    // Add the location field based on whether it's a struct or enum
    match &mut input.data {
        Data::Struct(data_struct) => {
            match &mut data_struct.fields {
                Fields::Named(fields) => {
                    // If it doesn't have a location field, add one
                    if !has_location(fields) {
                        // Create a new field with the #[snafu(implicit)] attribute
                        let location_field = location_field_impl(&crate_path);

                        // Add the field to the struct
                        fields.named.push(location_field);
                    }
                }
                _ => {
                    // Return an error for non-named fields
                    return Err(Error::new(
                        data_struct.fields.span(),
                        "suzunari_location can only be used on structs with named fields",
                    ));
                }
            }
        }
        Data::Enum(data_enum) => {
            // Check if all variants have named fields
            for variant in &mut data_enum.variants {
                match &mut variant.fields {
                    Fields::Named(fields) => {
                        // If it doesn't have a location field, add one
                        if !has_location(fields) {
                            // Create a new field with the #[snafu(implicit)] attribute
                            let location_field = location_field_impl(&crate_path);

                            // Add the field to the variant
                            fields.named.push(location_field);
                        }
                    }
                    Fields::Unit => {
                        // Create a new field with the #[snafu(implicit)] attribute
                        let location_field = location_field_impl(&crate_path);
                        let mut fields = Punctuated::new();
                        fields.push(location_field);
                        variant.fields = Fields::Named(syn::FieldsNamed {
                            brace_token: Default::default(),
                            named: fields,
                        });
                    }
                    _ => {
                        // Return an error for non-named fields
                        return Err(Error::new(
                            variant.span(),
                            "suzunari_location can only be used on enum variants with named fields",
                        ));
                    }
                }
            }
        }
        Data::Union(_) => {
            // Return an error for unions
            return Err(Error::new(
                input.span(),
                "suzunari_location cannot be used on unions",
            ));
        }
    }

    // Return the modified input
    Ok(quote! {
        #input
    })
}

pub(crate) fn suzunari_error_impl(stream: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse2(stream.clone())?;

    // Append Snafu and StackError derives
    let crate_path = get_crate_name("suzunari-error", &stream)?;
    let snafu_path = get_crate_name("snafu", &stream)?;

    // Generate #[suzunari_location] attribute
    let location_attribute = quote! { #[#crate_path::suzunari_location] };

    // Generate #[derive(Snafu, StackError)] attribute
    let derive_attribute = quote! { #[derive(Debug, #snafu_path::Snafu, #crate_path::StackError)] };

    // Combine attributes with the original struct/enum definition
    Ok(quote! {
        #location_attribute
        #derive_attribute
        #input
    })
}

fn location_field_impl(crate_path: &Ident) -> syn::Field {
    syn::Field {
        attrs: vec![syn::parse_quote!(#[snafu(implicit)])],
        vis: syn::Visibility::Inherited,
        ident: Some(format_ident!("location")),
        colon_token: Some(syn::token::Colon::default()),
        ty: syn::parse_quote!(#crate_path::Location),
        mutability: syn::FieldMutability::None,
    }
}
