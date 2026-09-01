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
//! The payload is checked whole rather than field by field: the key set at each
//! level, every value, and the absence of `source`. A bound fixed at the cost of
//! a field quietly leaving the payload would otherwise pass.
//!
//! `.build()` is used because these fixtures have no source field: there is no
//! `Result` to attach a context selector to.

#![cfg(feature = "serde")]

use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use serde::{Serialize, Serializer};
use serde_json::{Value, json};
use suzunari_error::*;

/// `Debug`, which `#[suzunari_error]` requires, but deliberately not
/// `Serialize`. Standing in for any ordinary type a caller might parameterise an
/// error with.
#[derive(Debug)]
struct Opaque;

/// The key set of a JSON object, so a level can be checked as a whole rather
/// than one probe at a time.
///
/// A set, not a sequence: `serde_json` holds an object's keys in alphabetical
/// order, so the payload's own field order is not observable here. That order is
/// what the recording serializer in the differential suite exists to check.
fn key_set(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("expected an object, got {value}"))
        .keys()
        .map(String::as_str)
        .collect()
}

/// Asserts the envelope around `context`: a node with no cause carries exactly
/// `type`, `message`, `location` and `context`, and `location` points into this
/// file.
fn assert_envelope(value: &Value, type_name: &str, message: &str) {
    assert_eq!(key_set(value), ["context", "location", "message", "type"]);
    assert_eq!(value["type"], json!(type_name));
    assert_eq!(value["message"], json!(message));

    assert_eq!(key_set(&value["location"]), ["column", "file", "line"]);
    let file = value["location"]["file"]
        .as_str()
        .expect("file is a string");
    assert!(
        file.ends_with("serde_bound_inference.rs"),
        "location points at {file}"
    );
    assert!(value["location"]["line"].as_u64().is_some_and(|n| n > 0));
    assert!(value["location"]["column"].as_u64().is_some_and(|n| n > 0));

    // Absence matters as much as presence: `source` is omitted, not null.
    assert!(!value.as_object().expect("an object").contains_key("source"));
}

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
#[suzu(display("no entry for {key}"))]
struct NotFound<S>
where
    S: Store + Debug,
    S::Key: Debug + Display + Serialize,
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

    assert_envelope(&value, "NotFound", "no entry for k1");
    assert_eq!(value["context"], json!({ "key": "k1" }));
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

    assert_envelope(&value, "Tagged", "tagged failure");
    // The marker still reaches the payload — serialized as unit, which is null
    // in JSON. Dropping the bound must not drop the field with it.
    assert_eq!(
        value["context"],
        json!({ "marker": null, "detail": "boom" })
    );
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
    reason: &'static str,
}

#[test]
fn a_serialize_with_field_bounds_nothing() {
    let error = RejectedSnafu {
        value: Opaque,
        reason: "unsupported",
    }
    .build();
    let value = serde_json::to_value::<Rejected<Opaque>>(error).unwrap();

    assert_envelope(&value, "Rejected", "rejected the value");
    // "Opaque" rather than an object: the attribute's function ran, so dropping
    // the bound did not quietly drop the attribute along with it.
    assert_eq!(
        value["context"],
        json!({ "value": "Opaque", "reason": "unsupported" })
    );
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
    attempts: u32,
}

#[test]
fn a_field_bound_of_the_users_own_replaces_the_inference() {
    let error = AuthFailedSnafu {
        credential: Redacted(Opaque),
        attempts: 3u32,
    }
    .build();
    let value = serde_json::to_value::<AuthFailed<Opaque>>(error).unwrap();

    assert_envelope(&value, "AuthFailed", "authentication failed");
    assert_eq!(
        value["context"],
        json!({ "credential": "<redacted>", "attempts": 3 })
    );
}

// ---------------------------------------------------------------------------
// A parameter no declared field uses
// ---------------------------------------------------------------------------

/// The parameter is the source's, and the source is skipped in the definition.
/// Bounding it would reject `io::Error`, which is what this instantiates.
#[suzunari_error(serialize)]
#[suzu(display("wrapping {label}"))]
struct Wrapping<T: Debug + Display, U>
where
    U: core::error::Error + Debug + 'static,
{
    label: T,
    source: U,
}

#[test]
fn a_parameter_only_the_source_uses_is_not_bounded() {
    let error: Wrapping<u32, std::io::Error> = std::fs::read("/nonexistent-suzunari-error")
        .context(WrappingSnafu { label: 7u32 })
        .unwrap_err();
    let value = serde_json::to_value(&error).unwrap();

    // A cause is present here, so the envelope carries `source` too.
    assert_eq!(
        key_set(&value),
        ["context", "location", "message", "source", "type"]
    );
    assert_eq!(value["type"], json!("Wrapping"));
    assert_eq!(value["message"], json!("wrapping 7"));
    assert_eq!(value["context"], json!({ "label": 7 }));

    // The source is not one of ours, so it continues the chain with `message`
    // alone rather than contributing its own fields.
    assert_eq!(key_set(&value["source"]), ["message"]);
    assert_eq!(
        value["source"]["message"],
        json!(std::io::Error::from_raw_os_error(2).to_string())
    );
}
