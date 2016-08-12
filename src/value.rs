//! Contains the different types of values understood by XML-RPC.

use chrono::{DateTime, FixedOffset};

use std::collections::BTreeMap;

/// The possible XML-RPC values.
#[derive(Debug, PartialEq)]
pub enum Value {
    /// `<i4>` or `<int>`, 32-bit signed integer.
    Int(i32),
    /// `<boolean>`, 0 == `false`, 1 == `true`.
    Bool(bool),
    /// `<string>`
    // FIXME zero-copy? `Cow<'static,..>`?
    String(String),
    /// `<double>`
    Double(f64),
    /// `<dateTime.iso8601>`, an ISO 8601 formatted date/time value.
    // FIXME We'll assume RFC 3339 semantics instead! Check if that's okay, if not, use an
    // alternative to `chrono`.
    DateTime(DateTime<FixedOffset>),
    /// `<base64>`, base64-encoded binary data.
    Base64(Vec<u8>),

    /// `<struct>`, a mapping of named values.
    Struct(BTreeMap<String, Value>),
    /// `<array>`, a list of arbitrary (heterogeneous) values.
    Array(Vec<Value>),
}

impl From<i32> for Value {
    fn from(other: i32) -> Self {
        Value::Int(other)
    }
}

impl From<bool> for Value {
    fn from(other: bool) -> Self {
        Value::Bool(other)
    }
}

impl From<String> for Value {
    fn from(other: String) -> Self {
        Value::String(other)
    }
}

impl From<f64> for Value {
    fn from(other: f64) -> Self {
        Value::Double(other)
    }
}

impl From<DateTime<FixedOffset>> for Value {
    fn from(other: DateTime<FixedOffset>) -> Self {
        Value::DateTime(other)
    }
}

impl From<Vec<u8>> for Value {
    fn from(other: Vec<u8>) -> Self {
        Value::Base64(other)
    }
}
