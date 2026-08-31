//! Which instantiations `#[suzunari_error(serialize)]` permits.
//!
//! The differential suite compares output, so it cannot see a `Serialize` bound
//! that is inferred too strictly. A parameter that did not need the bound
//! serializes to the same bytes as one that did, and a parameter that cannot
//! satisfy it produces no output to compare against. The property measured here
//! is the other one: a parameter is left unbounded whenever the declared fields
//! do not need it of it.
//!
//! Every fixture below is instantiated with a type that deliberately does not
//! implement `Serialize`, so a bound that is too strict shows up as a failure to
//! compile rather than as a wrong assertion.
//!
//! `.build()` is used because these fixtures have no source field: there is no
//! `Result` to attach a context selector to.

#![cfg(feature = "serde")]

use core::fmt::Debug;
use core::marker::PhantomData;
use serde::{Serialize, Serializer};
use suzunari_error::*;

/// `Debug`, which `#[suzunari_error]` requires, but deliberately not
/// `Serialize`. Standing in for any ordinary type a caller might parameterise an
/// error with.
#[derive(Debug)]
struct Opaque;

// ---------------------------------------------------------------------------
// A field whose type is an associated type of the parameter
// ---------------------------------------------------------------------------

trait Store {
    type Key;
}

#[derive(Debug)]
struct FileStore;

impl Store for FileStore {
    type Key = String;
}

/// `S::Key` is what reaches the payload, so `S::Key` is what needs `Serialize`.
/// `S` itself never does.
///
/// The `Debug` predicates are `derive(Debug)`'s own requirement rather than
/// serde's: it bounds the parameter and never the associated type, so the
/// clause has to be written out.
#[suzunari_error(serialize)]
#[suzu(display("no entry for the key"))]
struct NotFound<S>
where
    S: Store + Debug,
    S::Key: Debug + Serialize,
{
    key: S::Key,
}

/// `FileStore` is not `Serialize`; its `Key` is. Inferring `S: Serialize` here
/// rules out every marker type anyone would write.
#[test]
fn an_associated_type_field_bounds_the_associated_type() {
    let error = NotFoundSnafu {
        key: "k1".to_string(),
    }
    .build();
    let value = serde_json::to_value::<NotFound<FileStore>>(error).unwrap();

    assert_eq!(value["type"], "NotFound");
    assert_eq!(value["context"]["key"], "k1");
}

// ---------------------------------------------------------------------------
// PhantomData
// ---------------------------------------------------------------------------

/// `PhantomData<T>` is `Serialize` whether or not `T` is. serde carries a
/// hardcoded exception for it, and so must anything mirroring serde's inference.
#[suzunari_error(serialize)]
#[suzu(display("tagged failure"))]
struct Tagged<T: Debug> {
    marker: PhantomData<T>,
    detail: String,
}

#[test]
fn a_phantom_data_field_bounds_nothing() {
    let error = TaggedSnafu {
        marker: PhantomData,
        detail: "boom".to_string(),
    }
    .build();
    let value = serde_json::to_value::<Tagged<Opaque>>(error).unwrap();

    assert_eq!(value["type"], "Tagged");
    assert_eq!(value["context"]["detail"], "boom");
}

// ---------------------------------------------------------------------------
// serialize_with
// ---------------------------------------------------------------------------

/// Writes a value through `Debug`, so the field it is attached to needs
/// `T: Debug` and nothing about `Serialize`.
fn as_debug<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Debug,
    S: Serializer,
{
    serializer.collect_str(&format_args!("{value:?}"))
}

/// A field carrying `serialize_with` is written by that function, not by its own
/// `Serialize` impl, so it says nothing about what the parameter must implement.
#[suzunari_error(serialize)]
#[suzu(display("rejected the value"))]
struct Rejected<T: Debug> {
    #[serde(serialize_with = "as_debug")]
    value: T,
}

#[test]
fn a_serialize_with_field_bounds_nothing() {
    let error = RejectedSnafu { value: Opaque }.build();
    let value = serde_json::to_value::<Rejected<Opaque>>(error).unwrap();

    assert_eq!(value["type"], "Rejected");
    assert_eq!(value["context"]["value"], "Opaque");
}

// ---------------------------------------------------------------------------
// A field-level `bound` of the user's own
// ---------------------------------------------------------------------------

/// Serializes as a fixed string whatever it holds, so `T` needs nothing. The
/// shape serde's `PhantomData` exception covers for one type and leaves to
/// `#[serde(bound = "...")]` for every other.
#[derive(Debug)]
struct Redacted<T>(T);

impl<T> Serialize for Redacted<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("<redacted>")
    }
}

/// `#[serde(bound = "")]` is a field-level attribute, which this macro carries
/// into the definition verbatim. serde leaves such a field out of its own
/// inference, so an inference mirroring serde's has to leave it out too — or the
/// escape hatch the user reached for does nothing.
#[suzunari_error(serialize)]
#[suzu(display("authentication failed"))]
struct AuthFailed<T: Debug> {
    #[serde(bound = "")]
    credential: Redacted<T>,
}

#[test]
fn a_field_bound_of_the_users_own_replaces_the_inference() {
    let error = AuthFailedSnafu {
        credential: Redacted(Opaque),
    }
    .build();
    let value = serde_json::to_value::<AuthFailed<Opaque>>(error).unwrap();

    assert_eq!(value["type"], "AuthFailed");
    assert_eq!(value["context"]["credential"], "<redacted>");
}
