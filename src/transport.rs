use Request;

use std::io::Read;
use std::error::Error;

/// Request and response transport abstraction.
///
/// The `Transport` trait provides a way to send a `Request` to a server and to receive the
/// corresponding response. A `Transport` implementor is passed to [`Request::call`] in order to use
/// it to perform that request.
///
/// The most commonly used transport is simple HTTP: If the `reqwest` feature is enabled (it is by
/// default), the reqwest `RequestBuilder` will implement this trait and send the XML-RPC
/// [`Request`] via HTTP.
///
/// You can implement this trait for your own types if you want to customize how requests are sent.
/// You can modify HTTP headers or wrap requests in a completely different protocol.
///
/// [`Request::call`]: struct.Request.html#method.call
/// [`Request`]: struct.Request.html
pub trait Transport {
    // FIXME replace with `impl Trait` when stable
    /// The response stream returned by `transmit`.
    type Stream: Read;

    /// Transmits an XML-RPC request and returns the server's response.
    ///
    /// The response is returned as a `Self::Stream` - some type implementing the `Read` trait. The
    /// library will read all of the data and parse it as a response. It must be UTF-8 encoded XML,
    /// otherwise the call will fail.
    ///
    /// # Errors
    ///
    /// If a transport error occurs, it should be returned as a boxed error - the library will then
    /// return an appropriate [`Error`] to the caller.
    ///
    /// [`Error`]: struct.Error.html
    fn transmit(self, request: &Request) -> Result<Self::Stream, Box<Error + Send + Sync>>;
}

// FIXME: Link to `Transport` and `RequestBuilder` using intra-rustdoc links. Relative links break
// everything and abs. links don't work locally.
/// Provides helpers for implementing custom `Transport`s using reqwest.
///
/// This module will be disabled if the `reqwest` feature is not enabled.
///
/// The default [`Transport`] implementation for `RequestBuilder` looks roughly like
/// this:
///
/// ```notrust
/// // serialize request into `body` (a `Vec<u8>`)
///
/// build_headers(builder, body.len());
///
/// // send `body` using `builder` and get response
///
/// check_response(&response)?;
/// ```
///
/// From this, you can build your own custom transports.
///
/// [`Transport`]: ../trait.Transport.html
#[cfg(feature = "reqwest")]
pub mod http {
    extern crate reqwest;

    use {Request, Transport};
    use self::reqwest::{RequestBuilder, mime};
    use self::reqwest::header::{ContentType, ContentLength, UserAgent};

    use std::error::Error;

    /// Appends all HTTP headers required by the XML-RPC specification to the `RequestBuilder`.
    ///
    /// More specifically, the following headers are set:
    ///
    /// ```notrust
    /// User-Agent: Rust xmlrpc
    /// Content-Type: text/xml; charset=utf-8
    /// Content-Length: $body_len
    /// ```
    pub fn build_headers(builder: &mut RequestBuilder, body_len: u64) {
        // Set all required request headers
        // NB: The `Host` header is also required, but reqwest adds it automatically, since
        // HTTP/1.1 requires it.
        builder
            .header(UserAgent::new("Rust xmlrpc"))
            .header(ContentType("text/xml; charset=utf-8".parse().unwrap()))
            .header(ContentLength(body_len));
    }

    /// Checks that a reqwest `Response` has a status code indicating success and contains all
    /// required headers.
    ///
    /// In particular, the following headers must be present:
    ///
    /// ```notrust
    /// Content-Type: text/xml
    /// Content-Length: <body length>
    /// ```
    pub fn check_response(response: &reqwest::Response) -> Result<(), Box<Error + Send + Sync>> {
        // This is essentially an open-coded version of `Response::error_for_status` that does not
        // consume the response.
        if response.status().is_client_error() || response.status().is_server_error() {
            return Err(format!("server response indicates error: {}", response.status()).into());
        }

        // Check response headers
        // "The Content-Type is text/xml. Content-Length must be present and correct."
        if let Some(content) = response.headers().get::<ContentType>() {
            // (we ignore this if the header is missing completely)
            match (content.type_(), content.subtype()) {
                (mime::TEXT, mime::XML) => {},
                (ty, sub) => return Err(
                    format!("expected Content-Type 'text/xml', got '{}/{}'", ty, sub).into()
                ),
            }
        }

        response.headers().get::<ContentLength>()
            .ok_or_else(|| Box::<Error + Send + Sync>::from(format!("expected Content-Length header, but none was found")))?;

        Ok(())
    }

    /// Use a `RequestBuilder` as the transport.
    ///
    /// The request will be sent as specified in the XML-RPC specification: A default `User-Agent`
    /// will be set, along with the correct `Content-Type` and `Content-Length`.
    impl Transport for RequestBuilder {
        type Stream = reqwest::Response;

        fn transmit(mut self, request: &Request) -> Result<Self::Stream, Box<Error + Send + Sync>> {
            // First, build the body XML
            let mut body = Vec::new();
            // This unwrap never panics as we are using `Vec<u8>` as a `Write` implementor,
            // and not doing anything else that could return an `Err` in `write_as_xml()`.
            request.write_as_xml(&mut body).unwrap();

            build_headers(&mut self, body.len() as u64);

            let response = self
                .body(body)
                .send()?;

            check_response(&response)?;

            Ok(response)
        }
    }
}
