//! Contains the different types of values understood by XML-RPC.

use base64::encode;
use iso8601::DateTime;

use std::collections::BTreeMap;
use std::io::{self, Write};

/// The possible XML-RPC values.
#[derive(Debug, PartialEq)]
pub enum Value {
    /// `<i4>` or `<int>`, 32-bit signed integer.
    Int(i32),
    /// `<boolean>`, 0 == `false`, 1 == `true`.
    Bool(bool),
    /// `<string>`
    // FIXME zero-copy? `Cow<'static, ..>`?
    String(String),
    /// `<double>`
    Double(f64),
    /// `<dateTime.iso8601>`, an ISO 8601 formatted date/time value.
    DateTime(DateTime),
    /// `<base64>`, base64-encoded binary data.
    Base64(Vec<u8>),

    /// `<struct>`, a mapping of named values.
    Struct(BTreeMap<String, Value>),
    /// `<array>`, a list of arbitrary (heterogeneous) values.
    Array(Vec<Value>),

    // TODO Nil
}

impl Value {
    pub fn format<W: Write>(&self, fmt: &mut W) -> io::Result<()> {
        try!(writeln!(fmt, "<value>"));

        match *self {
            Value::Int(i) => {
                try!(writeln!(fmt, "<i4>{}</i4>", i));
            }
            Value::Bool(b) => {
                try!(writeln!(fmt, "<boolean>{}</boolean>", if b { "1" } else { "0" }));
            }
            Value::String(ref s) => {
                // FIXME escape
                try!(writeln!(fmt, "<string>{}</string>", s));
            }
            Value::Double(d) => {
                try!(writeln!(fmt, "<double>{}</double>", d));
            }
            Value::DateTime(_date_time) => {
                // TODO not easy :(
                unimplemented!();
            }
            Value::Base64(ref data) => {
                try!(writeln!(fmt, "<base64>{}</base64>", encode(data)));
            }
            Value::Struct(ref map) => {
                try!(writeln!(fmt, "<struct>"));
                for (ref name, ref value) in map {
                    try!(writeln!(fmt, "<member>"));
                    // FIXME escape
                    try!(writeln!(fmt, "<name>{}</name>", name));
                    try!(value.format(fmt));
                    try!(writeln!(fmt, "</member>"));
                }
                try!(writeln!(fmt, "</struct>"));
            }
            Value::Array(ref array) => {
                try!(writeln!(fmt, "<array>"));
                try!(writeln!(fmt, "<data>"));
                for value in array {
                    try!(value.format(fmt));
                }
                try!(writeln!(fmt, "</data>"));
                try!(writeln!(fmt, "</array>"));
            }
        }

        try!(writeln!(fmt, "</value>"));
        Ok(())
    }
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

impl From<DateTime> for Value {
    fn from(other: DateTime) -> Self {
        Value::DateTime(other)
    }
}

impl From<Vec<u8>> for Value {
    fn from(other: Vec<u8>) -> Self {
        Value::Base64(other)
    }
}
