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
//!
//! # Enums
//!
//! The definition is `untagged`, so a variant contributes no wrapper of its own
//! — the variant's name already reaches the payload as part of `type`.
//!
//! A variant that declares nothing still produces `{}` rather than `null`,
//! because `#[suzunari_error]` gives every unit variant an injected location
//! field first. A definition with a genuine unit variant does serialize as
//! `null` under `untagged`, but that shape never gets here.
//!
//! The node is built once per variant rather than once with the source chosen
//! inside it. Two variants can hold sources of different types, and the
//! specialized branch resolves each to its own — there is no single type the
//! `source` field could have.

use crate::attribute::Options;
use crate::helper::{combine_errors, find_location_field, find_source_field};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Attribute, Data, DeriveInput, Error, Field, Fields, FieldsNamed, Ident, Meta};

/// One struct, or one variant of an enum: the fields, and which of them are
/// metadata rather than declared.
struct Shape<'a> {
    variant: Option<&'a Ident>,
    fields: &'a FieldsNamed,
    location: Ident,
    source: Option<Ident>,
}

impl Shape<'_> {
    fn is_metadata(&self, ident: &Ident) -> bool {
        *ident == self.location || self.source.as_ref().is_some_and(|source| ident == source)
    }
}

/// Generates the `Serialize` impl and the marker impl for `input`.
///
/// `input` must already have been through location resolution, so every
/// struct/variant carries exactly one `#[stack(location)]` field.
pub(crate) fn generate_serialize_impl(
    input: &DeriveInput,
    crate_path: &TokenStream,
    options: &Options,
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

    check_container_attrs(input)?;
    let shapes = shapes(input)?;
    combine_errors(
        shapes
            .iter()
            .filter_map(|shape| check_serde_attrs(shape).err())
            .collect(),
    )?;

    let name = &input.ident;
    let serde = quote! { #crate_path::__private::serde };
    let ser = quote! { #crate_path::__private::ser };
    // `#[serde(crate = ...)]` takes a string, so the path is spelled twice.
    let serde_str = quote!(#serde).to_string();

    let context_items = context_definition(&shapes, input, options, &serde, &serde_str);
    let adapter = format_ident!("__SuzuContext");

    let node = |source: TokenStream| {
        quote! {
            #serde::Serialize::serialize(
                &#ser::StackErrorNode {
                    type_name: <Self as #crate_path::StackError>::type_name(self),
                    message: #ser::Message(self),
                    location: <Self as #crate_path::StackError>::location(self),
                    context: #adapter(self),
                    source: #source,
                },
                serializer,
            )
        }
    };
    // Autoref specialization: the specialized branch keys on the crate's
    // marker, never on `Serialize` alone. A foreign error that merely derives
    // `Serialize` would otherwise be inlined raw, producing a node with no
    // `type`, `message` or `location`.
    let resolve = |binding: &Ident| {
        quote! {
            ::core::option::Option::Some(
                (&&#ser::SourceNodeResolver(#binding)).source_node()
            )
        }
    };
    let no_source = quote! { ::core::option::Option::<()>::None };

    let dispatch = match &input.data {
        Data::Struct(_) => {
            let shape = &shapes[0];
            match &shape.source {
                Some(field) => {
                    let binding = format_ident!("__suzu_source");
                    let bind = quote! { let #binding = &self.#field; };
                    let node = node(resolve(&binding));
                    quote! { #bind #node }
                }
                None => node(no_source.clone()),
            }
        }
        _ => {
            let arms = shapes.iter().map(|shape| {
                let variant = shape.variant.expect("enum shapes carry a variant");
                match &shape.source {
                    Some(field) => {
                        let binding = format_ident!("__suzu_source");
                        let node = node(resolve(&binding));
                        quote! { Self::#variant { #field: #binding, .. } => { #node } }
                    }
                    None => {
                        let node = node(no_source.clone());
                        quote! { Self::#variant { .. } => { #node } }
                    }
                }
            });
            quote! { match self { #(#arms)* } }
        }
    };

    // The impl needs whatever `Display` and `StackError` need, which only the
    // derives that generated them know. Naming `Self: StackError` borrows their
    // bounds rather than guessing at them; `Error` carries `Display` along.
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let serialize_bounds = serialize_bounds(input, &serde);
    let outer_where = merge_where(
        where_clause,
        &quote! {
            where
                Self: #crate_path::StackError,
                #(#serialize_bounds,)*
        },
    );

    Ok(quote! {
        const _: () = {
            #context_items

            impl #impl_generics #serde::Serialize for #name #ty_generics #outer_where {
                fn serialize<__S>(&self, serializer: __S) -> ::core::result::Result<__S::Ok, __S::Error>
                where
                    __S: #serde::Serializer,
                {
                    use #ser::ResolveSourceNode as _;
                    use #ser::ResolveSourceNodeFallback as _;

                    #dispatch
                }
            }

            impl #impl_generics #ser::SerializeAsNode for #name #ty_generics #outer_where {}
        };
    })
}

/// Splits the input into one shape per struct or variant.
fn shapes(input: &DeriveInput) -> Result<Vec<Shape<'_>>, Error> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => Ok(vec![shape(None, fields)?]),
            _ => unreachable!("#[suzunari_error] already rejected non-named fields"),
        },
        Data::Enum(data) => {
            let mut shapes = Vec::new();
            let mut errors = Vec::new();
            for variant in &data.variants {
                match &variant.fields {
                    Fields::Named(fields) => match shape(Some(&variant.ident), fields) {
                        Ok(shape) => shapes.push(shape),
                        Err(error) => errors.push(error),
                    },
                    _ => unreachable!("#[suzunari_error] already rejected non-named fields"),
                }
            }
            combine_errors(errors)?;
            Ok(shapes)
        }
        Data::Union(_) => unreachable!("unions are rejected before this point"),
    }
}

fn shape<'a>(variant: Option<&'a Ident>, fields: &'a FieldsNamed) -> Result<Shape<'a>, Error> {
    Ok(Shape {
        variant,
        fields,
        location: find_location_field(fields)?
            .ident
            .clone()
            .expect("location field comes from FieldsNamed"),
        source: find_source_field(fields).and_then(|field| field.ident.clone()),
    })
}

/// Builds the `remote` definition plus the adapter that lets it sit in the
/// node's `context` field.
///
/// A type that declares no fields of its own — only the injected `location` —
/// still gets a definition, which serializes as an empty object. `context` is
/// present but empty, never absent: an absent `context` is reserved for a node
/// whose concrete type was erased, where the fields exist but are unreachable.
/// `type` cannot carry that distinction, because a type-erased node still has
/// one (`BoxedStackError` forwards `type_name()` to the value it holds).
fn context_definition(
    shapes: &[Shape<'_>],
    input: &DeriveInput,
    options: &Options,
    serde: &TokenStream,
    serde_str: &str,
) -> TokenStream {
    let name = &input.ident;
    let def = format_ident!("__SuzuContextDef");
    let adapter = format_ident!("__SuzuContext");
    // serde wants the bare path: naming the parameters is rejected with
    // "remove generic parameters from this path".
    let remote = name.to_string();

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // The adapter holds a borrow, so it needs a lifetime of its own on top of
    // whatever the error type already has.
    let mut adapter_generics = generics.clone();
    adapter_generics
        .params
        .insert(0, syn::parse_quote!('__suzu));
    let (adapter_impl, adapter_ty, _) = adapter_generics.split_for_impl();
    let serialize_bounds = serialize_bounds(input, serde);
    let adapter_where = merge_where(where_clause, &quote! { where #(#serialize_bounds,)* });

    // serde spells the same intent differently by shape: on a struct
    // `rename_all` renames fields, on an enum it renames variants — which
    // `untagged` never emits — and `rename_all_fields` renames the fields.
    // Measured. The option stays one spelling; picking serde's is this macro's
    // job, and there is nothing else in the payload it could mean.
    let rename_struct = options
        .rename_all
        .as_ref()
        .map(|case| quote! { , rename_all = #case });
    let rename_enum = options
        .rename_all
        .as_ref()
        .map(|case| quote! { , rename_all_fields = #case });

    let body = if matches!(input.data, Data::Struct(_)) {
        let mirrored = mirrored_fields(&shapes[0]);
        quote! {
            #[serde(remote = #remote, rename = #remote #rename_struct)]
            struct #def #impl_generics #where_clause {
                #(#mirrored,)*
            }
        }
    } else {
        let variants = shapes.iter().map(|shape| {
            let variant = shape.variant.expect("enum shapes carry a variant");
            let mirrored = mirrored_fields(shape);
            quote! { #variant { #(#mirrored,)* } }
        });
        quote! {
            // `untagged`: the variant's name is already in `type`, so a wrapper
            // here would repeat it. A variant whose fields are all skipped
            // serializes as `{}`, matching a struct that declares nothing.
            #[serde(remote = #remote, rename = #remote, untagged #rename_enum)]
            enum #def #impl_generics #where_clause {
                #(#variants,)*
            }
        }
    };

    quote! {
        // `rename` is required: a definition's own identifier is what reaches
        // `serialize_struct` as the struct name. JSON discards struct names,
        // so without this the leak survives until a `Token`-level comparison.
        #[derive(#serde::Serialize)]
        #[serde(crate = #serde_str)]
        #body

        /// Carries the borrow that the generated `serialize` needs; the
        /// definition itself is not a `Serialize` impl for the error type.
        struct #adapter #adapter_generics (&'__suzu #name #ty_generics) #where_clause;

        impl #adapter_impl #serde::Serialize for #adapter #adapter_ty #adapter_where {
            fn serialize<__S>(&self, serializer: __S) -> ::core::result::Result<__S::Ok, __S::Error>
            where
                __S: #serde::Serializer,
            {
                #def::serialize(self.0, serializer)
            }
        }
    }
}

/// A complete mirror: every field is present, and the metadata ones are
/// skipped. A declared field's own `#[serde(...)]` comes along unchanged — the
/// definition's fields are the same fields, so an attribute means there what it
/// would have meant on a struct the user derived directly.
fn mirrored_fields(shape: &Shape<'_>) -> Vec<TokenStream> {
    shape
        .fields
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().expect("FieldsNamed");
            let ty = &field.ty;
            if shape.is_metadata(ident) {
                return quote! { #[serde(skip)] #ident: #ty };
            }
            let attrs = serde_attrs(field);
            quote! { #(#attrs)* #ident: #ty }
        })
        .collect()
}

/// The field's own `#[serde(...)]` attributes.
fn serde_attrs(field: &Field) -> impl Iterator<Item = &Attribute> {
    field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
}

/// Rejects the serde attributes that cannot be transplanted.
///
/// Two kinds. An attribute on the `source` or `location` field would be
/// silently ignored, because those are skipped in the definition. And `getter`
/// cannot go anywhere: serde rejects it outside a `remote` definition, so a
/// user writing it would fail on the struct they could have written by hand
/// while succeeding here — and it changes where the value is read from.
fn check_serde_attrs(shape: &Shape<'_>) -> Result<(), Error> {
    let errors = shape
        .fields
        .named
        .iter()
        .flat_map(|field| {
            let metadata = shape.is_metadata(field.ident.as_ref().expect("FieldsNamed"));
            serde_attrs(field).filter_map(move |attr| {
                if metadata {
                    return Some(Error::new(
                        attr.span(),
                        "#[serde(...)] on the source or location field is ignored: \
                         both belong to the metadata level, not to `context`",
                    ));
                }
                getter_span(attr).map(|span| {
                    Error::new(
                        span,
                        "#[serde(getter = ...)] cannot be used here: serde accepts it only \
                         inside a remote definition, so it would apply to the generated \
                         definition and not to this type",
                    )
                })
            })
        })
        .collect();

    combine_errors(errors)
}

/// Where `getter` appears inside one `#[serde(...)]`, if it does.
///
/// A parse failure is ignored: serde owns this namespace and reports its own
/// syntax errors once the attribute reaches the definition.
fn getter_span(attr: &Attribute) -> Option<Span> {
    let Meta::List(list) = &attr.meta else {
        return None;
    };
    Punctuated::<Meta, Comma>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()?
        .iter()
        .find(|meta| meta.path().is_ident("getter"))
        .map(Spanned::span)
}

/// Removes every `#[serde(...)]` from the type's fields.
///
/// The error type itself has no `Serialize` derive, so an attribute left on it
/// would not compile. Its meaning moves to the generated definition.
pub(crate) fn strip_serde_attrs(input: &mut DeriveInput) {
    let strip = |fields: &mut Fields| {
        for field in fields.iter_mut() {
            field.attrs.retain(|attr| !attr.path().is_ident("serde"));
        }
    };
    match &mut input.data {
        Data::Struct(data) => strip(&mut data.fields),
        Data::Enum(data) => data.variants.iter_mut().for_each(|v| strip(&mut v.fields)),
        Data::Union(_) => {}
    }
}

/// `T: Serialize` for each of the type's parameters.
///
/// The same bound serde's own derive infers, which is the point: the payload
/// has to match what the user would have got from `#[derive(Serialize)]`.
fn serialize_bounds(input: &DeriveInput, serde: &TokenStream) -> Vec<TokenStream> {
    input
        .generics
        .type_params()
        .map(|param| {
            let ident = &param.ident;
            quote! { #ident: #serde::Serialize }
        })
        .collect()
}

/// Appends predicates to a `where` clause that may not exist yet.
fn merge_where(existing: Option<&syn::WhereClause>, extra: &TokenStream) -> TokenStream {
    match existing {
        Some(clause) => {
            let predicates = &clause.predicates;
            quote! { #extra #predicates }
        }
        None => extra.clone(),
    }
}

/// Rejects `#[serde(...)]` written on the type or on a variant.
///
/// The generated definition is a different container than the one the user
/// annotated. Measured across that level, the attributes are a mixed bag: some
/// change the payload in a way the type did not ask for, some do nothing at
/// all, and one fails only at runtime. The level is refused whole rather than
/// an allowlist maintained, which would have to track serde forever.
fn check_container_attrs(input: &DeriveInput) -> Result<(), Error> {
    let mut errors: Vec<Error> = container_errors(&input.attrs, false).collect();
    if let Data::Enum(data) = &input.data {
        for variant in &data.variants {
            errors.extend(container_errors(&variant.attrs, true));
        }
    }
    combine_errors(errors)
}

fn container_errors(attrs: &[Attribute], on_variant: bool) -> impl Iterator<Item = Error> + '_ {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
        .map(move |attr| Error::new(attr.span(), container_message(attr, on_variant)))
}

/// What to say about one rejected container attribute.
///
/// Two of them have a specific answer worth giving, since they are the ones a
/// user is most likely to reach for.
fn container_message(attr: &Attribute, on_variant: bool) -> String {
    let where_it_is = if on_variant { "a variant" } else { "the type" };
    let mentions = |name: &str| -> bool {
        let Meta::List(list) = &attr.meta else {
            return false;
        };
        Punctuated::<Meta, Comma>::parse_terminated
            .parse2(list.tokens.clone())
            .map(|metas| metas.iter().any(|meta| meta.path().is_ident(name)))
            .unwrap_or(false)
    };

    if mentions("rename_all") || mentions("rename_all_fields") {
        return format!(
            "#[serde(...)] on {where_it_is} is not supported. To rename the declared \
             fields, write #[suzunari_error(serialize(rename_all = \"...\"))] instead"
        );
    }
    if mentions("rename") {
        return format!(
            "#[serde(rename = ...)] on {where_it_is} is not supported. The name reaches \
             the payload as `type`, which comes from StackError::type_name() and is not \
             renameable"
        );
    }
    format!(
        "#[serde(...)] on {where_it_is} is not supported: the definition generated from it \
         is a different container, where the attribute would either change the payload or \
         do nothing. Field-level attributes are carried over; renaming is available as \
         #[suzunari_error(serialize(rename_all = \"...\"))]"
    )
}
