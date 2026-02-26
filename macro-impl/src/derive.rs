use crate::{get_crate_name, has_location};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields, FieldsNamed, Variant};

pub(crate) fn stack_error_impl(stream: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse2(stream.clone())?;
    let name = &input.ident;

    // Try to find the suzunari_error crate
    let crate_path = get_crate_name("suzunari-error", &stream)?;

    // Generate the implementation based on whether it's a struct or enum
    match &input.data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields) => Ok(generate_struct_impl(name, fields, &crate_path)),
                _ => {
                    // Return an error for non-named fields
                    Err(Error::new(
                        data_struct.fields.span(),
                        "StackError can only be derived for structs with named fields",
                    ))
                }
            }
        }
        Data::Enum(data_enum) => Ok(generate_enum_impl(name, &data_enum.variants, &crate_path)),
        Data::Union(_) => {
            // Return an error for unions
            Err(Error::new(
                stream.span(),
                "StackError cannot be derived for unions",
            ))
        }
    }
}

/// Generates the StackError implementation for a struct
fn generate_struct_impl(name: &Ident, fields: &FieldsNamed, crate_path: &Ident) -> TokenStream {
    // Return an error if the struct doesn't have the required fields
    if !has_location(fields) {
        let error = Error::new(
            fields.span(),
            "StackError requires a 'location' field of type Location",
        );
        return error.to_compile_error();
    }

    let boxed_impl = boxed_stack_error_impl(name, crate_path);

    // Generate the implementation
    quote! {
        impl #crate_path::StackError for #name {
            fn location(&self) -> &#crate_path::Location {
                &self.location
            }
        }
        impl core::fmt::Debug for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                #crate_path::StackError::fmt_stack(self, f)
            }
        }
        #boxed_impl
    }
}

/// Generates the StackError implementation for an enum
fn generate_enum_impl(
    name: &Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
    crate_path: &Ident,
) -> TokenStream {
    // Check if all variants have named fields
    for variant in variants {
        match &variant.fields {
            Fields::Named(_) => {
                // This is fine
            }
            _ => {
                let error = Error::new(
                    variant.span(),
                    "StackError can only be derived for enums with named fields in all variants",
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

    let boxed_impl = boxed_stack_error_impl(name, crate_path);

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
                #crate_path::StackError::fmt_stack(self, f)
            }
        }
        #boxed_impl
    }
}

/// Generates `From<T> for BoxedStackError` only when the alloc feature is enabled.
///
/// The branch is based on the proc-macro crate's own feature flag, not the
/// expansion-site's cfg. Controlled by `suzunari-error-macro-impl/alloc`,
/// so downstream crates do not need to declare an `alloc` feature themselves.
fn boxed_stack_error_impl(name: &Ident, crate_path: &Ident) -> TokenStream {
    if cfg!(feature = "alloc") {
        quote! {
            impl From<#name> for #crate_path::BoxedStackError {
                fn from(error: #name) -> Self {
                    #crate_path::BoxedStackError::new(error)
                }
            }
        }
    } else {
        quote! {}
    }
}
