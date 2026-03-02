use crate::helper::{combine_errors, find_location_field, find_source_field, get_crate_path};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Data, DeriveInput, Error, Fields, FieldsNamed, Generics, Variant};

pub(crate) fn stack_error_impl(stream: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse2(stream)?;
    let name = &input.ident;
    let generics = &input.generics;

    let crate_path = get_crate_path("suzunari-error");

    match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => Ok(generate_struct_impl(name, fields, &crate_path, generics)?),
            _ => Err(Error::new(
                data_struct.fields.span(),
                "StackError can only be derived for structs with named fields",
            )),
        },
        Data::Enum(data_enum) => {
            generate_enum_impl(name, &data_enum.variants, &crate_path, generics)
        }
        Data::Union(_) => Err(Error::new(
            input.ident.span(),
            "StackError cannot be derived for unions",
        )),
    }
}

/// Generates the StackError implementation for a struct
fn generate_struct_impl(
    name: &Ident,
    fields: &FieldsNamed,
    crate_path: &TokenStream,
    generics: &Generics,
) -> Result<TokenStream, Error> {
    let loc_field = find_location_field(fields)?;
    // find_location_field operates on FieldsNamed, so ident is always Some.
    let Some(loc_name) = loc_field.ident.as_ref() else {
        unreachable!("find_location_field operates on FieldsNamed; ident is always present");
    };

    let type_name_str = name.to_string();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let stack_source_impl = match find_source_field(fields) {
        Some(field) => {
            // find_source_field operates on FieldsNamed, so ident is always Some.
            let Some(field_name) = field.ident.as_ref() else {
                unreachable!("find_source_field operates on FieldsNamed; ident is always present");
            };
            quote! {
                fn stack_source(&self) -> Option<&dyn #crate_path::StackError> {
                    #crate_path::__private::StackSourceResolver(&self.#field_name).resolve()
                }
            }
        }
        None => quote! {},
    };

    let boxed_impl = boxed_stack_error_impl(name, crate_path, generics);

    Ok(quote! {
        impl #impl_generics #crate_path::StackError for #name #ty_generics #where_clause {
            fn location(&self) -> &#crate_path::Location {
                &self.#loc_name
            }
            fn type_name(&self) -> &'static str {
                #type_name_str
            }
            #stack_source_impl
        }
        #boxed_impl
    })
}

/// Generates the StackError implementation for an enum
fn generate_enum_impl(
    name: &Ident,
    variants: &Punctuated<Variant, Comma>,
    crate_path: &TokenStream,
    generics: &Generics,
) -> Result<TokenStream, Error> {
    // Analyze each variant: resolve location and source field names.
    // Accumulate all errors so the user sees every problem at once.
    struct VariantInfo<'a> {
        ident: &'a Ident,
        loc_name: &'a Ident,
        source_field_name: Option<&'a Ident>,
    }
    let mut variant_infos = Vec::with_capacity(variants.len());
    let mut errors = Vec::new();
    for variant in variants {
        let Fields::Named(fields) = &variant.fields else {
            errors.push(Error::new(
                variant.fields.span(),
                "StackError can only be derived for enums with named fields in all variants",
            ));
            continue;
        };
        let loc_field = match find_location_field(fields) {
            Ok(field) => field,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        // find_location_field operates on FieldsNamed, so ident is always Some.
        let Some(loc_name) = loc_field.ident.as_ref() else {
            unreachable!("find_location_field operates on FieldsNamed; ident is always present");
        };
        let source_field_name = find_source_field(fields).and_then(|f| f.ident.as_ref());
        variant_infos.push(VariantInfo {
            ident: &variant.ident,
            loc_name,
            source_field_name,
        });
    }
    combine_errors(errors)?;

    let enum_name_str = name.to_string();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let has_any_source = variant_infos.iter().any(|v| v.source_field_name.is_some());

    let location_match_arms = variant_infos.iter().map(|v| {
        let variant_name = v.ident;
        let loc_name = v.loc_name;
        quote! { #name::#variant_name { #loc_name, .. } => #loc_name, }
    });

    let type_name_match_arms = variant_infos.iter().map(|v| {
        let variant_name = v.ident;
        let full_name = format!("{enum_name_str}::{variant_name}");
        quote! { #name::#variant_name { .. } => #full_name, }
    });

    let stack_source_match_arms = variant_infos.iter().map(|v| {
        let variant_name = v.ident;
        match v.source_field_name {
            Some(field_name) => quote! {
                #name::#variant_name { #field_name, .. } => {
                    #crate_path::__private::StackSourceResolver(#field_name).resolve()
                }
            },
            None => quote! {
                #name::#variant_name { .. } => None,
            },
        }
    });

    let stack_source_impl = if has_any_source {
        quote! {
            fn stack_source(&self) -> Option<&dyn #crate_path::StackError> {
                match self {
                    #(#stack_source_match_arms)*
                }
            }
        }
    } else {
        quote! {}
    };

    let boxed_impl = boxed_stack_error_impl(name, crate_path, generics);

    Ok(quote! {
        impl #impl_generics #crate_path::StackError for #name #ty_generics #where_clause {
            fn location(&self) -> &#crate_path::Location {
                match self {
                    #(#location_match_arms)*
                }
            }
            fn type_name(&self) -> &'static str {
                match self {
                    #(#type_name_match_arms)*
                }
            }
            #stack_source_impl
        }
        #boxed_impl
    })
}

/// Generates `From<T> for BoxedStackError` only when the alloc feature is enabled.
///
/// Uses `cfg!(feature = "alloc")` on the proc-macro crate's own feature flag,
/// NOT the expansion-site's cfg. This is correct because:
/// - `suzunari-error-macro-impl/alloc` is only activated via `suzunari-error/alloc`
/// - Adding `#[cfg(feature = "alloc")]` to generated code would check the
///   downstream crate's features, which is incorrect — downstream crates
///   do not declare their own `alloc` feature
fn boxed_stack_error_impl(
    name: &Ident,
    crate_path: &TokenStream,
    generics: &Generics,
) -> TokenStream {
    if !cfg!(feature = "alloc") {
        return quote! {};
    }

    let (impl_generics, ty_generics, _) = generics.split_for_impl();

    // For generic types, add explicit bounds required by BoxedStackError::new.
    // For non-generic types, the where clause is redundant but harmless.
    let existing_predicates: Vec<_> = generics
        .where_clause
        .iter()
        .flat_map(|wc| wc.predicates.iter())
        .collect();

    quote! {
        impl #impl_generics From<#name #ty_generics> for #crate_path::BoxedStackError
        where
            #(#existing_predicates,)*
            #name #ty_generics: #crate_path::StackError + Send + Sync + 'static,
        {
            fn from(error: #name #ty_generics) -> Self {
                #crate_path::BoxedStackError::new(error)
            }
        }
    }
}
