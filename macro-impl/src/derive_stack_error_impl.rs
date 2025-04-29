use crate::{get_crate_name, has_location};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields, FieldsNamed, Variant};
use syn::__private::TokenStream2;

pub(crate) fn derive_stack_error_impl(stream: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse2(stream.clone()).unwrap();
    let name = &input.ident;

    // Try to find the suzunari_error crate
    let crate_path = get_crate_name("suzunari-error").unwrap();

    // Generate the implementation based on whether it's a struct or enum
    match &input.data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields) => {
                    generate_struct_impl(name, fields, &crate_path)
                },
                _ => {
                    // Return an error for non-named fields
                    let error = Error::new(
                        data_struct.fields.span(),
                        "StackError can only be derived for structs with named fields"
                    );
                    error.to_compile_error()
                }
            }
        },
        Data::Enum(data_enum) => {
            generate_enum_impl(name, &data_enum.variants, &crate_path)
        },
        Data::Union(_) => {
            // Return an error for unions
            let error = Error::new(
                stream.span(),
                "StackError cannot be derived for unions"
            );
            error.to_compile_error()
        }
    }
}

/// Generates the StackError implementation for a struct
fn generate_struct_impl(name: &Ident, fields: &FieldsNamed, crate_path: &Ident) -> TokenStream {
    // Return an error if the struct doesn't have the required fields
    if !has_location(fields) {
        let error = Error::new(
            fields.span(),
            "StackError requires a 'location' field of type Location"
        );
        return error.to_compile_error();
    }

    // Generate the implementation
    quote! {
        impl #crate_path::StackError for #name {
            fn location(&self) -> &#crate_path::Location {
                &self.location
            }
        }
        impl core::fmt::Debug for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                #crate_path::write_stack_error_log(f, self)
            }
        }
    }
}

/// Generates the StackError implementation for an enum
fn generate_enum_impl(name: &Ident, variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>, crate_path: &Ident) -> TokenStream2 {
    // Check if all variants have named fields
    for variant in variants {
        match &variant.fields {
            Fields::Named(_) => {
                // This is fine
            },
            _ => {
                let error = Error::new(
                    variant.span(),
                    "StackError can only be derived for enums with named fields in all variants"
                );
                return error.to_compile_error();
            }
        }
    }

    // Generate match arms for each method
    let location_match_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        quote! {
            #name::#variant_name { location, .. } => location,
        }
    });

    // Generate the implementation
    quote! {
        impl #crate_path::StackError for #name {
            fn location(&self) -> &#crate_path::Location {
                match self {
                    #(#location_match_arms)*
                }
            }
        }
        impl core::fmt::Debug for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                #crate_path::write_stack_error_log(f, self)
            }
        }
    }
}

