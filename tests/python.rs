//! Tests communication with a python3 XML-RPC server.

extern crate xmlrpc;

use xmlrpc::{Request, Value, Fault};

use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::net::TcpStream;

const PORT: u16 = 8000;
const URL: &'static str = "http://127.0.0.1:8000";

/// Kills a child process when dropped.
struct Reap(Child);

impl Drop for Reap {
    fn drop(&mut self) {
        // an error seems to mean that the process has already died, which we don't expect here
        self.0.kill().expect("process already died");
    }
}

fn setup() -> Result<Reap, ()> {
    let start = Instant::now();
    let mut child = match Command::new("python3")
        .arg("-m")
        .arg("xmlrpc.server")
        .spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("could not start python XML-RPC server, ignoring python test ({})", e);
            return Err(());
        },
    };

    // wait until someone listens on the port or the child dies
    let mut iteration = 0;
    loop {
        match child.try_wait().unwrap() {
            None => {}, // still running
            Some(status) => panic!("python process unexpectedly died: {}", status),
        }

        // try to connect to the server
        match TcpStream::connect(("127.0.0.1", PORT)) {
            Ok(_) => {
                // server should work now
                println!("connected to server after {:?} (iteration {})", Instant::now() - start, iteration);
                return Ok(Reap(child))
            },
            Err(_) => {},       // not yet ready
        }

        sleep(Duration::from_millis(50));

        iteration += 1;
    }
}

fn run_tests() {
    let pow = Request::new("pow").arg(2).arg(8).call_url(URL).unwrap();
    assert_eq!(pow.as_i64(), Some(2i64.pow(8)));

    // call with wrong operands should return a fault
    let err = Request::new("pow").arg(2).arg(2).arg("BLA").call_url(URL).unwrap_err();
    err.fault().expect("returned error was not a fault");

    // perform a multicall
    let result = Request::new_multicall(&[
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
}

fn main() {
    let mut reaper = match setup() {
        Ok(reap) => reap,
        Err(()) => return,
    };

    match reaper.0.try_wait().unwrap() {
        None => {},     // still running
        Some(status) => {
            panic!("python process unexpectedly exited: {}", status);
        }
    }

    run_tests();

    match reaper.0.try_wait().unwrap() {
        None => {},     // still running
        Some(status) => {
            panic!("python process unexpectedly exited: {}", status);
        }
    }
}
