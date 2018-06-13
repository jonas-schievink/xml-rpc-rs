//! You can use this example by executing `python3 -m xmlrpc.server` and then running
//! `cargo run --example client`.

extern crate xmlrpc;

use xmlrpc::{Request, Value};

fn main() {
    // The Python example server exports Python's `pow` method. Let's call it!
    let pow_request = Request::new("pow").arg(2).arg(8);    // Compute 2**8

    let request_result = pow_request.call_url("http://127.0.0.1:8000");

    println!("Result: {:?}", request_result);

    let pow_result = request_result.unwrap();
    assert_eq!(pow_result, Value::Int(2i32.pow(8)));
}
