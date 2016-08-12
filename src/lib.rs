extern crate base64;
extern crate hyper;
extern crate chrono;
extern crate xml;

mod error;
mod parser;
mod value;

pub use error::{RequestError, Fault};
pub use value::Value;

use parser::parse_response;
use hyper::client::{Client, Body};
use hyper::header::UserAgent;
use xml::EventReader;

/// A request to call a procedure.
pub struct Request<'a> {
    name: &'a str,
    args: Vec<(&'a str, Value)>,
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
    pub fn arg<T: Into<Value>>(mut self, name: &'a str, value: T) -> Self {
        self.args.push((name, value.into()));

        Request {
            name: self.name,
            args: self.args,
        }
    }

    /// Calls the method using the given `Client`.
    ///
    /// This will send the request to the `/` URL.
    ///
    /// Returns a `RequestResult` indicating whether the request was sent and processed successfully
    /// (according to the rules of XML-RPC).
    pub fn call(self, client: Client) -> RequestResult {
        // First, build the body XML
        let body = "";

        // Send XML-RPC request
        let response = try!(client.post("/")
            .header(UserAgent("Rust xmlrpc".to_string()))
            .body(Body::BufBody(body.as_bytes(), body.len()))
            .send());

        // FIXME Check that the response headers are correct

        // Read the response and parse it
        // FIXME `BufRead`?
        Ok(try!(parse_response(&mut EventReader::new(response))))
    }
}

/// The result of executing a request.
///
/// When the request was executed without major errors (like an HTTP error or a malformed response),
/// this is `Ok`. The `Response` can still denote a `Fault` if the server returned a `<fault>`
/// response.
pub type RequestResult = Result<Response, RequestError>;

/// A response from the server.
///
/// XML-RPC specifies that a call should either return a single `Value`, or a `<fault>`.
pub type Response = Result<Value, Fault>;
