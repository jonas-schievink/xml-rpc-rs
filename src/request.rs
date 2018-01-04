#[cfg(feature = "reqwest")]
extern crate reqwest;

use {Value, Response};
use error::RequestError;
use utils::escape_xml;
use transport::Transport;
use parser::parse_response;

use std::io::{self, Write};
use std::collections::BTreeMap;

/// A request to call a procedure.
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

    /// Performs the request using a `transport`.
    ///
    /// Returns a `RequestResult` indicating whether the request was sent and processed successfully
    /// (according to the rules of XML-RPC).
    pub fn call<T: Transport>(&self, transport: T) -> RequestResult {
        let mut reader = transport.transmit(self)
            .map_err(RequestError::TransportError)?;

        parse_response(&mut reader).map_err(RequestError::ParseError)
    }

    /// Performs the request on a URL.
    ///
    /// This is a convenience method that will internally create a new `reqwest::Client` and send a
    /// POST HTTP request to the given URL. If you only use this method to perform requests, you
    /// don't need to depend on `reqwest` yourself.
    ///
    /// This method is only available when the `reqwest` feature is enabled (this is the default).
    #[cfg(feature = "reqwest")]
    pub fn call_url(&self, url: &str) -> RequestResult {
        self.call(reqwest::Client::new().post(url))
    }

    /// Formats this `Request` as XML.
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

/// The result of executing a request.
///
/// When the request was executed without major errors (like an HTTP error or a malformed response),
/// this is `Ok`. The `Response` can still denote a `Fault` if the server returned a `<fault>`
/// response.
pub type RequestResult = Result<Response, RequestError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

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
}
