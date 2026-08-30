//! Does a field attribute mean the same thing inside a serde `remote`
//! definition as it does on a struct the user derives directly?
//!
//! `#[suzunari_error(serialize)]` copies every field-level `#[serde(...)]`
//! verbatim into a `remote` definition. That is only sound for attributes whose
//! behaviour survives the move, and `remote` is not documented for this use, so
//! each one has to be measured rather than assumed.
//!
//! The list below is every field-level attribute `serde_derive` accepts, read
//! out of its own source rather than from the documentation:
//!
//! ```text
//! alias  borrow  bound  default  deserialize_with  flatten  getter  rename
//! serialize_with  skip  skip_deserializing  skip_serializing
//! skip_serializing_if  with
//! ```
//!
//! **Measured against serde 1.0.229.** The list is closed only for that
//! version. A later serde may add an attribute, and because the design copies
//! attributes without an allowlist, a user could write it and it would be
//! transplanted untested. There is no way to catch that in code — re-run this
//! file when the serde dependency moves, and re-read the list above from
//! `serde_derive`'s source.

#![cfg(feature = "serde")]

mod support;

use serde::{Serialize, Serializer};
use support::record;

/// Declares one attribute's parity case.
///
/// The attribute is handed to both sides unchanged. Nothing here decides what
/// an attribute means or where it belongs — that is the point: a difference in
/// the recorded output is the attribute failing to survive `remote`, not a
/// disagreement between two hand-written expectations.
macro_rules! parity {
    (
        module: $module:ident,
        attribute: #[$attribute:meta],
        field: $field:ident : $ty:ty = $value:expr,
        $(items: { $($items:item)* },)?
    ) => {
        mod $module {
            use super::*;

            $($($items)*)?

            #[derive(Serialize)]
            #[allow(dead_code)] // read only through `Serialize`
            struct Direct {
                #[$attribute]
                $field: $ty,
                other: u32,
            }

            #[allow(dead_code)] // read only through the definition
            struct Plain {
                $field: $ty,
                other: u32,
            }

            #[derive(Serialize)]
            #[serde(remote = "Plain", rename = "Direct")]
            struct PlainDef {
                #[$attribute]
                $field: $ty,
                other: u32,
            }

            /// Carries the borrow, exactly as generated code has to.
            struct Through<'a>(&'a Plain);

            impl Serialize for Through<'_> {
                fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    PlainDef::serialize(self.0, serializer)
                }
            }

            #[test]
            fn survives_the_remote_definition() {
                let direct = Direct {
                    $field: $value,
                    other: 1,
                };
                let plain = Plain {
                    $field: $value,
                    other: 1,
                };

                assert_eq!(record(&direct), record(&Through(&plain)));
            }
        }
    };
}

// --- attributes that shape the serialized output ---------------------------

parity! {
    module: rename,
    attribute: #[serde(rename = "renamed")],
    field: value: String = "v".to_owned(),
}

parity! {
    module: skip,
    attribute: #[serde(skip)],
    field: value: String = "v".to_owned(),
}

parity! {
    module: skip_serializing,
    attribute: #[serde(skip_serializing)],
    field: value: String = "v".to_owned(),
}

parity! {
    module: skip_serializing_if,
    attribute: #[serde(skip_serializing_if = "is_short")],
    field: value: String = "v".to_owned(),
    items: {
        fn is_short(value: &str) -> bool {
            value.len() < 2
        }
    },
}

parity! {
    module: serialize_with,
    attribute: #[serde(serialize_with = "shout")],
    field: value: String = "v".to_owned(),
    items: {
        fn shout<S: Serializer>(value: &str, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&value.to_uppercase())
        }
    },
}

parity! {
    module: with,
    attribute: #[serde(with = "shouty")],
    field: value: String = "v".to_owned(),
    items: {
        mod shouty {
            use serde::Serializer;

            pub fn serialize<S: Serializer>(
                value: &str,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&value.to_uppercase())
            }
        }
    },
}

parity! {
    module: flatten,
    attribute: #[serde(flatten)],
    field: value: Nested = Nested { inner: 9 },
    items: {
        #[derive(Serialize)]
        pub struct Nested {
            pub inner: u32,
        }
    },
}

// --- attributes that only affect deserialization ---------------------------
//
// These change no output. They are still worth a case: the design copies every
// field attribute verbatim, so they have to *compile* on the definition, and a
// struct the user derives directly accepts them too.

parity! {
    module: alias,
    attribute: #[serde(alias = "other_name")],
    field: value: String = "v".to_owned(),
}

parity! {
    module: default,
    attribute: #[serde(default)],
    field: value: String = "v".to_owned(),
}

parity! {
    module: deserialize_with,
    attribute: #[serde(deserialize_with = "unused")],
    field: value: String = "v".to_owned(),
    items: {
        // Named by the attribute, but never called: only `Serialize` is derived.
        #[allow(dead_code)]
        fn unused<'de, D: serde::Deserializer<'de>>(_: D) -> Result<String, D::Error> {
            unreachable!("deserialization is not generated")
        }
    },
}

parity! {
    module: skip_deserializing,
    attribute: #[serde(skip_deserializing)],
    field: value: String = "v".to_owned(),
}

parity! {
    module: bound,
    attribute: #[serde(bound(serialize = "String: Serialize"))],
    field: value: String = "v".to_owned(),
}

// --- the two that need a shape of their own --------------------------------

/// `borrow` needs a lifetime, so it cannot go through the macro above.
mod borrow {
    use super::*;

    #[derive(Serialize)]
    #[allow(dead_code)]
    struct Direct<'a> {
        #[serde(borrow)]
        value: &'a str,
        other: u32,
    }

    #[allow(dead_code)]
    struct Plain<'a> {
        value: &'a str,
        other: u32,
    }

    #[derive(Serialize)]
    // serde wants the bare path here: naming the lifetime is rejected with
    // "remove generic parameters from this path". Worth remembering for the
    // macro's own generics support.
    #[serde(remote = "Plain", rename = "Direct")]
    struct PlainDef<'a> {
        #[serde(borrow)]
        value: &'a str,
        other: u32,
    }

    struct Through<'a, 'b>(&'b Plain<'a>);

    impl Serialize for Through<'_, '_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            PlainDef::serialize(self.0, serializer)
        }
    }

    #[test]
    fn survives_the_remote_definition() {
        let direct = Direct {
            value: "v",
            other: 1,
        };
        let plain = Plain {
            value: "v",
            other: 1,
        };

        assert_eq!(record(&direct), record(&Through(&plain)));
    }
}

/// A field pointing at *another* `remote` definition, from inside one.
///
/// `remote` itself is a container attribute, so it never appears on a field and
/// is absent from the list above. A field reaches a definition through
/// `with` instead. The `with` case earlier points at a plain module; this one
/// points at a definition, which is the nesting the generated code actually
/// produces when a user's declared field holds a foreign type.
mod remote_within_remote {
    use super::*;

    /// Stands in for a type from another crate: no `Serialize` of its own.
    #[allow(dead_code)]
    pub struct Foreign {
        pub n: u32,
    }

    #[derive(Serialize)]
    #[serde(remote = "Foreign", rename = "Foreign")]
    struct ForeignDef {
        n: u32,
    }

    #[derive(Serialize)]
    #[allow(dead_code)]
    struct Direct {
        #[serde(with = "ForeignDef")]
        value: Foreign,
        other: u32,
    }

    #[allow(dead_code)]
    struct Plain {
        value: Foreign,
        other: u32,
    }

    #[derive(Serialize)]
    #[serde(remote = "Plain", rename = "Direct")]
    struct PlainDef {
        #[serde(with = "ForeignDef")]
        value: Foreign,
        other: u32,
    }

    struct Through<'a>(&'a Plain);

    impl Serialize for Through<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            PlainDef::serialize(self.0, serializer)
        }
    }

    #[test]
    fn survives_the_remote_definition() {
        let direct = Direct {
            value: Foreign { n: 9 },
            other: 1,
        };
        let plain = Plain {
            value: Foreign { n: 9 },
            other: 1,
        };

        assert_eq!(record(&direct), record(&Through(&plain)));
    }
}

/// `getter` has no parity case, because there is nothing to compare it against.
///
/// serde rejects it outright on a struct that is not a `remote` definition:
///
/// ```text
/// error: #[serde(getter = "...")] can only be used in structs that have
///        #[serde(remote = "...")]
/// ```
///
/// The user's error type is not a `remote` definition; the one generated from
/// it is. So a `getter` a user writes would fail on the struct they could have
/// written by hand and succeed here — while changing where the value is read
/// from. It is the one field attribute that fails both halves of the reason for
/// copying them verbatim: it is not inert, and a directly derived struct does
/// not accept it.
///
/// It therefore has to be rejected by the macro rather than transplanted. There
/// is no test here; the case belongs with the other compile-time rejections.
mod getter {}
