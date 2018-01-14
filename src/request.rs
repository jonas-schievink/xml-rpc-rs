use {Value, Response};
use parser::parse_response;
use error::RequestError;
use utils::escape_xml;

use reqwest::{Body, Client, RequestBuilder, StatusCode};
use reqwest::header::{ContentType, ContentLength, UserAgent};

use std::io::{self, Cursor, Write};
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

    /// Calls the method using the given `Client`.
    ///
    /// Returns a `RequestResult` indicating whether the request was sent and processed successfully
    /// (according to the rules of XML-RPC).
    pub fn call(&self, client: &Client, url: &str) -> RequestResult {
        self.call_with(client, url, |_| {})
    }

    /// Calls the method, giving the closure a chance to amend the reqwest
    /// `RequestBuilder` before sending.
    pub fn call_with<F>(&self, client: &Client, url: &str, cb: F) -> RequestResult
        where F: Fn(&mut RequestBuilder)
    {
        // First, build the body XML
        let mut body = Vec::new();
        // This unwrap never panics as we are using `Vec<u8>` as a `Write` implementor,
        // and not doing anything else that could return an `Err` in `write_as_xml()`.
        self.write_as_xml(&mut body).unwrap();

        // Send XML-RPC request
        let mut builder = client.post(url);
        builder
            .header(UserAgent::new("Rust xmlrpc"))
            .header(ContentType("text/xml; charset=utf-8".parse().unwrap()))
            .header(ContentLength(body.len() as u64))
            .body(Body::new(Cursor::new(body)));
        cb(&mut builder);
        let mut response = builder.send()?;

        // FIXME Check that the response headers are correct
        if response.status() != StatusCode::Ok {
            Err(RequestError::HttpStatus(format!("{}", response.status())))
        } else {
            // Read the response and parse it
            // FIXME `BufRead`?
            Ok(parse_response(&mut response)?)
        }
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
