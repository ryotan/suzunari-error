//! `#[suzunari_error(serialize)]` — generates `impl Serialize` for an error type.
//!
//! The payload is the envelope described in `__private::ser`: metadata at the
//! top level, the type's own declared fields nested under `context`.
//!
//! # Why a serde `remote` definition
//!
//! The declared fields cannot simply be derived on the user's type: the type
//! also carries `source` and `location`, which belong to the metadata level,
//! and `source` is frequently not `Serialize` at all (`std::io::Error` appears
//! in this crate's own documentation). A `remote` definition gives us serde's
//! own field-handling code without putting `#[derive(Serialize)]` on the type.
//!
//! The definition is a **complete mirror** with `#[serde(skip)]` on the
//! metadata fields rather than a subset: omitting fields does not compile for
//! enums, and `skip` additionally lifts the `Serialize` bound on the skipped
//! field, which is what lets a non-`Serialize` source through.

use crate::helper::{find_location_field, find_source_field};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields, FieldsNamed, Ident};

/// Generates the `Serialize` impl and the marker impl for `input`.
///
/// `input` must already have been through location resolution, so every
/// struct/variant carries exactly one `#[stack(location)]` field.
pub(crate) fn generate_serialize_impl(
    input: &DeriveInput,
    crate_path: &TokenStream,
) -> Result<TokenStream, Error> {
    // A proc-macro cannot see the features of the crate invoking it, only its
    // own — hence the mirrored feature on this crate. Without this check the
    // user would get an unresolved-path error inside generated code instead.
    if !cfg!(feature = "serde") {
        return Err(Error::new(
            input.ident.span(),
            "#[suzunari_error(serialize)] requires the `serde` feature of suzunari-error",
        ));
    }

    if !input.generics.params.is_empty() {
        return Err(Error::new(
            input.generics.span(),
            "#[suzunari_error(serialize)] does not support generic types yet",
        ));
    }

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields,
            _ => unreachable!("#[suzunari_error] already rejected non-named fields"),
        },
        Data::Enum(_) => {
            return Err(Error::new(
                input.ident.span(),
                "#[suzunari_error(serialize)] does not support enums yet",
            ));
        }
        Data::Union(_) => unreachable!("unions are rejected before this point"),
    };

    let name = &input.ident;
    let serde = quote! { #crate_path::__private::serde };
    let ser = quote! { #crate_path::__private::ser };
    // `#[serde(crate = ...)]` takes a string, so the path is spelled twice.
    let serde_str = quote!(#serde).to_string();

    let location = find_location_field(fields)?
        .ident
        .clone()
        .expect("location field comes from FieldsNamed");
    let source = find_source_field(fields).and_then(|field| field.ident.clone());

    let (context_items, context_value) =
        context_parts(fields, name, &location, source.as_ref(), &serde, &serde_str);
    let source_value = match &source {
        // Autoref specialization: the specialized branch keys on the crate's
        // marker, never on `Serialize` alone. A foreign error that merely
        // derives `Serialize` would otherwise be inlined raw, producing a node
        // with no `type`, `message` or `location`.
        Some(field) => quote! {
            ::core::option::Option::Some(
                (&&#ser::SourceNodeResolver(&self.#field)).source_node()
            )
        },
        None => quote! { ::core::option::Option::<()>::None },
    };

    Ok(quote! {
        const _: () = {
            #context_items

            impl #serde::Serialize for #name {
                fn serialize<__S>(&self, serializer: __S) -> ::core::result::Result<__S::Ok, __S::Error>
                where
                    __S: #serde::Serializer,
                {
                    use #ser::ResolveSourceNode as _;
                    use #ser::ResolveSourceNodeFallback as _;

                    #serde::Serialize::serialize(
                        &#ser::StackErrorNode {
                            type_name: <Self as #crate_path::StackError>::type_name(self),
                            message: #ser::Message(self),
                            location: <Self as #crate_path::StackError>::location(self),
                            context: #context_value,
                            source: #source_value,
                        },
                        serializer,
                    )
                }
            }

            impl #ser::SerializeAsNode for #name {}
        };
    })
}

/// Builds the `remote` definition plus the adapter that lets it sit in the
/// node's `context` field, and the expression that fills that field.
///
/// A type that declares no fields of its own — only the injected `location` —
/// still gets a definition, which serializes as an empty object. `context` is
/// present but empty, never absent: an absent `context` is reserved for a node
/// whose concrete type was erased, where the fields exist but are unreachable.
/// `type` cannot carry that distinction, because a type-erased node still has
/// one (`BoxedStackError` forwards `type_name()` to the value it holds).
fn context_parts(
    fields: &FieldsNamed,
    name: &Ident,
    location: &Ident,
    source: Option<&Ident>,
    serde: &TokenStream,
    serde_str: &str,
) -> (TokenStream, TokenStream) {
    let is_metadata =
        |ident: &Ident| ident == location || source.is_some_and(|source| ident == source);

    // A complete mirror: every field is present, and the metadata ones are
    // skipped. User attributes are deliberately not carried over yet.
    let mirrored = fields.named.iter().map(|field| {
        let ident = field.ident.as_ref().expect("FieldsNamed");
        let ty = &field.ty;
        let skip = is_metadata(ident).then(|| quote! { #[serde(skip)] });
        quote! { #skip #ident: #ty }
    });

    let def = format_ident!("__SuzuContextDef");
    let adapter = format_ident!("__SuzuContext");
    let remote = name.to_string();

    let items = quote! {
        // `rename` is required: a definition's own identifier is what reaches
        // `serialize_struct` as the struct name. JSON discards struct names,
        // so without this the leak survives until a `Token`-level comparison.
        #[derive(#serde::Serialize)]
        #[serde(crate = #serde_str)]
        #[serde(remote = #remote, rename = #remote)]
        struct #def {
            #(#mirrored,)*
        }

        /// Carries the borrow that the generated `serialize` needs; the
        /// definition itself is not a `Serialize` impl for the error type.
        struct #adapter<'__suzu>(&'__suzu #name);

        impl #serde::Serialize for #adapter<'_> {
            fn serialize<__S>(&self, serializer: __S) -> ::core::result::Result<__S::Ok, __S::Error>
            where
                __S: #serde::Serializer,
            {
                #def::serialize(self.0, serializer)
            }
        }
    };

    (
        items,
        quote! { ::core::option::Option::Some(#adapter(self)) },
    )
}
