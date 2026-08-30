#!/usr/bin/env python3
"""Turn the pict output into the pairwise case file.

    python3 tests/support/generate-cases.py tests/support/differential-cases.pict \
        > tests/serde_differential_pairwise.rs

The generated Rust is committed and reviewed; neither pict nor this script is
needed at build time. Editing the level-to-code mapping below changes every
case at once, which is why the hand-written cases in serde_differential.rs are
kept as a separate check on it.
"""
import subprocess
import sys

HEADER = '''//! Pairwise cases, generated from `tests/support/differential-cases.pict`.
//!
//! Do not edit by hand. Regenerate with:
//!
//! ```text
//! python3 tests/support/generate-cases.py tests/support/differential-cases.pict \
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
//! `declare_case!` or this generator gets the split between `context` and
//! `metadata` wrong, every case here moves together and still agrees; the
//! hand-written ones do not.

#![cfg(feature = "serde")]

#[macro_use]
mod support;

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

/// A source that is itself opted in, so its own `context` survives nesting.
#[suzunari_error(serialize)]
#[suzu(display("inner failed"))]
struct InnerError {
    detail: String,
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
'''

CONTEXT = {
    ("some", "concrete"): (
        'label: String = "l".to_owned(), count: u32 = 7',
        'label: "l", count: 7u32',
    ),
    ("some", "option"): (
        'label: Option<String> = Some("l".to_owned()), count: Option<u32> = None',
        'label: Some("l".to_owned()), count: None::<u32>',
    ),
    ("none", "na"): ("", ""),
}

SOURCE_FIELD = {
    "none": "",
    "io_error": "source: std::io::Error,",
    "serialize_type": "source: InnerError,",
    "boxed": "source: BoxedStackError,",
    "foreign_serialize": "source: ForeignError,",
    "display_error": "#[suzu(from)] source: LibError,",
}

LOCATION_FIELD = {
    "injected": "",
    "suzu_named": "#[suzu(location)] at: Location,",
    "typed": "at: Location,",
    "stack_attr": "#[stack(location)] at: Location,",
}


def body(source, ty, selector):
    """The test body for one source level."""
    if source == "none":
        return f'''        fn failing() -> Result<(), {ty}> {{
            ensure!(false, {selector});
            Ok(())
        }}
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));'''
    if source == "io_error":
        return f'''        let error = std::fs::read(MISSING_PATH).context({selector}).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );'''
    if source in ("serialize_type", "boxed"):
        cause = "inner_error()" if source == "serialize_type" else "BoxedStackError::new(inner_error())"
        return f'''        let cause = {cause};
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context({selector}).unwrap_err();

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(record(&error).field("source").some(), &standalone);'''
    if source == "foreign_serialize":
        return f'''        let cause = ForeignError {{ code: 7 }};
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context({selector}).unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(recorded.field("source").some(), &error_node("foreign error 7"));
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);'''
    if source == "display_error":
        return f'''        let error = Err::<(), _>(LibError).context({selector}).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(record(&error).field("source").some(), &error_node("lib error"));'''
    raise ValueError(source)


def main(model):
    rows = subprocess.run(
        ["pict", model], capture_output=True, text=True, check=True
    ).stdout.strip().splitlines()
    header = rows[0].split("\t")
    out = [HEADER]

    for index, row in enumerate(rows[1:], start=1):
        levels = dict(zip(header, row.split("\t")))
        declared, field_ty = levels["DeclaredFields"], levels["FieldType"]
        source, location = levels["Source"], levels["Location"]

        context, args = CONTEXT[(declared, field_ty)]
        metadata = " ".join(
            part for part in (SOURCE_FIELD[source], LOCATION_FIELD[location]) if part
        )
        ty = f"Case{index:02}"
        selector = f"{ty}Snafu {{ {args} }}" if args else f"{ty}Snafu"

        out.append(f'''
/// DeclaredFields={declared}, FieldType={field_ty}, Source={source}, Location={location}
mod case_{index:02} {{
    use super::*;

    declare_case! {{
        error: {ty},
        display: "case {index:02}",
        context: {{ {context} }},
        metadata: {{ {metadata} }},
    }}

    #[test]
    fn matches_the_hand_written_equivalent() {{
{body(source, ty, selector)}
    }}
}}''')

    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.stdout.write(main(sys.argv[1]))
