# XML-RPC for Rust

[![crates.io](https://img.shields.io/crates/v/xmlrpc.svg)](https://crates.io/crates/xmlrpc)
[![docs.rs](https://docs.rs/xmlrpc/badge.svg)](https://docs.rs/xmlrpc/)
[![Build Status](https://travis-ci.org/jonas-schievink/xml-rpc-rs.svg?branch=master)](https://travis-ci.org/jonas-schievink/xml-rpc-rs)

This crate provides a simple implementation of the [XML-RPC specification](http://xmlrpc.scripting.com/spec.html) in stable Rust using `xml-rs` and `reqwest`.

Please refer to the [changelog](CHANGELOG.md) to see what changed in the last releases.

## Usage

Start by adding an entry to your `Cargo.toml`:

```toml
[dependencies]
xmlrpc = "0.11.0"
```

Then import the crate into your Rust code:

```rust
extern crate xmlrpc;
```

See [`examples/client.rs`](examples/client.rs) for a small example which connects to a running Python XML-RPC server and calls a method.
