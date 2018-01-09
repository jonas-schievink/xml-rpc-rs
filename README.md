# XML-RPC for Rust

[![crates.io](https://img.shields.io/crates/v/xmlrpc.svg)](https://crates.io/crates/xmlrpc)
[![docs.rs](https://docs.rs/xmlrpc/badge.svg)](https://docs.rs/xmlrpc/)
[![Build Status](https://travis-ci.org/jonas-schievink/xml-rpc-rs.svg?branch=master)](https://travis-ci.org/jonas-schievink/xml-rpc-rs)

This crate provides a simple implementation of the [XML-RPC spec](http://xmlrpc.scripting.com/spec.html) in pure Rust using `xml-rs` and `reqwest`.

See `examples/client.rs` for a small example which connects to a running Python XML-RPC server and calls a method.
