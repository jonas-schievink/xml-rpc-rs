#[cfg(feature = "http")]
extern crate reqwest;
#[cfg(feature = "http")]
extern crate mime;

use Value;
use error::{Error, RequestErrorKind};
use utils::escape_xml;
use transport::Transport;
use parser::parse_response;

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

    /// Creates a "multicall" request that will perform multiple requests at once.
    ///
    /// This requires that the server supports the [`system.multicall`] method.
    ///
    /// [`system.multicall`]: https://mirrors.talideon.com/articles/multicall.html
    #[allow(deprecated)]
    pub fn new_multicall<'r, I>(requests: I) -> Self
    where 'a: 'r, I: IntoIterator<Item=&'r Request<'a>> {
        Request {
            name: "system.multicall",
            args: vec![Value::Array(requests.into_iter().map(|req| {
                let mut multicall_struct: BTreeMap<String, Value> = BTreeMap::new();

                multicall_struct.insert("methodName".into(), req.name.into());
                multicall_struct.insert("params".into(), Value::Array(req.args.clone()));

                Value::Struct(multicall_struct)
            }).collect())],
        }
    }

    /// Appends an argument to be passed to the current list of arguments.
    pub fn arg<T: Into<Value>>(mut self, value: T) -> Self {
        self.args.push(value.into());
        self
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
            .map_err(RequestErrorKind::TransportError)?;

        let response = parse_response(&mut reader).map_err(RequestErrorKind::ParseError)?;

        let value = response.map_err(RequestErrorKind::Fault)?;
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
    /// This method is only available when the `http` feature is enabled (this is the default).
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
    #[cfg(feature = "http")]
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
    #[deprecated(since="0.11.2", note="use `Request::multicall` instead")]
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
