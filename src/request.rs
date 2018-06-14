#[cfg(feature = "reqwest")]
extern crate reqwest;

use Value;
use error::{Error, ErrorKind};
use utils::escape_xml;
use transport::Transport;
use parser::parse_response;

use serde::Serialize;
use std::io::{self, Write};
use std::collections::BTreeMap;

/// A request to call a procedure.
#[derive(Clone, Debug)]
pub struct Request<'a> {
    name: &'a str,
    args: Vec<Value>,
}

impl<'a> Request<'a> {
    /// Creates a new request to call a function named `name`.
    ///
    /// By default, no arguments are passed. Use the `arg` method to append arguments.
    pub fn new(name: &'a str) -> Self {
        Request {
            name,
            args: Vec::new(),
        }
    }

    /// Appends an argument to be passed to the current list of arguments.
    pub fn arg<T: Into<Value>>(mut self, value: T) -> Self {
        self.args.push(value.into());
        self
    }

    /// Appends a serializable value as an argument.
    pub fn arg_serializable<T: Serialize>(mut self, value: T) -> Result<Self, Error> {
        let value = value.serialize(&mut ::ser::Serializer::with_extensions())?;
        self.args.push(value);
        Ok(self)
    }

    /// Performs the request using a [`Transport`].
    ///
    /// If you want to send the request using an HTTP POST request, you can also use [`call_url`],
    /// which creates a suitable [`Transport`] internally.
    ///
    /// # Errors
    ///
    /// Any errors that occur while sending the request using the [`Transport`] will be returned to
    /// the caller. Additionally, if the response is malformed (invalid XML), or indicates that the
    /// method call failed, an error will also be returned.
    ///
    /// [`call_url`]: #method.call_url
    /// [`Transport`]: trait.Transport.html
    pub fn call<T: Transport>(&self, transport: T) -> Result<Value, Error> {
        let mut reader = transport.transmit(self)
            .map_err(ErrorKind::TransportError)?;

        let response = parse_response(&mut reader).map_err(|e| ErrorKind::from(e))?;

        let value = response.map_err(ErrorKind::Fault)?;
        Ok(value)
    }

    /// Performs the request on a URL.
    ///
    /// You can pass a `&str` or an already parsed reqwest URL.
    ///
    /// This is a convenience method that will internally create a new `reqwest::Client` and send an
    /// HTTP POST request to the given URL. If you only use this method to perform requests, you
    /// don't need to depend on `reqwest` yourself.
    ///
    /// This method is only available when the `reqwest` feature is enabled (this is the default).
    ///
    /// # Errors
    ///
    /// Since this is just a convenience wrapper around [`Request::call`], the same error conditions
    /// apply.
    ///
    /// Any reqwest errors will be propagated to the caller.
    ///
    /// [`Request::call`]: #method.call
    /// [`Transport`]: trait.Transport.html
    #[cfg(feature = "reqwest")]
    pub fn call_url<U: reqwest::IntoUrl>(&self, url: U) -> Result<Value, Error> {
        // While we could implement `Transport` for `T: IntoUrl`, such an impl might not be
        // completely obvious (as it applies to `&str`), so I've added this method instead.
        // Might want to reconsider if someone has an objection.
        self.call(reqwest::Client::new().post(url))
    }

    /// Formats this `Request` as a UTF-8 encoded XML document.
    ///
    /// # Errors
    ///
    /// Any errors reported by the writer will be propagated to the caller. If the writer never
    /// returns an error, neither will this method.
    pub fn write_as_xml<W: Write>(&self, fmt: &mut W) -> io::Result<()> {
        write!(fmt, r#"<?xml version="1.0" encoding="utf-8"?>"#)?;
        write!(fmt, r#"<methodCall>"#)?;
        write!(fmt, r#"    <methodName>{}</methodName>"#, escape_xml(&self.name))?;
        write!(fmt, r#"    <params>"#)?;
        for value in &self.args {
            write!(fmt, r#"        <param>"#)?;
            value.write_as_xml(fmt)?;
            write!(fmt, r#"        </param>"#)?;
        }
        write!(fmt, r#"    </params>"#)?;
        write!(fmt, r#"</methodCall>"#)?;
        Ok(())
    }

    /// Serialize this `Request` into an XML-RPC struct that can be passed to
    /// the [`system.multicall`](https://mirrors.talideon.com/articles/multicall.html)
    /// XML-RPC method, specifically a struct with two fields:
    ///
    /// * `methodName`: the request name
    /// * `params`: the request arguments
    pub fn into_multicall_struct(self) -> Value {
        let mut multicall_struct: BTreeMap<String, Value> = BTreeMap::new();

        multicall_struct.insert("methodName".into(), self.name.into());
        multicall_struct.insert("params".into(), Value::Array(self.args));

        Value::Struct(multicall_struct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;
    use std::{i64, f32};

    #[test]
    fn escapes_method_names() {
        let mut output: Vec<u8> = Vec::new();
        let req = Request::new("x<&x");

        req.write_as_xml(&mut output).unwrap();
        assert!(
            str::from_utf8(&output)
            .unwrap()
            .contains("<methodName>x&lt;&amp;x</methodName>"));
    }

    #[test]
    fn serialization() {
        #[derive(Serialize)]
        enum MyEnum {
            StructVariant {
                field: Option<()>,    // `Some(())` and `None` serialize the same, like in serde_json
            },
            TupleVariant(u8, i8),
            UnitVariant,
        }

        #[derive(Serialize)]
        struct MyStruct {
            empty_string: String,
            string: String,
            bool_: bool,
            opt_bool: Option<bool>,
            result: Result<i64, [u8; 4]>,
            large: i64,
            float: f32,
            myenum: [MyEnum; 3],
        }

        let mut output: Vec<u8> = Vec::new();
        Request::new("a").arg_serializable(MyStruct {
            empty_string: String::new(),
            string: "blablabli\0blo".to_string(),
            bool_: false,
            opt_bool: None,
            result: Err([0x99, 0xff, 0x00, 0x00]),
            large: i64::MIN,
            float: f32::EPSILON,
            myenum: [
                MyEnum::StructVariant { field: Some(()) },
                MyEnum::TupleVariant(100, -128),
                MyEnum::UnitVariant,
            ],
        }).unwrap().write_as_xml(&mut output).unwrap();

        assert_eq!("<?xml version=\"1.0\" encoding=\"utf-8\"?><methodCall>    <methodName>a</methodName>    <params>        <param><value>\n<struct>\n<member>\n<name>bool_</name>\n<value>\n<boolean>0</boolean>\n</value>\n</member>\n<member>\n<name>empty_string</name>\n<value>\n<string></string>\n</value>\n</member>\n<member>\n<name>float</name>\n<value>\n<double>0.00000011920928955078125</double>\n</value>\n</member>\n<member>\n<name>large</name>\n<value>\n<i8>-9223372036854775808</i8>\n</value>\n</member>\n<member>\n<name>myenum</name>\n<value>\n<array>\n<data>\n<value>\n<struct>\n<member>\n<name>StructVariant</name>\n<value>\n<struct>\n<member>\n<name>field</name>\n<value>\n<nil/>\n</value>\n</member>\n</struct>\n</value>\n</member>\n</struct>\n</value>\n<value>\n<struct>\n<member>\n<name>TupleVariant</name>\n<value>\n<array>\n<data>\n<value>\n<i4>100</i4>\n</value>\n<value>\n<i4>-128</i4>\n</value>\n</data>\n</array>\n</value>\n</member>\n</struct>\n</value>\n<value>\n<string>UnitVariant</string>\n</value>\n</data>\n</array>\n</value>\n</member>\n<member>\n<name>opt_bool</name>\n<value>\n<nil/>\n</value>\n</member>\n<member>\n<name>result</name>\n<value>\n<struct>\n<member>\n<name>Err</name>\n<value>\n<array>\n<data>\n<value>\n<i4>153</i4>\n</value>\n<value>\n<i4>255</i4>\n</value>\n<value>\n<i4>0</i4>\n</value>\n<value>\n<i4>0</i4>\n</value>\n</data>\n</array>\n</value>\n</member>\n</struct>\n</value>\n</member>\n<member>\n<name>string</name>\n<value>\n<string>blablabli\u{0}blo</string>\n</value>\n</member>\n</struct>\n</value>\n        </param>    </params></methodCall>", str::from_utf8(&output).unwrap());
    }
}
