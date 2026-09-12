//! serde `Serialize` support for error chains.
//!
//! **Not public API. Do not use.** This module is `#[doc(hidden)]` and not
//! covered by semver guarantees. It exists solely for generated code and for
//! this crate's own `Serialize` impls.
//!
//! # Three kinds of error node
//!
//! What an error node can say depends on what is reachable at that level of the
//! chain, which is the same thing that decides what
//! [`StackReport`](crate::StackReport) can print there:
//!
//! - [`StackErrorNode`] — the concrete type is known: `type`, `message`,
//!   `location`, `context`.
//! - [`TypeErasedStackErrorNode`] — only `&dyn StackError` is available, so the same
//!   minus `context`: the declared fields cannot be read.
//! - [`PlainErrorNode`] — not a `StackError` at all, so `message` alone.
//!
//! An absent `context` and an empty one mean different things, and `type`
//! cannot separate them: an erased error node still has a type name, because
//! the erasure forwards `type_name()` to the value it holds.
//!
//! # Two shapes to emit them in
//!
//! The three tell each other apart by which keys are present, which only works
//! where the format carries field names. So each has a sparse shape for those
//! formats — `SparseStackError`, `SparseTypeErasedStackError`, `SparsePlainError` —
//! and all three share `UniformError` for the rest, where every key is written
//! and an absent one carries `None`.
//!
//! [`Serializer::is_human_readable`] makes the choice. serde's own impls use it
//! the same way, and every binary format measured reports `false`: a
//! self-describing binary format therefore gets the uniform shape too, at the
//! cost of a few entries.
//!
//! Only the choice is written by hand; every shape is derived. A hand-written
//! impl must also call `skip_field` for every field it omits, and forgetting
//! that is invisible in JSON.

use crate::{Location, StackError};
use core::error::Error;
use core::fmt::Display;
use serde::{Serialize, Serializer};

// ---------------------------------------------------------------------------
// SerializeDisplay — a value's Display output
// ---------------------------------------------------------------------------

/// Serializes a value's `Display` output as a string.
///
/// `E` stays generic rather than collapsing to `&dyn Display` because
/// upcasting `dyn StackError` to `dyn Display` needs trait upcasting, which
/// stabilized in Rust 1.86 — later than this crate's MSRV of 1.85.
pub struct SerializeDisplay<'a, E: ?Sized>(pub &'a E);

impl<E: ?Sized + Display> Serialize for SerializeDisplay<'_, E> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // `collect_str` lets a serializer write directly instead of going
        // through `to_string()`. It does not guarantee no allocation occurs —
        // serde's own default implementation allocates — only that this crate
        // does not force one.
        serializer.collect_str(self.0)
    }
}

// ---------------------------------------------------------------------------
// Location — serialized through a serde `remote` definition
// ---------------------------------------------------------------------------

/// serde `remote` definition for [`core::panic::Location`].
///
/// This is `remote`'s on-label use: the fields are private and reachable only
/// through `file()` / `line()` / `column()`, which is what `getter` is for.
///
/// The `rename` is required: a def's own identifier is what reaches
/// `serialize_struct` as the struct name, so without it the data model would
/// carry `LocationDef`.
#[derive(Serialize)]
#[serde(remote = "core::panic::Location", rename = "Location")]
struct LocationDef<'a> {
    #[serde(getter = "core::panic::Location::file")]
    file: &'a str,
    #[serde(getter = "core::panic::Location::line")]
    line: u32,
    #[serde(getter = "core::panic::Location::column")]
    column: u32,
}

/// Bridges the [`Location`] alias — which is itself a reference — to
/// [`LocationDef`], whose generated `serialize` takes `&core::panic::Location`.
///
/// A named type rather than a `serialize_with` function, because the sparse
/// shapes carry a location and the uniform one carries `Option<_>`; a
/// `serialize_with` would have to be written twice, once per level of `Option`.
struct SerializeLocation(Location);

impl Serialize for SerializeLocation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        LocationDef::serialize(self.0, serializer)
    }
}

// ---------------------------------------------------------------------------
// Error node shapes
// ---------------------------------------------------------------------------

/// Phase 1 error node for an error whose concrete type is known.
///
/// `context` is not optional here. A concrete type always has one — empty when
/// it declares no fields of its own — and making that a type-level fact is what
/// keeps it from being conflated with [`TypeErasedStackErrorNode`], where the fields
/// exist but are unreachable.
pub struct StackErrorNode<M, C, Src> {
    /// `StackError::type_name()` — `"Type"` or `"Enum::Variant"`.
    pub type_name: &'static str,
    /// The error's `Display` output.
    pub message: M,
    /// Where the error was constructed.
    pub location: Location,
    /// The type's declared fields.
    pub context: C,
    /// The next error node in the chain. `None` when there is no cause.
    pub source: Option<Src>,
}

impl<M: Serialize, C: Serialize, Src: Serialize> Serialize for StackErrorNode<M, C, Src> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            SparseStackError {
                type_name: self.type_name,
                message: &self.message,
                location: SerializeLocation(self.location),
                context: &self.context,
                source: self.source.as_ref(),
            }
            .serialize(serializer)
        } else {
            UniformError {
                type_name: Some(self.type_name),
                message: &self.message,
                location: Some(SerializeLocation(self.location)),
                context: Some(&self.context),
                source: self.source.as_ref(),
            }
            .serialize(serializer)
        }
    }
}

/// Phase 1 error node for an error reached through a type-erased boundary.
///
/// Identical to [`StackErrorNode`] minus `context`: only `&dyn StackError` is
/// available, so the declared fields cannot be read. It carries no type
/// parameters because both the message and the continuation are fixed by that.
pub struct TypeErasedStackErrorNode<'a> {
    /// `StackError::type_name()`, forwarded from the value behind the erasure —
    /// so this is the wrapped error's name, never the wrapper's.
    pub type_name: &'static str,
    /// The error's `Display` output.
    pub message: SerializeDisplay<'a, dyn StackError + 'a>,
    /// Where the error was constructed.
    pub location: Location,
    /// The next error node in the chain. `None` when there is no cause.
    pub source: Option<NextErrorNode<'a>>,
}

impl Serialize for TypeErasedStackErrorNode<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            SparseTypeErasedStackError {
                type_name: self.type_name,
                message: &self.message,
                location: SerializeLocation(self.location),
                source: self.source.as_ref(),
            }
            .serialize(serializer)
        } else {
            UniformError {
                type_name: Some(self.type_name),
                message: &self.message,
                location: Some(SerializeLocation(self.location)),
                // The fields exist but are unreachable, which the uniform shape
                // says the same way an error node with no fields at all would.
                // The distinction JSON draws by omitting the key is lost here; a
                // reader of a fixed layout has no key to look for either way.
                context: NO_CONTEXT,
                source: self.source.as_ref(),
            }
            .serialize(serializer)
        }
    }
}

/// Phase 2 error node: a plain [`Error`] tail, with no location information.
pub struct PlainErrorNode<'a> {
    /// The error's `Display` output.
    pub message: SerializeDisplay<'a, dyn Error + 'static>,
    /// The next error node in the chain. `None` when there is no cause.
    pub source: Option<DynError<'a>>,
}

impl Serialize for PlainErrorNode<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            SparsePlainError {
                message: &self.message,
                source: self.source.as_ref(),
            }
            .serialize(serializer)
        } else {
            UniformError {
                type_name: None,
                message: &self.message,
                location: None,
                context: NO_CONTEXT,
                source: self.source.as_ref(),
            }
            .serialize(serializer)
        }
    }
}

// ---------------------------------------------------------------------------
// Emitted shapes
// ---------------------------------------------------------------------------
//
// The error nodes above decide which of these to fill in; these are what
// actually reach the serializer. Both sets are derived.
//
// The sparse three are renamed so that what reaches the serializer names the
// error node kind rather than the encoding. `UniformError` needs no rename: one
// struct serves all three kinds.
//
// No data format measured writes a struct name out — not JSON, and not the
// binary ones either. What observes these names is this crate's own test
// serializers: the token comparison, and the fixed-buffer one the core-only tier
// uses, which records the name so a wrong one cannot pass unnoticed.

/// What a [`StackErrorNode`] emits to a self-describing format.
#[derive(Serialize)]
#[serde(rename = "StackError")]
struct SparseStackError<'a, M, C, Src> {
    #[serde(rename = "type")]
    type_name: &'static str,
    message: &'a M,
    location: SerializeLocation,
    context: &'a C,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a Src>,
}

/// What a [`TypeErasedStackErrorNode`] emits to a self-describing format.
#[derive(Serialize)]
#[serde(rename = "TypeErasedStackError")]
struct SparseTypeErasedStackError<'a, M, Src> {
    #[serde(rename = "type")]
    type_name: &'static str,
    message: &'a M,
    location: SerializeLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a Src>,
}

/// What a [`PlainErrorNode`] emits to a self-describing format.
#[derive(Serialize)]
#[serde(rename = "PlainError")]
struct SparsePlainError<'a, M, Src> {
    message: &'a M,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a Src>,
}

/// What every error node emits to a format that is not self-describing.
///
/// One shape for all three, with the absent parts as `None`. A format with no
/// field names cannot express a key that is sometimes missing: a reader advances
/// by type, so it has to know how many fields to expect before it reads them.
/// Every `Option` here writes its own discriminant, which is exactly what such a
/// reader needs.
///
/// Making the three shapes identical also settles the phase boundary.
/// `NextErrorNode` is `untagged`, so its variants contribute no discriminant of
/// their own — but with one shape they no longer need to, because both variants
/// lay out the same five fields. A reader tells them apart by `type` being
/// absent.
#[derive(Serialize)]
struct UniformError<'a, M, C, Src> {
    #[serde(rename = "type")]
    type_name: Option<&'static str>,
    message: &'a M,
    location: Option<SerializeLocation>,
    context: Option<&'a C>,
    source: Option<&'a Src>,
}

/// `context` for an error node that has none, at the type the uniform shape
/// expects.
///
/// The `Option` is always `None`; the parameter only has to implement
/// `Serialize`, which is all `()` is for.
const NO_CONTEXT: Option<&()> = None;

/// Which error node shape the chain continues with, when only a trait object is
/// available to decide.
///
/// `untagged` so the variant contributes no wrapper of its own — the payload
/// carries the error node directly.
#[derive(Serialize)]
#[serde(untagged)]
pub enum NextErrorNode<'a> {
    /// The cause implements `StackError`; phase 1 continues.
    Stack(DynStackError<'a>),
    /// The cause is a plain `Error`; phase 2 begins.
    Plain(DynError<'a>),
}

// ---------------------------------------------------------------------------
// Trait-object adapters
// ---------------------------------------------------------------------------

/// Serializes a `&dyn StackError` as a [`TypeErasedStackErrorNode`] and walks the
/// chain. The concrete type is not available, so there is no `context` to emit.
pub struct DynStackError<'a>(pub &'a dyn StackError);

impl Serialize for DynStackError<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let error = self.0;
        TypeErasedStackErrorNode {
            type_name: error.type_name(),
            message: SerializeDisplay(error),
            location: error.location(),
            source: next_error_node(error),
        }
        .serialize(serializer)
    }
}

/// Serializes a `&dyn Error` as a [`PlainErrorNode`], following `Error::source()`.
pub struct DynError<'a>(pub &'a (dyn Error + 'static));

impl Serialize for DynError<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PlainErrorNode {
            message: SerializeDisplay(self.0),
            source: self.0.source().map(DynError),
        }
        .serialize(serializer)
    }
}

/// Continues the chain, preferring `stack_source()` so that nested levels keep
/// their location. Falls back to `Error::source()` for the phase 2 tail.
///
/// This mirrors `StackReport`'s traversal exactly; the two outputs must not
/// disagree about where the phase boundary lands.
fn next_error_node(error: &dyn StackError) -> Option<NextErrorNode<'_>> {
    if let Some(stack) = error.stack_source() {
        return Some(NextErrorNode::Stack(DynStackError(stack)));
    }
    error
        .source()
        .map(|cause| NextErrorNode::Plain(DynError(cause)))
}

// ---------------------------------------------------------------------------
// Marker — which source fields may be serialized by their own impl
// ---------------------------------------------------------------------------

/// Marks types whose `Serialize` impl emits one of this module's error node
/// shapes rather than an arbitrary shape of their own.
///
/// Source-field dispatch keys on this marker, never on `Serialize` alone. A
/// foreign error type that merely happens to derive `Serialize` would otherwise
/// be inlined raw, producing an error node with no `type`, `message` or
/// `location` and silently dropping everything below it.
///
/// `Serialize` is a supertrait because the specialized branch calls the type's
/// own impl; a type that cannot serialize itself must take the fallback path.
pub trait SerializeErrorNode: Serialize {}

// ---------------------------------------------------------------------------
// Source dispatch — autoref specialization on the marker
// ---------------------------------------------------------------------------

/// Wraps a source field so the two branches below can compete for it.
///
/// Generated code calls
/// `(&&SourceErrorNodeResolver(&self.source)).source_error_node()`. Method
/// resolution tries the specialized impl first — its target carries one more
/// `&` — and reaches the fallback only when the field's type is not
/// [`SerializeErrorNode`].
///
/// This is the trait-based form of the autoref specialization the parent module
/// already uses for `stack_source()`. The `Deref`-based form cannot work here:
/// the fallback branch has to borrow the value, and a `Deref` target cannot.
pub struct SourceErrorNodeResolver<'a, T>(pub &'a T);

/// Specialized branch: the field's own `Serialize` impl already emits an error
/// node, so nested levels keep their `context`.
pub trait ResolveSourceErrorNode {
    /// What the `source` key serializes as.
    type ErrorNode: Serialize;

    /// Resolves the wrapped source field to its error node.
    fn source_error_node(&self) -> Self::ErrorNode;
}

impl<'a, T: SerializeErrorNode> ResolveSourceErrorNode for &SourceErrorNodeResolver<'a, T> {
    type ErrorNode = &'a T;

    fn source_error_node(&self) -> Self::ErrorNode {
        self.0
    }
}

/// Fallback branch: the field is a plain `Error`, so the chain continues as
/// phase 2 and every level below it keeps only its `Display` output.
pub trait ResolveSourceErrorNodeFallback {
    /// What the `source` key serializes as.
    type ErrorNode: Serialize;

    /// Resolves the wrapped source field to its error node.
    fn source_error_node(&self) -> Self::ErrorNode;
}

impl<'a, T: Error + 'static> ResolveSourceErrorNodeFallback for SourceErrorNodeResolver<'a, T> {
    type ErrorNode = DynError<'a>;

    fn source_error_node(&self) -> Self::ErrorNode {
        DynError(self.0)
    }
}

// ---------------------------------------------------------------------------
// Impls for this crate's own types
// ---------------------------------------------------------------------------

#[cfg(feature = "alloc")]
mod alloc_impls {
    use super::{DynStackError, Serialize, SerializeErrorNode, Serializer};
    use crate::BoxedStackError;

    impl Serialize for BoxedStackError {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            DynStackError(self.inner()).serialize(serializer)
        }
    }

    // The type is erased, so no error node it produces carries `context`, but
    // `stack_source()` is still callable — the chain is not truncated here.
    impl SerializeErrorNode for BoxedStackError {}
}
