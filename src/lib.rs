//! An XML-RPC implementation in Rust.
//!
//! The `xmlrpc` crate provides a minimal implementation of the [XML-RPC specification].
//!
//! [XML-RPC specification]: http://xmlrpc.scripting.com/spec.html

#![doc(html_root_url = "https://docs.rs/xmlrpc/0.11.1")]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

extern crate base64;
extern crate iso8601;
extern crate xml;

#[cfg(feature = "serde")]
#[allow(unused)]    // derive is only used by tests
#[macro_use]
extern crate serde;

#[cfg(feature = "serde")]
mod ser;
#[cfg(feature = "serde")]
mod de;
pub mod error;
mod parser;
mod request;
mod value;
mod utils;
mod transport;

// TODO (breaking): Remove `Fault` reexport as it's rarely needed and is in `error`, too
pub use error::{Error, Fault};
pub use request::Request;
pub use value::{Value, Index};
pub use transport::Transport;

#[cfg(feature = "reqwest")]
pub use transport::http;
