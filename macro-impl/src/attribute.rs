use crate::helper::{get_crate_name, has_location};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields};

pub(crate) fn suzunari_location_impl(stream: TokenStream) -> TokenStream {
    let mut input: DeriveInput = syn::parse2(stream.clone()).unwrap();

    // Try to find the suzunari_error crate
    let crate_path = get_crate_name("suzunari-error").unwrap();

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
                    let error = Error::new(
                        data_struct.fields.span(),
                        "suzunari_location can only be used on structs with named fields",
                    );
                    return error.to_compile_error();
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
                        let error = Error::new(
                            variant.span(),
                            "suzunari_location can only be used on enum variants with named fields",
                        );
                        return error.to_compile_error();
                    }
                }
            }
        }
        Data::Union(_) => {
            // Return an error for unions
            let error = Error::new(input.span(), "suzunari_location cannot be used on unions");
            return error.to_compile_error();
        }
    }

    // Return the modified input
    quote! {
        #input
    }
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
