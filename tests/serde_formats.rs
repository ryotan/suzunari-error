//! Whether a consumer can read the payload back, format by format.
//!
//! Implementing `Serialize` offers this crate to every serde data format, so the
//! payload has to be decodable in each — not merely writable. Serialization
//! succeeding proves nothing on its own: a format with no field names accepts a
//! variable field set quite happily and produces bytes no reader can walk.
//!
//! The reader here is a set of hand-written mirror types. That is what a
//! consumer would write, and writing them by hand is the point: a schema derived
//! from this crate's own types would only show that it agrees with itself. The
//! Rust types are the single source of truth, and these mirrors are what someone
//! reading the documentation would produce from them.
//!
//! # What each format gets
//!
//! [`Serializer::is_human_readable`] decides which shape a format receives, and
//! every binary format measured here reports `false` — including the two that
//! carry field names. So CBOR and MessagePack receive the uniform shape as well,
//! which costs them a few entries they could have done without. That is the
//! price of a single, safe default; see the note in `__private::payload`.

#![cfg(feature = "serde")]

use serde::{Deserialize, Serialize, Serializer};
use suzunari_error::*;

// ---------------------------------------------------------------------------
// The chain under test: two levels of this crate's own, then a plain io::Error
// ---------------------------------------------------------------------------

#[suzunari_error(serialize)]
#[suzu(display("read failed for {path}"))]
struct ReadError {
    path: String,
    source: std::io::Error,
}

#[suzunari_error(serialize)]
#[suzu(display("fetch failed for {uri}"))]
struct FetchError {
    uri: String,
    source: ReadError,
}

const MISSING_PATH: &str = "/nonexistent-suzunari-error";

fn fetch_error() -> FetchError {
    let read = std::fs::read(MISSING_PATH)
        .context(ReadSnafu {
            path: MISSING_PATH.to_string(),
        })
        .unwrap_err();
    Err::<(), _>(read)
        .context(FetchSnafu { uri: "/foo" })
        .unwrap_err()
}

/// The message an `io::Error` for a missing file carries, without hardcoding the
/// platform's wording.
fn io_message() -> String {
    std::io::Error::from_raw_os_error(2).to_string()
}

// ---------------------------------------------------------------------------
// The mirror types — what a consumer writes
// ---------------------------------------------------------------------------
//
// One per level, because `context` is a different shape at each: that is the
// schema, and a reader needs it whatever the format. Every node carries the same
// five fields, so each mirror differs from the next only in the two types it
// names.

#[derive(Debug, Deserialize, PartialEq)]
struct Loc {
    file: String,
    line: u32,
    column: u32,
}

#[derive(Debug, Deserialize, PartialEq)]
struct FetchContext {
    uri: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ReadContext {
    path: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct FetchErrorNode {
    #[serde(rename = "type")]
    type_name: Option<String>,
    message: String,
    location: Option<Loc>,
    context: Option<FetchContext>,
    source: Option<Box<ReadErrorNode>>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ReadErrorNode {
    #[serde(rename = "type")]
    type_name: Option<String>,
    message: String,
    location: Option<Loc>,
    context: Option<ReadContext>,
    source: Option<Box<TailErrorNode>>,
}

/// The phase 2 tail. `type` and `location` are `None` because a plain `Error`
/// has neither, and `context` because there are no readable fields — none of
/// which the reader has to discover, since the layout is the same as above.
#[derive(Debug, Deserialize, PartialEq)]
struct TailErrorNode {
    #[serde(rename = "type")]
    type_name: Option<String>,
    message: String,
    location: Option<Loc>,
    context: Option<()>,
    source: Option<Box<TailErrorNode>>,
}

/// Asserts everything the payload should carry, at every level.
fn assert_chain(node: &FetchErrorNode) {
    assert_eq!(node.type_name.as_deref(), Some("FetchError"));
    assert_eq!(node.message, "fetch failed for /foo");
    assert_eq!(
        node.context,
        Some(FetchContext {
            uri: "/foo".to_string()
        })
    );
    let location = node.location.as_ref().expect("the outer node has one");
    assert!(location.file.ends_with("serde_formats.rs"));
    assert!(location.line > 0);
    assert!(location.column > 0);

    let read = node.source.as_deref().expect("a cause");
    assert_eq!(read.type_name.as_deref(), Some("ReadError"));
    assert_eq!(read.message, format!("read failed for {MISSING_PATH}"));
    assert_eq!(
        read.context,
        Some(ReadContext {
            path: MISSING_PATH.to_string()
        })
    );
    assert!(read.location.is_some());

    let tail = read.source.as_deref().expect("a cause");
    assert_eq!(tail.type_name, None);
    assert_eq!(tail.message, io_message());
    assert_eq!(tail.location, None);
    assert_eq!(tail.context, None);
    assert_eq!(tail.source, None);
}

// ---------------------------------------------------------------------------
// Which shape each format asks for
// ---------------------------------------------------------------------------

/// Reports what a format answers to `is_human_readable()`, which is what selects
/// the shape.
struct Probe;

impl Serialize for Probe {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let human_readable = serializer.is_human_readable();
        serializer.serialize_bool(human_readable)
    }
}

/// Text is human-readable; every binary format here is not — including the two
/// that carry field names and could have taken the compact shape.
#[test]
fn only_the_text_format_asks_for_the_readable_shape() {
    assert_eq!(serde_json::to_string(&Probe).unwrap(), "true");

    let config = bincode::config::standard();
    assert_eq!(
        bincode::serde::encode_to_vec(Probe, config).unwrap(),
        vec![0]
    );
    assert_eq!(postcard::to_allocvec(&Probe).unwrap(), vec![0]);
    // MessagePack `false`.
    assert_eq!(rmp_serde::to_vec(&Probe).unwrap(), vec![0xC2]);
    let mut cbor = Vec::new();
    ciborium::into_writer(&Probe, &mut cbor).unwrap();
    // CBOR simple value 20, `false`.
    assert_eq!(cbor, vec![0xF4]);
}

// ---------------------------------------------------------------------------
// Round trips
// ---------------------------------------------------------------------------

/// No field names, length-prefixed: the case that cannot work with a variable
/// field set, because a reader advances by type and has to know how many fields
/// to expect before it reads them.
#[test]
fn bincode_round_trips() {
    let config = bincode::config::standard();
    let bytes = bincode::serde::encode_to_vec(fetch_error(), config).unwrap();
    let (node, consumed): (FetchErrorNode, usize) =
        bincode::serde::decode_from_slice(&bytes, config).unwrap();

    // The whole stream is accounted for. A layout the reader guessed wrong would
    // leave bytes over, or run out early.
    assert_eq!(consumed, bytes.len());
    assert_chain(&node);
}

/// The same properties as bincode, in a format built for embedded targets — so
/// the conclusion does not rest on one crate's encoding choices.
#[test]
fn postcard_round_trips() {
    let bytes = postcard::to_allocvec(&fetch_error()).unwrap();
    let node: FetchErrorNode = postcard::from_bytes(&bytes).unwrap();

    assert_chain(&node);
}

/// Carries field names, so it could have read the compact shape — but reports
/// `is_human_readable() == false` and receives the uniform one, which it reads
/// just as well.
#[test]
fn messagepack_round_trips() {
    let bytes = rmp_serde::to_vec(&fetch_error()).unwrap();
    let node: FetchErrorNode = rmp_serde::from_slice(&bytes).unwrap();

    assert_chain(&node);
}

/// The other self-describing binary format, for the same reason.
#[test]
fn cbor_round_trips() {
    let mut bytes = Vec::new();
    ciborium::into_writer(&fetch_error(), &mut bytes).unwrap();
    let node: FetchErrorNode = ciborium::from_reader(&bytes[..]).unwrap();

    assert_chain(&node);
}

// ---------------------------------------------------------------------------
// The readable shape is still the readable shape
// ---------------------------------------------------------------------------

/// JSON keeps the shape it had: keys omitted rather than written as null.
///
/// This is the regression guard for the split. The uniform shape must not reach
/// a format that never needed it — the mirrors above would not even deserialize
/// from this payload, because three of their fields have no key to read.
#[test]
fn json_omits_rather_than_writing_null() {
    let value = serde_json::to_value(fetch_error()).unwrap();

    let tail = &value["source"]["source"];
    assert_eq!(tail["message"], serde_json::json!(io_message()));
    // Absent, not null: the phase 2 tail has no type, location or context, and
    // no cause below it.
    let tail = tail.as_object().expect("an object");
    assert_eq!(tail.keys().collect::<Vec<_>>(), ["message"]);

    // The levels above keep every key they had.
    let outer = value.as_object().expect("an object");
    assert_eq!(
        outer.keys().collect::<Vec<_>>(),
        ["context", "location", "message", "source", "type"]
    );
}
