//! Contains the different types of values understood by XML-RPC.

use utils::{escape_xml, format_datetime};

use base64::encode;
use iso8601::DateTime;

use std::collections::BTreeMap;
use std::io::{self, Write};

/// The possible XML-RPC values.
#[derive(Debug, PartialEq)]
pub enum Value {
    /// `<i4>` or `<int>`, 32-bit signed integer.
    Int(i32),
    /// `<i8>`, 64-bit signed integer.
    ///
    /// This is a non-standard feature that may not be supported on all servers
    /// or clients.
    Int64(i64),
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

    /// `</nil>`
    ///
    /// Ref: https://web.archive.org/web/20050911054235/http://ontosys.com/xml-rpc/extensions.php
    Nil
}

impl Value {
    /// Formats this `Value` as an XML `<value>` element.
    pub fn write_as_xml<W: Write>(&self, fmt: &mut W) -> io::Result<()> {
        writeln!(fmt, "<value>")?;

        match *self {
            Value::Int(i) => {
                writeln!(fmt, "<i4>{}</i4>", i)?;
            }
            Value::Int64(i) => {
                writeln!(fmt, "<i8>{}</i8>", i)?;
            }
            Value::Bool(b) => {
                writeln!(fmt, "<boolean>{}</boolean>", if b { "1" } else { "0" })?;
            }
            Value::String(ref s) => {
                writeln!(fmt, "<string>{}</string>", escape_xml(s))?;
            }
            Value::Double(d) => {
                writeln!(fmt, "<double>{}</double>", d)?;
            }
            Value::DateTime(date_time) => {
                writeln!(fmt, "<dateTime.iso8601>{}</dateTime.iso8601>", format_datetime(&date_time))?;
            }
            Value::Base64(ref data) => {
                writeln!(fmt, "<base64>{}</base64>", encode(data))?;
            }
            Value::Struct(ref map) => {
                writeln!(fmt, "<struct>")?;
                for (ref name, ref value) in map {
                    writeln!(fmt, "<member>")?;
                    writeln!(fmt, "<name>{}</name>", escape_xml(name))?;
                    value.write_as_xml(fmt)?;
                    writeln!(fmt, "</member>")?;
                }
                writeln!(fmt, "</struct>")?;
            }
            Value::Array(ref array) => {
                writeln!(fmt, "<array>")?;
                writeln!(fmt, "<data>")?;
                for value in array {
                    value.write_as_xml(fmt)?;
                }
                writeln!(fmt, "</data>")?;
                writeln!(fmt, "</array>")?;
            }
            Value::Nil => {
                writeln!(fmt, "<nil/>")?;
            }
        }

        writeln!(fmt, "</value>")?;
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

impl<'a> From<&'a str> for Value {
    fn from(other: &'a str) -> Self {
        Value::String(other.to_string())
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

// FIXME This impl isn't obvious - theoretically you can use <string> to transfer binary data!
// (also see https://github.com/jonas-schievink/xml-rpc-rs/issues/17)
impl From<Vec<u8>> for Value {
    fn from(other: Vec<u8>) -> Self {
        Value::Base64(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;
    use std::collections::BTreeMap;

    #[test]
    fn escapes_strings() {
        let mut output: Vec<u8> = Vec::new();

        Value::from("<xml>&nbsp;string").write_as_xml(&mut output).unwrap();
        assert_eq!(str::from_utf8(&output).unwrap(), "<value>\n<string>&lt;xml>&amp;nbsp;string</string>\n</value>\n");
    }

    #[test]
    fn escapes_struct_member_names() {
        let mut output: Vec<u8> = Vec::new();
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert("x&<x".to_string(), Value::from(true));

        Value::Struct(map).write_as_xml(&mut output).unwrap();
        assert_eq!(str::from_utf8(&output).unwrap(), "<value>\n<struct>\n<member>\n<name>x&amp;&lt;x</name>\n<value>\n<boolean>1</boolean>\n</value>\n</member>\n</struct>\n</value>\n");
    }
}
