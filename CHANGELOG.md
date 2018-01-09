# Changelog

## Unreleased (0.10.0)

### Breaking changes

* Replace ad-hoc API with a `Transport` trait that can be implemented to change the way the request is sent
* Stricter checking of server headers
* Removed `From<Vec<u8>>` impl of `Value`

### New Features

* Make the `reqwest` dependency optional - you can opt out and define your own `Transport` instead
* Add `Request::call_url`, an easy to use helper that calls a `&str` URL without needing to depend on `reqwest` in downstream crates
* Add the `http` module, containing a few helper methods for writing custom reqwest-based `Transport`s

## <= 0.8.0

* The API slowly grew to expose more internals in order to accommodate more use cases
