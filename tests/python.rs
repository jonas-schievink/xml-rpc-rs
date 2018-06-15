//! Tests communication with a python3 XML-RPC server.

extern crate xmlrpc;

use xmlrpc::{Request, Value, Fault};

use std::process::Command;

const URL: &'static str = "http://localhost:8000";

fn main() {
    let mut child = match Command::new("python3")
            .arg("-m")
            .arg("xmlrpc.server")
            .spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("could not start python XML-RPC server, ignoring python test ({})", e);
            return;
        },
    };

    match child.try_wait().unwrap() {
        None => {},     // still running
        Some(status) => {
            panic!("python process unexpectedly exited: {}", status);
        }
    }


    let pow = Request::new("pow").arg(2).arg(8).call_url(URL).unwrap();
    assert_eq!(pow.as_i64(), Some(2i64.pow(8)));

    // call with wrong operands should return a fault
    let err = Request::new("pow").arg(2).arg(2).arg("BLA").call_url(URL).unwrap_err();
    err.fault().expect("returned error was not a fault");

    // perform a multicall
    let result = Request::multicall(&[
        Request::new("pow").arg(2).arg(4),
        Request::new("add").arg(2).arg(4),
        Request::new("doesn't exist"),
    ]).call_url(URL).unwrap();
    // `result` now contains an array of results. on success, a 1-element array containing the
    // result is placed in the `result` array. on fault, the corresponding fault struct is used.
    let results = result.as_array().unwrap();
    assert_eq!(results[0], Value::Array(vec![Value::Int(16)]));
    assert_eq!(results[1], Value::Array(vec![Value::Int(6)]));
    Fault::from_value(&results[2]).expect("expected fault as third result");


    match child.try_wait().unwrap() {
        None => {},     // still running
        Some(status) => {
            panic!("python process unexpectedly exited: {}", status);
        }
    }

    child.kill().unwrap();
}
