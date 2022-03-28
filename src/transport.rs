use crate::request::Request;
use std::error::Error;
use std::io::Read;

/// Request and response transport abstraction.
///
/// The `Transport` trait provides a way to send a `Request` to a server and to receive the
/// corresponding response. A `Transport` implementor is passed to [`Request::call`] in order to use
/// it to perform that request.
///
/// The most commonly used transport is simple HTTP: If the `http` feature is enabled (it is by
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
    fn transmit(self, request: &Request<'_>) -> std::result::Result<Self::Stream, Box<dyn Error + Send + Sync + 'static>>;
}

// FIXME: Link to `Transport` and `RequestBuilder` using intra-rustdoc links. Relative links break
// everything and abs. links don't work locally.
/// Provides helpers for implementing custom `Transport`s using reqwest.
///
/// This module will be disabled if the `http` feature is not enabled.
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
#[cfg(feature = "http")]
pub mod http {
    extern crate mime;
    extern crate reqwest;

    use std::io::Cursor;
    use self::mime::Mime;
    use self::reqwest::RequestBuilder;
    use self::reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, USER_AGENT};
    use crate::request::Request;
    use crate::transport::Transport;
    use tokio::runtime::Runtime;

    use std::error::Error;
    use std::str::FromStr;

    /// Appends all HTTP headers required by the XML-RPC specification to the `RequestBuilder`.
    ///
    /// More specifically, the following headers are set:
    ///
    /// ```notrust
    /// User-Agent: Rust xmlrpc
    /// Content-Type: text/xml; charset=utf-8
    /// Content-Length: $body_len
    /// ```
    pub fn build_headers(builder: RequestBuilder, body_len: u64) -> RequestBuilder {
        // Set all required request headers
        // NB: The `Host` header is also required, but reqwest adds it automatically, since
        // HTTP/1.1 requires it.
        builder
            .header(USER_AGENT, "Rust xmlrpc")
            .header(CONTENT_TYPE, "text/xml; charset=utf-8")
            .header(CONTENT_LENGTH, body_len)
    }

    // fn send_request(builder: RequestBuilder, body: Vec<u8>) -> reqwest::Response {
    //     let body = build_headers(builder, body.len() as u64).body(body);
    //     let resp = async move {body.send().await};

    //     std::future::block_on(resp)
    // }


    /// Checks that a reqwest `Response` has a status code indicating success and verifies certain
    /// headers.
    pub fn check_response(
        response: &reqwest::Response,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // This is essentially an open-coded version of `Response::error_for_status` that does not
        // consume the response.
        if response.status().is_client_error() || response.status().is_server_error() {
            return Err(format!("server response indicates error: {}", response.status()).into());
        }

        // Check response headers
        // "The Content-Type is text/xml. Content-Length must be present and correct."
        if let Some(content) = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Mime::from_str(value).ok())
        {
            // (we ignore this if the header is missing completely)
            match (content.type_(), content.subtype()) {
                (mime::TEXT, mime::XML) => {}
                (ty, sub) => {
                    return Err(
                        format!("expected Content-Type 'text/xml', got '{}/{}'", ty, sub).into(),
                    )
                }
            }
        }

        // We ignore the Content-Length header because it doesn't matter for the parser and reqwest
        // will remove it when the response is gzip compressed.

        Ok(())
    }

    // private execution of the transport
    // fn transport_exec(builder: RequestBuilder) -> Result<&'static[u8], Box<dyn Error + Send + Sync>> {

    // }

    /// Use a `RequestBuilder` as the transport.
    ///
    /// The request will be sent as specified in the XML-RPC specification: A default `User-Agent`
    /// will be set, along with the correct `Content-Type` and `Content-Length`.
    impl Transport for RequestBuilder {
        //type Stream = reqwest::Response;
        //type Stream = &'static[u8];
        type Stream = Cursor<String>;

        fn transmit(
            self,
            request: &Request<'_>,
        ) -> Result<Self::Stream, Box<(dyn Error + Send + Sync + 'static)>> {
            // First, build the body XML
            let mut body = Vec::new();
            // This unwrap never panics as we are using `Vec<u8>` as a `Write` implementor,
            // and not doing anything else that could return an `Err` in `write_as_xml()`.
            request.write_as_xml(&mut body).unwrap();
            // async part needs to go to separate thread because of interference with caller
            // see https://stackoverflow.com/questions/52521201/how-do-i-synchronously-return-a-value-calculated-in-an-asynchronous-future-in-st
            // https://stackoverflow.com/questions/62536566/how-can-i-create-a-tokio-runtime-inside-another-tokio-runtime-without-getting-th
            
            // check https://stackoverflow.com/questions/61996298/using-rust-libraries-reqwest-and-select-in-conjunction
            // let resp = async move {
            //     match build_headers(self, body.len() as u64).body(body).send().await {
            //         Ok(r) => Ok(r),
            //         Err(e) => Err(e)
            //     }
            // };
            // //let response: reqwest::Response;
            // let response = std::thread::spawn( || { match Runtime::new().unwrap().block_on(resp) {
            //     Ok(o) => o,
            //     Err(_) => ()
            // } }).join();

            // if response.is_ok() {
            // check_response(&response.as_ref().unwrap())?;
            // }

            // let bv = async move { 
            //     match response.unwrap().text().await {
            //         Ok(r) => Ok(r),
            //         Err(e) => Err(e)
            //     }
            // };
            // let rv = std::thread::spawn( || {Runtime::new().unwrap().block_on(bv).unwrap()}).join();
            
            // rewrite all, async block
            let ab = async move {
                let rv = build_headers(self, body.len() as u64).body(body).send().await?;
                check_response(&rv).unwrap();
                rv.text().await
            };

            let rs = std::thread::spawn( || {Runtime::new().unwrap().block_on(ab)}).join().unwrap().unwrap();//.map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>);
            let rc = Cursor::new(rs);

            Ok(rc) //.map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
        }
    }
}