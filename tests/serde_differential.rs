//! Differential tests: the generated definition against a struct written by hand.
//!
//! Each case pairs an error type with the plain struct a user would have written
//! for the same declared fields — same name, same fields, same order,
//! `#[derive(Serialize)]` and nothing else. `context` must record identically
//! through both. The hand-written side carries no attribute that lines it up
//! with us; if it needed one, that would be an expectation smuggled into the
//! oracle.
//!
//! These are written out by hand on purpose. Once the case table is lifted into
//! a macro driven by factor levels, a handful of hand-written cases stay behind
//! to keep watch on the macro's own assembly.

#![cfg(feature = "serde")]

#[macro_use]
mod support;

use support::{Record, error_node, record};
use suzunari_error::*;

/// Declared fields, and a cause.
mod declared_fields_with_source {
    use super::*;

    declare_case! {
        error: LookupError,
        display: "lookup failed for {key}",
        context: { key: String = "k".to_owned(), attempts: u32 = 3 },
        metadata: { source: std::io::Error, },
    }

    #[test]
    fn context_matches_a_directly_written_struct() {
        let error = std::fs::read("/nonexistent-suzunari-error")
            .context(LookupSnafu {
                key: "k",
                attempts: 3u32,
            })
            .unwrap_err();

        assert_context_matches(&error);
    }
}

/// Declared fields, no cause. Neither the earlier JSON tests nor the token ones
/// covered this combination.
mod declared_fields_without_source {
    use super::*;

    #[suzunari_error(serialize)]
    #[suzu(display("{field} is out of range"))]
    struct ValidationError {
        field: String,
        limit: u32,
    }

    mod oracle {
        #[derive(serde::Serialize)]
        pub struct ValidationError {
            pub field: String,
            pub limit: u32,
        }
    }

    fn validate() -> Result<(), ValidationError> {
        ensure!(
            false,
            ValidationSnafu {
                field: "age",
                limit: 120u32,
            }
        );
        Ok(())
    }

    #[test]
    fn context_matches_a_directly_written_struct() {
        let error = validate().unwrap_err();

        let expected = oracle::ValidationError {
            field: "age".to_owned(),
            limit: 120,
        };

        assert_eq!(record(&error).field("context"), &record(&expected));
    }
}

/// No declared fields, but a cause. The `context` is an empty struct, and it
/// still has to match one the user wrote — including the announced field count.
mod no_declared_fields_with_source {
    use super::*;

    #[suzunari_error(serialize)]
    #[suzu(display("wrapped"))]
    struct WrapError {
        source: std::io::Error,
    }

    mod oracle {
        #[derive(serde::Serialize)]
        pub struct WrapError {}
    }

    #[test]
    fn context_matches_a_directly_written_struct() {
        let error = std::fs::read("/nonexistent-suzunari-error")
            .context(WrapSnafu)
            .unwrap_err();

        assert_eq!(
            record(&error).field("context"),
            &record(&oracle::WrapError {})
        );
    }
}

/// The location field under a name of the user's choosing. It belongs to the
/// metadata level whatever it is called, so it must not reach `context`.
mod custom_location_name {
    use super::*;

    #[suzunari_error(serialize)]
    #[suzu(display("failed at {at}"))]
    struct CustomLocationError {
        label: String,
        #[suzu(location)]
        at: Location,
    }

    mod oracle {
        #[derive(serde::Serialize)]
        pub struct CustomLocationError {
            pub label: String,
        }
    }

    fn fail() -> Result<(), CustomLocationError> {
        ensure!(false, CustomLocationSnafu { label: "l" });
        Ok(())
    }

    #[test]
    fn context_matches_a_directly_written_struct() {
        let error = fail().unwrap_err();

        let expected = oracle::CustomLocationError {
            label: "l".to_owned(),
        };

        assert_eq!(record(&error).field("context"), &record(&expected));
    }
}

// ---------------------------------------------------------------------------
// `source`, one case per kind of field type
//
// Two rules cover them all. When the field's type is one of ours, the nested
// result must equal serializing that value on its own. When it is not, the
// chain continues as a phase 2 node carrying only a message — and for a foreign
// type that happens to derive `Serialize`, the nested result must specifically
// *not* be its own serialization, or its fields would have replaced the node.
// ---------------------------------------------------------------------------

/// The source field is another opted-in error type.
mod source_is_a_serialize_type {
    use super::*;

    #[suzunari_error(serialize)]
    #[suzu(display("inner failed"))]
    struct InnerError {
        detail: String,
    }

    #[suzunari_error(serialize)]
    #[suzu(display("outer failed"))]
    struct OuterError {
        source: InnerError,
    }

    fn inner() -> Result<(), InnerError> {
        ensure!(false, InnerSnafu { detail: "d" });
        Ok(())
    }

    #[test]
    fn nesting_matches_serializing_the_source_on_its_own() {
        let cause = inner().unwrap_err();
        let standalone = record(&cause);
        let outer = Err::<(), _>(cause).context(OuterSnafu).unwrap_err();

        assert_eq!(record(&outer).field("source").some(), &standalone);
    }
}

/// The source field is `BoxedStackError`, which is also one of ours.
mod source_is_boxed {
    use super::*;

    #[suzunari_error(serialize)]
    #[suzu(display("inner failed"))]
    struct InnerError {
        detail: String,
    }

    #[suzunari_error(serialize)]
    #[suzu(display("outer failed"))]
    struct OuterError {
        source: BoxedStackError,
    }

    fn inner() -> Result<(), InnerError> {
        ensure!(false, InnerSnafu { detail: "d" });
        Ok(())
    }

    #[test]
    fn nesting_matches_serializing_the_source_on_its_own() {
        let boxed = BoxedStackError::new(inner().unwrap_err());
        let standalone = record(&boxed);
        let outer = Err::<(), _>(boxed).context(OuterSnafu).unwrap_err();
        let recorded = record(&outer);

        assert_eq!(recorded.field("source").some(), &standalone);
        // The concrete type is gone at this boundary, so the nested node has no
        // context even though the value behind it declared a field.
        assert!(matches!(
            recorded.field("source").some(),
            Record::Struct {
                name: "BoxedStackErrorNode",
                ..
            }
        ));
    }
}

/// The source field is a foreign error with no `Serialize` impl at all.
mod source_is_a_foreign_error {
    use super::*;

    #[suzunari_error(serialize)]
    #[suzu(display("read failed"))]
    struct ReadError {
        source: std::io::Error,
    }

    #[test]
    fn the_chain_continues_as_a_phase_two_node() {
        let error = std::fs::read("/nonexistent-suzunari-error")
            .context(ReadSnafu)
            .unwrap_err();

        let expected = std::io::Error::from_raw_os_error(2).to_string();
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&expected)
        );
    }
}

/// The source field is a foreign error that happens to derive `Serialize`.
mod source_is_a_foreign_serialize_type {
    use super::*;

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
    #[suzu(display("wrapped"))]
    struct WrapError {
        source: ForeignError,
    }

    #[test]
    fn the_foreign_impl_does_not_replace_the_node() {
        let cause = ForeignError { code: 7 };
        let standalone = record(&cause);
        let error = Err::<(), _>(cause).context(WrapSnafu).unwrap_err();
        let recorded = record(&error);
        let source = recorded.field("source").some();

        assert_eq!(source, &error_node("foreign error 7"));
        // Had dispatch keyed on `Serialize` instead of the crate's marker, this
        // is what would have landed in the payload.
        assert_ne!(source, &standalone);
    }
}

/// The source field went through `#[suzu(from)]`, so its type is
/// `DisplayError<T>`. That type deliberately has no `Serialize` impl, so it
/// must take the same path as any other foreign error.
mod source_is_a_display_error {
    use super::*;

    #[derive(Debug)]
    struct LibError;

    impl std::fmt::Display for LibError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("lib error")
        }
    }

    #[suzunari_error(serialize)]
    #[suzu(display("adapted"))]
    struct AdaptError {
        #[suzu(from)]
        source: LibError,
    }

    #[test]
    fn the_chain_continues_as_a_phase_two_node() {
        let error = Err::<(), _>(LibError).context(AdaptSnafu).unwrap_err();

        assert_eq!(
            record(&error).field("source").some(),
            &error_node("lib error")
        );
    }
}

/// No source field at all.
mod no_source {
    use super::*;

    #[suzunari_error(serialize)]
    #[suzu(display("leaf"))]
    struct LeafError {
        detail: String,
    }

    fn leaf() -> Result<(), LeafError> {
        ensure!(false, LeafSnafu { detail: "d" });
        Ok(())
    }

    #[test]
    fn the_key_is_absent_rather_than_null() {
        let record = record(&leaf().unwrap_err());
        let Record::Struct { fields, .. } = &record else {
            panic!("expected a struct record");
        };
        assert!(!fields.iter().any(|(name, _)| *name == "source"));
    }
}
