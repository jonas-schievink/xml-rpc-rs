//! This example shows how to transmit a request with a custom HTTP header.

extern crate xmlrpc;
extern crate reqwest;

use xmlrpc::{Request, Transport};
use xmlrpc::http::{build_headers, check_response};

use reqwest::{Client, RequestBuilder};
use reqwest::header::Cookie;

use std::error::Error;

/// Custom transport that adds a cookie header.
struct MyTransport(RequestBuilder);

impl Transport for MyTransport {
    type Stream = reqwest::Response;

    fn transmit(mut self, request: &Request) -> Result<Self::Stream, Box<Error + Send + Sync>> {
        let mut body = Vec::new();
        request.write_as_xml(&mut body).expect("could not write request to buffer (this should never happen)");

        build_headers(&mut self.0, body.len() as u64);

        // Our custom header will be a `Cookie` header
        let mut cookie = Cookie::new();
        cookie.set("SESSION", "123abc");
        self.0.header(cookie);

        let response = self.0
            .body(body)
            .send()?;

        check_response(&response)?;

        Ok(response)
    }
}

fn main() {
    let request = Request::new("pow").arg(2).arg(8);

    let tp = MyTransport(Client::new().post("http://localhost/target"));
    let result = request.call(tp);

    println!("Result: {:?}", result);
}
