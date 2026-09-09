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
                name: "TypeErasedStackError",
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

/// A declared field holding a struct of the user's own that derives
/// `Serialize`.
///
/// Unlike the source field, a declared field has no dispatch: whatever it holds
/// serializes through its own impl, which is the whole point of `context`. The
/// "a foreign `Serialize` impl must not replace the node" rule applies to
/// `source` alone, where replacing the node would cut the chain short.
mod declared_field_is_a_serialize_struct {
    use super::*;

    // `Debug` because `#[suzunari_error]` derives it on the error type, which
    // is a requirement the crate already had; `Serialize` is what `context`
    // needs.
    #[derive(Debug, serde::Serialize)]
    struct Request {
        method: String,
        path: String,
    }

    #[suzunari_error(serialize)]
    #[suzu(display("request rejected"))]
    struct RejectedError {
        request: Request,
        retries: u32,
    }

    mod oracle {
        use super::Request;

        #[derive(serde::Serialize)]
        pub struct RejectedError {
            pub request: Request,
            pub retries: u32,
        }
    }

    fn request() -> Request {
        Request {
            method: "GET".to_owned(),
            path: "/v1".to_owned(),
        }
    }

    fn reject() -> Result<(), RejectedError> {
        ensure!(
            false,
            RejectedSnafu {
                request: request(),
                retries: 2u32,
            }
        );
        Ok(())
    }

    #[test]
    fn it_nests_through_its_own_impl() {
        let error = reject().unwrap_err();

        let expected = oracle::RejectedError {
            request: request(),
            retries: 2,
        };

        assert_eq!(record(&error).field("context"), &record(&expected));

        // And it really is nested, not flattened into `context`.
        let recorded = record(&error);
        let nested = recorded.field("context").field("request");
        assert!(matches!(
            nested,
            Record::Struct {
                name: "Request",
                ..
            }
        ));
    }
}

/// Container-level serde attributes on a struct used as a declared field.
///
/// They apply normally. The type is the user's own and carries its own
/// `#[derive(Serialize)]`, so nothing about it passes through the generated
/// definition — the definition only names the field and lets the type's own
/// impl run. The rule that rejects container attributes on the error type is
/// about the definition being a *different container* than the one annotated,
/// which does not arise here.
mod container_attributes_on_a_nested_struct {
    use super::*;

    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Renamed {
        request_id: String,
        retry_count: u32,
    }

    /// `transparent` makes the struct serialize as its single field, so the
    /// declared field becomes a bare string rather than an object.
    #[derive(Debug, serde::Serialize)]
    #[serde(transparent)]
    struct Wrapped {
        inner: String,
    }

    #[suzunari_error(serialize)]
    #[suzu(display("nested attributes"))]
    struct NestedError {
        renamed: Renamed,
        wrapped: Wrapped,
    }

    mod oracle {
        use super::{Renamed, Wrapped};

        #[derive(serde::Serialize)]
        pub struct NestedError {
            pub renamed: Renamed,
            pub wrapped: Wrapped,
        }
    }

    fn parts() -> (Renamed, Wrapped) {
        (
            Renamed {
                request_id: "r1".to_owned(),
                retry_count: 2,
            },
            Wrapped {
                inner: "w".to_owned(),
            },
        )
    }

    fn fail() -> Result<(), NestedError> {
        let (renamed, wrapped) = parts();
        ensure!(false, NestedSnafu { renamed, wrapped });
        Ok(())
    }

    #[test]
    fn they_apply_exactly_as_they_would_on_their_own() {
        let error = fail().unwrap_err();
        let (renamed, wrapped) = parts();

        let expected = oracle::NestedError { renamed, wrapped };
        assert_eq!(record(&error).field("context"), &record(&expected));

        // rename_all reached the nested field names.
        let recorded = record(&error);
        let nested = recorded.field("context").field("renamed");
        assert!(nested.has_field("requestId"));
        assert!(!nested.has_field("request_id"));

        // transparent collapsed the struct to its single field.
        assert_eq!(
            recorded.field("context").field("wrapped"),
            &Record::Str("w".to_owned())
        );
    }
}

/// A field named `source` that is not the source, alongside a real source under
/// another name.
///
/// This is the combination that a name-based shortcut gets wrong: codegen has to
/// use snafu's own rule, so that the `source(false)` field lands in `context`
/// and the renamed one continues the chain.
///
/// It is also a live example of why the payload nests rather than flattens.
/// Both keys are called `source`; they do not collide only because they sit at
/// different levels. Flattened, one would have silently overwritten the other.
mod source_named_field_that_is_not_the_source {
    use super::*;

    #[suzunari_error(serialize)]
    #[suzu(display("both kinds of source"))]
    struct BothError {
        #[suzu(source(false))]
        source: String,
        #[suzu(source)]
        cause: std::io::Error,
    }

    mod oracle {
        #[derive(serde::Serialize)]
        pub struct BothError {
            pub source: String,
        }
    }

    #[test]
    fn the_declared_one_stays_in_context_and_the_renamed_one_continues_the_chain() {
        let error = std::fs::read("/nonexistent-suzunari-error")
            .context(BothSnafu {
                source: "not-an-error".to_owned(),
            })
            .unwrap_err();

        let expected = oracle::BothError {
            source: "not-an-error".to_owned(),
        };
        assert_eq!(record(&error).field("context"), &record(&expected));

        let expected_tail = std::io::Error::from_raw_os_error(2).to_string();
        assert_eq!(
            record(&error).field("source").some(),
            &error_node(&expected_tail)
        );
    }
}

/// `serialize(rename_all = …)` renames the declared fields, and nothing else.
///
/// serde spells the same intent differently by shape — `rename_all` on a
/// struct, `rename_all_fields` on an enum, where plain `rename_all` renames
/// variants that `untagged` never emits. Measured, and the reason the option
/// stays one spelling.
mod rename_all {
    use super::*;

    #[suzunari_error(serialize(rename_all = "camelCase"))]
    #[suzu(display("struct case"))]
    struct StructError {
        file_path: &'static str,
        nested: Plain,
    }

    /// A field type of the user's own, with no case setting. Its keys must not
    /// be touched: `rename_all` is per container, and this is another one.
    #[derive(Debug, serde::Serialize)]
    struct Plain {
        inner_field: u32,
    }

    #[suzunari_error(serialize(rename_all = "camelCase"))]
    enum EnumError {
        #[suzu(display("read"))]
        ReadFailed { file_path: &'static str },
        #[suzu(display("missing"))]
        Missing,
    }

    fn struct_error() -> StructError {
        fn failing() -> Result<(), StructError> {
            ensure!(
                false,
                StructSnafu {
                    file_path: "/p",
                    nested: Plain { inner_field: 1 },
                }
            );
            Ok(())
        }
        failing().unwrap_err()
    }

    #[test]
    fn it_renames_declared_fields_only() {
        let recorded = record(&struct_error());
        let context = recorded.field("context");

        assert!(context.has_field("filePath"));
        assert!(!context.has_field("file_path"));
        // The nested type is a container of its own and keeps its own names.
        assert!(context.field("nested").has_field("inner_field"));

        // The envelope is untouched. Its keys are fields of `StackErrorNode`,
        // which no user attribute reaches.
        for key in ["type", "message", "location", "context"] {
            assert!(recorded.has_field(key), "missing {key}");
        }
        assert!(recorded.field("location").has_field("file"));
    }

    #[test]
    fn an_enum_renames_variant_fields_not_variants() {
        fn read() -> Result<(), EnumError> {
            ensure!(false, ReadFailedSnafu { file_path: "/p" });
            Ok(())
        }
        fn missing() -> Result<(), EnumError> {
            ensure!(false, MissingSnafu);
            Ok(())
        }

        let read = record(&read().unwrap_err());
        assert!(read.field("context").has_field("filePath"));
        // The variant reaches the payload through `type`, which is not
        // renameable — it comes from `type_name()`.
        assert_eq!(
            read.field("type"),
            &Record::Str("EnumError::ReadFailed".to_owned())
        );

        // A variant that declares nothing still gets an empty context.
        let missing = record(&missing().unwrap_err());
        assert_eq!(
            missing.field("context"),
            &Record::Struct {
                name: "EnumError",
                announced_len: 0,
                fields: Vec::new(),
            }
        );
    }
}

/// A parameter is bound by `Serialize` only where a declared field uses it.
///
/// Bounding every parameter would reject ordinary types: one that names the
/// source's type is normal, the source is skipped in the definition, and
/// `io::Error` is the commonest thing to put there. This is the inference serde
/// makes for its own derive.
mod parameter_bounds {
    use super::*;

    /// The parameter names the source's type and nothing else.
    #[suzunari_error(serialize)]
    #[suzu(display("wrapping {}", core::any::type_name::<T>()))]
    struct WrapError<T: core::fmt::Debug + core::error::Error + 'static> {
        source: T,
    }

    /// The parameter is a declared field the definition skips.
    #[suzunari_error(serialize)]
    #[suzu(display("skipped"))]
    struct SkipError<T: core::fmt::Debug> {
        #[serde(skip)]
        value: T,
    }

    /// Neither of the above can serialize, and both have to work anyway.
    #[derive(Debug)]
    struct NotSerialize;

    #[test]
    fn a_parameter_only_the_source_uses_needs_none() {
        let error = std::fs::read("/nonexistent-suzunari-error")
            .context(WrapSnafu)
            .unwrap_err();
        let recorded = record(&error);

        assert!(recorded.field("context").as_empty_struct());
        assert_eq!(
            recorded.field("source").some(),
            &error_node(&std::io::Error::from_raw_os_error(2).to_string())
        );
    }

    #[test]
    fn a_skipped_declared_field_needs_none_either() {
        fn failing() -> Result<(), SkipError<NotSerialize>> {
            ensure!(
                false,
                SkipSnafu {
                    value: NotSerialize
                }
            );
            Ok(())
        }

        let recorded = record(&failing().unwrap_err());
        assert!(recorded.field("context").as_empty_struct());
    }
}
