//! This example shows how to transmit a request with a custom HTTP header.

extern crate reqwest;
extern crate xmlrpc;

use futures::executor::block_on;
use xmlrpc::http::{build_headers, check_response};
use xmlrpc::{Request, Transport};

use reqwest::{Client, RequestBuilder};
use reqwest::header::COOKIE;

use std::error::Error;
use std::io::Cursor;

/// Custom transport that adds a cookie header.
struct MyTransport(RequestBuilder);

impl Transport for MyTransport {
    type Stream = Cursor<String>;

    fn transmit(self, request: &Request) -> Result<Self::Stream, Box<dyn Error + Send + Sync>> {
        let mut body = Vec::new();
        request
            .write_as_xml(&mut body)
            .expect("could not write request to buffer (this should never happen)");

        let response = async move {build_headers(self.0, body.len() as u64)
            .header(COOKIE, "SESSION=123abc") // Our custom header will be a `Cookie` header
            .body(body)
            .send().await.unwrap()};
        
        let resp = block_on(response);

        check_response(&resp)?;

        let rs = async move {resp.text().await.unwrap()};
        let rv = Cursor::new(block_on(rs));

        Ok(rv)
    }
}

fn main() {
    let request = Request::new("pow").arg(2).arg(8);

    let tp = MyTransport(Client::new().post("http://localhost/target"));
    let result = request.call(tp);

    println!("Result: {:?}", result);
}
