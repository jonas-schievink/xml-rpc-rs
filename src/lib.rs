//! An XML-RPC implementation in Rust.
//!
//! The `xmlrpc` crate provides a minimal implementation of the [XML-RPC specification].
//!
//! [XML-RPC specification]: http://xmlrpc.scripting.com/spec.html

#![doc(html_root_url = "https://docs.rs/xmlrpc/0.11.1")]
#![warn(missing_debug_implementations)]

extern crate base64;
extern crate iso8601;
extern crate xml;

mod error;
mod parser;
mod request;
mod value;
mod utils;
mod transport;

pub use error::{Error, Fault};
pub use request::Request;
pub use value::{Value, Index};
pub use transport::Transport;

#[cfg(feature = "reqwest")]
pub use transport::http;
