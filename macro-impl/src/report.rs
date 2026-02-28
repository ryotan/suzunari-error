use crate::helper::get_crate_name;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Error, ItemFn, ReturnType, Type};

pub(crate) fn report_impl(attr: TokenStream, stream: TokenStream) -> Result<TokenStream, Error> {
    // #[report] takes no arguments
    if !attr.is_empty() {
        return Err(Error::new(
            attr.span(),
            "#[report] does not accept arguments",
        ));
    }

    let input: ItemFn = syn::parse2(stream.clone())?;

    // Reject function qualifiers that the closure wrap cannot preserve.
    if input.sig.asyncness.is_some() {
        return Err(Error::new(
            input.sig.asyncness.span(),
            "#[report] does not support async functions. Place #[report] below #[tokio::main] or similar runtime attributes so that async is resolved first.",
        ));
    }
    if input.sig.unsafety.is_some() {
        return Err(Error::new(
            input.sig.unsafety.span(),
            "#[report] does not support unsafe functions",
        ));
    }
    if input.sig.abi.is_some() {
        return Err(Error::new(
            input.sig.abi.span(),
            "#[report] does not support extern functions",
        ));
    }
    if input.sig.constness.is_some() {
        return Err(Error::new(
            input.sig.constness.span(),
            "#[report] does not support const functions",
        ));
    }

    // Generic parameters are not supported
    if !input.sig.generics.params.is_empty() {
        return Err(Error::new(
            input.sig.generics.span(),
            "#[report] does not support generic parameters",
        ));
    }
    if input.sig.generics.where_clause.is_some() {
        return Err(Error::new(
            input.sig.generics.where_clause.span(),
            "#[report] does not support where clauses",
        ));
    }

    // Extract the return type — must be Result<(), E>
    let ReturnType::Type(_, ref return_type) = input.sig.output else {
        return Err(Error::new(
            input.sig.fn_token.span(),
            "#[report] requires the function to return Result<(), E>",
        ));
    };

    let crate_path = get_crate_name("suzunari-error", &stream)?;
    let error_type = extract_result_error_type(return_type)?;

    let vis = &input.vis;
    let sig_ident = &input.sig.ident;
    let sig_inputs = &input.sig.inputs;
    let body = &input.block;
    let attrs = &input.attrs;
    let original_return_type = return_type;

    Ok(quote! {
        #(#attrs)*
        #vis fn #sig_ident(#sig_inputs) -> #crate_path::StackReport<#error_type> {
            (|| -> #original_return_type #body)().into()
        }
    })
}

/// Extracts `E` from `Result<(), E>`.
fn extract_result_error_type(ty: &Type) -> Result<&Type, Error> {
    let Type::Path(type_path) = ty else {
        return Err(Error::new(
            ty.span(),
            "#[report] requires the return type to be Result<(), E>",
        ));
    };

    let last_segment = type_path.path.segments.last().ok_or_else(|| {
        Error::new(
            ty.span(),
            "#[report] requires the return type to be Result<(), E>",
        )
    })?;

    if last_segment.ident != "Result" {
        return Err(Error::new(
            last_segment.ident.span(),
            "#[report] requires the return type to be Result<(), E>",
        ));
    }

    let syn::PathArguments::AngleBracketed(ref args) = last_segment.arguments else {
        return Err(Error::new(
            last_segment.span(),
            "#[report] requires the return type to be Result<(), E>",
        ));
    };

    if args.args.len() != 2 {
        return Err(Error::new(
            args.span(),
            "#[report] requires the return type to be Result<(), E>",
        ));
    }

    // Validate Ok type is ()
    let syn::GenericArgument::Type(ref ok_type) = args.args[0] else {
        return Err(Error::new(
            args.args[0].span(),
            "#[report] requires the return type to be Result<(), E>",
        ));
    };
    if !matches!(ok_type, Type::Tuple(t) if t.elems.is_empty()) {
        return Err(Error::new(
            ok_type.span(),
            "#[report] requires the Ok type to be (), only Result<(), E> is supported",
        ));
    }

    let syn::GenericArgument::Type(ref error_type) = args.args[1] else {
        return Err(Error::new(
            args.args[1].span(),
            "#[report] requires the return type to be Result<(), E>",
        ));
    };

    Ok(error_type)
}
