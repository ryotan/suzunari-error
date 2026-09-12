//! Serialization of error chains through the type-erased `BoxedStackError`.
//!
//! These tests cover the macro-independent half of the serde feature: the node
//! shapes and the chain walk. Because the concrete type is erased here, no node
//! carries `context` — that arrives with `#[suzunari_error(serialize)]`.

#![cfg(feature = "serde")]

use serde_test::Configure;
use suzunari_error::*;

#[suzunari_error]
#[suzu(display("read failed for {path}"))]
struct ReadError {
    path: String,
    source: std::io::Error,
}

#[suzunari_error]
#[suzu(display("fetch failed"))]
struct FetchError {
    source: BoxedStackError,
}

/// The `io::Error` is built rather than read from the filesystem. A real one is
/// worded by the platform — "No such file or directory (os error 2)" on Unix,
/// "The system cannot find the file specified. (os error 2)" on Windows — and
/// the token assertions below compare the tail's message exactly.
fn read_error() -> ReadError {
    Err::<(), _>(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no such file",
    ))
    .context(ReadSnafu {
        path: "/nonexistent-suzunari-error",
    })
    .unwrap_err()
}

/// A `StackError` followed by a plain `Error` tail: phase 1 then phase 2.
#[test]
fn serializes_stack_error_then_plain_error_tail() {
    let boxed = BoxedStackError::new(read_error());
    let value = serde_json::to_value(&boxed).unwrap();

    assert_eq!(value["type"], "ReadError");
    assert_eq!(
        value["message"],
        "read failed for /nonexistent-suzunari-error"
    );
    assert!(
        value["location"]["file"]
            .as_str()
            .unwrap()
            .ends_with("serde_serialize.rs")
    );
    assert!(value["location"]["line"].is_number());
    assert!(value["location"]["column"].is_number());

    // The concrete type is erased by BoxedStackError, so `path` is unreachable.
    assert!(value.get("context").is_none());

    // Phase 2: the io::Error tail carries a message and nothing else.
    let tail = &value["source"];
    assert_eq!(tail["message"], "no such file");
    assert!(tail.get("type").is_none());
    assert!(tail.get("location").is_none());
    assert!(tail.get("source").is_none());
}

/// Two `StackError` levels: the chain must not truncate at the erased boundary.
#[test]
fn serializes_nested_stack_errors() {
    let inner = BoxedStackError::new(read_error());
    let outer = Err::<(), _>(inner).context(FetchSnafu).unwrap_err();
    let value = serde_json::to_value(BoxedStackError::new(outer)).unwrap();

    assert_eq!(value["type"], "FetchError");
    assert_eq!(value["source"]["type"], "ReadError");
    assert!(value["source"]["location"]["line"].is_number());
    assert_eq!(value["source"]["source"]["message"], "no such file");
}

/// An error with no cause omits `source` rather than emitting null.
#[test]
fn omits_source_when_there_is_no_cause() {
    #[suzunari_error]
    #[suzu(display("no cause"))]
    struct LeafError {}

    fn leaf() -> Result<(), LeafError> {
        ensure!(false, LeafSnafu);
        Ok(())
    }

    let value = serde_json::to_value(BoxedStackError::new(leaf().unwrap_err())).unwrap();

    assert_eq!(value["type"], "LeafError");
    assert!(value.get("source").is_none());
    assert!(value.get("context").is_none());

    // The shortest node there is: no `context` because the type is erased, no
    // `source` because there is no cause.
    let boxed = BoxedStackError::new(leaf().unwrap_err());
    let location = boxed.location();
    serde_test::assert_ser_tokens(
        &boxed.readable(),
        &[
            serde_test::Token::Struct {
                name: "TypeErasedStackError",
                len: 3,
            },
            serde_test::Token::Str("type"),
            serde_test::Token::Str("LeafError"),
            serde_test::Token::Str("message"),
            serde_test::Token::Str("no cause"),
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
            serde_test::Token::StructEnd,
        ],
    );
}

/// Struct names and field counts, which a JSON string comparison discards.
///
/// Guards two things JSON cannot show: that the `Location` remote definition
/// does not leak its own identifier (`LocationDef`) into the data model, and
/// that the two node shapes stay distinguishable rather than one reusing the
/// other's struct name for a different field set.
#[test]
fn node_struct_names_and_field_counts() {
    let boxed = BoxedStackError::new(read_error());
    let location = boxed.location();

    serde_test::assert_ser_tokens(
        &boxed.readable(),
        &[
            serde_test::Token::Struct {
                name: "TypeErasedStackError",
                len: 4,
            },
            serde_test::Token::Str("type"),
            serde_test::Token::Str("ReadError"),
            serde_test::Token::Str("message"),
            serde_test::Token::Str("read failed for /nonexistent-suzunari-error"),
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
            serde_test::Token::Str("source"),
            // `Option` contributes a `Some` of its own. JSON discards it, but a
            // non-self-describing format would encode a discriminant here.
            serde_test::Token::Some,
            serde_test::Token::Struct {
                name: "PlainError",
                len: 1,
            },
            serde_test::Token::Str("message"),
            serde_test::Token::Str("no such file"),
            serde_test::Token::StructEnd,
            serde_test::Token::StructEnd,
        ],
    );
}

// ---------------------------------------------------------------------------
// The shape a format without field names gets
// ---------------------------------------------------------------------------

/// Every node emits the same five fields, so a reader that advances by type
/// rather than by name knows the layout before it starts.
///
/// The compact side has to be asserted separately: `assert_ser_tokens` requires
/// a `Configure` marker precisely because the two representations differ, so a
/// test that only marks itself `readable` leaves this one unmeasured.
#[test]
fn every_node_has_the_same_five_fields_without_field_names() {
    let boxed = BoxedStackError::new(read_error());
    let location = boxed.location();

    serde_test::assert_ser_tokens(
        &boxed.compact(),
        &[
            // The erased node. `context` is `None` — the fields exist but
            // cannot be read — and the count is 5 rather than the 4 a
            // self-describing format sees.
            serde_test::Token::Struct {
                name: "UniformError",
                len: 5,
            },
            serde_test::Token::Str("type"),
            serde_test::Token::Some,
            serde_test::Token::Str("ReadError"),
            serde_test::Token::Str("message"),
            serde_test::Token::Str("read failed for /nonexistent-suzunari-error"),
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
            serde_test::Token::None,
            serde_test::Token::Str("source"),
            serde_test::Token::Some,
            // The phase 2 tail, in the same shape: `type` and `location` are
            // `None` because a plain `Error` has neither, and that is what tells
            // a reader the phase changed — no key needs to be missing for it.
            serde_test::Token::Struct {
                name: "UniformError",
                len: 5,
            },
            serde_test::Token::Str("type"),
            serde_test::Token::None,
            serde_test::Token::Str("message"),
            serde_test::Token::Str("no such file"),
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
