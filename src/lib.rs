//! An XML-RPC implementation in Rust.
//!
//! The `xmlrpc` crate provides a minimal implementation of the [XML-RPC specification].
//!
//! [XML-RPC specification]: http://xmlrpc.scripting.com/spec.html

#![doc(html_root_url = "https://docs.rs/xmlrpc/0.13.1")]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

extern crate base64;
extern crate iso8601;
extern crate xml;

mod error;
mod parser;
mod request;
mod transport;
mod utils;
mod value;

pub use error::{Error, Fault};
pub use request::Request;
pub use transport::Transport;
pub use value::{Index, Value};

#[cfg(feature = "http")]
pub use transport::http;
