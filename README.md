# XML-RPC for Rust

[![crates.io](https://img.shields.io/crates/v/xmlrpc.svg)](https://crates.io/crates/xmlrpc)
[![docs.rs](https://docs.rs/xmlrpc/badge.svg)](https://docs.rs/xmlrpc/)
[![Build Status](https://travis-ci.org/jonas-schievink/xml-rpc-rs.svg?branch=master)](https://travis-ci.org/jonas-schievink/xml-rpc-rs)

This crate provides a simple implementation of the [XML-RPC specification](http://xmlrpc.scripting.com/spec.html) in stable Rust using `xml-rs` and `reqwest`.

Please refer to the [changelog](CHANGELOG.md) to see what changed in the last releases.

## Rust support

This crate uses the same Rust versioning policy as [tokio]: It supports the last
3 stable Rust releases. Increasing the minimum supported version is not
considered a breaking change as long as the latest 3 versions are still
supported.

## Usage

Start by adding an entry to your `Cargo.toml`:

```toml
[dependencies]
xmlrpc = "0.12.0"
```

Then import the crate into your Rust code:

```rust
extern crate xmlrpc;
```

See [`examples/client.rs`](examples/client.rs) for a small example which connects to a running Python XML-RPC server and calls a method. A more elaborate example that demonstrates how to implement a custom `Transport` to set a cookie header is provided in [`examples/custom-header.rs`](examples/custom-header.rs).

[tokio]: https://github.com/tokio-rs/tokio
