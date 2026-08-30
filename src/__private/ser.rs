//! serde `Serialize` support for error chains.
//!
//! **Not public API. Do not use.** This module is `#[doc(hidden)]` and not
//! covered by semver guarantees. It exists solely for generated code and for
//! this crate's own `Serialize` impls.
//!
//! # Node shapes
//!
//! The payload mirrors [`StackReport`](crate::StackReport)'s two phases, so the
//! human-readable and machine-readable outputs show the same information:
//!
//! - [`StackErrorNode`] — phase 1, concrete type known: `type`, `message`,
//!   `location`, `context`.
//! - [`BoxedStackErrorNode`] — phase 1, reached through a type-erased boundary:
//!   the same minus `context`, because the declared fields are unreachable.
//! - [`ErrorNode`] — phase 2, a plain [`Error`] tail: `message` only.
//!
//! The split is deliberate. An absent `context` and an empty one mean different
//! things, and `type` cannot separate them: an erased node still has a type
//! name, because the erasure forwards `type_name()` to the value it holds.
//!
//! All three are `#[derive(Serialize)]` structs that the adapters below fill in.
//! Nothing here calls `serialize_struct` by hand: a hand-written impl must also
//! call `skip_field` for every field it omits, and forgetting that is invisible
//! in JSON.

use crate::{Location, StackError};
use core::error::Error;
use core::fmt::Display;
use serde::{Serialize, Serializer};

// ---------------------------------------------------------------------------
// Message — a value's Display output
// ---------------------------------------------------------------------------

/// Serializes a value's `Display` output as a string.
///
/// `E` stays generic rather than collapsing to `&dyn Display` because
/// upcasting `dyn StackError` to `dyn Display` needs trait upcasting, which
/// stabilized in Rust 1.86 — later than this crate's MSRV of 1.85.
pub struct Message<'a, E: ?Sized>(pub &'a E);

impl<E: ?Sized + Display> Serialize for Message<'_, E> {
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
/// The `rename` is required. A def's own identifier is what reaches
/// `serialize_struct` as the struct name, so without it the payload's data
/// model would carry `LocationDef`. Self-describing formats such as JSON
/// discard struct names, which is why this kind of leak survives until a
/// `Token`-level comparison catches it.
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
fn serialize_location<S: Serializer>(
    location: &Location,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    LocationDef::serialize(location, serializer)
}

// ---------------------------------------------------------------------------
// Node shapes
// ---------------------------------------------------------------------------

/// Phase 1 node for an error whose concrete type is known.
///
/// `context` is not optional here. A concrete type always has one — empty when
/// it declares no fields of its own — and making that a type-level fact is what
/// keeps it from being conflated with [`BoxedStackErrorNode`], where the fields
/// exist but are unreachable.
#[derive(Serialize)]
pub struct StackErrorNode<M, C, Src> {
    /// `StackError::type_name()` — `"Type"` or `"Enum::Variant"`.
    #[serde(rename = "type")]
    pub type_name: &'static str,
    /// The error's `Display` output.
    pub message: M,
    /// Where the error was constructed.
    #[serde(serialize_with = "serialize_location")]
    pub location: Location,
    /// The type's declared fields.
    pub context: C,
    /// The next node in the chain. `None` when there is no cause.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Src>,
}

/// Phase 1 node for an error reached through a type-erased boundary.
///
/// Identical to [`StackErrorNode`] minus `context`: only `&dyn StackError` is
/// available, so the declared fields cannot be read. It carries no type
/// parameters because both the message and the continuation are fixed by that.
#[derive(Serialize)]
pub struct BoxedStackErrorNode<'a> {
    /// `StackError::type_name()`, forwarded from the value behind the erasure —
    /// so this is the wrapped error's name, never the wrapper's.
    #[serde(rename = "type")]
    pub type_name: &'static str,
    /// The error's `Display` output.
    pub message: Message<'a, dyn StackError + 'a>,
    /// Where the error was constructed.
    #[serde(serialize_with = "serialize_location")]
    pub location: Location,
    /// The next node in the chain. `None` when there is no cause.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<NextNode<'a>>,
}

/// Phase 2 node: a plain [`Error`] tail, with no location information.
#[derive(Serialize)]
pub struct ErrorNode<'a> {
    /// The error's `Display` output.
    pub message: Message<'a, dyn Error + 'static>,
    /// The next node in the chain. `None` when there is no cause.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<DynError<'a>>,
}

/// Which node shape the chain continues with, when only a trait object is
/// available to decide.
///
/// `untagged` so the variant contributes no wrapper of its own — the payload
/// carries the node directly.
#[derive(Serialize)]
#[serde(untagged)]
pub enum NextNode<'a> {
    /// The cause implements `StackError`; phase 1 continues.
    Stack(DynStackError<'a>),
    /// The cause is a plain `Error`; phase 2 begins.
    Plain(DynError<'a>),
}

// ---------------------------------------------------------------------------
// Trait-object adapters
// ---------------------------------------------------------------------------

/// Serializes a `&dyn StackError` as a [`BoxedStackErrorNode`] and walks the
/// chain. The concrete type is not available, so there is no `context` to emit.
pub struct DynStackError<'a>(pub &'a dyn StackError);

impl Serialize for DynStackError<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let error = self.0;
        BoxedStackErrorNode {
            type_name: error.type_name(),
            message: Message(error),
            location: error.location(),
            source: next_node(error),
        }
        .serialize(serializer)
    }
}

/// Serializes a `&dyn Error` as an [`ErrorNode`], following `Error::source()`.
pub struct DynError<'a>(pub &'a (dyn Error + 'static));

impl Serialize for DynError<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorNode {
            message: Message(self.0),
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
fn next_node(error: &dyn StackError) -> Option<NextNode<'_>> {
    if let Some(stack) = error.stack_source() {
        return Some(NextNode::Stack(DynStackError(stack)));
    }
    error.source().map(|cause| NextNode::Plain(DynError(cause)))
}

// ---------------------------------------------------------------------------
// Marker — which source fields may be serialized by their own impl
// ---------------------------------------------------------------------------

/// Marks types whose `Serialize` impl emits one of this module's node shapes
/// rather than an arbitrary shape of their own.
///
/// Source-field dispatch keys on this marker, never on `Serialize` alone. A
/// foreign error type that merely happens to derive `Serialize` would otherwise
/// be inlined raw, producing a node with no `type`, `message` or `location` and
/// silently dropping everything below it.
///
/// `Serialize` is a supertrait because the specialized branch calls the type's
/// own impl; a type that cannot serialize itself must take the fallback path.
pub trait SerializeAsNode: Serialize {}

// ---------------------------------------------------------------------------
// Source dispatch — autoref specialization on the marker
// ---------------------------------------------------------------------------

/// Wraps a source field so the two branches below can compete for it.
///
/// Generated code calls
/// `(&&SourceNodeResolver(&self.source)).source_node()`. Method resolution
/// tries the specialized impl first — its target carries one more `&` — and
/// reaches the fallback only when the field's type is not [`SerializeAsNode`].
///
/// This is the trait-based form of the autoref specialization the parent module
/// already uses for `stack_source()`. The `Deref`-based form cannot work here:
/// the fallback branch has to borrow the value, and a `Deref` target cannot.
pub struct SourceNodeResolver<'a, T>(pub &'a T);

/// Specialized branch: the field's own `Serialize` impl already emits a node,
/// so nested levels keep their `context`.
pub trait ResolveSourceNode {
    /// What the `source` key serializes as.
    type Node: Serialize;

    /// Resolves the wrapped source field to its node.
    fn source_node(&self) -> Self::Node;
}

impl<'a, T: SerializeAsNode> ResolveSourceNode for &SourceNodeResolver<'a, T> {
    type Node = &'a T;

    fn source_node(&self) -> Self::Node {
        self.0
    }
}

/// Fallback branch: the field is a plain `Error`, so the chain continues as
/// phase 2 and every level below it keeps only its `Display` output.
pub trait ResolveSourceNodeFallback {
    /// What the `source` key serializes as.
    type Node: Serialize;

    /// Resolves the wrapped source field to its node.
    fn source_node(&self) -> Self::Node;
}

impl<'a, T: Error + 'static> ResolveSourceNodeFallback for SourceNodeResolver<'a, T> {
    type Node = DynError<'a>;

    fn source_node(&self) -> Self::Node {
        DynError(self.0)
    }
}

// ---------------------------------------------------------------------------
// Impls for this crate's own types
// ---------------------------------------------------------------------------

#[cfg(feature = "alloc")]
mod alloc_impls {
    use super::{DynStackError, Serialize, SerializeAsNode, Serializer};
    use crate::BoxedStackError;

    impl Serialize for BoxedStackError {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            DynStackError(self.inner()).serialize(serializer)
        }
    }

    // The type is erased, so no node it produces carries `context`, but
    // `stack_source()` is still callable — the chain is not truncated here.
    impl SerializeAsNode for BoxedStackError {}
}
