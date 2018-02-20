//! Contains the different types of values understood by XML-RPC.

use utils::{escape_xml, format_datetime};

use base64::encode;
use iso8601::DateTime;

use std::collections::BTreeMap;
use std::io::{self, Write};

/// The possible XML-RPC values.
///
/// Nested values can be accessed by using [`get`](#method.get) method and Rust's square-bracket
/// indexing operator.
///
/// A string index can be used to access a value in a `Struct`, and a `usize` index can be used to
/// access an element of an `Array`.
///
/// # Examples
///
/// ```
/// # use std::collections::BTreeMap;
/// # use xmlrpc::{Value, Index};
/// let nothing = Value::Nil;
///
/// let person = Value::Struct(vec![
///     ("name".to_string(), Value::from("John Doe")),
///     ("age".to_string(), Value::from(37)),
///     ("children".to_string(), Value::Array(vec![
///         Value::from("Mark"),
///         Value::from("Jennyfer")
///     ])),
/// ].into_iter().collect());
///
/// // get
/// assert_eq!(nothing.get("name"), None);
/// assert_eq!(person.get("name"), Some(&Value::from("John Doe")));
/// assert_eq!(person.get("SSN"), None);
///
/// // index
/// assert_eq!(nothing["name"], Value::Nil);
/// assert_eq!(person["name"], Value::from("John Doe"));
/// assert_eq!(person["age"], Value::Int(37));
/// assert_eq!(person["SSN"], Value::Nil);
/// assert_eq!(person["children"][0], Value::from("Mark"));
/// assert_eq!(person["children"][0]["age"], Value::Nil);
/// assert_eq!(person["children"][2], Value::Nil);
///
/// // extract values
/// assert_eq!(person["name"].as_str(), Some("John Doe"));
/// assert_eq!(person["age"].as_i32(), Some(37));
/// assert_eq!(person["age"].as_bool(), None);
/// assert_eq!(person["children"].as_array().unwrap().len(), 2);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `<i4>` or `<int>`, 32-bit signed integer.
    Int(i32),
    /// `<i8>`, 64-bit signed integer.
    ///
    /// This is an XMLRPC extension and may not be supported by all clients / servers.
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

    /// `<nil/>`, the empty (Unit) value.
    ///
    /// This is an XMLRPC [extension][ext] and may not be supported by all clients / servers.
    ///
    /// [ext]: https://web.archive.org/web/20050911054235/http://ontosys.com/xml-rpc/extensions.php
    Nil,
}

impl Value {
    /// Formats this `Value` as an XML `<value>` element.
    ///
    /// # Errors
    ///
    /// Any error reported by the writer will be propagated to the caller.
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

    /// Returns an inner struct or array value indexed by `index`.
    ///
    /// Returns `None` if the member doesn't exist or `self` is neither a struct nor an array.
    ///
    /// You can also use Rust's square-bracket indexing syntax to perform this operation if you want
    /// a default value instead of an `Option`. Refer to the top-level [examples](#examples) for
    /// details.
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        index.get(self)
    }

    /// If the `Value` is a normal integer (`Value::Int`), returns associated value. Returns `None`
    /// otherwise.
    ///
    /// In particular, `None` is also returned if `self` is a `Value::Int64`. Use [`as_i64`] to
    /// handle this case.
    ///
    /// [`as_i64`]: #method.as_i64
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Value::Int(i) => Some(i),
            _ => None
        }
    }

    /// If the `Value` is an integer, returns associated value. Returns `None` otherwise.
    ///
    /// This works with both `Value::Int` and `Value::Int64`.
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Value::Int(i) => Some(i64::from(i)),
            Value::Int64(i) => Some(i),
            _ => None
        }
    }

    /// If the `Value` is a boolean, returns associated value. Returns `None` otherwise.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(b) => Some(b),
            _ => None
        }
    }

    /// If the `Value` is a string, returns associated value. Returns `None` otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Value::String(ref s) => Some(s),
            _ => None
        }
    }

    /// If the `Value` is a floating point number, returns associated value. Returns `None`
    /// otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::Double(d) => Some(d),
            _ => None
        }
    }

    /// If the `Value` is a date/time, returns associated value. Returns `None` otherwise.
    pub fn as_datetime(&self) -> Option<DateTime> {
        match *self {
            Value::DateTime(dt) => Some(dt),
            _ => None
        }
    }

    /// If the `Value` is base64 binary data, returns associated value. Returns `None` otherwise.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match *self {
            Value::Base64(ref data) => Some(data),
            _ => None
        }
    }

    /// If the `Value` is a struct, returns associated map. Returns `None` otherwise.
    pub fn as_struct(&self) -> Option<&BTreeMap<String, Value>> {
        match *self {
            Value::Struct(ref map) => Some(map),
            _ => None
        }
    }

    /// If the `Value` is an array, returns associated slice. Returns `None` otherwise.
    pub fn as_array(&self) -> Option<&[Value]> {
        match *self {
            Value::Array(ref array) => Some(array),
            _ => None
        }
    }
}

impl From<i32> for Value {
    fn from(other: i32) -> Self {
        Value::Int(other)
    }
}

impl From<i64> for Value {
    fn from(other: i64) -> Self {
        Value::Int64(other)
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

impl From<Vec<u8>> for Value {
    fn from(other: Vec<u8>) -> Self {
        Value::Base64(other)
    }
}

mod sealed {
    /// A trait that is only nameable (and thus implementable) inside this crate.
    pub trait Sealed {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl Sealed for usize {}
    impl<'a, I> Sealed for &'a I where I: Sealed + ?Sized {}
}

/// A type that can be used to index into a [`Value`].
///
/// You can use Rust's regular indexing syntax to access components of [`Value`]s. Refer to the
/// examples on [`Value`] for details.
///
/// This trait can not be implemented by custom types.
///
/// [`Value`]: enum.Value.html
pub trait Index: sealed::Sealed {
    /// Gets an inner value of a given value represented by self.
    #[doc(hidden)]
    fn get<'v>(&self, value: &'v Value) -> Option<&'v Value>;
}

impl Index for str {
    fn get<'v>(&self, value: &'v Value) -> Option<&'v Value> {
        if let Value::Struct(ref map) = *value {
            map.get(self)
        } else {
            None
        }
    }
}

impl Index for String {
    fn get<'v>(&self, value: &'v Value) -> Option<&'v Value> {
        if let Value::Struct(ref map) = *value {
            map.get(self)
        } else {
            None
        }
    }
}

impl Index for usize {
    fn get<'v>(&self, value: &'v Value) -> Option<&'v Value> {
        if let Value::Array(ref array) = *value {
            array.get(*self)
        } else {
            None
        }
    }
}

impl<'a, I> Index for &'a I where I: Index + ?Sized {
    fn get<'v>(&self, value: &'v Value) -> Option<&'v Value> {
        (*self).get(value)
    }
}

impl<I> ::std::ops::Index<I> for Value where I: Index {
    type Output = Value;
    fn index(&self, index: I) -> &Self::Output {
        index.get(self).unwrap_or(&Value::Nil)
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

    #[test]
    fn access_nested_values() {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert("name".to_string(), Value::from("John Doe"));
        map.insert("age".to_string(), Value::from(37));
        map.insert("children".to_string(), Value::Array(vec![Value::from("Mark"), Value::from("Jennyfer")]));
        let value = Value::Struct(map);

        assert_eq!(value.get("name"), Some(&Value::from("John Doe")));
        assert_eq!(value.get("age"), Some(&Value::from(37)));
        assert_eq!(value.get("birthdate"), None);
        assert_eq!(Value::Nil.get("age"), None);
        assert_eq!(value["name"], Value::from("John Doe"));
        assert_eq!(value["age"], Value::from(37));
        assert_eq!(value["birthdate"], Value::Nil);
        assert_eq!(Value::Nil["age"], Value::Nil);
        assert_eq!(value["children"][0], Value::from("Mark"));
        assert_eq!(value["children"][1], Value::from("Jennyfer"));
        assert_eq!(value["children"][2], Value::Nil);

        assert_eq!(value["age"].as_i32(), Some(37));
        assert_eq!(value["children"][0].as_str(), Some("Mark"));
    }
}
