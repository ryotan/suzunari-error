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

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=container_attributed_struct, FieldAttr=none, Location=suzu_named, Source=serialize_type, SourceBinding=named_source, SourceFalseField=absent
mod case_01 {
    use super::*;

    declare_case! {
        error: Case01,
        display: "case 01",
        context: { label: Renamed = Renamed { field_name: "x" }, filler: u32 = 1u32 },
        metadata: { source: InnerError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case01Snafu {
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
                name: "StackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=container_attributed_struct, FieldAttr=none, Location=injected, Source=none, SourceFalseField=present
mod case_02 {
    use super::*;

    declare_case! {
        error: Case02,
        display: "case 02",
        context: { #[suzu(source(false))] source: Renamed = Renamed { field_name: "x" } },
        metadata: {  },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case02> {
            ensure!(
                false,
                Case02Snafu {
                    source: Renamed { field_name: "x" }
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=serialize_struct, FieldAttr=none, Location=stack_attr, Source=display_error, SourceBinding=renamed_with_attr, SourceFalseField=absent
mod case_03 {
    use super::*;

    declare_case! {
        error: Case03,
        display: "case 03",
        context: { label: Detail = Detail { code: 3 } },
        metadata: { #[suzu(from)] cause: LibError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(Case03Snafu {
                label: Detail { code: 3 },
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=container_attributed_struct, FieldAttr=none, Location=typed, Source=foreign_serialize, SourceBinding=renamed_with_attr, SourceFalseField=absent
mod case_04 {
    use super::*;

    declare_case! {
        error: Case04,
        display: "case 04",
        context: { label: Renamed = Renamed { field_name: "x" }, filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: ForeignError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case04Snafu {
                label: Renamed { field_name: "x" },
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

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=map_like, FieldAttr=none, Location=suzu_named, Source=display_error, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_05 {
    use super::*;

    declare_case! {
        error: Case05,
        display: "case 05",
        context: { label: BTreeMap<&'static str, u32> = pairs(), #[suzu(source(false))] source: &'static str = "not-an-error" },
        metadata: { #[suzu(from)] cause: LibError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(Case05Snafu {
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

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=scalar, FieldAttr=none, Location=injected, Source=serialize_type, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_06 {
    use super::*;

    declare_case! {
        error: Case06,
        display: "case 06",
        context: { label: &'static str = "l", #[suzu(source(false))] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: InnerError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case06Snafu {
                label: "l",
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
                name: "StackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=zero, Location=injected, Source=io_error, SourceBinding=named_source, SourceFalseField=absent
mod case_07 {
    use super::*;

    declare_case! {
        error: Case07,
        display: "case 07",
        context: {  },
        metadata: { source: std::io::Error, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case07Snafu)
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=option, FieldAttr=none, Location=typed, Source=none, SourceFalseField=absent
mod case_08 {
    use super::*;

    declare_case! {
        error: Case08,
        display: "case 08",
        context: { label: Option<u32> = Some(7u32), filler: u32 = 1u32 },
        metadata: { at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case08> {
            ensure!(
                false,
                Case08Snafu {
                    label: Some(7u32),
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

/// TypeShape=struct, Generics=none, DeclaredFields=zero, Location=stack_attr, Source=serialize_type, SourceBinding=renamed_with_attr, SourceFalseField=absent
mod case_09 {
    use super::*;

    declare_case! {
        error: Case09,
        display: "case 09",
        context: {  },
        metadata: { #[suzu(source)] cause: InnerError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(Case09Snafu).unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=none, Location=suzu_named, Source=none, SourceFalseField=present
mod case_10 {
    use super::*;

    declare_case! {
        error: Case10,
        display: "case 10",
        context: { label: Detail = Detail { code: 3 }, #[suzu(source(false))] source: &'static str = "not-an-error" },
        metadata: { #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case10> {
            ensure!(
                false,
                Case10Snafu {
                    label: Detail { code: 3 },
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

/// TypeShape=struct, Generics=none, DeclaredFields=zero, Location=stack_attr, Source=none, SourceFalseField=absent
mod case_11 {
    use super::*;

    declare_case! {
        error: Case11,
        display: "case 11",
        context: {  },
        metadata: { #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case11> {
            ensure!(false, Case11Snafu);
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=scalar, FieldAttr=none, Location=typed, Source=display_error, SourceBinding=named_source, SourceFalseField=absent
mod case_12 {
    use super::*;

    declare_case! {
        error: Case12,
        display: "case 12",
        context: { label: &'static str = "l" },
        metadata: { #[suzu(from)] source: LibError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(Case12Snafu { label: "l" })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=option, FieldAttr=none, Location=stack_attr, Source=boxed, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_13 {
    use super::*;

    declare_case! {
        error: Case13,
        display: "case 13",
        context: { #[suzu(source(false))] source: Option<u32> = Some(7u32) },
        metadata: { #[suzu(source)] cause: BoxedStackError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case13Snafu { source: Some(7u32) })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "BoxedStackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=container_attributed_struct, FieldAttr=none, Location=stack_attr, Source=boxed, SourceBinding=named_source, SourceFalseField=absent
mod case_14 {
    use super::*;

    declare_case! {
        error: Case14,
        display: "case 14",
        context: { label: Renamed = Renamed { field_name: "x" }, filler: u32 = 1u32 },
        metadata: { source: BoxedStackError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case14Snafu {
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
                name: "BoxedStackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=scalar, FieldAttr=none, Location=suzu_named, Source=none, SourceFalseField=present
mod case_15 {
    use super::*;

    declare_case! {
        error: Case15,
        display: "case 15",
        context: { #[suzu(source(false))] source: &'static str = "l" },
        metadata: { #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case15> {
            ensure!(false, Case15Snafu { source: "l" });
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=scalar, FieldAttr=none, Location=stack_attr, Source=foreign_serialize, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_16 {
    use super::*;

    declare_case! {
        error: Case16,
        display: "case 16",
        context: { #[suzu(source(false))] source: &'static str = "l" },
        metadata: { #[suzu(source)] cause: ForeignError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case16Snafu { source: "l" })
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

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=scalar, FieldAttr=none, Location=typed, Source=boxed, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_17 {
    use super::*;

    declare_case! {
        error: Case17,
        display: "case 17",
        context: { #[suzu(source(false))] source: &'static str = "l" },
        metadata: { #[suzu(source)] cause: BoxedStackError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case17Snafu { source: "l" })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "BoxedStackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=scalar, FieldAttr=none, Location=typed, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_18 {
    use super::*;

    declare_case! {
        error: Case18,
        display: "case 18",
        context: { #[suzu(source(false))] source: &'static str = "l" },
        metadata: { #[suzu(source)] cause: std::io::Error, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case18Snafu { source: "l" })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=zero, Location=suzu_named, Source=foreign_serialize, SourceBinding=named_source, SourceFalseField=absent
mod case_19 {
    use super::*;

    declare_case! {
        error: Case19,
        display: "case 19",
        context: {  },
        metadata: { source: ForeignError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(Case19Snafu).unwrap_err();
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

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=container_attributed_struct, FieldAttr=none, Location=suzu_named, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=absent
mod case_20 {
    use super::*;

    declare_case! {
        error: Case20,
        display: "case 20",
        context: { label: Renamed = Renamed { field_name: "x" }, filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: std::io::Error, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case20Snafu {
                label: Renamed { field_name: "x" },
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

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=serialize_struct, FieldAttr=none, Location=typed, Source=serialize_type, SourceBinding=named_source, SourceFalseField=absent
mod case_21 {
    use super::*;

    declare_case! {
        error: Case21,
        display: "case 21",
        context: { label: Detail = Detail { code: 3 } },
        metadata: { source: InnerError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case21Snafu {
                label: Detail { code: 3 },
            })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=serialize_struct, FieldAttr=none, Location=injected, Source=foreign_serialize, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_22 {
    use super::*;

    declare_case! {
        error: Case22,
        display: "case 22",
        context: { #[suzu(source(false))] source: Detail = Detail { code: 3 } },
        metadata: { #[suzu(source)] cause: ForeignError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case22Snafu {
                source: Detail { code: 3 },
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

/// TypeShape=struct, Generics=none, DeclaredFields=zero, Location=typed, Source=display_error, SourceBinding=named_source, SourceFalseField=absent
mod case_23 {
    use super::*;

    declare_case! {
        error: Case23,
        display: "case 23",
        context: {  },
        metadata: { #[suzu(from)] source: LibError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError).context(Case23Snafu).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=map_like, FieldAttr=none, Location=typed, Source=none, SourceFalseField=absent
mod case_24 {
    use super::*;

    declare_case! {
        error: Case24,
        display: "case 24",
        context: { label: BTreeMap<&'static str, u32> = pairs() },
        metadata: { at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case24> {
            ensure!(false, Case24Snafu { label: pairs() });
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=zero, Location=injected, Source=boxed, SourceBinding=renamed_with_attr, SourceFalseField=absent
mod case_25 {
    use super::*;

    declare_case! {
        error: Case25,
        display: "case 25",
        context: {  },
        metadata: { #[suzu(source)] cause: BoxedStackError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(Case25Snafu).unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "BoxedStackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=map_like, FieldAttr=none, Location=stack_attr, Source=io_error, SourceBinding=named_source, SourceFalseField=absent
mod case_26 {
    use super::*;

    declare_case! {
        error: Case26,
        display: "case 26",
        context: { label: BTreeMap<&'static str, u32> = pairs(), filler: u32 = 1u32 },
        metadata: { source: std::io::Error, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case26Snafu {
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

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=option, FieldAttr=none, Location=injected, Source=display_error, SourceBinding=named_source, SourceFalseField=absent
mod case_27 {
    use super::*;

    declare_case! {
        error: Case27,
        display: "case 27",
        context: { label: Option<u32> = Some(7u32) },
        metadata: { #[suzu(from)] source: LibError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(Case27Snafu { label: Some(7u32) })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=map_like, FieldAttr=none, Location=injected, Source=foreign_serialize, SourceBinding=named_source, SourceFalseField=absent
mod case_28 {
    use super::*;

    declare_case! {
        error: Case28,
        display: "case 28",
        context: { label: BTreeMap<&'static str, u32> = pairs(), filler: u32 = 1u32 },
        metadata: { source: ForeignError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case28Snafu {
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

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=none, Location=stack_attr, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_29 {
    use super::*;

    declare_case! {
        error: Case29,
        display: "case 29",
        context: { label: Detail = Detail { code: 3 }, #[suzu(source(false))] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: std::io::Error, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case29Snafu {
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

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=option, FieldAttr=none, Location=suzu_named, Source=foreign_serialize, SourceBinding=named_source, SourceFalseField=absent
mod case_30 {
    use super::*;

    declare_case! {
        error: Case30,
        display: "case 30",
        context: { label: Option<u32> = Some(7u32), filler: u32 = 1u32 },
        metadata: { source: ForeignError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case30Snafu {
                label: Some(7u32),
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

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=container_attributed_struct, FieldAttr=none, Location=stack_attr, Source=display_error, SourceBinding=named_source, SourceFalseField=absent
mod case_31 {
    use super::*;

    declare_case! {
        error: Case31,
        display: "case 31",
        context: { label: Renamed = Renamed { field_name: "x" } },
        metadata: { #[suzu(from)] source: LibError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(Case31Snafu {
                label: Renamed { field_name: "x" },
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=serialize_struct, FieldAttr=none, Location=suzu_named, Source=boxed, SourceBinding=named_source, SourceFalseField=absent
mod case_32 {
    use super::*;

    declare_case! {
        error: Case32,
        display: "case 32",
        context: { label: Detail = Detail { code: 3 }, filler: u32 = 1u32 },
        metadata: { source: BoxedStackError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case32Snafu {
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
                name: "BoxedStackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=map_like, FieldAttr=none, Location=stack_attr, Source=serialize_type, SourceBinding=renamed_with_attr, SourceFalseField=absent
mod case_33 {
    use super::*;

    declare_case! {
        error: Case33,
        display: "case 33",
        context: { label: BTreeMap<&'static str, u32> = pairs() },
        metadata: { #[suzu(source)] cause: InnerError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case33Snafu { label: pairs() })
            .unwrap_err();
        let recorded = record(&error);

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(recorded.field("source").some(), &standalone);
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "StackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=one, FieldType=option, FieldAttr=none, Location=injected, Source=io_error, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_34 {
    use super::*;

    declare_case! {
        error: Case34,
        display: "case 34",
        context: { #[suzu(source(false))] source: Option<u32> = Some(7u32) },
        metadata: { #[suzu(source)] cause: std::io::Error, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case34Snafu { source: Some(7u32) })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=map_like, FieldAttr=none, Location=typed, Source=boxed, SourceBinding=renamed_with_attr, SourceFalseField=present
mod case_35 {
    use super::*;

    declare_case! {
        error: Case35,
        display: "case 35",
        context: { label: BTreeMap<&'static str, u32> = pairs(), #[suzu(source(false))] source: &'static str = "not-an-error" },
        metadata: { #[suzu(source)] cause: BoxedStackError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case35Snafu {
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
                name: "BoxedStackErrorNode",
                ..
            }
        ));
    }
}

/// TypeShape=struct, Generics=none, DeclaredFields=many, FieldType=option, FieldAttr=none, Location=suzu_named, Source=serialize_type, SourceBinding=renamed_with_attr, SourceFalseField=absent
mod case_36 {
    use super::*;

    declare_case! {
        error: Case36,
        display: "case 36",
        context: { label: Option<u32> = Some(7u32), filler: u32 = 1u32 },
        metadata: { #[suzu(source)] cause: InnerError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case36Snafu {
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
                name: "StackErrorNode",
                ..
            }
        ));
    }
}
