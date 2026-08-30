//! Pairwise cases, generated from `tests/support/differential-cases.pict`.
//!
//! Do not edit by hand. Regenerate with:
//!
//! ```text
//! python3 tests/support/generate-cases.py tests/support/differential-cases.pict //!     > tests/serde_differential_pairwise.rs
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

/// DeclaredFields=some, FieldType=concrete, Source=none, Location=stack_attr
mod case_01 {
    use super::*;

    declare_case! {
        error: Case01,
        display: "case 01",
        context: { label: String = "l".to_owned(), count: u32 = 7 },
        metadata: { #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case01> {
            ensure!(
                false,
                Case01Snafu {
                    label: "l",
                    count: 7u32
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// DeclaredFields=some, FieldType=option, Source=serialize_type, Location=typed
mod case_02 {
    use super::*;

    declare_case! {
        error: Case02,
        display: "case 02",
        context: { label: Option<String> = Some("l".to_owned()), count: Option<u32> = None },
        metadata: { source: InnerError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case02Snafu {
                label: Some("l".to_owned()),
                count: None::<u32>,
            })
            .unwrap_err();

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(record(&error).field("source").some(), &standalone);
    }
}

/// DeclaredFields=none, FieldType=na, Source=io_error, Location=stack_attr
mod case_03 {
    use super::*;

    declare_case! {
        error: Case03,
        display: "case 03",
        context: {  },
        metadata: { source: std::io::Error, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case03Snafu)
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// DeclaredFields=some, FieldType=concrete, Source=foreign_serialize, Location=injected
mod case_04 {
    use super::*;

    declare_case! {
        error: Case04,
        display: "case 04",
        context: { label: String = "l".to_owned(), count: u32 = 7 },
        metadata: { source: ForeignError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case04Snafu {
                label: "l",
                count: 7u32,
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

/// DeclaredFields=some, FieldType=option, Source=io_error, Location=injected
mod case_05 {
    use super::*;

    declare_case! {
        error: Case05,
        display: "case 05",
        context: { label: Option<String> = Some("l".to_owned()), count: Option<u32> = None },
        metadata: { source: std::io::Error, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case05Snafu {
                label: Some("l".to_owned()),
                count: None::<u32>,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// DeclaredFields=none, FieldType=na, Source=none, Location=suzu_named
mod case_06 {
    use super::*;

    declare_case! {
        error: Case06,
        display: "case 06",
        context: {  },
        metadata: { #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case06> {
            ensure!(false, Case06Snafu);
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// DeclaredFields=none, FieldType=na, Source=serialize_type, Location=stack_attr
mod case_07 {
    use super::*;

    declare_case! {
        error: Case07,
        display: "case 07",
        context: {  },
        metadata: { source: InnerError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(Case07Snafu).unwrap_err();

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(record(&error).field("source").some(), &standalone);
    }
}

/// DeclaredFields=some, FieldType=concrete, Source=display_error, Location=suzu_named
mod case_08 {
    use super::*;

    declare_case! {
        error: Case08,
        display: "case 08",
        context: { label: String = "l".to_owned(), count: u32 = 7 },
        metadata: { #[suzu(from)] source: LibError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(Case08Snafu {
                label: "l",
                count: 7u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// DeclaredFields=some, FieldType=option, Source=serialize_type, Location=suzu_named
mod case_09 {
    use super::*;

    declare_case! {
        error: Case09,
        display: "case 09",
        context: { label: Option<String> = Some("l".to_owned()), count: Option<u32> = None },
        metadata: { source: InnerError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case09Snafu {
                label: Some("l".to_owned()),
                count: None::<u32>,
            })
            .unwrap_err();

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(record(&error).field("source").some(), &standalone);
    }
}

/// DeclaredFields=some, FieldType=option, Source=foreign_serialize, Location=suzu_named
mod case_10 {
    use super::*;

    declare_case! {
        error: Case10,
        display: "case 10",
        context: { label: Option<String> = Some("l".to_owned()), count: Option<u32> = None },
        metadata: { source: ForeignError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case10Snafu {
                label: Some("l".to_owned()),
                count: None::<u32>,
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

/// DeclaredFields=some, FieldType=option, Source=display_error, Location=stack_attr
mod case_11 {
    use super::*;

    declare_case! {
        error: Case11,
        display: "case 11",
        context: { label: Option<String> = Some("l".to_owned()), count: Option<u32> = None },
        metadata: { #[suzu(from)] source: LibError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError)
            .context(Case11Snafu {
                label: Some("l".to_owned()),
                count: None::<u32>,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// DeclaredFields=none, FieldType=na, Source=display_error, Location=injected
mod case_12 {
    use super::*;

    declare_case! {
        error: Case12,
        display: "case 12",
        context: {  },
        metadata: { #[suzu(from)] source: LibError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError).context(Case12Snafu).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// DeclaredFields=some, FieldType=concrete, Source=boxed, Location=suzu_named
mod case_13 {
    use super::*;

    declare_case! {
        error: Case13,
        display: "case 13",
        context: { label: String = "l".to_owned(), count: u32 = 7 },
        metadata: { source: BoxedStackError, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case13Snafu {
                label: "l",
                count: 7u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(record(&error).field("source").some(), &standalone);
    }
}

/// DeclaredFields=none, FieldType=na, Source=boxed, Location=injected
mod case_14 {
    use super::*;

    declare_case! {
        error: Case14,
        display: "case 14",
        context: {  },
        metadata: { source: BoxedStackError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(Case14Snafu).unwrap_err();

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(record(&error).field("source").some(), &standalone);
    }
}

/// DeclaredFields=some, FieldType=concrete, Source=serialize_type, Location=injected
mod case_15 {
    use super::*;

    declare_case! {
        error: Case15,
        display: "case 15",
        context: { label: String = "l".to_owned(), count: u32 = 7 },
        metadata: { source: InnerError, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = inner_error();
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case15Snafu {
                label: "l",
                count: 7u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(record(&error).field("source").some(), &standalone);
    }
}

/// DeclaredFields=some, FieldType=concrete, Source=io_error, Location=suzu_named
mod case_16 {
    use super::*;

    declare_case! {
        error: Case16,
        display: "case 16",
        context: { label: String = "l".to_owned(), count: u32 = 7 },
        metadata: { source: std::io::Error, #[suzu(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case16Snafu {
                label: "l",
                count: 7u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// DeclaredFields=none, FieldType=na, Source=io_error, Location=typed
mod case_17 {
    use super::*;

    declare_case! {
        error: Case17,
        display: "case 17",
        context: {  },
        metadata: { source: std::io::Error, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = std::fs::read(MISSING_PATH)
            .context(Case17Snafu)
            .unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&io_message())
        );
    }
}

/// DeclaredFields=some, FieldType=concrete, Source=none, Location=typed
mod case_18 {
    use super::*;

    declare_case! {
        error: Case18,
        display: "case 18",
        context: { label: String = "l".to_owned(), count: u32 = 7 },
        metadata: { at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case18> {
            ensure!(
                false,
                Case18Snafu {
                    label: "l",
                    count: 7u32
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// DeclaredFields=none, FieldType=na, Source=foreign_serialize, Location=typed
mod case_19 {
    use super::*;

    declare_case! {
        error: Case19,
        display: "case 19",
        context: {  },
        metadata: { source: ForeignError, at: Location, },
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

/// DeclaredFields=some, FieldType=option, Source=boxed, Location=stack_attr
mod case_20 {
    use super::*;

    declare_case! {
        error: Case20,
        display: "case 20",
        context: { label: Option<String> = Some("l".to_owned()), count: Option<u32> = None },
        metadata: { source: BoxedStackError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case20Snafu {
                label: Some("l".to_owned()),
                count: None::<u32>,
            })
            .unwrap_err();

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(record(&error).field("source").some(), &standalone);
    }
}

/// DeclaredFields=some, FieldType=option, Source=none, Location=injected
mod case_21 {
    use super::*;

    declare_case! {
        error: Case21,
        display: "case 21",
        context: { label: Option<String> = Some("l".to_owned()), count: Option<u32> = None },
        metadata: {  },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        fn failing() -> Result<(), Case21> {
            ensure!(
                false,
                Case21Snafu {
                    label: Some("l".to_owned()),
                    count: None::<u32>
                }
            );
            Ok(())
        }
        let error = failing().unwrap_err();

        assert_context_matches(&error);
        assert!(!has_source(&record(&error)));
    }
}

/// DeclaredFields=none, FieldType=na, Source=foreign_serialize, Location=stack_attr
mod case_22 {
    use super::*;

    declare_case! {
        error: Case22,
        display: "case 22",
        context: {  },
        metadata: { source: ForeignError, #[stack(location)] at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(Case22Snafu).unwrap_err();
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

/// DeclaredFields=some, FieldType=option, Source=boxed, Location=typed
mod case_23 {
    use super::*;

    declare_case! {
        error: Case23,
        display: "case 23",
        context: { label: Option<String> = Some("l".to_owned()), count: Option<u32> = None },
        metadata: { source: BoxedStackError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let cause = BoxedStackError::new(inner_error());
        let standalone = record(&cause);
        let error = Err::<(), _>(cause)
            .context(Case23Snafu {
                label: Some("l".to_owned()),
                count: None::<u32>,
            })
            .unwrap_err();

        assert_context_matches(&error);
        // The source is one of ours, so nesting it must not change it.
        assert_eq!(record(&error).field("source").some(), &standalone);
    }
}

/// DeclaredFields=none, FieldType=na, Source=display_error, Location=typed
mod case_24 {
    use super::*;

    declare_case! {
        error: Case24,
        display: "case 24",
        context: {  },
        metadata: { #[suzu(from)] source: LibError, at: Location, },
    }

    #[test]
    fn matches_the_hand_written_equivalent() {
        let error = Err::<(), _>(LibError).context(Case24Snafu).unwrap_err();

        assert_context_matches(&error);
        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}
