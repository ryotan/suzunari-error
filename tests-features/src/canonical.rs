//! A serializer that writes into a caller-supplied buffer and allocates
//! nothing.
//!
//! The recording serializer used by the main test suite builds a tree out of
//! `Vec` and `String`, so it cannot run in the core-only tier. This one writes a
//! canonical form into a fixed buffer instead, and two values are compared as
//! bytes. The comparison is the same one: the form carries struct names, the
//! field count the serializer announced, and field order, none of which survive
//! a JSON string.
//!
//! It exists mainly to prove something no other tier can. `collect_str` has a
//! default implementation — by way of `to_string()` — whenever serde has an
//! allocator. With serde taken core-only that default is gone and a serializer
//! must write the value out itself, so a `Display` reaching the payload through
//! this serializer has provably not been allocated on the way.
//!
//! # Overflow must not truncate
//!
//! Running out of room is an error, never a short write. Two values that both
//! overflow would otherwise produce the same truncated bytes and compare equal,
//! turning the oracle into a test that passes when it has seen nothing.

use core::fmt::{self, Display, Write};
use serde::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer,
};

/// What can go wrong on the way into the buffer.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// The value did not fit. Deliberately not a short write; see the module
    /// documentation.
    Overflow,
    /// A part of serde's data model this serializer does not write. The
    /// payloads it is pointed at do not reach these, and a silent gap would be
    /// worse than a loud one.
    Unsupported(&'static str),
    /// Raised by serde itself. The message cannot be kept without an allocator.
    Custom,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Overflow => f.write_str("buffer too small"),
            Error::Unsupported(what) => write!(f, "unsupported: {what}"),
            Error::Custom => f.write_str("serialization failed"),
        }
    }
}

impl core::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: Display>(_: T) -> Self {
        Error::Custom
    }
}

/// A byte buffer that refuses to overflow.
pub struct Buffer<'a> {
    bytes: &'a mut [u8],
    written: usize,
    /// Set when a write did not fit, so that a `fmt::Error` — which carries
    /// nothing — can be reported as the overflow it was.
    overflowed: bool,
}

impl<'a> Buffer<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Buffer {
            bytes,
            written: 0,
            overflowed: false,
        }
    }

    /// What has been written so far.
    pub fn filled(&self) -> &[u8] {
        &self.bytes[..self.written]
    }

    fn put(&mut self, text: &str) -> Result<(), Error> {
        let end = self.written + text.len();
        if end > self.bytes.len() {
            self.overflowed = true;
            return Err(Error::Overflow);
        }
        self.bytes[self.written..end].copy_from_slice(text.as_bytes());
        self.written = end;
        Ok(())
    }
}

impl Write for Buffer<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.put(text).map_err(|_| fmt::Error)
    }
}

/// Writes `value`'s canonical form into `bytes`, and returns the part used.
pub fn canonical<'a, T>(bytes: &'a mut [u8], value: &T) -> Result<&'a [u8], Error>
where
    T: Serialize + ?Sized,
{
    let mut buffer = Buffer::new(bytes);
    value.serialize(&mut buffer)?;
    let written = buffer.written;
    Ok(&bytes[..written])
}

/// Writes a number without allocating, by way of `core::fmt`.
fn put_display<T: Display>(buffer: &mut Buffer<'_>, value: T) -> Result<(), Error> {
    write!(buffer, "{value}").map_err(|_| Error::Overflow)
}

macro_rules! write_display {
    ($($method:ident($ty:ty)),* $(,)?) => {
        $(fn $method(self, value: $ty) -> Result<(), Error> {
            put_display(self, value)
        })*
    };
}

macro_rules! unsupported {
    ($($method:ident($($arg:ty),*) -> $ret:ty),* $(,)?) => {
        $(fn $method(self, $(_: $arg),*) -> Result<$ret, Error> {
            Err(Error::Unsupported(stringify!($method)))
        })*
    };
}

impl<'a, 'b> Serializer for &'a mut Buffer<'b> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    write_display!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_f32(f32),
        serialize_f64(f64),
        serialize_char(char),
    );

    fn serialize_str(self, value: &str) -> Result<(), Error> {
        self.put("\"")?;
        self.put(value)?;
        self.put("\"")
    }

    /// Required rather than defaulted once serde has no allocator, which is
    /// what makes this tier the only place the no-allocation claim is testable.
    fn collect_str<T: Display + ?Sized>(self, value: &T) -> Result<(), Error> {
        self.put("\"")?;
        write!(self, "{value}").map_err(|_| Error::Overflow)?;
        self.put("\"")
    }

    fn serialize_none(self) -> Result<(), Error> {
        self.put("?-")
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Error> {
        self.put("?")?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), Error> {
        self.put("()")
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<(), Error> {
        self.put("U\"")?;
        self.put(name)?;
        self.put("\"")
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self, Error> {
        self.put("S\"")?;
        self.put(name)?;
        self.put("\"(")?;
        put_display(self, len)?;
        self.put("){")?;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self, Error> {
        self.put("V\"")?;
        self.put(name)?;
        self.put("\"::\"")?;
        self.put(variant)?;
        self.put("\"(")?;
        put_display(self, len)?;
        self.put("){")?;
        Ok(self)
    }

    unsupported!(
        serialize_bytes(&[u8]) -> (),
        serialize_seq(Option<usize>) -> Self,
        serialize_map(Option<usize>) -> Self,
    );

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Error> {
        Err(Error::Unsupported("serialize_unit_variant"))
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<(), Error> {
        Err(Error::Unsupported("serialize_newtype_struct"))
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<(), Error> {
        Err(Error::Unsupported("serialize_newtype_variant"))
    }

    fn serialize_tuple(self, _: usize) -> Result<Self, Error> {
        Err(Error::Unsupported("serialize_tuple"))
    }

    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, Error> {
        Err(Error::Unsupported("serialize_tuple_struct"))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Error> {
        Err(Error::Unsupported("serialize_tuple_variant"))
    }
}

/// Writes one `name=value` pair, comma-separated.
fn put_field<T: Serialize + ?Sized>(
    buffer: &mut Buffer<'_>,
    key: &'static str,
    value: &T,
) -> Result<(), Error> {
    if !buffer.filled().ends_with(b"{") {
        buffer.put(",")?;
    }
    buffer.put(key)?;
    buffer.put("=")?;
    value.serialize(buffer)
}

macro_rules! impl_struct_like {
    ($($trait:ident),* $(,)?) => {
        $(impl $trait for &mut Buffer<'_> {
            type Ok = ();
            type Error = Error;

            fn serialize_field<T: Serialize + ?Sized>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<(), Error> {
                put_field(self, key, value)
            }

            // Recorded rather than ignored: a definition that announces a field
            // and then omits it is the failure this whole comparison exists to
            // catch, and the default body is a no-op.
            fn skip_field(&mut self, key: &'static str) -> Result<(), Error> {
                if !self.filled().ends_with(b"{") {
                    self.put(",")?;
                }
                self.put("-")?;
                self.put(key)
            }

            fn end(self) -> Result<(), Error> {
                self.put("}")
            }
        })*
    };
}

impl_struct_like!(SerializeStruct, SerializeStructVariant);

macro_rules! impl_unsupported_compound {
    ($($trait:ident :: $method:ident),* $(,)?) => {
        $(impl $trait for &mut Buffer<'_> {
            type Ok = ();
            type Error = Error;

            fn $method<T: Serialize + ?Sized>(&mut self, _: &T) -> Result<(), Error> {
                Err(Error::Unsupported(stringify!($trait)))
            }

            fn end(self) -> Result<(), Error> {
                Err(Error::Unsupported(stringify!($trait)))
            }
        })*
    };
}

impl_unsupported_compound!(
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field,
);

impl SerializeMap for &mut Buffer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, _: &T) -> Result<(), Error> {
        Err(Error::Unsupported("SerializeMap"))
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, _: &T) -> Result<(), Error> {
        Err(Error::Unsupported("SerializeMap"))
    }

    fn end(self) -> Result<(), Error> {
        Err(Error::Unsupported("SerializeMap"))
    }
}
