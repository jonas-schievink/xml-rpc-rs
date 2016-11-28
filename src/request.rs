use {Value, Response};
use parser::parse_response;
use error::RequestError;
use utils::escape_xml;

use hyper::client::{Client, Body};
use hyper::header::{ContentType, UserAgent};

use std::io::{self, Write};

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
            name: name,
            args: Vec::new(),
        }
    }

    /// Appends an argument to be passed to the current list of arguments.
    pub fn arg<T: Into<Value>>(mut self, value: T) -> Self {
        self.args.push(value.into());

        Request {
            name: self.name,
            args: self.args,
        }
    }

    /// Calls the method using the given `Client`.
    ///
    /// Returns a `RequestResult` indicating whether the request was sent and processed successfully
    /// (according to the rules of XML-RPC).
    pub fn call(&self, client: &Client, url: &str) -> RequestResult {
        use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};

        // First, build the body XML
        let mut body = Vec::new();
        try!(self.write_as_xml(&mut body));

        // Send XML-RPC request
        let mut response = try!(client.post(url)
            .header(UserAgent("Rust xmlrpc".to_string()))
            .header(ContentType(Mime(TopLevel::Text, SubLevel::Xml, vec![(Attr::Charset, Value::Utf8)])))
            .body(Body::BufBody(&body, body.len()))
            .send());

        // FIXME Check that the response headers are correct

        // Read the response and parse it
        // FIXME `BufRead`?
        Ok(try!(parse_response(&mut response)))
    }

    /// Formats this `Request` as XML.
    pub fn write_as_xml<W: Write>(&self, fmt: &mut W) -> io::Result<()> {
        try!(write!(fmt, r#"<?xml version="1.0" encoding="utf-8"?>"#));
        try!(write!(fmt, r#"<methodCall>"#));
        try!(write!(fmt, r#"    <methodName>{}</methodName>"#, escape_xml(&self.name)));
        try!(write!(fmt, r#"    <params>"#));
        for value in &self.args {
            try!(write!(fmt, r#"        <param>"#));
            try!(value.format(fmt));
            try!(write!(fmt, r#"        </param>"#));
        }
        try!(write!(fmt, r#"    </params>"#));
        try!(write!(fmt, r#"</methodCall>"#));
        Ok(())
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
