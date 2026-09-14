# Reading the payload

The payload is a chain of **error nodes**, one per level of the error chain. Each
carries `type`, `message`, `location`, `context` and `source`; `source` holds the
next one, and the last has none.

`context` holds the type's **declared fields**: the fields it declares itself,
other than the one holding the source and the one holding the location.

This crate implements `Serialize` and not `Deserialize`. The payload is data to
read, not an error to rebuild: `context` is a different shape at every level, and
an `io::Error` at the end of a chain cannot be reconstructed from the message it
left behind.

A consumer therefore defines its own data structures and reads into those. What
follows is what those look like, and how to check a payload against a schema.

## What each format receives

The payload takes one of two shapes, chosen by
[`Serializer::is_human_readable`](serde::Serializer::is_human_readable).

| Shape | Fields | Formats |
|---|---|---|
| Sparse | Keys are omitted where they do not apply | JSON, and other text formats |
| Uniform | Every error node carries the same five keys, absent parts as `null` | Every binary format |

The sparse shape tells its three kinds of error node apart by which keys are
present: a cause that does not implement `StackError` has no `type`, and nor does
anything below it, since only `Error::source()` is left to follow. An error node
whose concrete type was erased has no `context`. That only works where the format
carries field names.

The uniform shape exists for formats where it does not. A reader of a fixed
layout advances by type and has to know how many fields to expect before it reads
them, so nothing may be omitted; `null` says what an absent key said. Every binary
format reports `is_human_readable() == false`, including the self-describing ones,
which therefore receive the uniform shape as well.

## Data structures

One per level, because `context` differs at each. Every error node carries the
same five fields in the uniform shape, so the structures differ only in the two
types they name.

```
# #[cfg(feature = "serde")]
# fn demo() -> Result<(), Box<dyn std::error::Error>> {
# use suzunari_error::*;
# #[suzunari_error(serialize)]
# #[suzu(display("read failed for {path}"))]
# struct ReadError { path: String, source: std::io::Error }
# #[suzunari_error(serialize)]
# #[suzu(display("fetch failed for {uri}"))]
# struct FetchError { uri: String, source: ReadError }
# let read = std::fs::read("/nonexistent-suzunari-error")
#     .context(ReadSnafu { path: "/etc/hosts".to_string() }).unwrap_err();
# let error = Err::<(), _>(read).context(FetchSnafu { uri: "/foo".to_string() }).unwrap_err();
# let postcard_bytes = postcard::to_allocvec(&error)?;
# let bincode_bytes = bincode::serde::encode_to_vec(&error, bincode::config::standard())?;
# let msgpack_bytes = rmp_serde::to_vec(&error)?;
# let mut cbor_bytes = Vec::new();
# ciborium::into_writer(&error, &mut cbor_bytes)?;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Location {
    file: String,
    line: u32,
    column: u32,
}

#[derive(Debug, Deserialize, PartialEq)]
struct FetchErrorNode {
    #[serde(rename = "type")]
    type_name: Option<String>,
    message: String,
    location: Option<Location>,
    context: Option<FetchContext>,
    source: Option<Box<ReadErrorNode>>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct FetchContext {
    uri: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ReadErrorNode {
    #[serde(rename = "type")]
    type_name: Option<String>,
    message: String,
    location: Option<Location>,
    context: Option<ReadContext>,
    source: Option<Box<PlainErrorNode>>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ReadContext {
    path: String,
}

/// A cause that does not implement `StackError`. `type`, `location` and
/// `context` are `None` because a plain `Error` has none of them.
#[derive(Debug, Deserialize, PartialEq)]
struct PlainErrorNode {
    #[serde(rename = "type")]
    type_name: Option<String>,
    message: String,
    location: Option<Location>,
    context: Option<()>,
    source: Option<Box<PlainErrorNode>>,
}

// Only the call differs. The data structures above are the same for every
// format, and so is what comes out of them.
let from_postcard: FetchErrorNode = postcard::from_bytes(&postcard_bytes)?;

let config = bincode::config::standard();
let (from_bincode, _): (FetchErrorNode, usize) =
    bincode::serde::decode_from_slice(&bincode_bytes, config)?;

let from_messagepack: FetchErrorNode = rmp_serde::from_slice(&msgpack_bytes)?;

let from_cbor: FetchErrorNode = ciborium::from_reader(&cbor_bytes[..])?;

assert_eq!(from_postcard.type_name.as_deref(), Some("FetchError"));
assert_eq!(
    from_postcard.context.as_ref().map(|c| c.uri.as_str()),
    Some("/foo")
);
assert_eq!(from_postcard, from_bincode);
assert_eq!(from_postcard, from_messagepack);
assert_eq!(from_postcard, from_cbor);
# Ok(())
# }
# #[cfg(feature = "serde")]
# demo().unwrap();
```


## JSON Schema — the sparse shape

For a consumer reading JSON, in any language. The three kinds of error node appear
as a `oneOf`, since which keys are present is what tells them apart.

```
# #[cfg(feature = "serde")]
# fn demo() -> Result<(), Box<dyn std::error::Error>> {
# use suzunari_error::*;
# #[suzunari_error(serialize)]
# #[suzu(display("read failed for {path}"))]
# struct ReadError { path: String, source: std::io::Error }
# let error = std::fs::read("/nonexistent-suzunari-error")
#     .context(ReadSnafu { path: "/etc/hosts".to_string() }).unwrap_err();
let schema = serde_json::json!({
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "$ref": "#/$defs/node",
    "$defs": {
        "location": {
            "type": "object",
            "properties": {
                "file": { "type": "string" },
                "line": { "type": "integer", "minimum": 0 },
                "column": { "type": "integer", "minimum": 0 }
            },
            "required": ["file", "line", "column"],
            "additionalProperties": false
        },
        "node": {
            "oneOf": [
                { "$ref": "#/$defs/stack" },
                { "$ref": "#/$defs/erased" },
                { "$ref": "#/$defs/plain" }
            ]
        },
        "stack": {
            "type": "object",
            "properties": {
                "type": { "type": "string" },
                "message": { "type": "string" },
                "location": { "$ref": "#/$defs/location" },
                "context": { "type": "object" },
                "source": { "$ref": "#/$defs/node" }
            },
            "required": ["type", "message", "location", "context"],
            "additionalProperties": false
        },
        "erased": {
            "type": "object",
            "properties": {
                "type": { "type": "string" },
                "message": { "type": "string" },
                "location": { "$ref": "#/$defs/location" },
                "source": { "$ref": "#/$defs/node" }
            },
            "required": ["type", "message", "location"],
            "additionalProperties": false
        },
        "plain": {
            "type": "object",
            "properties": {
                "message": { "type": "string" },
                "source": { "$ref": "#/$defs/node" }
            },
            "required": ["message"],
            "additionalProperties": false
        }
    }
});

let payload = serde_json::to_value(&error)?;
let validator = jsonschema::validator_for(&schema)?;
assert!(validator.is_valid(&payload));
# Ok(())
# }
# #[cfg(feature = "serde")]
# demo().unwrap();
```

## CDDL — the uniform shape

For a consumer reading CBOR. One rule covers every error node, because every error
node has the same five entries.

The `=> ` spelling is deliberate: the `cddl` crate misreads a nested map written
with the `key: value` shorthand.

```
# #[cfg(feature = "serde")]
# fn demo() -> Result<(), Box<dyn std::error::Error>> {
# use suzunari_error::*;
# #[suzunari_error(serialize)]
# #[suzu(display("read failed for {path}"))]
# struct ReadError { path: String, source: std::io::Error }
# let error = std::fs::read("/nonexistent-suzunari-error")
#     .context(ReadSnafu { path: "/etc/hosts".to_string() }).unwrap_err();
let schema = r#"
node = {
  "type"     => tstr / null,
  "message"  => tstr,
  "location" => location / null,
  "context"  => { * tstr => any } / null,
  "source"   => node / null,
}

location = {
  "file"   => tstr,
  "line"   => uint,
  "column" => uint,
}
"#;

let mut payload = Vec::new();
ciborium::into_writer(&error, &mut payload)?;
cddl::validate_cbor_from_slice(schema, &payload, None)?;
# Ok(())
# }
# #[cfg(feature = "serde")]
# demo().unwrap();
```
