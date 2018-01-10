//! An XML-RPC implementation in Rust.
//!
//! The `xmlrpc` crate provides a minimal implementation of the [XML-RPC spec][spec].
//!
//!
//! [spec]: http://xmlrpc.scripting.com/spec.html

extern crate base64;
extern crate iso8601;
extern crate xml;

mod error;
mod parser;
mod request;
mod value;
mod utils;
mod transport;

pub use error::{RequestError, Fault};
pub use request::{Request, RequestResult};
pub use value::Value;
pub use transport::Transport;

#[cfg(feature = "reqwest")]
pub use transport::http;
