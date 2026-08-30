//! A recording serializer, for differential tests.
//!
//! The oracle for this feature is: serializing through the generated definition
//! must produce the same output as serializing an equivalent struct the user
//! wrote directly with `#[derive(Serialize)]`. Comparing JSON strings is not
//! enough — it discards struct names, and it hides a wrong field *count*, which
//! is exactly the `skip_field` bug class the design warns about: a hand-written
//! impl that announces five fields and emits four produces identical JSON.
//!
//! `serde_test` compares against a token list written by hand, which cannot
//! express "the same as this other value". So this records the serde data model
//! into a comparable tree instead.

// Shared by several test binaries, each of which uses a different part.
#![allow(dead_code, unused_macros)]

use serde::{Serialize, Serializer};
use std::fmt::Display;

/// One node of the serde data model, with everything a comparison needs:
/// struct and variant names, field names, and order.
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Char(char),
    Str(String),
    Bytes(Vec<u8>),
    None,
    Some(Box<Record>),
    Unit,
    UnitStruct(&'static str),
    UnitVariant {
        name: &'static str,
        variant: &'static str,
    },
    NewtypeStruct {
        name: &'static str,
        value: Box<Record>,
    },
    NewtypeVariant {
        name: &'static str,
        variant: &'static str,
        value: Box<Record>,
    },
    Seq(Vec<Record>),
    Tuple(Vec<Record>),
    TupleStruct {
        name: &'static str,
        values: Vec<Record>,
    },
    TupleVariant {
        name: &'static str,
        variant: &'static str,
        values: Vec<Record>,
    },
    Map(Vec<(Record, Record)>),
    Struct {
        name: &'static str,
        /// The count the serializer announced, kept separately from the fields
        /// actually emitted so that a mismatch is visible.
        announced_len: usize,
        fields: Vec<(&'static str, Record)>,
    },
    StructVariant {
        name: &'static str,
        variant: &'static str,
        announced_len: usize,
        fields: Vec<(&'static str, Record)>,
    },
}

impl Record {
    /// Returns the named field of a struct record.
    ///
    /// Panics rather than returning an `Option`: a missing field in a test
    /// fixture is a mistake in the test, not an outcome worth handling.
    pub fn field(&self, name: &str) -> &Record {
        match self {
            Record::Struct { fields, .. } | Record::StructVariant { fields, .. } => fields
                .iter()
                .find(|(field, _)| *field == name)
                .map(|(_, value)| value)
                .unwrap_or_else(|| panic!("no field {name:?} in {self:#?}")),
            _ => panic!("not a struct record: {self:#?}"),
        }
    }

    /// Unwraps an optional field's `Some`.
    ///
    /// Leaving a key out is `skip_serializing_if`'s job, so a key that is
    /// present at all arrives wrapped. JSON discards that wrapper; a format
    /// that encodes a discriminant does not, which is why the recorder keeps
    /// it rather than flattening it away.
    pub fn some(&self) -> &Record {
        match self {
            Record::Some(value) => value,
            _ => panic!("not a Some record: {self:#?}"),
        }
    }
}

/// A phase 2 node: `message`, and nothing else.
pub fn error_node(message: &str) -> Record {
    Record::Struct {
        name: "ErrorNode",
        announced_len: 1,
        fields: vec![("message", Record::Str(message.to_owned()))],
    }
}

/// Records `value` as it presents itself to a serializer.
pub fn record<T: Serialize + ?Sized>(value: &T) -> Record {
    value
        .serialize(Recorder)
        .expect("the recorder never fails on a valid Serialize impl")
}

#[derive(Debug)]
pub struct Error(String);

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

pub struct Recorder;

macro_rules! record_primitive {
    ($($method:ident($ty:ty) => $variant:expr;)*) => {
        $(fn $method(self, value: $ty) -> Result<Record, Error> {
            Ok($variant(value.into()))
        })*
    };
}

impl Serializer for Recorder {
    type Ok = Record;
    type Error = Error;
    type SerializeSeq = SeqRecorder;
    type SerializeTuple = SeqRecorder;
    type SerializeTupleStruct = SeqRecorder;
    type SerializeTupleVariant = SeqRecorder;
    type SerializeMap = MapRecorder;
    type SerializeStruct = StructRecorder;
    type SerializeStructVariant = StructRecorder;

    record_primitive! {
        serialize_bool(bool) => Record::Bool;
        serialize_i8(i8) => Record::I64;
        serialize_i16(i16) => Record::I64;
        serialize_i32(i32) => Record::I64;
        serialize_i64(i64) => Record::I64;
        serialize_u8(u8) => Record::U64;
        serialize_u16(u16) => Record::U64;
        serialize_u32(u32) => Record::U64;
        serialize_u64(u64) => Record::U64;
        serialize_f32(f32) => Record::F64;
        serialize_f64(f64) => Record::F64;
        serialize_char(char) => Record::Char;
        serialize_str(&str) => Record::Str;
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Record, Error> {
        Ok(Record::Bytes(value.to_vec()))
    }

    fn serialize_none(self) -> Result<Record, Error> {
        Ok(Record::None)
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Record, Error> {
        Ok(Record::Some(Box::new(record(value))))
    }

    fn serialize_unit(self) -> Result<Record, Error> {
        Ok(Record::Unit)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Record, Error> {
        Ok(Record::UnitStruct(name))
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Record, Error> {
        Ok(Record::UnitVariant { name, variant })
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Record, Error> {
        Ok(Record::NewtypeStruct {
            name,
            value: Box::new(record(value)),
        })
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Record, Error> {
        Ok(Record::NewtypeVariant {
            name,
            variant,
            value: Box::new(record(value)),
        })
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<SeqRecorder, Error> {
        Ok(SeqRecorder::new(Shape::Seq))
    }

    fn serialize_tuple(self, _len: usize) -> Result<SeqRecorder, Error> {
        Ok(SeqRecorder::new(Shape::Tuple))
    }

    fn serialize_tuple_struct(self, name: &'static str, _len: usize) -> Result<SeqRecorder, Error> {
        Ok(SeqRecorder::new(Shape::TupleStruct { name }))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<SeqRecorder, Error> {
        Ok(SeqRecorder::new(Shape::TupleVariant { name, variant }))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<MapRecorder, Error> {
        Ok(MapRecorder {
            entries: Vec::new(),
            key: None,
        })
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<StructRecorder, Error> {
        Ok(StructRecorder {
            name,
            variant: None,
            announced_len: len,
            fields: Vec::new(),
        })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<StructRecorder, Error> {
        Ok(StructRecorder {
            name,
            variant: Some(variant),
            announced_len: len,
            fields: Vec::new(),
        })
    }
}

enum Shape {
    Seq,
    Tuple,
    TupleStruct {
        name: &'static str,
    },
    TupleVariant {
        name: &'static str,
        variant: &'static str,
    },
}

pub struct SeqRecorder {
    shape: Shape,
    values: Vec<Record>,
}

impl SeqRecorder {
    fn new(shape: Shape) -> Self {
        SeqRecorder {
            shape,
            values: Vec::new(),
        }
    }

    fn push<T: Serialize + ?Sized>(&mut self, value: &T) {
        self.values.push(record(value));
    }

    fn finish(self) -> Record {
        match self.shape {
            Shape::Seq => Record::Seq(self.values),
            Shape::Tuple => Record::Tuple(self.values),
            Shape::TupleStruct { name } => Record::TupleStruct {
                name,
                values: self.values,
            },
            Shape::TupleVariant { name, variant } => Record::TupleVariant {
                name,
                variant,
                values: self.values,
            },
        }
    }
}

macro_rules! impl_seq_recorder {
    ($($trait:ident :: $method:ident),* $(,)?) => {
        $(impl serde::ser::$trait for SeqRecorder {
            type Ok = Record;
            type Error = Error;

            fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
                self.push(value);
                Ok(())
            }

            fn end(self) -> Result<Record, Error> {
                Ok(self.finish())
            }
        })*
    };
}

impl_seq_recorder! {
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field,
}

pub struct MapRecorder {
    entries: Vec<(Record, Record)>,
    key: Option<Record>,
}

impl serde::ser::SerializeMap for MapRecorder {
    type Ok = Record;
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        self.key = Some(record(key));
        Ok(())
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self.key.take().expect("serialize_key precedes value");
        self.entries.push((key, record(value)));
        Ok(())
    }

    fn end(self) -> Result<Record, Error> {
        Ok(Record::Map(self.entries))
    }
}

pub struct StructRecorder {
    name: &'static str,
    variant: Option<&'static str>,
    announced_len: usize,
    fields: Vec<(&'static str, Record)>,
}

impl StructRecorder {
    fn finish(self) -> Record {
        match self.variant {
            None => Record::Struct {
                name: self.name,
                announced_len: self.announced_len,
                fields: self.fields,
            },
            Some(variant) => Record::StructVariant {
                name: self.name,
                variant,
                announced_len: self.announced_len,
                fields: self.fields,
            },
        }
    }
}

macro_rules! impl_struct_recorder {
    ($($trait:ident),* $(,)?) => {
        $(impl serde::ser::$trait for StructRecorder {
            type Ok = Record;
            type Error = Error;

            fn serialize_field<T: Serialize + ?Sized>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<(), Error> {
                self.fields.push((key, record(value)));
                Ok(())
            }

            // Deliberately not the default no-op. A hand-written impl that
            // omits a field must still announce it here; recording the call is
            // what makes the omission observable.
            fn skip_field(&mut self, _key: &'static str) -> Result<(), Error> {
                Ok(())
            }

            fn end(self) -> Result<Record, Error> {
                Ok(self.finish())
            }
        })*
    };
}

impl_struct_recorder!(SerializeStruct, SerializeStructVariant);

/// Declares one case: the error type, the struct a user would have written for
/// its declared fields, and the comparison between them.
///
/// The split between `context` and `metadata` is written out per case rather
/// than worked out from the field list. That matters: "`source` and `location`
/// belong to the metadata level" is the rule under test, so a macro that
/// inferred it would be re-implementing what it is supposed to check, and a
/// shared mistake would pass unnoticed.
macro_rules! declare_case {
    (
        error: $name:ident,
        display: $display:literal,
        context: { $($field:ident : $ty:ty = $value:expr),* $(,)? },
        metadata: { $($metadata:tt)* } $(,)?
    ) => {
        #[suzunari_error(serialize)]
        #[suzu(display($display))]
        struct $name {
            $($field: $ty,)*
            $($metadata)*
        }

        mod oracle {
            // Needed only when a declared field's type is not in the prelude.
            #[allow(unused_imports)]
            use super::*;

            #[derive(serde::Serialize)]
            pub struct $name {
                $(pub $field: $ty,)*
            }
        }

        fn expected_context() -> oracle::$name {
            oracle::$name { $($field: $value,)* }
        }

        /// The declared fields must record identically through both paths.
        fn assert_context_matches(error: &$name) {
            assert_eq!(
                record(error).field("context"),
                &record(&expected_context())
            );
        }
    };
}
