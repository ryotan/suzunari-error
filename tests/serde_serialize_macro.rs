//! `#[suzunari_error(serialize)]` — the opt-in that makes an error type's own
//! declared fields reachable as `context`.
//!
//! Structs only for now; enums, attribute transplanting and `rename_all` follow.

#![cfg(feature = "serde")]

use serde_test::Configure;
use suzunari_error::*;

#[suzunari_error(serialize)]
#[suzu(display("lookup failed for {key}"))]
struct LookupError {
    key: String,
    attempts: u32,
    source: std::io::Error,
}

#[suzunari_error(serialize)]
#[suzu(display("request failed"))]
struct RequestError {
    endpoint: String,
    source: LookupError,
}

fn lookup_error() -> LookupError {
    std::fs::read("/nonexistent-suzunari-error")
        .context(LookupSnafu {
            key: "k",
            attempts: 3u32,
        })
        .unwrap_err()
}

/// The opted-in type carries its declared fields under `context`, and the
/// non-`Serialize` `source` field stays out of them.
#[test]
fn declared_fields_appear_under_context() {
    let value = serde_json::to_value(lookup_error()).unwrap();

    assert_eq!(value["type"], "LookupError");
    assert_eq!(value["message"], "lookup failed for k");
    assert!(value["location"]["line"].is_number());

    assert_eq!(value["context"]["key"], "k");
    assert_eq!(value["context"]["attempts"], 3);
    // `source` is hoisted to the metadata level; `location` is renamed there.
    // Neither belongs among the declared fields.
    assert!(value["context"].get("source").is_none());
    assert!(value["context"].get("location").is_none());

    // A non-Serialize source still serializes, via the fallback branch.
    assert!(
        value["source"]["message"]
            .as_str()
            .unwrap()
            .contains("os error 2")
    );
    assert!(value["source"].get("context").is_none());
}

/// The `context` object must not carry the generated definition's own
/// identifier as its struct name. JSON discards struct names, so this leak is
/// only visible at the `Token` level.
#[test]
fn context_object_does_not_leak_the_definition_name() {
    let error = lookup_error();
    let location = error.location();

    serde_test::assert_ser_tokens(
        &error.readable(),
        &[
            serde_test::Token::Struct {
                name: "StackErrorNode",
                len: 5,
            },
            serde_test::Token::Str("type"),
            serde_test::Token::Str("LookupError"),
            serde_test::Token::Str("message"),
            serde_test::Token::Str("lookup failed for k"),
            serde_test::Token::Str("location"),
            serde_test::Token::Struct {
                name: "Location",
                len: 3,
            },
            serde_test::Token::Str("file"),
            serde_test::Token::Str(location.file()),
            serde_test::Token::Str("line"),
            serde_test::Token::U32(location.line()),
            serde_test::Token::Str("column"),
            serde_test::Token::U32(location.column()),
            serde_test::Token::StructEnd,
            serde_test::Token::Str("context"),
            // Not `__SuzuContextDef`.
            serde_test::Token::Struct {
                name: "LookupError",
                len: 2,
            },
            serde_test::Token::Str("key"),
            serde_test::Token::Str("k"),
            serde_test::Token::Str("attempts"),
            serde_test::Token::U32(3),
            serde_test::Token::StructEnd,
            serde_test::Token::Str("source"),
            serde_test::Token::Some,
            serde_test::Token::Struct {
                name: "ErrorNode",
                len: 1,
            },
            serde_test::Token::Str("message"),
            serde_test::Token::Str("No such file or directory (os error 2)"),
            serde_test::Token::StructEnd,
            serde_test::Token::StructEnd,
        ],
    );
}

/// A source that is itself an opted-in type keeps its own `context`. This is
/// what the type-erased walk cannot do, and the reason dispatch is specialized.
#[test]
fn nested_serialize_source_keeps_its_context() {
    let value = serde_json::to_value(
        Err::<(), _>(lookup_error())
            .context(RequestSnafu { endpoint: "/v1" })
            .unwrap_err(),
    )
    .unwrap();

    assert_eq!(value["type"], "RequestError");
    assert_eq!(value["context"]["endpoint"], "/v1");

    let inner = &value["source"];
    assert_eq!(inner["type"], "LookupError");
    assert_eq!(inner["context"]["key"], "k");
    assert_eq!(inner["context"]["attempts"], 3);
    assert!(inner["location"]["line"].is_number());
    assert!(
        inner["source"]["message"]
            .as_str()
            .unwrap()
            .contains("os error 2")
    );
}

/// A foreign error that happens to derive `Serialize` must not be inlined raw:
/// dispatch keys on the crate's marker, not on `Serialize`.
#[test]
fn foreign_serialize_source_is_not_inlined() {
    #[derive(Debug, serde::Serialize)]
    struct ForeignError {
        code: u32,
    }

    impl std::fmt::Display for ForeignError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "foreign error {}", self.code)
        }
    }

    impl std::error::Error for ForeignError {}

    #[suzunari_error(serialize)]
    #[suzu(display("wrapper"))]
    struct WrapperError {
        source: ForeignError,
    }

    fn wrap() -> Result<(), WrapperError> {
        Err::<(), _>(ForeignError { code: 7 }).context(WrapperSnafu)?;
        Ok(())
    }

    let value = serde_json::to_value(wrap().unwrap_err()).unwrap();

    // Not `{"code": 7}` — the node keeps its diagnostic shape.
    assert_eq!(value["source"]["message"], "foreign error 7");
    assert!(value["source"].get("code").is_none());
}

/// A struct that declares no fields of its own emits an empty `context`, not an
/// absent one.
///
/// Absence is reserved for a node whose concrete type was erased, where the
/// fields exist but are unreachable. `type` cannot carry that distinction: an
/// erased node still has one, because `BoxedStackError` forwards `type_name()`.
#[test]
fn empty_context_is_distinct_from_an_erased_one() {
    #[suzunari_error(serialize)]
    #[suzu(display("bare"))]
    struct BareError {}

    fn bare() -> Result<(), BareError> {
        ensure!(false, BareSnafu);
        Ok(())
    }

    let declared = serde_json::to_value(bare().unwrap_err()).unwrap();
    assert_eq!(declared["type"], "BareError");
    assert!(declared["context"].as_object().unwrap().is_empty());
    assert!(declared.get("source").is_none());

    // The empty context still names the error type, not the definition: it goes
    // through the same generated definition as a populated one.
    let error = bare().unwrap_err();
    let location = error.location();
    serde_test::assert_ser_tokens(
        &error.readable(),
        &[
            serde_test::Token::Struct {
                name: "StackErrorNode",
                len: 4,
            },
            serde_test::Token::Str("type"),
            serde_test::Token::Str("BareError"),
            serde_test::Token::Str("message"),
            serde_test::Token::Str("bare"),
            serde_test::Token::Str("location"),
            serde_test::Token::Struct {
                name: "Location",
                len: 3,
            },
            serde_test::Token::Str("file"),
            serde_test::Token::Str(location.file()),
            serde_test::Token::Str("line"),
            serde_test::Token::U32(location.line()),
            serde_test::Token::Str("column"),
            serde_test::Token::U32(location.column()),
            serde_test::Token::StructEnd,
            serde_test::Token::Str("context"),
            serde_test::Token::Struct {
                name: "BareError",
                len: 0,
            },
            serde_test::Token::StructEnd,
            serde_test::Token::StructEnd,
        ],
    );

    // Same error, reached through a type-erased boundary.
    let erased = serde_json::to_value(BoxedStackError::new(bare().unwrap_err())).unwrap();
    assert_eq!(erased["type"], "BareError");
    assert!(erased.get("context").is_none());
}

/// The declared fields still reach `context` where the format has no field
/// names — as `Some`, alongside the `None`s that stand in for what an erased or
/// phase 2 node lacks.
///
/// Asserted separately from the readable side: `assert_ser_tokens` demands a
/// `Configure` marker because the representations differ, so marking the other
/// tests `readable` would otherwise leave this shape unmeasured.
#[test]
fn declared_fields_reach_context_without_field_names() {
    let error = lookup_error();
    let location = error.location();

    serde_test::assert_ser_tokens(
        &error.compact(),
        &[
            serde_test::Token::Struct {
                name: "StackErrorNode",
                len: 5,
            },
            serde_test::Token::Str("type"),
            serde_test::Token::Some,
            serde_test::Token::Str("LookupError"),
            serde_test::Token::Str("message"),
            serde_test::Token::Str("lookup failed for k"),
            serde_test::Token::Str("location"),
            serde_test::Token::Some,
            serde_test::Token::Struct {
                name: "Location",
                len: 3,
            },
            serde_test::Token::Str("file"),
            serde_test::Token::Str(location.file()),
            serde_test::Token::Str("line"),
            serde_test::Token::U32(location.line()),
            serde_test::Token::Str("column"),
            serde_test::Token::U32(location.column()),
            serde_test::Token::StructEnd,
            serde_test::Token::Str("context"),
            serde_test::Token::Some,
            // Still the error type's own name, not the definition's.
            serde_test::Token::Struct {
                name: "LookupError",
                len: 2,
            },
            serde_test::Token::Str("key"),
            serde_test::Token::Str("k"),
            serde_test::Token::Str("attempts"),
            serde_test::Token::U32(3),
            serde_test::Token::StructEnd,
            serde_test::Token::Str("source"),
            serde_test::Token::Some,
            // The phase 2 tail: `type` and `location` absent as values, not as
            // keys, so the field count matches every other node.
            serde_test::Token::Struct {
                name: "StackErrorNode",
                len: 5,
            },
            serde_test::Token::Str("type"),
            serde_test::Token::None,
            serde_test::Token::Str("message"),
            serde_test::Token::Str("No such file or directory (os error 2)"),
            serde_test::Token::Str("location"),
            serde_test::Token::None,
            serde_test::Token::Str("context"),
            serde_test::Token::None,
            serde_test::Token::Str("source"),
            serde_test::Token::None,
            serde_test::Token::StructEnd,
            serde_test::Token::StructEnd,
        ],
    );
}
