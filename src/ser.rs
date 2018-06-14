//! Serialization support via serde.

#![allow(missing_debug_implementations)]   // mostly useless for all the serializers in here

use {Error, Value};

use serde::ser::{self, Serialize, Impossible, Error as _Error};

use std::iter;
use std::fmt::Display;
use std::marker::PhantomData;
use std::collections::BTreeMap;

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self where T: Display {
        ::error::ErrorKind::String(format!("{}", msg)).into()
    }
}

impl Error {
    fn key_must_be_string() -> Self {
        Self::custom("map keys must be strings")
    }
}

impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> where
        S: ser::Serializer {

        match *self {
            Value::Int(i) => serializer.serialize_i32(i),
            Value::Int64(i) => serializer.serialize_i64(i),
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Double(f) => serializer.serialize_f64(f),
            Value::DateTime(ref _datetime) => unimplemented!(),  // TODO
            Value::Base64(ref bytes) => serializer.serialize_bytes(bytes),
            Value::Struct(ref map) => map.serialize(serializer),
            Value::Array(ref values) => values.serialize(serializer),
            Value::Nil => serializer.serialize_unit(),
        }
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;

/// Specifies the behaviour of the serializer when encountering values that might need XML-RPC
/// extensions to express.
pub trait ExtensionBehaviour {
    /// Whether to use `Value::Nil` to encode unit values and absence of values.
    ///
    /// If `false`, unit values and `None` cannot be serialized and result in an error.
    const USE_NIL: bool;

    /// Whether to use `Value::Int64` to encode `u32` values.
    ///
    /// If `false`, any attempt to serialize a `u32` will fail. The caller can manually convert to
    /// an `i32` instead.
    const USE_INT64: bool;
}

/// Allow the use of `nil` and `int64` extensions.
pub enum ExtensionUse {}

impl ExtensionBehaviour for ExtensionUse {
    const USE_NIL: bool = true;
    const USE_INT64: bool = true;
}

/// Never use `nil` and `int64` extensions.
#[allow(unused)]
pub enum ExtensionAvoid {}

impl ExtensionBehaviour for ExtensionAvoid {
    const USE_NIL: bool = false;
    const USE_INT64: bool = false;
}

/// A serializer that produces an XML-RPC `<value>` tag.
pub(crate) struct Serializer<E: ExtensionBehaviour = ExtensionUse> {
    _phantom: PhantomData<E>,
}

impl<E: ExtensionBehaviour> Serializer<E> {
    pub fn new() -> Self {
        Self { _phantom: PhantomData }
    }
}

impl Serializer<ExtensionUse> {
    pub(crate) fn with_extensions() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl Serializer<ExtensionAvoid> {
    #[allow(unused)]
    pub(crate) fn without_extensions() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<'a, E: ExtensionBehaviour> ser::Serializer for &'a mut Serializer<E> {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = SerializeArray<E>;
    type SerializeTuple = Self::SerializeSeq;
    type SerializeTupleStruct = Self::SerializeSeq;
    type SerializeTupleVariant = SerializeTupleVariant<E>;
    type SerializeMap = SerializeMap<E>;
    type SerializeStruct = Self::SerializeMap;
    type SerializeStructVariant = SerializeStructVariant<E>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.serialize_i32(v.into())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.serialize_i32(v.into())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        Ok(Value::Int(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        if E::USE_INT64 {
            Ok(Value::Int64(v))
        } else {
            Err(Error::custom("cannot serialize i64: use of `int64` extension disabled"))
        }
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.serialize_i32(v.into())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.serialize_i32(v.into())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        // only the lower half of all u32s fit in `<int>`, which is i32, so we always use `Int64`
        self.serialize_i64(v.into())
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
        // u64 is special, because half of its values can't fit in *any* XML-RPC integer
        // to avoid runtime surprises, reject *all* of them and require the user to convert.
        Err(Error::custom("cannot serialize u64: please use a smaller integer type"))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.serialize_f64(v.into())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        Ok(Value::Double(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        Ok(Value::Base64(v.into()))
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        // do the same thing serde_json does
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok> where
        T: Serialize {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        if E::USE_NIL {
            Ok(Value::Nil)
        } else {
            Err(Error::custom("cannot serialize unit-like value (use of `nil` extension disabled)"))
        }
    }

    fn serialize_unit_struct(self, _name: &str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(self, _name: &str, _variant_index: u32, variant: &str) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &str, value: &T) -> Result<Self::Ok> where
        T: Serialize {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(self, _name: &str, _variant_index: u32, variant: &str, value: &T) -> Result<Self::Ok> where
        T: Serialize {
        // enum variant that contains a single, unnamed type
        // need to encode the variant name, and the contained value in some way
        // we mimic serde_json/serde-yaml here and create a struct with a single KV pair
        let value = value.serialize(&mut Serializer::<E>::new())?;

        Ok(Value::Struct(iter::once((variant.to_string(), value)).collect()))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeArray::with_capacity(len.unwrap_or(0)))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &str, len: usize) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(self, _name: &str, _variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant> {
        Ok(SerializeTupleVariant::with_name_and_capacity(variant, len))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeMap::new())
    }

    fn serialize_struct(self, _name: &str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(self, _name: &str, _variant_index: u32, variant: &str, _len: usize) -> Result<Self::SerializeStructVariant> {
        Ok(SerializeStructVariant::new(variant.to_string()))
    }
}

pub struct SerializeArray<E: ExtensionBehaviour> {
    _phantom: PhantomData<E>,
    array: Vec<Value>,
}

impl<E: ExtensionBehaviour> SerializeArray<E> {
    fn with_capacity(cap: usize) -> Self {
        Self {
            _phantom: PhantomData,
            array: Vec::with_capacity(cap),
        }
    }

    fn push<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        self.array.push(value.serialize(&mut Serializer::<E>::new())?);
        Ok(())
    }
}

impl<E: ExtensionBehaviour> ser::SerializeSeq for SerializeArray<E> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Value::Array(self.array))
    }
}

impl<E: ExtensionBehaviour> ser::SerializeTuple for SerializeArray<E> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Value::Array(self.array))
    }
}

impl<E: ExtensionBehaviour> ser::SerializeTupleStruct for SerializeArray<E> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Value::Array(self.array))
    }
}

pub struct SerializeTupleVariant<E: ExtensionBehaviour> {
    _phantom: PhantomData<E>,
    name: &'static str,
    values: Vec<Value>,
}

impl<E: ExtensionBehaviour> SerializeTupleVariant<E> {
    pub fn with_name_and_capacity(name: &'static str, cap: usize) -> Self {
        Self {
            _phantom: PhantomData,
            name,
            values: Vec::with_capacity(cap),
        }
    }
}

impl<E: ExtensionBehaviour> ser::SerializeTupleVariant for SerializeTupleVariant<E> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        self.values.push(value.serialize(&mut Serializer::<E>::new())?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Value::Struct(iter::once((self.name.to_string(), Value::Array(self.values))).collect()))
    }
}

pub struct SerializeMap<E: ExtensionBehaviour> {
    _phantom: PhantomData<E>,
    next_key: Option<String>,
    map: BTreeMap<String, Value>,
}

impl<E: ExtensionBehaviour> SerializeMap<E> {
    fn new() -> Self {
        Self {
            _phantom: PhantomData,
            next_key: None,
            map: BTreeMap::new(),
        }
    }
}

impl<E: ExtensionBehaviour> ser::SerializeMap for SerializeMap<E> {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()> where
        T: Serialize {

        if let Some(old_key) = self.next_key.as_ref() {
            panic!("serialize_key called twice in a row (first key = {}, new key = {:?})", old_key, key.serialize(&mut Serializer::<E>::new()));
        }

        self.next_key = Some(key.serialize(KeySerializer)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {

        if let Some(key) = self.next_key.take() {
            let old = self.map.insert(key, value.serialize(&mut Serializer::<E>::new())?);
            if let Some(old) = old {
                panic!("map contains duplicate key (old value = {:?})", old);
            } else {
                Ok(())
            }
        } else {
            panic!("serialize_value called before serialize_key");
        }
    }

    fn end(self) -> Result<Self::Ok> {
        assert!(self.next_key.is_none(), "serialize_key called without serialize_value");
        Ok(Value::Struct(self.map))
    }
}

impl<E: ExtensionBehaviour> ser::SerializeStruct for SerializeMap<E> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &str, value: &T) -> Result<()> where
        T: Serialize {

        let old = self.map.insert(key.to_string(), value.serialize(&mut Serializer::<E>::new())?);
        if let Some(old) = old {
            panic!("duplicate struct field {} (first value = {:?}, new value = {:?})", key, old, value.serialize(&mut Serializer::<E>::new()));
        }

        Ok(())
    }

    fn end(self) -> Result<Value> {
        assert!(self.next_key.is_none(), "serialize_key called without serialize_value");
        Ok(Value::Struct(self.map))
    }
}

/// `Serializer` for map/struct keys. Only supports serializing strings.
struct KeySerializer;

impl ser::Serializer for KeySerializer {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        Ok(v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        Ok(v.to_string())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok> where
        T: Serialize {
        // even if the `some` *would* be a string, reject it
        Err(Error::key_must_be_string())
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_unit_struct(self, _name: &str) -> Result<Self::Ok> {
        Err(Error::key_must_be_string())
    }

    fn serialize_unit_variant(self, _name: &str, _variant_index: u32, variant: &str) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &str, value: &T) -> Result<Self::Ok> where
        T: Serialize {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(self, _name: &str, _variant_index: u32, _variant: &str, _value: &T) -> Result<Self::Ok> where
        T: Serialize {

        Err(Error::key_must_be_string())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::key_must_be_string())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::key_must_be_string())
    }

    fn serialize_tuple_struct(self, _name: &str, _len: usize) -> Result<Self::SerializeTupleStruct> {
        Err(Error::key_must_be_string())
    }

    fn serialize_tuple_variant(self, _name: &str, _variant_index: u32, _variant: &str, _len: usize) -> Result<Self::SerializeTupleVariant> {
        Err(Error::key_must_be_string())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::key_must_be_string())
    }

    fn serialize_struct(self, _name: &str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::key_must_be_string())
    }

    fn serialize_struct_variant(self, _name: &str, _variant_index: u32, _variant: &str, _len: usize) -> Result<Self::SerializeStructVariant> {
        Err(Error::key_must_be_string())
    }
}

pub struct SerializeStructVariant<E: ExtensionBehaviour> {
    _phantom: PhantomData<E>,
    variant: String,
    fields: BTreeMap<String, Value>,
}

impl<E: ExtensionBehaviour> SerializeStructVariant<E> {
    fn new(variant: String) -> Self {
        Self {
            _phantom: PhantomData,
            variant,
            fields: BTreeMap::new(),
        }
    }
}

impl<E: ExtensionBehaviour> ser::SerializeStructVariant for SerializeStructVariant<E> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &str, value: &T) -> Result<()> where
        T: Serialize {

        let old = self.fields.insert(key.to_string(), value.serialize(&mut Serializer::<E>::new())?);
        if let Some(old) = old {
            panic!("duplicate struct field (key = {}, initial value = {:?}, duplicate value = {:?})", key, old, value.serialize(&mut Serializer::<E>::new()));
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Value::Struct(iter::once((self.variant, Value::Struct(self.fields))).collect()))
    }
}

// TODO: impl Serialize for Value
