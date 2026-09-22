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

#![cfg(all(feature = "serde", feature = "std"))]

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

/// Supplies an associated type, so that a field can be typed `T::Key`.
pub trait Keyed {
    type Key;
}

/// What `assoc_param` is instantiated with. Deliberately **not** `Serialize`: a
/// field typed `T::Key` needs `T::Key: Serialize` and says nothing about `T`, so
/// an inference bounding the parameter instead would turn this marker away.
#[derive(Debug)]
pub struct Marker;

impl Keyed for Marker {
    type Key = &'static str;
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

/// An opted-in enum used as a source, so the variant's own `context` has to
/// survive nesting just as a struct's does.
#[suzunari_error(serialize)]
enum InnerEnumError {
    #[suzu(display("inner enum failed"))]
    Failed { detail: &'static str },
}

fn inner_enum_error() -> InnerEnumError {
    fn failing() -> Result<(), InnerEnumError> {
        ensure!(false, FailedSnafu { detail: "d" });
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
    "wrapped_param": ("Option<T>", "Some(9u32)"),
    "borrowed": ("&'a str", '"b"'),
    "assoc_type": ("T::Key", '"k"'),
}

# (declaration, where predicate, instantiation) for the parameter a declared
# field uses. Lifetimes are declared first, as Rust requires.
#
# `assoc_param` is the only level needing a where predicate. `derive(Debug)`
# bounds the parameter and never the associated type, so `T::Key: Debug` has to
# be written out — for `Serialize` nothing has to be, since both the definition's
# derive and this crate's inference work it out.
DECLARED_PARAMS = {
    "none": (None, None, None),
    "type_param": ("T: ::core::fmt::Debug", None, "u32"),
    "lifetime": ("'a", None, "'static"),
    "assoc_param": (
        "T: Keyed + ::core::fmt::Debug",
        "T::Key: ::core::fmt::Debug",
        "Marker",
    ),
}

# The parameter the source field uses. Instantiated with `io::Error`, which is
# not `Serialize`: bounding it would reject the commonest source there is.
SOURCE_PARAM = ("U: ::core::error::Error + ::core::fmt::Debug + 'static", "std::io::Error")


def generics(levels, with_source):
    """A parameter list and the instantiation the test uses.

    `with_source` builds the error type's, which carries the source's parameter
    as well; without it, the hand-written struct's, which declares only the
    fields and so would not compile with a parameter it never names.
    """
    declaration, predicate, concrete = DECLARED_PARAMS[levels["DeclaredParam"]]
    params = [p for p in (declaration,) if p]
    args = [a for a in (concrete,) if a]
    if with_source and levels["Source"] == "generic_source":
        # Declared after the lifetime, if there is one: Rust requires that order.
        params.append(SOURCE_PARAM[0])
        args.append(SOURCE_PARAM[1])
    if not params:
        return "", ""
    where = f" where {predicate}" if predicate else ""
    return f"<{', '.join(params)}>{where}", f"<{', '.join(args)}>"


# `bound` replaces the bound serde would have inferred, so what it says depends
# on which parameter the declared field uses.
BOUNDS = {
    "type_param": "T: serde::Serialize",
    "lifetime": "&'a str: serde::Serialize",
    "assoc_param": "T::Key: serde::Serialize",
}

# UPPERCASE rather than camelCase: the generated field names are single lowercase
# words, and camelCase would leave every one of them untouched.
RENAME_CASE = "UPPERCASE"


def options(levels):
    """What goes inside `#[suzunari_error(...)]`, and the oracle's counterpart."""
    if levels["RenameAll"] == "absent":
        return "serialize", ""
    return (
        f'serialize(rename_all = "{RENAME_CASE}")',
        f'#[serde(rename_all = "{RENAME_CASE}")]',
    )

SOURCE_TYPES = {
    "io_error": "std::io::Error",
    "serialize_type": "InnerError",
    "serialize_enum": "InnerEnumError",
    "boxed": "BoxedStackError",
    "foreign_serialize": "ForeignError",
    "display_error": "LibError",
    "generic_source": "U",
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
        return f'#[serde(bound(serialize = "{BOUNDS[levels["DeclaredParam"]]}"))]'
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


def selector(levels, ty_name, fields):
    """The snafu context selector. For an enum it is named after the variant."""
    base = "UnderSnafu" if levels["TypeShape"] != "struct" else f"{ty_name}Snafu"
    if not fields:
        return base
    args = ", ".join(f"{name}: {value}" for _, _, name, _, value in fields)
    return f"{base} {{ {args} }}"


def body(levels, ty_name, sel):
    ty_name = ty_name + generics(levels, True)[1]
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
    if source in ("serialize_type", "serialize_enum", "boxed"):
        cause = {
            "serialize_type": "inner_error()",
            "serialize_enum": "inner_enum_error()",
            "boxed": "BoxedStackError::new(inner_error())",
        }[source]
        node = "TypeErasedStackError" if source == "boxed" else "StackError"
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
    if source == "generic_source":
        # The parameter is instantiated with `io::Error`, which is not
        # `Serialize`. That the case compiles at all is half the assertion: a
        # bound on the source's parameter would have rejected it.
        return f'''        let error = std::fs::read(MISSING_PATH).context({sel}).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );'''
    raise ValueError(source)


# The extra variant added when the enum also holds the other shape. It is never
# the one being serialized; it exists so the definition has to handle both.
OTHER_VARIANT = {
    "enum_struct_variant": '#[suzu(display("other"))] Other,',
    "enum_unit_variant": '#[suzu(display("other"))] Other { note: u32 },',
}


def declaration(
    levels, index, ty_name, generics, concrete,
    oracle_generics, oracle_concrete, context, metadata,
):
    """The `declare_case!` invocation for one case."""
    opts, oracle = options(levels)
    shape = levels["TypeShape"]
    others = OTHER_VARIANT[shape] if levels["EnumHasOtherShape"] == "yes" else ""

    if shape == "enum_unit_variant":
        # The model forces zero declared fields, no source and an injected
        # location here, so there is nothing else to write.
        return f"""    declare_case! {{
        error: {ty_name},
        unit_variant: Under,
        others: {{ {others} }},
        options: {{ {opts} }},
        oracle_attrs: {{ {oracle} }},
        display: "case {index:02}",
    }}"""
    if shape == "enum_struct_variant":
        return f"""    declare_case! {{
        error: {ty_name},
        variant: Under,
        others: {{ {others} }},
        options: {{ {opts} }},
        oracle_attrs: {{ {oracle} }},
        generics: {{ {generics} }},
        concrete: {{ {concrete} }},
        oracle_generics: {{ {oracle_generics} }},
        oracle_concrete: {{ {oracle_concrete} }},
        display: "case {index:02}",
        context: {{ {context} }},
        metadata: {{ {metadata} }},
    }}"""
    return f"""    declare_case! {{
        error: {ty_name},
        options: {{ {opts} }},
        oracle_attrs: {{ {oracle} }},
        generics: {{ {generics} }},
        concrete: {{ {concrete} }},
        oracle_generics: {{ {oracle_generics} }},
        oracle_concrete: {{ {oracle_concrete} }},
        display: "case {index:02}",
        context: {{ {context} }},
        metadata: {{ {metadata} }},
    }}"""


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
        generic_params, concrete = generics(levels, True)
        oracle_params, oracle_concrete = generics(levels, False)
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

        decl = declaration(
            levels, index, ty_name, generic_params, concrete,
            oracle_params, oracle_concrete, context, metadata,
        )
        out.append(f'''
/// {summary}
mod case_{index:02} {{
    use super::*;

{decl}

    #[test]
    fn test_matches_the_hand_written_equivalent() {{
{body(levels, ty_name, selector(levels, ty_name, fields))}
    }}
}}''')

    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.stdout.write(main(sys.argv[1]))
