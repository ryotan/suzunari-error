#!/usr/bin/env python3
"""Turn the pict output into the pairwise case file.

    python3 tests/support/generate-cases.py tests/support/differential-cases.pict \
        > tests/serde_differential_pairwise.rs

The generated Rust is committed and reviewed; neither pict nor this script is
needed at build time. Editing the level-to-code mapping below changes every case
at once, which is why the hand-written cases in serde_differential.rs are kept as
a separate check on it.

Two rules from the model decide the field layout:

- `FieldType` describes the first declared field. Any further declared field is
  a plain scalar.
- The `source`-named field of `SourceFalseField` is always last and counts
  towards `DeclaredFields`, so "one" plus "present" means it is the only one and
  `FieldType` describes it.
"""
import subprocess
import sys

HEADER = '''//! Pairwise cases, generated from `tests/support/differential-cases.pict`.
//!
//! Do not edit by hand. Regenerate with:
//!
//! ```text
//! python3 tests/support/generate-cases.py tests/support/differential-cases.pict \\
//!     > tests/serde_differential_pairwise.rs
//! ```
//!
//! then review the diff — the committed output is what counts, not the script.
//!
//! Each case pairs an error type with the struct a user would have written for
//! its declared fields, and checks both halves of the payload: `context`
//! against that struct, and `source` against what the source level implies.
//!
//! The hand-written cases in `serde_differential.rs` stay behind on purpose. If
//! `declare_case!` or the generator gets the split between `context` and
//! `metadata` wrong, every case here moves together and still agrees; the
//! hand-written ones do not.

#![cfg(feature = "serde")]

#[macro_use]
mod support;

use std::collections::BTreeMap;
use support::{Record, error_node, record};
use suzunari_error::*;

/// A path that does not exist, so that `read` fails the same way everywhere.
const MISSING_PATH: &str = "/nonexistent-suzunari-error";

/// The message an `io::Error` for a missing file carries, without hardcoding
/// the platform's wording.
fn io_message() -> String {
    std::io::Error::from_raw_os_error(2).to_string()
}

/// Whether the payload carries a `source` key at all.
fn has_source(recorded: &Record) -> bool {
    let Record::Struct { fields, .. } = recorded else {
        panic!("expected a struct record");
    };
    fields.iter().any(|(name, _)| *name == "source")
}

// --- types used as declared fields -----------------------------------------

/// A struct of the user's own, derived plainly.
#[derive(Debug, serde::Serialize)]
pub struct Detail {
    pub code: u32,
}

/// The same, but carrying a container attribute. It has to reach the payload
/// untouched: the definition names the type, and the type's own impl runs.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Renamed {
    pub field_name: &'static str,
}

/// A field that serializes as a map rather than as a struct.
fn pairs() -> BTreeMap<&'static str, u32> {
    BTreeMap::from([("a", 1), ("b", 2)])
}

// --- types used as the source ----------------------------------------------

/// A source that is itself opted in, so its own `context` survives nesting.
#[suzunari_error(serialize)]
#[suzu(display("inner failed"))]
struct InnerError {
    detail: &'static str,
}

fn inner_error() -> InnerError {
    fn failing() -> Result<(), InnerError> {
        ensure!(false, InnerSnafu { detail: "d" });
        Ok(())
    }
    failing().unwrap_err()
}

/// A foreign error that happens to derive `Serialize`. Dispatch must not let
/// its own fields replace the node.
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

/// A type with no `Error` impl, reached through `#[suzu(from)]`.
#[derive(Debug)]
struct LibError;

impl std::fmt::Display for LibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("lib error")
    }
}

// --- helpers named by transplanted attributes ------------------------------
//
// Referenced by absolute path so they resolve the same from the error type and
// from the hand-written struct, which sit in different modules.

/// Always true, so `skip_serializing_if` takes the skipping branch. That is the
/// branch where a definition announcing a field it never writes would show.
pub fn always_skip<T>(_: &T) -> bool {
    true
}

/// Serializes any field through its `Debug`, so one function covers every
/// `FieldType` level.
pub fn debug_string<T: std::fmt::Debug, S: serde::Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&format_args!("{value:?}"))
}
'''

# (rust type, expression). One expression fills both the snafu selector and the
# hand-written struct, so the two sides start from the same value.
FIELD_TYPES = {
    "scalar": ("&'static str", '"l"'),
    "option": ("Option<u32>", "Some(7u32)"),
    "serialize_struct": ("Detail", "Detail { code: 3 }"),
    "container_attributed_struct": ("Renamed", 'Renamed { field_name: "x" }'),
    "map_like": ("BTreeMap<&'static str, u32>", "pairs()"),
    "generic_param": ("T", "9u32"),
    "borrowed": ("&'a str", '"b"'),
}

# The parameters on the error type, and the instantiation the test uses. A
# parameter no field uses does not compile, which is why the model ties these to
# the field type both ways.
GENERICS = {
    "none": ("", ""),
    "type_param": ("<T: ::core::fmt::Debug>", "<u32>"),
    "lifetime": ("<'a>", "<'static>"),
}

# `bound` replaces the bound serde would have inferred, so what it says depends
# on which parameter there is.
BOUNDS = {
    "type_param": "T: serde::Serialize",
    "lifetime": "&'a str: serde::Serialize",
}

SOURCE_TYPES = {
    "io_error": "std::io::Error",
    "serialize_type": "InnerError",
    "boxed": "BoxedStackError",
    "foreign_serialize": "ForeignError",
    "display_error": "LibError",
}

# The attribute placed on the first declared field. It reaches both the error
# type and the hand-written struct: on the latter it is plain serde behaviour,
# which is what the comparison is against.
FIELD_ATTRS = {
    "none": "",
    "rename": '#[serde(rename = "renamed")]',
    "skip": "#[serde(skip)]",
    "skip_serializing_if": '#[serde(skip_serializing_if = "crate::always_skip")]',
    "serialize_with": '#[serde(serialize_with = "crate::debug_string")]',
    "flatten": "#[serde(flatten)]",
    "deser_only": '#[serde(alias = "other")]',
    # Filled in per case: see `field_attr`.
    "bound": None,
}


def field_attr(levels):
    """The attribute for the first declared field."""
    if levels["FieldAttr"] == "bound":
        return f'#[serde(bound(serialize = "{BOUNDS[levels["Generics"]]}"))]'
    return FIELD_ATTRS[levels["FieldAttr"]]

LOCATION_FIELD = {
    "injected": "",
    "suzu_named": "#[suzu(location)] at: Location,",
    "typed": "at: Location,",
    "stack_attr": "#[stack(location)] at: Location,",
}


def declared_fields(levels):
    """The declared fields in order.

    Each is (shared attributes, error-only attributes, name, type, expression).
    """
    if levels["DeclaredFields"] == "zero":
        return []

    ty, value = FIELD_TYPES[levels["FieldType"]]
    source_false = levels["SourceFalseField"] == "present"
    attr = field_attr(levels)

    if levels["DeclaredFields"] == "one":
        # With the source-named field present it is the only one, so FieldType
        # and FieldAttr describe it.
        name = "source" if source_false else "label"
        suzu = "#[suzu(source(false))]" if source_false else ""
        return [(attr, suzu, name, ty, value)]

    first = (attr, "", "label", ty, value)
    if source_false:
        last = ("", "#[suzu(source(false))]", "source", "&'static str", '"not-an-error"')
    else:
        last = ("", "", "filler", "u32", "1u32")
    return [first, last]


def source_field(levels):
    """The source field's tokens, or empty when there is no source."""
    source = levels["Source"]
    if source == "none":
        return ""

    ty = SOURCE_TYPES[source]
    named = levels["SourceBinding"] == "named_source"
    if source == "display_error":
        # `from` generates the source conversion itself, and conflicts with a
        # written `source`, so the attribute does the binding either way.
        return f"#[suzu(from)] {'source' if named else 'cause'}: {ty},"
    if named:
        return f"source: {ty},"
    return f"#[suzu(source)] cause: {ty},"


def selector(ty_name, fields):
    if not fields:
        return f"{ty_name}Snafu"
    args = ", ".join(f"{name}: {value}" for _, _, name, _, value in fields)
    return f"{ty_name}Snafu {{ {args} }}"


def body(levels, ty_name, sel):
    ty_name = ty_name + GENERICS[levels["Generics"]][1]
    source = levels["Source"]
    if source == "none":
        return f'''        fn failing() -> Result<(), {ty_name}> {{
            ensure!(false, {sel});
            Ok(())
        }}
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));'''
    if source == "io_error":
        return f'''        let error = std::fs::read(MISSING_PATH).context({sel}).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );'''
    if source in ("serialize_type", "boxed"):
        cause = (
            "inner_error()"
            if source == "serialize_type"
            else "BoxedStackError::new(inner_error())"
        )
        node = "StackErrorNode" if source == "serialize_type" else "BoxedStackErrorNode"
        return f'''        let cause = {cause};
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context({sel}).unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {{ name: "{node}", .. }}
        ));'''
    if source == "foreign_serialize":
        return f'''        let cause = ForeignError {{ code: 7 }};
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context({sel}).unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(recorded.field("source").some(), &error_node("foreign error 7"));
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);'''
    if source == "display_error":
        return f'''        let error = Err::<(), _>(LibError).context({sel}).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(record(&error).field("source").some(), &error_node("lib error"));'''
    raise ValueError(source)


def main(model):
    rows = (
        subprocess.run(["pict", model], capture_output=True, text=True, check=True)
        .stdout.strip()
        .splitlines()
    )
    header = rows[0].split("\t")
    out = [HEADER]

    for index, row in enumerate(rows[1:], start=1):
        levels = dict(zip(header, row.split("\t")))
        ty_name = f"Case{index:02}"
        generics, concrete = GENERICS[levels["Generics"]]
        fields = declared_fields(levels)
        context = ", ".join(
            f"[{shared}] [{error_only}] {name}: {ty} = {value}"
            for shared, error_only, name, ty, value in fields
        )
        metadata = " ".join(
            part
            for part in (source_field(levels), LOCATION_FIELD[levels["Location"]])
            if part
        )
        summary = ", ".join(f"{k}={v}" for k, v in levels.items() if v != "na")

        out.append(f'''
/// {summary}
mod case_{index:02} {{
    use super::*;

    declare_case! {{
        error: {ty_name},
        generics: {{ {generics} }},
        concrete: {{ {concrete} }},
        display: "case {index:02}",
        context: {{ {context} }},
        metadata: {{ {metadata} }},
    }}

    #[test]
    fn matches_the_hand_written_equivalent() {{
{body(levels, ty_name, selector(ty_name, fields))}
    }}
}}''')

    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.stdout.write(main(sys.argv[1]))
