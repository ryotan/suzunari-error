//! Pairwise cases, generated from `tests/support/differential-cases.pict`.
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

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=scalar, FieldAttr=serialize_with, Location=injected, Source=foreign_serialize, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_01 {
    use super::*;

    declare_case! {
        error: Case01,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 01",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [] label: &'static str = "l", [] [] filler: u32 = 1u32 },
        metadata: { source: ForeignError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: "l",
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=one, FieldType=container_attributed_struct, FieldAttr=skip_serializing_if, Location=typed, Source=foreign_serialize, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_02 {
    use super::*;

    declare_case! {
        error: Case02,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 02",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [#[suzu(source(false))]] source: Renamed = Renamed { field_name: "x" } },
        metadata: { #[suzu(source)] cause: ForeignError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                source: Renamed { field_name: "x" },
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=struct, DeclaredParam=type_param, DeclaredFields=many, FieldType=wrapped_param, FieldAttr=none, Location=stack_attr, Source=foreign_serialize, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_03 {
    use super::*;

    declare_case! {
        error: Case03,
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 03",
        context: { [] [] label: Option<T> = Some(9u32), [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: ForeignError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case03Snafu {
                label: Some(9u32),
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=one, FieldType=wrapped_param, FieldAttr=deser_only, Location=suzu_named, Source=serialize_type, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=absent
mod case_04 {
    use super::*;

    declare_case! {
        error: Case04,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 04",
        context: { [#[serde(alias = "other")]] [#[suzu(source(false))]] source: Option<T> = Some(9u32) },
        metadata: { #[suzu(source)] cause: InnerError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { source: Some(9u32) })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=struct, DeclaredParam=none, DeclaredFields=one, FieldType=container_attributed_struct, FieldAttr=skip, Location=stack_attr, Source=serialize_enum, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_05 {
    use super::*;

    declare_case! {
        error: Case05,
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 05",
        context: { [#[serde(skip)]] [] label: Renamed = Renamed { field_name: "x" } },
        metadata: { source: InnerEnumError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case05Snafu {
                label: Renamed { field_name: "x" },
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=type_param, DeclaredFields=many, FieldType=generic_param, FieldAttr=rename, Location=typed, Source=serialize_type, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_06 {
    use super::*;

    declare_case! {
        error: Case06,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 06",
        context: { [#[serde(rename = "renamed")]] [] label: T = 9u32, [] [] filler: u32 = 1u32 },
        metadata: { source: InnerError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: 9u32,
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=struct, DeclaredParam=type_param, DeclaredFields=one, FieldType=generic_param, FieldAttr=bound, Location=injected, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=absent
mod case_07 {
    use super::*;

    declare_case! {
        error: Case07,
        options: { serialize },
        oracle_attrs: {  },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 07",
        context: { [#[serde(bound(serialize = "T: serde::Serialize"))]] [#[suzu(source(false))]] source: T = 9u32 },
        metadata: { #[suzu(source)] cause: std::io::Error, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case07Snafu { source: 9u32 })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=map_like, FieldAttr=flatten, Location=suzu_named, Source=serialize_enum, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_08 {
    use super::*;

    declare_case! {
        error: Case08,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 08",
        context: { [#[serde(flatten)]] [] label: BTreeMap<&'static str, u32> = pairs(), [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: InnerEnumError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: pairs(),
                source: "not-an-error",
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=struct, DeclaredParam=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=skip_serializing_if, Location=suzu_named, Source=serialize_type, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_09 {
    use super::*;

    declare_case! {
        error: Case09,
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 09",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [] label: Detail = Detail { code: 3 }, [] [] filler: u32 = 1u32 },
        metadata: { source: InnerError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case09Snafu {
                label: Detail { code: 3 },
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=deser_only, Location=stack_attr, Source=io_error, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_10 {
    use super::*;

    declare_case! {
        error: Case10,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 10",
        context: { [#[serde(alias = "other")]] [] label: Detail = Detail { code: 3 }, [] [] filler: u32 = 1u32 },
        metadata: { source: std::io::Error, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: Detail { code: 3 },
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=option, FieldAttr=rename, Location=typed, Source=serialize_enum, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=absent
mod case_11 {
    use super::*;

    declare_case! {
        error: Case11,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 11",
        context: { [#[serde(rename = "renamed")]] [#[suzu(source(false))]] source: Option<u32> = Some(7u32) },
        metadata: { #[suzu(source)] cause: InnerEnumError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { source: Some(7u32) })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=lifetime, DeclaredFields=many, FieldType=borrowed, FieldAttr=bound, Location=injected, Source=serialize_type, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_12 {
    use super::*;

    declare_case! {
        error: Case12,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <'a> },
        concrete: { <'static> },
        oracle_generics: { <'a> },
        oracle_concrete: { <'static> },
        display: "case 12",
        context: { [#[serde(bound(serialize = "&'a str: serde::Serialize"))]] [] label: &'a str = "b", [] [] filler: u32 = 1u32 },
        metadata: { source: InnerError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: "b",
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=many, FieldType=generic_param, FieldAttr=skip, Location=stack_attr, Source=foreign_serialize, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=absent
mod case_13 {
    use super::*;

    declare_case! {
        error: Case13,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 13",
        context: { [#[serde(skip)]] [] label: T = 9u32, [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: ForeignError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: 9u32,
                source: "not-an-error",
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=zero, Location=suzu_named, Source=foreign_serialize, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=absent
mod case_14 {
    use super::*;

    declare_case! {
        error: Case14,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 14",
        context: {  },
        metadata: { #[suzu(source)] cause: ForeignError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(UnderSnafu).unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=struct, DeclaredParam=assoc_param, DeclaredFields=one, FieldType=assoc_type, FieldAttr=serialize_with, Location=typed, Source=generic_source, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_15 {
    use super::*;

    declare_case! {
        error: Case15,
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: Keyed + ::core::fmt::Debug, U: ::core::error::Error + ::core::fmt::Debug + 'static> where T::Key: ::core::fmt::Debug },
        concrete: { <Marker, std::io::Error> },
        oracle_generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        oracle_concrete: { <Marker> },
        display: "case 15",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [#[suzu(source(false))]] source: T::Key = "k" },
        metadata: { #[suzu(source)] cause: U, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case15Snafu { source: "k" })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=map_like, FieldAttr=none, Location=injected, Source=display_error, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_16 {
    use super::*;

    declare_case! {
        error: Case16,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 16",
        context: { [] [] label: BTreeMap<&'static str, u32> = pairs() },
        metadata: { #[suzu(from)] source: LibError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(UnderSnafu { label: pairs() })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=lifetime, DeclaredFields=many, FieldType=borrowed, FieldAttr=skip_serializing_if, Location=stack_attr, Source=display_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=absent
mod case_17 {
    use super::*;

    declare_case! {
        error: Case17,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <'a> },
        concrete: { <'static> },
        oracle_generics: { <'a> },
        oracle_concrete: { <'static> },
        display: "case 17",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [] label: &'a str = "b", [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: { #[suzu(from)] cause: LibError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(UnderSnafu {
                label: "b",
                source: "not-an-error",
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=assoc_param, DeclaredFields=many, FieldType=assoc_type, FieldAttr=deser_only, Location=injected, Source=none, SourceFalseField=absent, RenameAll=absent
mod case_18 {
    use super::*;

    declare_case! {
        error: Case18,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        concrete: { <Marker> },
        oracle_generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        oracle_concrete: { <Marker> },
        display: "case 18",
        context: { [#[serde(alias = "other")]] [] label: T::Key = "k", [] [] filler: u32 = 1u32 },
        metadata: {  },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case18<Marker>> {
            ensure!(
                false,
                UnderSnafu {
                    label: "k",
                    filler: 1u32
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=type_param, DeclaredFields=many, FieldType=wrapped_param, FieldAttr=rename, Location=injected, Source=generic_source, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_19 {
    use super::*;

    declare_case! {
        error: Case19,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <T: ::core::fmt::Debug, U: ::core::error::Error + ::core::fmt::Debug + 'static> },
        concrete: { <u32, std::io::Error> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 19",
        context: { [#[serde(rename = "renamed")]] [] label: Option<T> = Some(9u32), [] [] filler: u32 = 1u32 },
        metadata: { source: U, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: Some(9u32),
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=serialize_struct, FieldAttr=skip, Location=typed, Source=display_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_20 {
    use super::*;

    declare_case! {
        error: Case20,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 20",
        context: { [#[serde(skip)]] [#[suzu(source(false))]] source: Detail = Detail { code: 3 } },
        metadata: { #[suzu(from)] cause: LibError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(UnderSnafu {
                source: Detail { code: 3 },
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=type_param, DeclaredFields=one, FieldType=generic_param, FieldAttr=none, Location=suzu_named, Source=boxed, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_21 {
    use super::*;

    declare_case! {
        error: Case21,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 21",
        context: { [] [#[suzu(source(false))]] source: T = 9u32 },
        metadata: { #[suzu(source)] cause: BoxedStackError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { source: 9u32 })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=zero, Location=stack_attr, Source=generic_source, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_22 {
    use super::*;

    declare_case! {
        error: Case22,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <U: ::core::error::Error + ::core::fmt::Debug + 'static> },
        concrete: { <std::io::Error> },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 22",
        context: {  },
        metadata: { source: U, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH).context(UnderSnafu).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=serialize_struct, FieldAttr=flatten, Location=injected, Source=foreign_serialize, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_23 {
    use super::*;

    declare_case! {
        error: Case23,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 23",
        context: { [#[serde(flatten)]] [] label: Detail = Detail { code: 3 } },
        metadata: { source: ForeignError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: Detail { code: 3 },
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=struct, DeclaredParam=none, DeclaredFields=zero, Location=typed, Source=boxed, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_24 {
    use super::*;

    declare_case! {
        error: Case24,
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 24",
        context: {  },
        metadata: { source: BoxedStackError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(Case24Snafu).unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=assoc_param, DeclaredFields=one, FieldType=assoc_type, FieldAttr=rename, Location=suzu_named, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_25 {
    use super::*;

    declare_case! {
        error: Case25,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        concrete: { <Marker> },
        oracle_generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        oracle_concrete: { <Marker> },
        display: "case 25",
        context: { [#[serde(rename = "renamed")]] [#[suzu(source(false))]] source: T::Key = "k" },
        metadata: { #[suzu(source)] cause: std::io::Error, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu { source: "k" })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=struct, DeclaredParam=none, DeclaredFields=one, FieldType=map_like, FieldAttr=deser_only, Location=typed, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_26 {
    use super::*;

    declare_case! {
        error: Case26,
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 26",
        context: { [#[serde(alias = "other")]] [#[suzu(source(false))]] source: BTreeMap<&'static str, u32> = pairs() },
        metadata: { #[suzu(source)] cause: std::io::Error, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case26Snafu { source: pairs() })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=skip, Location=injected, Source=boxed, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_27 {
    use super::*;

    declare_case! {
        error: Case27,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 27",
        context: { [#[serde(skip)]] [] label: Detail = Detail { code: 3 }, [] [] filler: u32 = 1u32 },
        metadata: { source: BoxedStackError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: Detail { code: 3 },
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=option, FieldAttr=skip, Location=suzu_named, Source=generic_source, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_28 {
    use super::*;

    declare_case! {
        error: Case28,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <U: ::core::error::Error + ::core::fmt::Debug + 'static> },
        concrete: { <std::io::Error> },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 28",
        context: { [#[serde(skip)]] [] label: Option<u32> = Some(7u32), [] [] filler: u32 = 1u32 },
        metadata: { source: U, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: Some(7u32),
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=one, FieldType=scalar, FieldAttr=serialize_with, Location=suzu_named, Source=display_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_29 {
    use super::*;

    declare_case! {
        error: Case29,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 29",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [#[suzu(source(false))]] source: &'static str = "l" },
        metadata: { #[suzu(from)] cause: LibError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(UnderSnafu { source: "l" })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=struct, DeclaredParam=lifetime, DeclaredFields=one, FieldType=borrowed, FieldAttr=none, Location=typed, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_30 {
    use super::*;

    declare_case! {
        error: Case30,
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <'a> },
        concrete: { <'static> },
        oracle_generics: { <'a> },
        oracle_concrete: { <'static> },
        display: "case 30",
        context: { [] [#[suzu(source(false))]] source: &'a str = "b" },
        metadata: { #[suzu(source)] cause: std::io::Error, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case30Snafu { source: "b" })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=assoc_param, DeclaredFields=one, FieldType=assoc_type, FieldAttr=bound, Location=stack_attr, Source=foreign_serialize, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_31 {
    use super::*;

    declare_case! {
        error: Case31,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        concrete: { <Marker> },
        oracle_generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        oracle_concrete: { <Marker> },
        display: "case 31",
        context: { [#[serde(bound(serialize = "T::Key: serde::Serialize"))]] [] label: T::Key = "k" },
        metadata: { source: ForeignError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { label: "k" })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=assoc_param, DeclaredFields=many, FieldType=assoc_type, FieldAttr=skip, Location=stack_attr, Source=serialize_type, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_32 {
    use super::*;

    declare_case! {
        error: Case32,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        concrete: { <Marker> },
        oracle_generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        oracle_concrete: { <Marker> },
        display: "case 32",
        context: { [#[serde(skip)]] [] label: T::Key = "k", [] [] filler: u32 = 1u32 },
        metadata: { source: InnerError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: "k",
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=many, FieldType=wrapped_param, FieldAttr=skip_serializing_if, Location=typed, Source=io_error, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_33 {
    use super::*;

    declare_case! {
        error: Case33,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 33",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [] label: Option<T> = Some(9u32), [] [] filler: u32 = 1u32 },
        metadata: { source: std::io::Error, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: Some(9u32),
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=struct, DeclaredParam=none, DeclaredFields=many, FieldType=container_attributed_struct, FieldAttr=rename, Location=injected, Source=display_error, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_34 {
    use super::*;

    declare_case! {
        error: Case34,
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 34",
        context: { [#[serde(rename = "renamed")]] [] label: Renamed = Renamed { field_name: "x" }, [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(from)] cause: LibError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(Case34Snafu {
                label: Renamed { field_name: "x" },
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=struct, DeclaredParam=none, DeclaredFields=one, FieldType=scalar, FieldAttr=serialize_with, Location=stack_attr, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_35 {
    use super::*;

    declare_case! {
        error: Case35,
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 35",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [#[suzu(source(false))]] source: &'static str = "l" },
        metadata: { #[suzu(source)] cause: std::io::Error, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case35Snafu { source: "l" })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=struct, DeclaredParam=none, DeclaredFields=one, FieldType=option, FieldAttr=serialize_with, Location=stack_attr, Source=none, SourceFalseField=present, RenameAll=present
mod case_36 {
    use super::*;

    declare_case! {
        error: Case36,
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 36",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [#[suzu(source(false))]] source: Option<u32> = Some(7u32) },
        metadata: { #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case36> {
            ensure!(false, Case36Snafu { source: Some(7u32) });
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=scalar, FieldAttr=skip_serializing_if, Location=typed, Source=boxed, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_37 {
    use super::*;

    declare_case! {
        error: Case37,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 37",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [] label: &'static str = "l" },
        metadata: { #[suzu(source)] cause: BoxedStackError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { label: "l" })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=many, FieldType=generic_param, FieldAttr=deser_only, Location=injected, Source=display_error, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_38 {
    use super::*;

    declare_case! {
        error: Case38,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 38",
        context: { [#[serde(alias = "other")]] [] label: T = 9u32, [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(from)] cause: LibError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(UnderSnafu {
                label: 9u32,
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=option, FieldAttr=skip_serializing_if, Location=injected, Source=display_error, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_39 {
    use super::*;

    declare_case! {
        error: Case39,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 39",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [] label: Option<u32> = Some(7u32), [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(from)] source: LibError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(UnderSnafu {
                label: Some(7u32),
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=one, FieldType=wrapped_param, FieldAttr=serialize_with, Location=stack_attr, Source=boxed, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_40 {
    use super::*;

    declare_case! {
        error: Case40,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 40",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [] label: Option<T> = Some(9u32) },
        metadata: { source: BoxedStackError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { label: Some(9u32) })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=struct, DeclaredParam=none, DeclaredFields=many, FieldType=map_like, FieldAttr=flatten, Location=stack_attr, Source=serialize_type, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_41 {
    use super::*;

    declare_case! {
        error: Case41,
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 41",
        context: { [#[serde(flatten)]] [] label: BTreeMap<&'static str, u32> = pairs(), [] [] filler: u32 = 1u32 },
        metadata: { source: InnerError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case41Snafu {
                label: pairs(),
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=assoc_param, DeclaredFields=many, FieldType=assoc_type, FieldAttr=skip_serializing_if, Location=injected, Source=serialize_enum, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=absent
mod case_42 {
    use super::*;

    declare_case! {
        error: Case42,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        concrete: { <Marker> },
        oracle_generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        oracle_concrete: { <Marker> },
        display: "case 42",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [] label: T::Key = "k", [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: InnerEnumError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: "k",
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=container_attributed_struct, FieldAttr=deser_only, Location=suzu_named, Source=boxed, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_43 {
    use super::*;

    declare_case! {
        error: Case43,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 43",
        context: { [#[serde(alias = "other")]] [] label: Renamed = Renamed { field_name: "x" }, [] [] filler: u32 = 1u32 },
        metadata: { source: BoxedStackError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: Renamed { field_name: "x" },
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=scalar, FieldAttr=none, Location=injected, Source=serialize_enum, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=absent
mod case_44 {
    use super::*;

    declare_case! {
        error: Case44,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 44",
        context: { [] [] label: &'static str = "l", [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: InnerEnumError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: "l",
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=map_like, FieldAttr=skip_serializing_if, Location=injected, Source=generic_source, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_45 {
    use super::*;

    declare_case! {
        error: Case45,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <U: ::core::error::Error + ::core::fmt::Debug + 'static> },
        concrete: { <std::io::Error> },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 45",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [] label: BTreeMap<&'static str, u32> = pairs(), [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: U, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: pairs(),
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=map_like, FieldAttr=rename, Location=typed, Source=none, SourceFalseField=absent, RenameAll=absent
mod case_46 {
    use super::*;

    declare_case! {
        error: Case46,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 46",
        context: { [#[serde(rename = "renamed")]] [] label: BTreeMap<&'static str, u32> = pairs(), [] [] filler: u32 = 1u32 },
        metadata: { at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case46> {
            ensure!(
                false,
                UnderSnafu {
                    label: pairs(),
                    filler: 1u32
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=many, FieldType=wrapped_param, FieldAttr=bound, Location=injected, Source=display_error, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_47 {
    use super::*;

    declare_case! {
        error: Case47,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 47",
        context: { [#[serde(bound(serialize = "T: serde::Serialize"))]] [] label: Option<T> = Some(9u32), [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(from)] source: LibError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(UnderSnafu {
                label: Some(9u32),
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=map_like, FieldAttr=rename, Location=injected, Source=boxed, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_48 {
    use super::*;

    declare_case! {
        error: Case48,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 48",
        context: { [#[serde(rename = "renamed")]] [] label: BTreeMap<&'static str, u32> = pairs(), [] [] filler: u32 = 1u32 },
        metadata: { source: BoxedStackError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: pairs(),
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=many, FieldType=wrapped_param, FieldAttr=serialize_with, Location=injected, Source=serialize_enum, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_49 {
    use super::*;

    declare_case! {
        error: Case49,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 49",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [] label: Option<T> = Some(9u32), [] [] filler: u32 = 1u32 },
        metadata: { source: InnerEnumError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: Some(9u32),
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=lifetime, DeclaredFields=many, FieldType=borrowed, FieldAttr=deser_only, Location=suzu_named, Source=serialize_enum, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=absent
mod case_50 {
    use super::*;

    declare_case! {
        error: Case50,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <'a> },
        concrete: { <'static> },
        oracle_generics: { <'a> },
        oracle_concrete: { <'static> },
        display: "case 50",
        context: { [#[serde(alias = "other")]] [] label: &'a str = "b", [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: InnerEnumError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: "b",
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=zero, Location=injected, Source=serialize_enum, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=absent
mod case_51 {
    use super::*;

    declare_case! {
        error: Case51,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 51",
        context: {  },
        metadata: { #[suzu(source)] cause: InnerEnumError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(UnderSnafu).unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=assoc_param, DeclaredFields=one, FieldType=assoc_type, FieldAttr=none, Location=typed, Source=display_error, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_52 {
    use super::*;

    declare_case! {
        error: Case52,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        concrete: { <Marker> },
        oracle_generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        oracle_concrete: { <Marker> },
        display: "case 52",
        context: { [] [] label: T::Key = "k" },
        metadata: { #[suzu(from)] cause: LibError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(UnderSnafu { label: "k" })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=container_attributed_struct, FieldAttr=flatten, Location=typed, Source=generic_source, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=absent
mod case_53 {
    use super::*;

    declare_case! {
        error: Case53,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <U: ::core::error::Error + ::core::fmt::Debug + 'static> },
        concrete: { <std::io::Error> },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 53",
        context: { [#[serde(flatten)]] [] label: Renamed = Renamed { field_name: "x" } },
        metadata: { #[suzu(source)] cause: U, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: Renamed { field_name: "x" },
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=type_param, DeclaredFields=one, FieldType=generic_param, FieldAttr=bound, Location=suzu_named, Source=serialize_enum, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_54 {
    use super::*;

    declare_case! {
        error: Case54,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 54",
        context: { [#[serde(bound(serialize = "T: serde::Serialize"))]] [] label: T = 9u32 },
        metadata: { source: InnerEnumError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { label: 9u32 })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=option, FieldAttr=deser_only, Location=injected, Source=boxed, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_55 {
    use super::*;

    declare_case! {
        error: Case55,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 55",
        context: { [#[serde(alias = "other")]] [] label: Option<u32> = Some(7u32), [] [] filler: u32 = 1u32 },
        metadata: { source: BoxedStackError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: Some(7u32),
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=type_param, DeclaredFields=one, FieldType=wrapped_param, FieldAttr=skip, Location=suzu_named, Source=none, SourceFalseField=absent, RenameAll=present
mod case_56 {
    use super::*;

    declare_case! {
        error: Case56,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 56",
        context: { [#[serde(skip)]] [] label: Option<T> = Some(9u32) },
        metadata: { #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case56<u32>> {
            ensure!(false, UnderSnafu { label: Some(9u32) });
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=rename, Location=stack_attr, Source=serialize_enum, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_57 {
    use super::*;

    declare_case! {
        error: Case57,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 57",
        context: { [#[serde(rename = "renamed")]] [] label: Detail = Detail { code: 3 }, [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: InnerEnumError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_enum_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: Detail { code: 3 },
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=zero, Location=typed, Source=io_error, SourceBinding=named_source, SourceFalseField=absent, RenameAll=absent
mod case_58 {
    use super::*;

    declare_case! {
        error: Case58,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 58",
        context: {  },
        metadata: { source: std::io::Error, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH).context(UnderSnafu).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=option, FieldAttr=skip, Location=injected, Source=io_error, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_59 {
    use super::*;

    declare_case! {
        error: Case59,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 59",
        context: { [#[serde(skip)]] [] label: Option<u32> = Some(7u32) },
        metadata: { source: std::io::Error, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu { label: Some(7u32) })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=lifetime, DeclaredFields=many, FieldType=borrowed, FieldAttr=rename, Location=injected, Source=generic_source, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_60 {
    use super::*;

    declare_case! {
        error: Case60,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <'a, U: ::core::error::Error + ::core::fmt::Debug + 'static> },
        concrete: { <'static, std::io::Error> },
        oracle_generics: { <'a> },
        oracle_concrete: { <'static> },
        display: "case 60",
        context: { [#[serde(rename = "renamed")]] [] label: &'a str = "b", [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: U, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: "b",
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=none, Location=injected, Source=generic_source, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_61 {
    use super::*;

    declare_case! {
        error: Case61,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <U: ::core::error::Error + ::core::fmt::Debug + 'static> },
        concrete: { <std::io::Error> },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 61",
        context: { [] [] label: Detail = Detail { code: 3 }, [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: U, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: Detail { code: 3 },
                filler: 1u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=map_like, FieldAttr=serialize_with, Location=injected, Source=foreign_serialize, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_62 {
    use super::*;

    declare_case! {
        error: Case62,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 62",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [] label: BTreeMap<&'static str, u32> = pairs(), [] [] filler: u32 = 1u32 },
        metadata: { source: ForeignError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: pairs(),
                filler: 1u32,
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=many, FieldType=scalar, FieldAttr=skip_serializing_if, Location=injected, Source=none, SourceFalseField=absent, RenameAll=present
mod case_63 {
    use super::*;

    declare_case! {
        error: Case63,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 63",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [] label: &'static str = "l", [] [] filler: u32 = 1u32 },
        metadata: {  },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case63> {
            ensure!(
                false,
                UnderSnafu {
                    label: "l",
                    filler: 1u32
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=one, FieldType=option, FieldAttr=none, Location=injected, Source=serialize_type, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_64 {
    use super::*;

    declare_case! {
        error: Case64,
        variant: Under,
        others: {  },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 64",
        context: { [] [] label: Option<u32> = Some(7u32) },
        metadata: { source: InnerError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { label: Some(7u32) })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=lifetime, DeclaredFields=one, FieldType=borrowed, FieldAttr=serialize_with, Location=injected, Source=boxed, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=absent
mod case_65 {
    use super::*;

    declare_case! {
        error: Case65,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize },
        oracle_attrs: {  },
        generics: { <'a> },
        concrete: { <'static> },
        oracle_generics: { <'a> },
        oracle_concrete: { <'static> },
        display: "case 65",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [] label: &'a str = "b" },
        metadata: { #[suzu(source)] cause: BoxedStackError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { label: "b" })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=lifetime, DeclaredFields=many, FieldType=borrowed, FieldAttr=none, Location=injected, Source=none, SourceFalseField=absent, RenameAll=present
mod case_66 {
    use super::*;

    declare_case! {
        error: Case66,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <'a> },
        concrete: { <'static> },
        oracle_generics: { <'a> },
        oracle_concrete: { <'static> },
        display: "case 66",
        context: { [] [] label: &'a str = "b", [] [] filler: u32 = 1u32 },
        metadata: {  },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case66<'static>> {
            ensure!(
                false,
                UnderSnafu {
                    label: "b",
                    filler: 1u32
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_unit_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=zero, Location=injected, Source=none, SourceFalseField=absent, RenameAll=absent
mod case_67 {
    use super::*;

    declare_case! {
        error: Case67,
        unit_variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        display: "case 67",
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case67> {
            ensure!(false, UnderSnafu);
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=zero, Location=suzu_named, Source=display_error, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_68 {
    use super::*;

    declare_case! {
        error: Case68,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 68",
        context: {  },
        metadata: { #[suzu(from)] source: LibError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError).context(UnderSnafu).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=scalar, FieldAttr=deser_only, Location=injected, Source=generic_source, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_69 {
    use super::*;

    declare_case! {
        error: Case69,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <U: ::core::error::Error + ::core::fmt::Debug + 'static> },
        concrete: { <std::io::Error> },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 69",
        context: { [#[serde(alias = "other")]] [] label: &'static str = "l", [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: U, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: "l",
                source: "not-an-error",
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=option, FieldAttr=rename, Location=injected, Source=foreign_serialize, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_70 {
    use super::*;

    declare_case! {
        error: Case70,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 70",
        context: { [#[serde(rename = "renamed")]] [#[suzu(source(false))]] source: Option<u32> = Some(7u32) },
        metadata: { #[suzu(source)] cause: ForeignError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { source: Some(7u32) })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=container_attributed_struct, FieldAttr=none, Location=typed, Source=io_error, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_71 {
    use super::*;

    declare_case! {
        error: Case71,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 71",
        context: { [] [] label: Renamed = Renamed { field_name: "x" } },
        metadata: { source: std::io::Error, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: Renamed { field_name: "x" },
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=assoc_param, DeclaredFields=one, FieldType=assoc_type, FieldAttr=bound, Location=typed, Source=boxed, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_72 {
    use super::*;

    declare_case! {
        error: Case72,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        concrete: { <Marker> },
        oracle_generics: { <T: Keyed + ::core::fmt::Debug> where T::Key: ::core::fmt::Debug },
        oracle_concrete: { <Marker> },
        display: "case 72",
        context: { [#[serde(bound(serialize = "T::Key: serde::Serialize"))]] [#[suzu(source(false))]] source: T::Key = "k" },
        metadata: { #[suzu(source)] cause: BoxedStackError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { source: "k" })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=flatten, Location=suzu_named, Source=none, SourceFalseField=absent, RenameAll=present
mod case_73 {
    use super::*;

    declare_case! {
        error: Case73,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 73",
        context: { [#[serde(flatten)]] [] label: Detail = Detail { code: 3 }, [] [] filler: u32 = 1u32 },
        metadata: { #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case73> {
            ensure!(
                false,
                UnderSnafu {
                    label: Detail { code: 3 },
                    filler: 1u32
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=one, FieldType=generic_param, FieldAttr=skip_serializing_if, Location=suzu_named, Source=none, SourceFalseField=present, RenameAll=present
mod case_74 {
    use super::*;

    declare_case! {
        error: Case74,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 74",
        context: { [#[serde(skip_serializing_if = "crate::always_skip")]] [#[suzu(source(false))]] source: T = 9u32 },
        metadata: { #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case74<u32>> {
            ensure!(false, UnderSnafu { source: 9u32 });
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=container_attributed_struct, FieldAttr=serialize_with, Location=suzu_named, Source=serialize_type, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_75 {
    use super::*;

    declare_case! {
        error: Case75,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 75",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [] label: Renamed = Renamed { field_name: "x" }, [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: InnerError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: Renamed { field_name: "x" },
                source: "not-an-error",
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=flatten, Location=typed, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_76 {
    use super::*;

    declare_case! {
        error: Case76,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 76",
        context: { [#[serde(flatten)]] [] label: Detail = Detail { code: 3 }, [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: std::io::Error, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: Detail { code: 3 },
                source: "not-an-error",
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=scalar, FieldAttr=skip, Location=injected, Source=serialize_type, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_77 {
    use super::*;

    declare_case! {
        error: Case77,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 77",
        context: { [#[serde(skip)]] [] label: &'static str = "l" },
        metadata: { #[suzu(source)] cause: InnerError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { label: "l" })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=map_like, FieldAttr=flatten, Location=suzu_named, Source=display_error, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_78 {
    use super::*;

    declare_case! {
        error: Case78,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 78",
        context: { [#[serde(flatten)]] [] label: BTreeMap<&'static str, u32> = pairs(), [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: { #[suzu(from)] cause: LibError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(UnderSnafu {
                label: pairs(),
                source: "not-an-error",
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=many, FieldType=generic_param, FieldAttr=bound, Location=injected, Source=generic_source, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_79 {
    use super::*;

    declare_case! {
        error: Case79,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug, U: ::core::error::Error + ::core::fmt::Debug + 'static> },
        concrete: { <u32, std::io::Error> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 79",
        context: { [#[serde(bound(serialize = "T: serde::Serialize"))]] [] label: T = 9u32, [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: U, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(UnderSnafu {
                label: 9u32,
                source: "not-an-error",
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=one, FieldType=generic_param, FieldAttr=serialize_with, Location=injected, Source=none, SourceFalseField=present, RenameAll=present
mod case_80 {
    use super::*;

    declare_case! {
        error: Case80,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 80",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [#[suzu(source(false))]] source: T = 9u32 },
        metadata: {  },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case80<u32>> {
            ensure!(false, UnderSnafu { source: 9u32 });
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=type_param, DeclaredFields=many, FieldType=generic_param, FieldAttr=bound, Location=typed, Source=none, SourceFalseField=absent, RenameAll=present
mod case_81 {
    use super::*;

    declare_case! {
        error: Case81,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <T: ::core::fmt::Debug> },
        concrete: { <u32> },
        oracle_generics: { <T: ::core::fmt::Debug> },
        oracle_concrete: { <u32> },
        display: "case 81",
        context: { [#[serde(bound(serialize = "T: serde::Serialize"))]] [] label: T = 9u32, [] [] filler: u32 = 1u32 },
        metadata: { at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case81<u32>> {
            ensure!(
                false,
                UnderSnafu {
                    label: 9u32,
                    filler: 1u32
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=serialize_struct, FieldAttr=serialize_with, Location=suzu_named, Source=foreign_serialize, SourceBinding=named_source, SourceFalseField=absent, RenameAll=present
mod case_82 {
    use super::*;

    declare_case! {
        error: Case82,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 82",
        context: { [#[serde(serialize_with = "crate::debug_string")]] [] label: Detail = Detail { code: 3 } },
        metadata: { source: ForeignError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                label: Detail { code: 3 },
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=scalar, FieldAttr=rename, Location=suzu_named, Source=serialize_type, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_83 {
    use super::*;

    declare_case! {
        error: Case83,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 83",
        context: { [#[serde(rename = "renamed")]] [#[suzu(source(false))]] source: &'static str = "l" },
        metadata: { #[suzu(source)] cause: InnerError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu { source: "l" })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=zero, Location=stack_attr, Source=serialize_type, SourceBinding=renamed_with_attr, SourceFalseField=absent, RenameAll=present
mod case_84 {
    use super::*;

    declare_case! {
        error: Case84,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 84",
        context: {  },
        metadata: { #[suzu(source)] cause: InnerError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(UnderSnafu).unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=one, FieldType=serialize_struct, FieldAttr=flatten, Location=suzu_named, Source=boxed, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_85 {
    use super::*;

    declare_case! {
        error: Case85,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 85",
        context: { [#[serde(flatten)]] [#[suzu(source(false))]] source: Detail = Detail { code: 3 } },
        metadata: { #[suzu(source)] cause: BoxedStackError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(UnderSnafu {
                source: Detail { code: 3 },
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "TypeErasedStackError",
                ..
            }
        ));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=lifetime, DeclaredFields=one, FieldType=borrowed, FieldAttr=skip, Location=injected, Source=none, SourceFalseField=absent, RenameAll=present
mod case_86 {
    use super::*;

    declare_case! {
        error: Case86,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <'a> },
        concrete: { <'static> },
        oracle_generics: { <'a> },
        oracle_concrete: { <'static> },
        display: "case 86",
        context: { [#[serde(skip)]] [] label: &'a str = "b" },
        metadata: {  },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case86<'static>> {
            ensure!(false, UnderSnafu { label: "b" });
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=many, FieldType=container_attributed_struct, FieldAttr=flatten, Location=injected, Source=none, SourceFalseField=present, RenameAll=present
mod case_87 {
    use super::*;

    declare_case! {
        error: Case87,
        variant: Under,
        others: { #[suzu(display("other"))] Other, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 87",
        context: { [#[serde(flatten)]] [] label: Renamed = Renamed { field_name: "x" }, [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: {  },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case87> {
            ensure!(
                false,
                UnderSnafu {
                    label: Renamed { field_name: "x" },
                    source: "not-an-error"
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=enum_unit_variant, EnumHasOtherShape=yes, DeclaredParam=none, DeclaredFields=zero, Location=injected, Source=none, SourceFalseField=absent, RenameAll=present
mod case_88 {
    use super::*;

    declare_case! {
        error: Case88,
        unit_variant: Under,
        others: { #[suzu(display("other"))] Other { note: u32 }, },
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        display: "case 88",
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case88> {
            ensure!(false, UnderSnafu);
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=struct, DeclaredParam=lifetime, DeclaredFields=many, FieldType=borrowed, FieldAttr=deser_only, Location=typed, Source=foreign_serialize, SourceBinding=renamed_with_attr, SourceFalseField=present, RenameAll=present
mod case_89 {
    use super::*;

    declare_case! {
        error: Case89,
        options: { serialize(rename_all = "UPPERCASE") },
        oracle_attrs: { #[serde(rename_all = "UPPERCASE")] },
        generics: { <'a> },
        concrete: { <'static> },
        oracle_generics: { <'a> },
        oracle_concrete: { <'static> },
        display: "case 89",
        context: { [#[serde(alias = "other")]] [] label: &'a str = "b", [] [#[suzu(source(false))]] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: ForeignError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case89Snafu {
                label: "b",
                source: "not-an-error",
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        assert_eq!(
            recorded.field("source").some(),
            &error_node("foreign error 7")
        );
        // Keying dispatch on `Serialize` rather than the marker would have put
        // the foreign type's own fields here.
        assert_ne!(recorded.field("source").some(), &standalone);
    }
}

/// TypeShape=enum_struct_variant, EnumHasOtherShape=no, DeclaredParam=none, DeclaredFields=one, FieldType=map_like, FieldAttr=skip, Location=stack_attr, Source=none, SourceFalseField=present, RenameAll=absent
mod case_90 {
    use super::*;

    declare_case! {
        error: Case90,
        variant: Under,
        others: {  },
        options: { serialize },
        oracle_attrs: {  },
        generics: {  },
        concrete: {  },
        oracle_generics: {  },
        oracle_concrete: {  },
        display: "case 90",
        context: { [#[serde(skip)]] [#[suzu(source(false))]] source: BTreeMap<&'static str, u32> = pairs() },
        metadata: { #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case90> {
            ensure!(false, UnderSnafu { source: pairs() });
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}
